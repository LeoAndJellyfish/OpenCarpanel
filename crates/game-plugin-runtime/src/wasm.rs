use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    fs,
    path::Path,
};

use opensimdash_adapter_api::{
    AdapterDescriptor, AdapterError, AdapterId, AdapterOutput, GameAdapter,
};
use opensimdash_game_plugin_api::{
    GAME_PLUGIN_ABI_VERSION, GamePluginManifest, MAX_PLUGIN_DATAGRAM_BYTES,
    MAX_PLUGIN_MODULE_BYTES, MAX_PLUGIN_OUTPUT_BYTES, PluginDecodeOutput, PluginRuntime,
};
use opensimdash_telemetry_core::MonotonicTimestamp;
use serde_json::Value;
use wasmi::{
    Config, Engine, Instance, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder,
    TypedFunc,
};

const MAX_LINEAR_MEMORY_BYTES: usize = 4 * 1024 * 1024;
const MAX_TABLE_ELEMENTS: usize = 4_096;
const FUEL_PER_DECODE: u64 = 5_000_000;
const MAX_OUTPUT_JSON_DEPTH: usize = 32;
const MAX_OUTPUT_JSON_NODES: usize = 8_192;
const MAX_OUTPUT_STRING_BYTES: usize = 4_096;
const MAX_OUTPUT_KEY_BYTES: usize = 128;
const MAX_EVENT_NAME_BYTES: usize = 128;
const MAX_SESSION_ID_BYTES: usize = 256;
const MAX_WASM_ERROR_BYTES: usize = 512;

struct StoreData {
    limits: StoreLimits,
}

/// Sandboxed ABI v1 decoder exposed through the native [`GameAdapter`] contract.
pub struct WasmGameAdapter {
    descriptor: AdapterDescriptor,
    max_datagram_bytes: usize,
    store: Store<StoreData>,
    memory: Memory,
    input_ptr: TypedFunc<(), i32>,
    input_capacity: TypedFunc<(), i32>,
    output_ptr: TypedFunc<(), i32>,
    output_capacity: TypedFunc<(), i32>,
    decode: TypedFunc<(i32, i64), i32>,
}

impl Debug for WasmGameAdapter {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WasmGameAdapter")
            .field("descriptor", &self.descriptor)
            .field("max_datagram_bytes", &self.max_datagram_bytes)
            .finish_non_exhaustive()
    }
}

impl WasmGameAdapter {
    /// Loads and validates a sandboxed decoder from disk.
    ///
    /// # Errors
    ///
    /// Returns [`WasmPluginError`] for manifest, module, import, export, or ABI failures.
    pub fn from_file(
        manifest: &GamePluginManifest,
        module_path: &Path,
    ) -> Result<Self, WasmPluginError> {
        let metadata = fs::metadata(module_path).map_err(|error| wasm_error(error.to_string()))?;
        if !metadata.is_file()
            || metadata.len() == 0
            || metadata.len() > u64::try_from(MAX_PLUGIN_MODULE_BYTES).unwrap_or(u64::MAX)
        {
            return Err(wasm_error(
                "WASM module size is outside the supported range",
            ));
        }
        let bytes = fs::read(module_path).map_err(|error| wasm_error(error.to_string()))?;
        Self::from_bytes(manifest, &bytes)
    }

