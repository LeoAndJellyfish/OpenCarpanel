//! Bounded capture format and deterministic UDP replay primitives.

mod format;
mod replay;

pub use format::{
    CaptureError, CaptureHeader, CaptureReader, CaptureRecord, CaptureWriter, MAX_CAPTURE_BYTES,
    MAX_DATAGRAM_LEN,
};
pub use replay::{
    DatagramSink, ReplayClock, ReplayError, SystemReplayClock, replay_records, replay_stream,
};
