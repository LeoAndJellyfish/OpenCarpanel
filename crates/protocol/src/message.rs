use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use opencarpanel_game_plugin_api::GamePluginMetadata;
use opencarpanel_telemetry_core::{TelemetryEvent, TelemetryField, TelemetrySnapshot};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

/// Current major version of the Host/browser wire protocol.
pub const PROTOCOL_VERSION: u16 = 1;

/// Message sent from a dashboard client to the Host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClientMessage {
    /// Wire protocol major version.
    #[schemars(schema_with = "protocol_version_schema")]
    pub v: u16,
    /// Versioned message body.
    #[serde(flatten)]
    pub payload: ClientPayload,
}

impl ClientMessage {
    /// Wraps a client payload using the current protocol version.
    #[must_use]
    pub const fn new(payload: ClientPayload) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            payload,
        }
    }
}

/// Payloads accepted from a dashboard client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientPayload {
    /// Opens or resumes a paired device session.
    Hello(ClientHello),
    /// Confirms that a reliable event was consumed.
    EventAck(EventAckMessage),
    /// Requests the newest complete snapshot after a visibility or state reset.
    SnapshotRequest(SnapshotRequestMessage),
}

/// First message sent by a dashboard client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClientHello {
    /// One-time token copied from the QR-code fragment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairing_token: Option<String>,
    /// Previously issued device session used for reconnects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_session: Option<String>,
    /// Human-readable bounded label shown in local device management.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
    /// Last reliable event sequence consumed by the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_event_seq: Option<u64>,
    /// Requested snapshot publication frequency.
    #[schemars(range(min = 1, max = 120))]
    pub snapshot_hz: u16,
}

/// Acknowledges one reliable event sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EventAckMessage {
    /// Highest contiguous event sequence consumed by the client.
    pub seq: u64,
}

/// Requests an immediate copy of the Host's latest snapshot.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SnapshotRequestMessage {}

/// Message sent from the Host to a dashboard client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ServerMessage {
    /// Wire protocol major version.
    #[schemars(schema_with = "protocol_version_schema")]
    pub v: u16,
    /// Versioned message body.
    #[serde(flatten)]
    pub payload: ServerPayload,
}

impl ServerMessage {
    /// Wraps a server payload using the current protocol version.
    #[must_use]
    pub const fn new(payload: ServerPayload) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            payload,
        }
    }
}

/// Payloads emitted by the Host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
// Snapshot is deliberately inline: this value is built on the publication hot
// path and boxing it would add one heap allocation for every emitted snapshot.
#[allow(clippy::large_enum_variant)]
pub enum ServerPayload {
    /// Confirms pairing or session resumption.
    Hello(ServerHello),
    /// Replaceable latest telemetry state.
    Snapshot(SnapshotMessage),
    /// Ordered reliable telemetry event.
    Event(EventMessage),
    /// Fields made available by the active adapter.
    Capabilities(CapabilitiesMessage),
    /// Indicates that reliable events can no longer be replayed completely.
    ResyncRequired(ResyncRequiredMessage),
    /// Indicates that the data source is temporarily stale.
    Stale(StaleMessage),
    /// Structured failure safe to show or diagnose.
    Error(ErrorMessage),
}

/// Host response to a successful hello.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerHello {
    /// Host application version.
    pub server_version: String,
    /// Protocol version selected by the Host.
    pub protocol_version: u16,
    /// Device session issued after one-time pairing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_session: Option<String>,
}

/// Replaceable latest telemetry state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotMessage {
    /// Snapshot publication sequence.
    pub seq: u64,
    /// Host-monotonic capture time in microseconds.
    pub captured_at_us: u64,
    /// Complete canonical telemetry state.
    pub data: TelemetrySnapshot,
}

/// Ordered reliable event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EventMessage {
    /// Reliable event sequence.
    pub seq: u64,
    /// Canonical event data.
    pub data: TelemetryEvent,
}

/// Active adapter capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CapabilitiesMessage {
    /// Stable canonical telemetry paths.
    pub fields: Vec<TelemetryField>,
    /// Namespaced adapter extension paths.
    pub extensions: Vec<String>,
    /// Installed and built-in game plugins available to this Host.
    #[serde(default)]
    pub plugins: Vec<GamePluginMetadata>,
}