    /// Loads and validates a sandboxed decoder from verified bytes.
    ///
    /// # Errors
    ///
    /// Returns [`WasmPluginError`] for imports, missing exports, limits, or ABI mismatch.
    pub fn from_bytes(
        manifest: &GamePluginManifest,
        module_bytes: &[u8],
    ) -> Result<Self, WasmPluginError> {
        manifest
            .validate()
            .map_err(|error| wasm_error(error.to_string()))?;
        if !matches!(manifest.runtime, PluginRuntime::Wasm { .. }) {
            return Err(wasm_error("WasmGameAdapter requires a WASM manifest"));
        }
        if module_bytes.is_empty() || module_bytes.len() > MAX_PLUGIN_MODULE_BYTES {
            return Err(wasm_error(
                "WASM module size is outside the supported range",
            ));
        }

        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, module_bytes)
            .map_err(|error| wasm_error(format!("module validation failed: {error}")))?;
        if module.imports().next().is_some() {
            return Err(wasm_error(
                "WASM decoder must not import Host or WASI functions",
            ));
        }
        let limits = StoreLimitsBuilder::new()
            .memory_size(MAX_LINEAR_MEMORY_BYTES)
            .table_elements(MAX_TABLE_ELEMENTS)
            .instances(1)
            .memories(1)
            .tables(1)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(&engine, StoreData { limits });
        store.limiter(|data| &mut data.limits);
        store
            .set_fuel(FUEL_PER_DECODE)
            .map_err(|error| wasm_error(error.to_string()))?;
        let linker = Linker::new(&engine);
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .map_err(|error| wasm_error(format!("module instantiation failed: {error}")))?;
        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| wasm_error("WASM decoder does not export memory"))?;
        let abi_version = typed::<(), i32>(instance, &store, "osd_plugin_abi_version")?;
        let input_ptr = typed::<(), i32>(instance, &store, "osd_input_ptr")?;
        let input_capacity = typed::<(), i32>(instance, &store, "osd_input_capacity")?;
        let output_ptr = typed::<(), i32>(instance, &store, "osd_output_ptr")?;
        let output_capacity = typed::<(), i32>(instance, &store, "osd_output_capacity")?;
        let decode = typed::<(i32, i64), i32>(instance, &store, "osd_decode")?;
        let actual_abi = call(abi_version, &mut store, ())?;
        if actual_abi != i32::from(GAME_PLUGIN_ABI_VERSION) {
            return Err(wasm_error(format!(
                "WASM decoder ABI {actual_abi} is unsupported; expected {GAME_PLUGIN_ABI_VERSION}"
            )));
        }
        let descriptor = AdapterDescriptor::new(
            AdapterId::new(manifest.id.clone()).map_err(|error| wasm_error(error.to_string()))?,
            manifest.name.clone(),
            manifest.protocol.version.clone(),
            manifest.capabilities.iter().copied().collect(),
        );
        let max_datagram_bytes = usize::try_from(manifest.ingress.max_datagram_bytes)
            .unwrap_or(MAX_PLUGIN_DATAGRAM_BYTES as usize);
        Ok(Self {
            descriptor,
            max_datagram_bytes,
            store,
            memory,
            input_ptr,
            input_capacity,
            output_ptr,
            output_capacity,
            decode,
        })
    }

    fn decode_inner(
        &mut self,
        datagram: &[u8],
        received_at: MonotonicTimestamp,
        output: &mut AdapterOutput,
    ) -> Result<(), WasmPluginError> {
        if datagram.len() > self.max_datagram_bytes {
            return Err(wasm_error("datagram exceeds the plugin ingress limit"));
        }
        self.store
            .set_fuel(FUEL_PER_DECODE)
            .map_err(|error| wasm_error(error.to_string()))?;
        let input_ptr = non_negative(call(self.input_ptr, &mut self.store, ())?, "input pointer")?;
        let input_capacity = non_negative(
            call(self.input_capacity, &mut self.store, ())?,
            "input capacity",
        )?;
        if datagram.len() > input_capacity {
            return Err(wasm_error(
                "guest input buffer is smaller than the datagram",
            ));
        }
        self.memory
            .write(&mut self.store, input_ptr, datagram)
            .map_err(|_| wasm_error("guest input buffer is outside linear memory"))?;
        let input_len = i32::try_from(datagram.len())
            .map_err(|_| wasm_error("datagram length does not fit ABI v1"))?;
        let captured_at = i64::try_from(received_at.as_micros())
            .map_err(|_| wasm_error("Host timestamp does not fit ABI v1"))?;
        let result = call(self.decode, &mut self.store, (input_len, captured_at))?;
        if result == 0 {
            return Err(wasm_error("datagram was not recognized"));
        }
        if result < 0 {
            return Err(wasm_error("decoder rejected the recognized datagram"));
        }
        let output_len =
            usize::try_from(result).map_err(|_| wasm_error("decoder output length is invalid"))?;
        if output_len > MAX_PLUGIN_OUTPUT_BYTES {
            return Err(wasm_error("decoder output exceeds the 256 KiB limit"));
        }
        let output_ptr = non_negative(
            call(self.output_ptr, &mut self.store, ())?,
            "output pointer",
        )?;
        let output_capacity = non_negative(
            call(self.output_capacity, &mut self.store, ())?,
            "output capacity",
        )?;
        if output_len > output_capacity {
            return Err(wasm_error("decoder output exceeds its exported buffer"));
        }
        let mut encoded = vec![0_u8; output_len];
        self.memory
            .read(&self.store, output_ptr, &mut encoded)
            .map_err(|_| wasm_error("guest output buffer is outside linear memory"))?;
        let value: Value = serde_json::from_slice(&encoded)
            .map_err(|error| wasm_error(format!("decoder output JSON is invalid: {error}")))?;
        validate_json_shape(&value)?;
        let mut decoded: PluginDecodeOutput = serde_json::from_value(value)
            .map_err(|error| wasm_error(format!("decoder output contract is invalid: {error}")))?;
        decoded
            .validate_bounds()
            .map_err(|error| wasm_error(error.to_string()))?;
        validate_output_semantics(&decoded, self.descriptor.id.as_str())?;
        for update in &mut decoded.updates {
            update.received_at = received_at;
        }
        for event in &mut decoded.events {
            event.occurred_at = received_at;
        }
        output.updates.append(&mut decoded.updates);
        output.events.append(&mut decoded.events);
        Ok(())
    }
}

