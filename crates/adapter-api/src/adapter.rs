use std::{
    collections::BTreeSet,
    error::Error,
    fmt::{self, Display, Formatter},
    iter::FromIterator,
};

use opensimdash_telemetry_core::{
    MonotonicTimestamp, TelemetryEvent, TelemetryField, TelemetryUpdate,
};

const MAX_ADAPTER_ID_LEN: usize = 64;

/// Stable lowercase identifier for a game adapter.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdapterId(String);

impl AdapterId {
    /// Creates an adapter identifier from a lowercase ASCII slug.
    ///
    /// Valid identifiers contain lowercase letters, digits, and single internal hyphens.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterIdError`] when the identifier is empty, longer than 64 bytes,
    /// contains unsupported characters, or has a leading, trailing, or repeated hyphen.
    pub fn new(value: impl Into<String>) -> Result<Self, AdapterIdError> {
        let value = value.into();
        if is_valid_adapter_id(&value) {
            Ok(Self(value))
        } else {
            Err(AdapterIdError { value })
        }
    }

    /// Returns the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_valid_adapter_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.len() > MAX_ADAPTER_ID_LEN {
        return false;
    }

    let is_alphanumeric = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    if !is_alphanumeric(bytes[0]) || !is_alphanumeric(bytes[bytes.len() - 1]) {
        return false;
    }

    let mut previous_was_hyphen = false;
    for &byte in bytes {
        if is_alphanumeric(byte) {
            previous_was_hyphen = false;
        } else if byte == b'-' && !previous_was_hyphen {
            previous_was_hyphen = true;
        } else {
            return false;
        }
    }

    true
}

/// Error returned for an invalid [`AdapterId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterIdError {
    value: String,
}

impl AdapterIdError {
    /// Returns the rejected identifier.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl Display for AdapterIdError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid adapter id: {:?}", self.value)
    }
}

impl Error for AdapterIdError {}

/// Ordered set of canonical telemetry fields supplied by an adapter.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CapabilitySet {
    fields: BTreeSet<TelemetryField>,
}

impl CapabilitySet {
    /// Creates an empty capability set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fields: BTreeSet::new(),
        }
    }

    /// Inserts a field, returning whether it was newly inserted.
    pub fn insert(&mut self, field: TelemetryField) -> bool {
        self.fields.insert(field)
    }

    /// Returns whether the adapter supplies `field`.
    #[must_use]
    pub fn contains(&self, field: TelemetryField) -> bool {
        self.fields.contains(&field)
    }

    /// Returns capabilities in deterministic field order.
    #[must_use]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &TelemetryField> + DoubleEndedIterator {
        self.fields.iter()
    }

    /// Returns the number of distinct capabilities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Returns whether no capabilities are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

impl FromIterator<TelemetryField> for CapabilitySet {
    fn from_iter<T: IntoIterator<Item = TelemetryField>>(iter: T) -> Self {
        Self {
            fields: iter.into_iter().collect(),
        }
    }
}

impl<const N: usize> From<[TelemetryField; N]> for CapabilitySet {
    fn from(fields: [TelemetryField; N]) -> Self {
        fields.into_iter().collect()
    }
}

/// Static identity and capabilities of one game adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterDescriptor {
    /// Stable machine-readable identifier.
    pub id: AdapterId,
    /// Human-readable game or adapter name.
    pub display_name: String,
    /// Game telemetry protocol version understood by the adapter.
    pub protocol_version: String,
    /// Canonical fields supplied by the adapter.
    pub capabilities: CapabilitySet,
}

impl AdapterDescriptor {
    /// Creates an adapter descriptor.
    #[must_use]
    pub fn new(
        id: AdapterId,
        display_name: impl Into<String>,
        protocol_version: impl Into<String>,
        capabilities: CapabilitySet,
    ) -> Self {
        Self {
            id,
            display_name: display_name.into(),
            protocol_version: protocol_version.into(),
            capabilities,
        }
    }
}

/// Reusable output buffers filled while decoding one datagram.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AdapterOutput {
    /// Partial state updates emitted by the decoded datagram.
    pub updates: Vec<TelemetryUpdate>,
    /// Discrete events emitted by the decoded datagram.
    pub events: Vec<TelemetryEvent>,
}

impl AdapterOutput {
    /// Creates reusable output buffers with independent capacities.
    #[must_use]
    pub fn with_capacity(update_capacity: usize, event_capacity: usize) -> Self {
        Self {
            updates: Vec::with_capacity(update_capacity),
            events: Vec::with_capacity(event_capacity),
        }
    }

    /// Removes decoded values while retaining allocated storage for the next datagram.
    pub fn clear(&mut self) {
        self.updates.clear();
        self.events.clear();
    }
}

/// Failure returned while validating adapter configuration or decoding a datagram.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdapterError {
    /// The datagram does not conform to the selected game protocol.
    MalformedPacket {
        /// Short diagnostic reason that does not contain raw packet data.
        reason: String,
    },
    /// The datagram uses a protocol version this adapter cannot decode.
    UnsupportedProtocol {
        /// Version accepted by the adapter.
        expected: String,
        /// Version observed in the datagram.
        actual: String,
    },
    /// Adapter configuration is invalid.
    InvalidConfiguration {
        /// Actionable validation reason.
        reason: String,
    },
}

impl AdapterError {
    /// Creates a malformed-packet error without retaining raw datagram bytes.
    #[must_use]
    pub fn malformed_packet(reason: impl Into<String>) -> Self {
        Self::MalformedPacket {
            reason: reason.into(),
        }
    }

    /// Creates an unsupported-protocol error.
    #[must_use]
    pub fn unsupported_protocol(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::UnsupportedProtocol {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Creates an invalid-configuration error.
    #[must_use]
    pub fn invalid_configuration(reason: impl Into<String>) -> Self {
        Self::InvalidConfiguration {
            reason: reason.into(),
        }
    }
}

impl From<AdapterIdError> for AdapterError {
    fn from(error: AdapterIdError) -> Self {
        Self::invalid_configuration(error.to_string())
    }
}

impl Display for AdapterError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedPacket { reason } => write!(formatter, "malformed packet: {reason}"),
            Self::UnsupportedProtocol { expected, actual } => write!(
                formatter,
                "unsupported protocol version {actual}; expected {expected}"
            ),
            Self::InvalidConfiguration { reason } => {
                write!(formatter, "invalid adapter configuration: {reason}")
            }
        }
    }
}

impl Error for AdapterError {}

/// Contract implemented by every built-in game telemetry adapter.
pub trait GameAdapter: Send {
    /// Returns stable identity, protocol, and capability metadata.
    fn descriptor(&self) -> &AdapterDescriptor;

    /// Decodes one datagram and appends its normalized values to `output`.
    ///
    /// The caller must invoke [`AdapterOutput::clear`] before reusing an output buffer.
    /// Implementations must not retain `datagram` or panic on malformed input.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError`] when the datagram or adapter configuration is invalid.
    fn decode(
        &mut self,
        datagram: &[u8],
        received_at: MonotonicTimestamp,
        output: &mut AdapterOutput,
    ) -> Result<(), AdapterError>;
}