/// Reliable-event history can no longer satisfy the requested sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResyncRequiredMessage {
    /// Oldest event still present in the bounded buffer.
    pub oldest_available_event_seq: u64,
    /// Newest event present when this message was built.
    pub newest_event_seq: u64,
}

/// Reason that live telemetry is not currently advancing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StaleReason {
    /// The game has stopped producing UDP data.
    GameDataTimeout,
    /// The adapter is not currently connected to a data source.
    DataSourceDisconnected,
    /// A new game session requires a fresh full snapshot.
    SessionChanged,
}

/// Explicit stale-state notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StaleMessage {
    /// Host-monotonic time at which the state became stale.
    pub since_us: u64,
    /// Why the state is stale.
    pub reason: StaleReason,
}

/// Stable machine-readable error categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The client must pair or resume a device session.
    PairingRequired,
    /// A one-time pairing token was invalid.
    InvalidPairingToken,
    /// A one-time pairing token was valid but expired.
    PairingTokenExpired,
    /// The requested wire protocol version is unsupported.
    UnsupportedVersion,
    /// The message shape or value was invalid.
    InvalidMessage,
    /// The incoming message exceeded a configured bound.
    MessageTooLarge,
    /// The Host could not complete the operation.
    Internal,
}

/// Structured protocol error sent to a client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ErrorMessage {
    /// Stable programmatic error code.
    pub code: ErrorCode,
    /// Sanitized human-readable explanation.
    pub message: String,
    /// Whether retrying without user intervention can succeed.
    pub retryable: bool,
}

/// Failure to decode a versioned wire message.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WireDecodeError {
    /// The envelope uses a protocol version this build cannot interpret.
    UnsupportedVersion {
        /// Version accepted by this build.
        expected: u16,
        /// Version provided by the peer.
        actual: u16,
    },
    /// JSON or the typed message shape is invalid.
    InvalidMessage {
        /// Sanitized parser explanation; input bytes are never included.
        message: String,
    },
}

impl Display for WireDecodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion { expected, actual } => write!(
                formatter,
                "unsupported protocol version {actual}; expected {expected}"
            ),
            Self::InvalidMessage { message } => {
                write!(formatter, "invalid protocol message: {message}")
            }
        }
    }
}

impl Error for WireDecodeError {}

/// Decodes and validates a dashboard-to-Host message.
///
/// # Errors
///
/// Returns [`WireDecodeError`] for malformed JSON, unknown message types, or
/// unsupported protocol versions.
pub fn decode_client_message(bytes: &[u8]) -> Result<ClientMessage, WireDecodeError> {
    decode_versioned(bytes)
}

/// Decodes and validates a Host-to-dashboard message.
///
/// # Errors
///
/// Returns [`WireDecodeError`] for malformed JSON, unknown message types, or
/// unsupported protocol versions.
pub fn decode_server_message(bytes: &[u8]) -> Result<ServerMessage, WireDecodeError> {
    decode_versioned(bytes)
}

fn decode_versioned<T>(bytes: &[u8]) -> Result<T, WireDecodeError>
where
    T: DeserializeOwned,
{
    let value: Value = serde_json::from_slice(bytes).map_err(|error| invalid_message(&error))?;
    let raw_version =
        value
            .get("v")
            .and_then(Value::as_u64)
            .ok_or_else(|| WireDecodeError::InvalidMessage {
                message: "missing or invalid `v` field".into(),
            })?;
    let actual = u16::try_from(raw_version).map_err(|_| WireDecodeError::InvalidMessage {
        message: "protocol version is outside the supported integer range".into(),
    })?;

    if actual != PROTOCOL_VERSION {
        return Err(WireDecodeError::UnsupportedVersion {
            expected: PROTOCOL_VERSION,
            actual,
        });
    }

    serde_json::from_value(value).map_err(|error| invalid_message(&error))
}

fn invalid_message(error: &serde_json::Error) -> WireDecodeError {
    WireDecodeError::InvalidMessage {
        message: error.to_string(),
    }
}

fn protocol_version_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "integer",
        "const": PROTOCOL_VERSION
    })
}