impl GameAdapter for WasmGameAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn decode(
        &mut self,
        datagram: &[u8],
        received_at: MonotonicTimestamp,
        output: &mut AdapterOutput,
    ) -> Result<(), AdapterError> {
        self.decode_inner(datagram, received_at, output)
            .map_err(|error| AdapterError::malformed_packet(error.to_string()))
    }
}

fn typed<Params, Results>(
    instance: Instance,
    store: &Store<StoreData>,
    name: &str,
) -> Result<TypedFunc<Params, Results>, WasmPluginError>
where
    Params: wasmi::WasmParams,
    Results: wasmi::WasmResults,
{
    instance.get_typed_func(store, name).map_err(|_| {
        wasm_error(format!(
            "WASM decoder export {name} has the wrong signature"
        ))
    })
}

fn call<Params, Results>(
    function: TypedFunc<Params, Results>,
    store: &mut Store<StoreData>,
    params: Params,
) -> Result<Results, WasmPluginError>
where
    Params: wasmi::WasmParams,
    Results: wasmi::WasmResults,
{
    function
        .call(store, params)
        .map_err(|error| wasm_error(format!("WASM decoder trapped: {error}")))
}

fn non_negative(value: i32, field: &str) -> Result<usize, WasmPluginError> {
    usize::try_from(value).map_err(|_| wasm_error(format!("guest {field} is negative")))
}

