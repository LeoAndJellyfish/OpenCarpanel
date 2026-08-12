//! ETS2 and ATS telemetry decoding for the `OpenCarpanel` SCS SDK bridge.

mod adapter;
mod cursor;
mod error;
mod protocol;

use cursor::Cursor;

pub use adapter::{AtsAdapter, Ets2Adapter};
pub use error::DecodeError;
pub use protocol::{
    ATS_GAME_ID, BRIDGE_MAGIC, BRIDGE_PACKET_LEN, BRIDGE_PROTOCOL_VERSION, BridgeGame,
    BridgePacket, ETS2_GAME_ID,
};

/// Stable identifier exposed by the Euro Truck Simulator 2 adapter.
pub const ETS2_ADAPTER_ID: &str = "ets2";

/// Stable identifier exposed by the American Truck Simulator adapter.
pub const ATS_ADAPTER_ID: &str = "ats";
