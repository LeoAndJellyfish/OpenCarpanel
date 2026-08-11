use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// Failure while decoding an F1 24 UDP datagram.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecodeError {
    /// The datagram ended before a field could be read.
    UnexpectedEnd {
        /// Byte offset at which the field begins.
        offset: usize,
        /// Number of bytes required for the field.
        needed: usize,
        /// Number of bytes still available.
        remaining: usize,
    },
    /// The packet identifies a different F1 game-year format.
    UnsupportedPacketFormat {
        /// Packet format accepted by this adapter.
        expected: u16,
        /// Packet format read from the datagram.
        actual: u16,
    },
}

impl Display for DecodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEnd {
                offset,
                needed,
                remaining,
            } => write!(
                formatter,
                "datagram ended at offset {offset}: needed {needed} bytes, {remaining} remain"
            ),
            Self::UnsupportedPacketFormat { expected, actual } => write!(
                formatter,
                "unsupported F1 packet format {actual}; expected {expected}"
            ),
        }
    }
}

impl Error for DecodeError {}
