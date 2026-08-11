//! F1 24 UDP decoding and normalization.

mod adapter;
mod cursor;
mod error;
mod header;
mod mapping;
mod packets;

use cursor::Cursor;
use mapping::map_player_sample;
use packets::{CarTelemetrySample, decode_player_sample};

pub use adapter::{F1_24Adapter, decode_player_car_telemetry};
pub use error::DecodeError;
pub use header::{F1_24_PACKET_FORMAT, PACKET_HEADER_LEN, PacketHeader};
pub use packets::{
    CAR_TELEMETRY_DATA_LEN, CAR_TELEMETRY_PACKET_ID, CAR_TELEMETRY_PACKET_LEN,
    CAR_TELEMETRY_PACKET_VERSION, F1_24_CAR_COUNT,
};

/// Stable identifier exposed by the F1 24 adapter.
pub const ADAPTER_ID: &str = "f1-24";
