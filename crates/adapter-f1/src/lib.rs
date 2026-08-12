//! Shared F1 24 and F1 25 UDP decoding and normalization.

mod adapter;
mod cursor;
mod error;
mod header;
mod mapping;
mod packets;

use cursor::Cursor;
use mapping::map_player_sample;
use packets::{CarTelemetrySample, decode_player_sample};

pub use adapter::{F1_24Adapter, F1_25Adapter};
pub use error::DecodeError;
pub use header::{PACKET_HEADER_LEN, PacketHeader};
pub use packets::{
    CAR_TELEMETRY_DATA_LEN, CAR_TELEMETRY_PACKET_ID, CAR_TELEMETRY_PACKET_LEN,
    CAR_TELEMETRY_PACKET_VERSION, F1_CAR_COUNT,
};

/// Stable identifier exposed by the F1 24 adapter.
pub const F1_24_ADAPTER_ID: &str = "f1-24";

/// Stable identifier exposed by the F1 25 adapter.
pub const F1_25_ADAPTER_ID: &str = "f1-25";

/// Packet format emitted by F1 24.
pub const F1_24_PACKET_FORMAT: u16 = 2024;

/// Original packet format emitted by F1 25 when UDP mode is set to F1 25.
pub const F1_25_PACKET_FORMAT: u16 = 2025;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct F1Protocol {
    adapter_id: &'static str,
    display_name: &'static str,
    protocol_version: &'static str,
    packet_format: u16,
}

const F1_24_PROTOCOL: F1Protocol = F1Protocol {
    adapter_id: F1_24_ADAPTER_ID,
    display_name: "EA Sports F1 24",
    protocol_version: "2024/v27.2x",
    packet_format: F1_24_PACKET_FORMAT,
};

const F1_25_PROTOCOL: F1Protocol = F1Protocol {
    adapter_id: F1_25_ADAPTER_ID,
    display_name: "EA Sports F1 25",
    protocol_version: "2025/v3",
    packet_format: F1_25_PACKET_FORMAT,
};

/// Decodes one F1 24 Car Telemetry datagram into a canonical player update.
///
/// # Errors
///
/// Returns [`DecodeError`] for a malformed header, packet version, length,
/// player index, ratio, or enum value.
pub fn decode_f1_24_player_car_telemetry(
    datagram: &[u8],
    received_at: opencarpanel_telemetry_core::MonotonicTimestamp,
) -> Result<opencarpanel_telemetry_core::TelemetryUpdate, DecodeError> {
    decode_player_car_telemetry(datagram, received_at, F1_24_PROTOCOL)
}

/// Decodes one F1 25 Car Telemetry datagram into a canonical player update.
///
/// # Errors
///
/// Returns [`DecodeError`] for a malformed header, packet version, length,
/// player index, ratio, or enum value.
pub fn decode_f1_25_player_car_telemetry(
    datagram: &[u8],
    received_at: opencarpanel_telemetry_core::MonotonicTimestamp,
) -> Result<opencarpanel_telemetry_core::TelemetryUpdate, DecodeError> {
    decode_player_car_telemetry(datagram, received_at, F1_25_PROTOCOL)
}

fn decode_player_car_telemetry(
    datagram: &[u8],
    received_at: opencarpanel_telemetry_core::MonotonicTimestamp,
    protocol: F1Protocol,
) -> Result<opencarpanel_telemetry_core::TelemetryUpdate, DecodeError> {
    let (header, payload) = PacketHeader::decode(datagram, protocol.packet_format)?;
    let sample = decode_player_sample(&header, payload, datagram.len())?;
    map_player_sample(&header, sample, received_at)
}
