//! ETS2 and ATS telemetry decoding for the `OpenSimDash` SCS SDK bridge.

mod adapter;
mod cursor;
mod error;
mod protocol;

use cursor::Cursor;

pub use adapter::{AtsAdapter, Ets2Adapter};
pub use error::DecodeError;
pub use protocol::{
    ATS_GAME_ID, BRIDGE_JOB_TEXT_LEN, BRIDGE_MAGIC, BRIDGE_PACKET_LEN, BRIDGE_PROTOCOL_V1,
    BRIDGE_PROTOCOL_VERSION, BRIDGE_V1_PACKET_LEN, BridgeGame, BridgeJob, BridgeLights,
    BridgePacket, ETS2_GAME_ID,
};

/// Stable identifier exposed by the Euro Truck Simulator 2 adapter.
pub const ETS2_ADAPTER_ID: &str = "ets2";

/// Stable identifier exposed by the American Truck Simulator adapter.
pub const ATS_ADAPTER_ID: &str = "ats";
