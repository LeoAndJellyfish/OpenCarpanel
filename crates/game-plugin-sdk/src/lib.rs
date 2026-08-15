//! Guest-side helpers for implementing an `OpenCarpanel` ABI v1 decoder in Rust.
//!
//! The [`export_game_plugin!`] macro owns fixed input/output buffers and exports
//! the complete core-WASM ABI without granting WASI or Host imports.

use opencarpanel_game_plugin_api::{
    GAME_PLUGIN_ABI_VERSION, MAX_PLUGIN_DATAGRAM_BYTES, MAX_PLUGIN_OUTPUT_BYTES, PluginDecodeOutput,
};

pub use opencarpanel_game_plugin_api;
pub use opencarpanel_telemetry_core;

/// Result of decoding one Host-owned input datagram.
#[derive(Debug, Clone, PartialEq)]
pub enum DecodeResult {
    /// Bytes belong to another plugin/protocol.
    Unrecognized,
    /// Bytes match this protocol but violate its contract.
    Invalid,
    /// Valid canonical updates and events.
    Recognized(PluginDecodeOutput),
}

/// Stateful decoder implemented by a game plugin.
pub trait GamePlugin: Default {
    /// Inspects one datagram and emits bounded canonical telemetry.
    fn decode(&mut self, datagram: &[u8], received_at_us: u64) -> DecodeResult;
}

/// Fixed guest input capacity used by the exported ABI.
pub const INPUT_CAPACITY: usize = MAX_PLUGIN_DATAGRAM_BYTES as usize;
/// Fixed guest output capacity used by the exported ABI.
pub const OUTPUT_CAPACITY: usize = MAX_PLUGIN_OUTPUT_BYTES;

/// Serializes one recognized response into the fixed guest output buffer.
///
/// This function is public for native unit tests; plugin modules normally call
/// it through [`export_game_plugin!`].
#[must_use]
pub fn encode_result(result: DecodeResult, output: &mut [u8]) -> i32 {
    match result {
        DecodeResult::Unrecognized => 0,
        DecodeResult::Invalid => -1,
        DecodeResult::Recognized(mut decoded) => {
            decoded.schema_version = GAME_PLUGIN_ABI_VERSION;
            if decoded.validate_bounds().is_err() {
                return -2;
            }
            let Ok(encoded) = serde_json::to_vec(&decoded) else {
                return -3;
            };
            if encoded.len() > output.len() {
                return -4;
            }
            output[..encoded.len()].copy_from_slice(&encoded);
            i32::try_from(encoded.len()).unwrap_or(-4)
        }
    }
}

/// Exports one [`GamePlugin`] implementation as an `OpenCarpanel` core-WASM module.
///
/// The generated module has no imports. Build it for `wasm32-unknown-unknown`
/// with `crate-type = ["cdylib"]` and `panic = "abort"`.
#[macro_export]
macro_rules! export_game_plugin {
    ($plugin:ty) => {
        std::thread_local! {
            static OCP_PLUGIN: std::cell::RefCell<$plugin> =
                std::cell::RefCell::new(<$plugin as std::default::Default>::default());
            static OCP_INPUT: std::cell::RefCell<std::boxed::Box<[u8]>> =
                std::cell::RefCell::new(vec![0_u8; $crate::INPUT_CAPACITY].into_boxed_slice());
            static OCP_OUTPUT: std::cell::RefCell<std::boxed::Box<[u8]>> =
                std::cell::RefCell::new(vec![0_u8; $crate::OUTPUT_CAPACITY].into_boxed_slice());
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn ocp_plugin_abi_version() -> i32 {
            i32::from($crate::opencarpanel_game_plugin_api::GAME_PLUGIN_ABI_VERSION)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn ocp_input_ptr() -> i32 {
            OCP_INPUT.with(|buffer| buffer.borrow_mut().as_mut_ptr() as i32)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn ocp_input_capacity() -> i32 {
            i32::try_from($crate::INPUT_CAPACITY).unwrap_or(0)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn ocp_output_ptr() -> i32 {
            OCP_OUTPUT.with(|buffer| buffer.borrow_mut().as_mut_ptr() as i32)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn ocp_output_capacity() -> i32 {
            i32::try_from($crate::OUTPUT_CAPACITY).unwrap_or(0)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn ocp_decode(input_len: i32, received_at_us: i64) -> i32 {
            let Ok(input_len) = usize::try_from(input_len) else {
                return -5;
            };
            let Ok(received_at_us) = u64::try_from(received_at_us) else {
                return -5;
            };
            if input_len > $crate::INPUT_CAPACITY {
                return -5;
            }
            OCP_INPUT.with(|input| {
                OCP_PLUGIN.with(|plugin| {
                    let input = input.borrow();
                    let result = <$plugin as $crate::GamePlugin>::decode(
                        &mut *plugin.borrow_mut(),
                        &input[..input_len],
                        received_at_us,
                    );
                    OCP_OUTPUT
                        .with(|output| $crate::encode_result(result, &mut output.borrow_mut()))
                })
            })
        }
    };
}

#[cfg(test)]
mod tests {
    use opencarpanel_game_plugin_api::PluginDecodeOutput;

    use super::*;

    #[test]
    fn encodes_recognized_and_control_results_without_allocation_contract_leaks() {
        let mut output = vec![0_u8; 1_024];
        assert_eq!(encode_result(DecodeResult::Unrecognized, &mut output), 0);
        assert_eq!(encode_result(DecodeResult::Invalid, &mut output), -1);
        let length = encode_result(
            DecodeResult::Recognized(PluginDecodeOutput::default()),
            &mut output,
        );
        assert!(length > 0);
        let value: serde_json::Value =
            serde_json::from_slice(&output[..usize::try_from(length).unwrap_or_default()])
                .unwrap_or_default();
        assert_eq!(value["schemaVersion"], GAME_PLUGIN_ABI_VERSION);
    }
}
