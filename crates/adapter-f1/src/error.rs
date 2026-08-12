use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// Failure while decoding an F1 UDP datagram.
#[derive(Debug, Clone, PartialEq)]
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
    /// The packet type has a version this adapter does not implement.
    UnsupportedPacketVersion {
        /// Packet type identifier.
        packet_id: u8,
        /// Version accepted by this adapter.
        expected: u8,
        /// Version read from the datagram.
        actual: u8,
    },
    /// A packet-specific decoder received the wrong packet id.
    UnexpectedPacketId {
        /// Packet id accepted by the decoder.
        expected: u8,
        /// Packet id read from the header.
        actual: u8,
    },
    /// The fixed-size packet does not match the official packed layout.
    InvalidPacketLength {
        /// Packet type identifier.
        packet_id: u8,
        /// Official packed packet length.
        expected: usize,
        /// Received datagram length.
        actual: usize,
    },
    /// The player car index lies outside the official car array.
    InvalidPlayerIndex {
        /// Index read from the common header.
        index: u8,
        /// Number of entries in the car array.
        car_count: usize,
    },
    /// A ratio field is non-finite or outside the inclusive unit interval.
    InvalidNormalizedValue {
        /// Official field name without the `m_` prefix.
        field: &'static str,
        /// Rejected value.
        value: f32,
    },
    /// An integer is not a documented member of an enum-like field.
    InvalidEnumValue {
        /// Official field name without the `m_` prefix.
        field: &'static str,
        /// Rejected numeric value.
        actual: u8,
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
            Self::UnsupportedPacketVersion {
                packet_id,
                expected,
                actual,
            } => write!(
                formatter,
                "unsupported version {actual} for packet {packet_id}; expected {expected}"
            ),
            Self::UnexpectedPacketId { expected, actual } => {
                write!(
                    formatter,
                    "unexpected packet id {actual}; expected {expected}"
                )
            }
            Self::InvalidPacketLength {
                packet_id,
                expected,
                actual,
            } => write!(
                formatter,
                "invalid packet {packet_id} length {actual}; expected {expected}"
            ),
            Self::InvalidPlayerIndex { index, car_count } => write!(
                formatter,
                "player car index {index} is outside the {car_count}-entry car array"
            ),
            Self::InvalidNormalizedValue { field, value } => write!(
                formatter,
                "invalid {field} value {value}; expected a finite value in 0.0..=1.0"
            ),
            Self::InvalidEnumValue { field, actual } => {
                write!(formatter, "invalid {field} enum value {actual}")
            }
        }
    }
}

impl Error for DecodeError {}
