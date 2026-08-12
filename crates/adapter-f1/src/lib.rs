//! Shared F1 24 and F1 25 UDP decoding and normalization, including the
//! F1 25 2026 Season Pack wire format.

mod adapter;
mod cursor;
mod error;
mod header;
mod mapping;
mod packets;

use cursor::Cursor;
use mapping::map_player_sample;
use packets::{CarTelemetryLayout, CarTelemetrySample, decode_player_sample};

pub use adapter::{F1_24Adapter, F1_25Adapter};
pub use error::DecodeError;
pub use header::{PACKET_HEADER_LEN, PacketHeader};
pub use packets::{
    CAR_TELEMETRY_DATA_LEN, CAR_TELEMETRY_PACKET_ID, CAR_TELEMETRY_PACKET_LEN,
    CAR_TELEMETRY_PACKET_VERSION, F1_25_2026_CAR_COUNT, F1_25_2026_CAR_TELEMETRY_DATA_LEN,
    F1_25_2026_CAR_TELEMETRY_PACKET_LEN, F1_CAR_COUNT,
};

/// Stable identifier exposed by the F1 24 adapter.
pub const F1_24_ADAPTER_ID: &str = "f1-24";

/// Stable identifier exposed by the F1 25 adapter.
pub const F1_25_ADAPTER_ID: &str = "f1-25";

/// Packet format emitted by F1 24.
pub const F1_24_PACKET_FORMAT: u16 = 2024;

/// Original packet format emitted by F1 25 when UDP mode is set to F1 25.
pub const F1_25_PACKET_FORMAT: u16 = 2025;

/// Packet format emitted by F1 25 when UDP mode is set to 2026 Season Pack.
pub const F1_25_2026_PACKET_FORMAT: u16 = 2026;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct F1Protocol {
    adapter_id: &'static str,
    display_name: &'static str,
    protocol_version: &'static str,
    packet_formats: &'static [u16],
    car_telemetry_layouts: &'static [CarTelemetryLayout],
}

const F1_24_PACKET_FORMATS: &[u16] = &[F1_24_PACKET_FORMAT];
const F1_25_PACKET_FORMATS: &[u16] = &[F1_25_PACKET_FORMAT, F1_25_2026_PACKET_FORMAT];

const F1_24_CAR_TELEMETRY_LAYOUTS: &[CarTelemetryLayout] = &[CarTelemetryLayout {
    packet_format: F1_24_PACKET_FORMAT,
    car_count: F1_CAR_COUNT,
    car_telemetry_data_len: CAR_TELEMETRY_DATA_LEN,
    packet_len: CAR_TELEMETRY_PACKET_LEN,
}];

const F1_25_CAR_TELEMETRY_LAYOUTS: &[CarTelemetryLayout] = &[
    CarTelemetryLayout {
        packet_format: F1_25_PACKET_FORMAT,
        car_count: F1_CAR_COUNT,
        car_telemetry_data_len: CAR_TELEMETRY_DATA_LEN,
        packet_len: CAR_TELEMETRY_PACKET_LEN,
    },
    CarTelemetryLayout {
        packet_format: F1_25_2026_PACKET_FORMAT,
        car_count: F1_25_2026_CAR_COUNT,
        car_telemetry_data_len: F1_25_2026_CAR_TELEMETRY_DATA_LEN,
        packet_len: F1_25_2026_CAR_TELEMETRY_PACKET_LEN,
    },
];

const F1_24_PROTOCOL: F1Protocol = F1Protocol {
    adapter_id: F1_24_ADAPTER_ID,
    display_name: "EA Sports F1 24",
    protocol_version: "2024/v27.2x",
    packet_formats: F1_24_PACKET_FORMATS,
    car_telemetry_layouts: F1_24_CAR_TELEMETRY_LAYOUTS,
};

const F1_25_PROTOCOL: F1Protocol = F1Protocol {
    adapter_id: F1_25_ADAPTER_ID,
    display_name: "EA Sports F1 25",
    protocol_version: "2025/v3 + 2026/v10",
    packet_formats: F1_25_PACKET_FORMATS,
    car_telemetry_layouts: F1_25_CAR_TELEMETRY_LAYOUTS,
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
    let (header, payload, layout) = decode_header_and_layout(datagram, protocol)?;
    let sample = decode_player_sample(&header, payload, datagram.len(), layout)?;
    map_player_sample(&header, sample, received_at)
}

fn decode_header_and_layout(
    datagram: &[u8],
    protocol: F1Protocol,
) -> Result<(PacketHeader, &[u8], CarTelemetryLayout), DecodeError> {
    let (header, payload) = PacketHeader::decode_any_format(datagram)?;
    let layout = protocol
        .car_telemetry_layouts
        .iter()
        .copied()
        .find(|layout| layout.packet_format == header.packet_format)
        .ok_or({
            if let [expected] = protocol.packet_formats {
                DecodeError::UnsupportedPacketFormat {
                    expected: *expected,
                    actual: header.packet_format,
                }
            } else {
                DecodeError::UnsupportedPacketFormats {
                    expected: protocol.packet_formats,
                    actual: header.packet_format,
                }
            }
        })?;
    Ok((header, payload, layout))
}
