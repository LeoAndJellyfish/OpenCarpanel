//! Minimal six-byte UDP decoder used by the game plugin development guide.

use opensimdash_game_plugin_api::PluginDecodeOutput;
use opensimdash_game_plugin_sdk::{DecodeResult, GamePlugin, export_game_plugin};
use opensimdash_telemetry_core::{MonotonicTimestamp, TelemetryUpdate};

const MAGIC: &[u8; 4] = b"OSD1";
const PACKET_BYTES: usize = 6;

#[derive(Debug, Default)]
struct ExamplePlugin;

impl GamePlugin for ExamplePlugin {
    fn decode(&mut self, datagram: &[u8], received_at_us: u64) -> DecodeResult {
        if datagram.get(..MAGIC.len()) != Some(MAGIC) {
            return DecodeResult::Unrecognized;
        }
        if datagram.len() != PACKET_BYTES {
            return DecodeResult::Invalid;
        }

        let speed_kmh = u16::from_le_bytes([datagram[4], datagram[5]]);
        let mut update = TelemetryUpdate {
            received_at: MonotonicTimestamp::from_micros(received_at_us),
            ..TelemetryUpdate::default()
        };
        update.vehicle.speed_mps = Some(f32::from(speed_kmh) / 3.6);
        DecodeResult::Recognized(PluginDecodeOutput {
            schema_version: 1,
            updates: vec![update],
            events: Vec::new(),
        })
    }
}

export_game_plugin!(ExamplePlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_only_the_example_packet_and_normalizes_speed() -> Result<(), &'static str> {
        let mut plugin = ExamplePlugin;
        assert_eq!(plugin.decode(b"other", 1), DecodeResult::Unrecognized);
        assert_eq!(plugin.decode(b"OSD1", 1), DecodeResult::Invalid);

        let DecodeResult::Recognized(output) = plugin.decode(b"OSD1\x68\x01", 42) else {
            return Err("example packet was not recognized");
        };
        assert_eq!(output.updates.len(), 1);
        assert_eq!(output.updates[0].received_at.as_micros(), 42);
        assert_eq!(output.updates[0].vehicle.speed_mps, Some(100.0));
        Ok(())
    }
}
