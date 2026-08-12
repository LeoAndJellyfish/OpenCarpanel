mod car_telemetry;

pub(crate) use car_telemetry::{CarTelemetrySample, decode_player_sample};

/// Packet id assigned to Car Telemetry by the F1 24/25 specifications.
pub const CAR_TELEMETRY_PACKET_ID: u8 = 6;

/// Car Telemetry packet version implemented for F1 24/25.
pub const CAR_TELEMETRY_PACKET_VERSION: u8 = 1;

/// Number of car entries in the F1 24 and original F1 25 Car Telemetry packet.
pub const F1_CAR_COUNT: usize = 22;

/// Packed byte length of one F1 24 or original F1 25 `CarTelemetryData` entry.
pub const CAR_TELEMETRY_DATA_LEN: usize = 60;

/// Packed byte length of one complete F1 24 or original F1 25 Car Telemetry datagram.
pub const CAR_TELEMETRY_PACKET_LEN: usize = 1_352;

/// Number of car entries in a 2026 Season Pack Car Telemetry packet.
pub const F1_25_2026_CAR_COUNT: usize = 24;

/// Packed byte length of one 2026 Season Pack `CarTelemetryData` entry.
pub const F1_25_2026_CAR_TELEMETRY_DATA_LEN: usize = 59;

/// Packed byte length of one complete 2026 Season Pack Car Telemetry datagram.
pub const F1_25_2026_CAR_TELEMETRY_PACKET_LEN: usize = 1_448;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CarTelemetryLayout {
    pub(crate) packet_format: u16,
    pub(crate) car_count: usize,
    pub(crate) car_telemetry_data_len: usize,
    pub(crate) packet_len: usize,
}
