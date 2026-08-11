mod car_telemetry;

pub(crate) use car_telemetry::{CarTelemetrySample, decode_player_sample};

/// Packet id assigned to Car Telemetry by the F1 24 specification.
pub const CAR_TELEMETRY_PACKET_ID: u8 = 6;

/// Car Telemetry packet version in specification v27.2x.
pub const CAR_TELEMETRY_PACKET_VERSION: u8 = 1;

/// Number of car entries in a Car Telemetry packet.
pub const F1_24_CAR_COUNT: usize = 22;

/// Packed byte length of one `CarTelemetryData` entry.
pub const CAR_TELEMETRY_DATA_LEN: usize = 60;

/// Packed byte length of one complete `PacketCarTelemetryData` datagram.
pub const CAR_TELEMETRY_PACKET_LEN: usize = 1_352;
