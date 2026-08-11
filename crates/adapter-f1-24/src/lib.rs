//! F1 24 UDP decoding and normalization.

mod cursor;
mod error;
mod header;

use cursor::Cursor;

pub use error::DecodeError;
pub use header::{F1_24_PACKET_FORMAT, PACKET_HEADER_LEN, PacketHeader};

/// Stable identifier exposed by the F1 24 adapter.
pub const ADAPTER_ID: &str = "f1-24";
