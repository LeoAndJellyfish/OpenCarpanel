use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// Failure while decoding one `OpenSimDash` SCS bridge datagram.
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
    /// The packet does not carry the `OpenSimDash` SCS bridge signature.
    UnsupportedMagic {
        /// Four bytes observed at the start of the packet.
        actual: [u8; 4],
    },
    /// The bridge packet uses an unsupported protocol version.
    UnsupportedVersion {
        /// Version accepted by this build.
        expected: u8,
        /// Version observed in the packet.
        actual: u8,
    },
    /// The concrete adapter was given another SCS game identifier.
    UnsupportedGame {
        /// Game identifier accepted by the adapter.
        expected: u8,
        /// Game identifier observed in the packet.
        actual: u8,
    },
    /// The bridge packet is not the exact length for its version.
    InvalidLength {
        /// Required datagram length for the decoded version.
        expected: usize,
        /// Received datagram length.
        actual: usize,
    },
    /// A header flags or reserved field was non-zero.
    UnsupportedFlags {
        /// Rejected flags byte.
        flags: u8,
        /// Rejected reserved byte.
        reserved: u8,
    },
    /// A v2 light bit outside the defined mask was set.
    UnsupportedLightBits {
        /// Rejected light bit mask.
        bits: u16,
    },
    /// A v2 state bit outside the defined mask was set.
    UnsupportedStateBits {
        /// Rejected state bit mask.
        bits: u16,
    },
    /// A floating-point measurement was not finite.
    NonFiniteValue {
        /// Stable field name.
        field: &'static str,
    },
    /// A measurement that must be non-negative was negative.
    NegativeValue {
        /// Stable field name.
        field: &'static str,
        /// Rejected value.
        value: f32,
    },
    /// An RPM field was negative or outside the canonical u16 range.
    InvalidRpm {
        /// Stable field name.
        field: &'static str,
        /// Rejected value.
        value: f32,
    },
    /// A pedal field was outside the inclusive unit interval.
    InvalidNormalizedValue {
        /// Stable field name.
        field: &'static str,
        /// Rejected value.
        value: f32,
    },
    /// A fixed-width v2 job label was not valid UTF-8.
    InvalidUtf8 {
        /// Stable field name.
        field: &'static str,
    },
    /// A fixed-width v2 job label was not NUL-terminated and zero-padded.
    InvalidTextPadding {
        /// Stable field name.
        field: &'static str,
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
            Self::UnsupportedMagic { actual } => {
                write!(formatter, "unsupported SCS bridge magic {actual:02x?}")
            }
            Self::UnsupportedVersion { expected, actual } => write!(
                formatter,
                "unsupported SCS bridge version {actual}; expected {expected}"
            ),
            Self::UnsupportedGame { expected, actual } => write!(
                formatter,
                "unsupported SCS bridge game id {actual}; expected {expected}"
            ),
            Self::InvalidLength { expected, actual } => write!(
                formatter,
                "invalid SCS bridge packet length {actual}; expected {expected}"
            ),
            Self::UnsupportedFlags { flags, reserved } => write!(
                formatter,
                "unsupported SCS bridge flags {flags:#04x} or reserved byte {reserved:#04x}"
            ),
            Self::UnsupportedLightBits { bits } => {
                write!(formatter, "unsupported SCS bridge light bits {bits:#06x}")
            }
            Self::UnsupportedStateBits { bits } => {
                write!(formatter, "unsupported SCS bridge state bits {bits:#06x}")
            }
            Self::NonFiniteValue { field } => {
                write!(formatter, "SCS bridge field {field} must be finite")
            }
            Self::NegativeValue { field, value } => write!(
                formatter,
                "SCS bridge field {field} must be non-negative, got {value}"
            ),
            Self::InvalidRpm { field, value } => write!(
                formatter,
                "invalid SCS bridge {field} value {value}; expected 0..=65535"
            ),
            Self::InvalidNormalizedValue { field, value } => write!(
                formatter,
                "invalid SCS bridge {field} value {value}; expected 0.0..=1.0"
            ),
            Self::InvalidUtf8 { field } => {
                write!(formatter, "SCS bridge field {field} must be valid UTF-8")
            }
            Self::InvalidTextPadding { field } => write!(
                formatter,
                "SCS bridge field {field} must be NUL-terminated and zero-padded"
            ),
        }
    }
}

impl Error for DecodeError {}