fn validate_json_shape(root: &Value) -> Result<(), WasmPluginError> {
    let mut pending = vec![(root, 1_usize)];
    let mut nodes = 0_usize;
    while let Some((value, depth)) = pending.pop() {
        nodes = nodes.saturating_add(1);
        if nodes > MAX_OUTPUT_JSON_NODES {
            return Err(wasm_error("decoder output contains too many JSON values"));
        }
        if depth > MAX_OUTPUT_JSON_DEPTH {
            return Err(wasm_error("decoder output JSON nesting is too deep"));
        }
        match value {
            Value::String(value) if value.len() > MAX_OUTPUT_STRING_BYTES => {
                return Err(wasm_error("decoder output contains an oversized string"));
            }
            Value::Array(values) => {
                pending.extend(values.iter().map(|value| (value, depth + 1)));
            }
            Value::Object(values) => {
                for (key, value) in values {
                    if key.len() > MAX_OUTPUT_KEY_BYTES {
                        return Err(wasm_error(
                            "decoder output contains an oversized object key",
                        ));
                    }
                    pending.push((value, depth + 1));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_output_semantics(
    output: &PluginDecodeOutput,
    plugin_id: &str,
) -> Result<(), WasmPluginError> {
    for update in &output.updates {
        if update.session_id.as_ref().is_some_and(|session_id| {
            session_id.is_empty() || session_id.len() > MAX_SESSION_ID_BYTES
        }) {
            return Err(wasm_error(
                "decoder session id is outside the supported range",
            ));
        }
        if update
            .extensions
            .keys()
            .any(|namespace| namespace != plugin_id)
        {
            return Err(wasm_error(
                "decoder extension values must use the plugin id as their namespace",
            ));
        }
    }
    for event in &output.events {
        if !valid_event_name(&event.name) {
            return Err(wasm_error("decoder event name is invalid"));
        }
    }
    Ok(())
}

fn valid_event_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_EVENT_NAME_BYTES
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

/// Sandboxed module validation or execution failure.
#[derive(Debug)]
pub struct WasmPluginError {
    message: String,
}

impl Display for WasmPluginError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for WasmPluginError {}

fn wasm_error(message: impl Into<String>) -> WasmPluginError {
    let mut message = message.into();
    if message.len() > MAX_WASM_ERROR_BYTES {
        let mut end = MAX_WASM_ERROR_BYTES.saturating_sub(3);
        while !message.is_char_boundary(end) {
            end = end.saturating_sub(1);
        }
        message.truncate(end);
        message.push_str("...");
    }
    WasmPluginError { message }
}

#[cfg(test)]
mod tests {
    use opensimdash_game_plugin_api::{PluginRuntime, parse_manifest};
    use opensimdash_telemetry_core::{TelemetryEvent, TelemetryUpdate};
    use serde_json::json;
    use sha2::Digest as _;

    use super::*;

    fn wasm_manifest(module: &[u8]) -> Result<GamePluginManifest, Box<dyn Error>> {
        let mut manifest =
            parse_manifest(include_bytes!("../../../plugins/games/f1-24/manifest.json"))?;
        manifest.id = "example-game".to_owned();
        manifest.runtime = PluginRuntime::Wasm {
            abi_version: GAME_PLUGIN_ABI_VERSION,
            module: "decoder.wasm".to_owned(),
            sha256: format!("{:x}", sha2::Sha256::digest(module)),
        };
        Ok(manifest)
    }

    #[test]
    fn runs_a_bounded_decoder_and_overwrites_guest_timestamps() -> Result<(), Box<dyn Error>> {
        let response = r#"{"schemaVersion":1,"updates":[{"receivedAt":1,"vehicle":{"speedMps":12.5}}],"events":[]}"#;
        let wat = format!(
            r#"(module
              (memory (export "memory") 6 64)
              (data (i32.const 65536) "{}")
              (func (export "osd_plugin_abi_version") (result i32) i32.const 1)
              (func (export "osd_input_ptr") (result i32) i32.const 0)
              (func (export "osd_input_capacity") (result i32) i32.const 65536)
              (func (export "osd_output_ptr") (result i32) i32.const 65536)
              (func (export "osd_output_capacity") (result i32) i32.const 262144)
              (func (export "osd_decode") (param i32 i64) (result i32) i32.const {})
            )"#,
            wat_escape(response),
            response.len(),
        );
        let module = wat::parse_str(wat)?;
        let manifest = wasm_manifest(&module)?;
        let mut adapter = WasmGameAdapter::from_bytes(&manifest, &module)?;
        let mut output = AdapterOutput::default();
        adapter.decode(b"packet", MonotonicTimestamp::from_micros(42), &mut output)?;
        assert_eq!(output.updates.len(), 1);
        assert_eq!(output.updates[0].received_at.as_micros(), 42);
        assert_eq!(output.updates[0].vehicle.speed_mps, Some(12.5));
        Ok(())
    }

    #[test]
    fn rejects_modules_with_imports() -> Result<(), Box<dyn Error>> {
        let module =
            wat::parse_str(r#"(module (import "wasi_snapshot_preview1" "fd_write" (func)))"#)?;
        let manifest = wasm_manifest(&module)?;
        let result = WasmGameAdapter::from_bytes(&manifest, &module);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn fuel_stops_a_decoder_that_never_returns() -> Result<(), Box<dyn Error>> {
        let module = wat::parse_str(
            r#"(module
              (memory (export "memory") 6 64)
              (func (export "osd_plugin_abi_version") (result i32) i32.const 1)
              (func (export "osd_input_ptr") (result i32) i32.const 0)
              (func (export "osd_input_capacity") (result i32) i32.const 65536)
              (func (export "osd_output_ptr") (result i32) i32.const 65536)
              (func (export "osd_output_capacity") (result i32) i32.const 262144)
              (func (export "osd_decode") (param i32 i64) (result i32)
                (loop $forever br $forever)
                i32.const 0)
            )"#,
        )?;
        let manifest = wasm_manifest(&module)?;
        let mut adapter = WasmGameAdapter::from_bytes(&manifest, &module)?;
        let result = adapter.decode(
            b"packet",
            MonotonicTimestamp::from_micros(42),
            &mut AdapterOutput::default(),
        );
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn rejects_a_module_whose_initial_memory_exceeds_the_limit() -> Result<(), Box<dyn Error>> {
        let module = wat::parse_str(
            r#"(module
              (memory (export "memory") 65 65)
              (func (export "osd_plugin_abi_version") (result i32) i32.const 1)
              (func (export "osd_input_ptr") (result i32) i32.const 0)
              (func (export "osd_input_capacity") (result i32) i32.const 65536)
              (func (export "osd_output_ptr") (result i32) i32.const 65536)
              (func (export "osd_output_capacity") (result i32) i32.const 262144)
              (func (export "osd_decode") (param i32 i64) (result i32) i32.const 0)
            )"#,
        )?;
        let manifest = wasm_manifest(&module)?;
        assert!(WasmGameAdapter::from_bytes(&manifest, &module).is_err());
        Ok(())
    }

    #[test]
    fn rejects_foreign_extension_namespaces_and_unsafe_event_names() {
        let mut update = TelemetryUpdate::default();
        update
            .extensions
            .insert("another-plugin".to_owned(), json!({"value": 1}));
        let output = PluginDecodeOutput {
            schema_version: GAME_PLUGIN_ABI_VERSION,
            updates: vec![update],
            events: Vec::new(),
        };
        assert!(validate_output_semantics(&output, "example-game").is_err());

        let output = PluginDecodeOutput {
            schema_version: GAME_PLUGIN_ABI_VERSION,
            updates: Vec::new(),
            events: vec![TelemetryEvent {
                name: "Unsafe Event".to_owned(),
                occurred_at: MonotonicTimestamp::from_micros(0),
                data: Value::Null,
            }],
        };
        assert!(validate_output_semantics(&output, "example-game").is_err());
    }

    #[test]
    fn rejects_deep_or_oversized_json_shapes() {
        let mut deep = Value::Null;
        for _ in 0..=MAX_OUTPUT_JSON_DEPTH {
            deep = json!([deep]);
        }
        assert!(validate_json_shape(&deep).is_err());
        assert!(
            validate_json_shape(&Value::String("x".repeat(MAX_OUTPUT_STRING_BYTES + 1))).is_err()
        );
    }

    #[test]
    fn runtime_errors_are_bounded_before_reaching_diagnostics() {
        let error = wasm_error("界".repeat(MAX_WASM_ERROR_BYTES));
        assert!(error.to_string().len() <= MAX_WASM_ERROR_BYTES);
        assert!(error.to_string().ends_with("..."));
    }

    fn wat_escape(value: &str) -> String {
        value
            .bytes()
            .map(|byte| match byte {
                b'"' => "\\22".to_owned(),
                b'\\' => "\\5c".to_owned(),
                0x20..=0x7e => char::from(byte).to_string(),
                _ => format!("\\{byte:02x}"),
            })
            .collect()
    }
}
