use std::{sync::Arc, time::Duration};

use axum::{
    extract::{
        State,
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code},
    },
    http::{HeaderMap, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use futures_util::StreamExt;
use opensimdash_protocol::{
    CapabilitiesMessage, ClientHello, ClientPayload, ErrorCode, ErrorMessage, PROTOCOL_VERSION,
    ServerHello, ServerMessage, ServerPayload, SnapshotMessage, WireDecodeError,
    decode_client_message,
};
use tokio::{sync::broadcast, time::MissedTickBehavior};

use crate::{
    HostState,
    events::ReplayBatch,
    http::HttpState,
    pairing::{PairingError, PairingService},
};

const APPLICATION_MESSAGE_LIMIT: usize = 16 * 1024;
const TRANSPORT_MESSAGE_LIMIT: usize = 64 * 1024;
const FIRST_MESSAGE_TIMEOUT: Duration = Duration::from_secs(5);
/// Maximum number of concurrent dashboard WebSocket sessions.
pub const MAX_WEBSOCKET_CONNECTIONS: usize = 8;

pub(crate) async fn upgrade(
    State(state): State<HttpState>,
    headers: HeaderMap,
    websocket: WebSocketUpgrade,
) -> Response {
    if !origin_matches_host(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(connection_permit) = Arc::clone(&state.websocket_connections).try_acquire_owned() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    websocket
        .write_buffer_size(8 * 1024)
        .max_write_buffer_size(64 * 1024)
        .max_message_size(TRANSPORT_MESSAGE_LIMIT)
        .max_frame_size(TRANSPORT_MESSAGE_LIMIT)
        .on_upgrade(move |socket| async move {
            handle_socket(socket, state.host, state.pairing, state.snapshot_hz_limit).await;
            drop(connection_permit);
        })
}

fn origin_matches_host(headers: &HeaderMap) -> bool {
    let mut origins = headers.get_all(header::ORIGIN).iter();
    let Some(origin) = origins.next() else {
        // Non-browser local clients do not have to synthesize an Origin header.
        return true;
    };
    if origins.next().is_some() {
        return false;
    }
    let Some(host) = headers.get(header::HOST) else {
        return false;
    };
    let (Ok(origin), Ok(host)) = (origin.to_str(), host.to_str()) else {
        return false;
    };
    let Ok(origin) = origin.parse::<Uri>() else {
        return false;
    };
    let valid_scheme = origin
        .scheme_str()
        .is_some_and(|scheme| matches!(scheme, "http" | "https"));
    valid_scheme
        && origin
            .authority()
            .is_some_and(|authority| authority.as_str().eq_ignore_ascii_case(host))
}

async fn handle_socket(
    mut socket: WebSocket,
    host: Arc<HostState>,
    pairing: Arc<PairingService>,
    snapshot_hz_limit: u16,
) {
    let hello = match read_hello(&mut socket).await {
        Ok(hello) => hello,
        Err(failure) => {
            send_failure_and_close(&mut socket, failure).await;
            return;
        }
    };

    let authentication = match pairing.authenticate(&hello).await {
        Ok(authentication) => authentication,
        Err(error) => {
            send_failure_and_close(&mut socket, ConnectionFailure::from(error)).await;
            return;
        }
    };

    let mut event_receiver = host.subscribe_events();
    let mut snapshot_receiver = host.subscribe_snapshots();
    if send_payload(
        &mut socket,
        ServerPayload::Hello(ServerHello {
            server_version: env!("CARGO_PKG_VERSION").to_owned(),
            protocol_version: PROTOCOL_VERSION,
            device_session: authentication.new_device_session,
        }),
    )
    .await
    .is_err()
    {
        return;
    }
    if send_capabilities(&mut socket, &host).await.is_err() {
        return;
    }

    let mut last_event_sent = hello.last_event_seq.unwrap_or(0);
    if send_replay(&mut socket, &host, &mut last_event_sent)
        .await
        .is_err()
    {
        return;
    }
    let initial_snapshot = snapshot_receiver.borrow().as_ref().clone();
    if send_snapshot(&mut socket, initial_snapshot).await.is_err() {
        return;
    }

    let effective_snapshot_hz = hello.snapshot_hz.min(snapshot_hz_limit.max(1));
    let snapshot_period = Duration::from_secs_f64(1.0 / f64::from(effective_snapshot_hz));
    let mut snapshot_tick = tokio::time::interval(snapshot_period);
    snapshot_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let _initial_tick = snapshot_tick.tick().await;
    let mut snapshot_dirty = false;

    loop {
        tokio::select! {
            incoming = socket.next() => {
                if !handle_client_message(&mut socket, incoming, &snapshot_receiver).await {
                    return;
                }
            }
            changed = snapshot_receiver.changed() => {
                if changed.is_err() {
                    return;
                }
                snapshot_dirty = true;
            }
            _instant = snapshot_tick.tick(), if snapshot_dirty => {
                snapshot_dirty = false;
                let snapshot = snapshot_receiver.borrow_and_update().as_ref().clone();
                if send_snapshot(&mut socket, snapshot).await.is_err() {
                    return;
                }
            }
            event = event_receiver.recv() => {
                match event {
                    Ok(event) if event.seq > last_event_sent => {
                        last_event_sent = event.seq;
                        if send_payload(&mut socket, ServerPayload::Event(event)).await.is_err() {
                            return;
                        }
                    }
                    Ok(_duplicate) => {}
                    Err(broadcast::error::RecvError::Lagged(_skipped)) => {
                        if send_replay(&mut socket, &host, &mut last_event_sent).await.is_err() {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        }
    }
}

async fn read_hello(socket: &mut WebSocket) -> Result<ClientHello, ConnectionFailure> {
    let incoming = tokio::time::timeout(FIRST_MESSAGE_TIMEOUT, socket.next())
        .await
        .map_err(|_elapsed| ConnectionFailure::pairing_required("hello message timed out"))?
        .ok_or_else(|| ConnectionFailure::invalid_message("connection closed before hello"))?
        .map_err(|_error| ConnectionFailure::invalid_message("failed to read hello message"))?;
    let text = match incoming {
        Message::Text(text) => text,
        Message::Binary(_bytes) => {
            return Err(ConnectionFailure::unsupported_message(
                "hello must be a JSON text message",
            ));
        }
        Message::Close(_frame) => {
            return Err(ConnectionFailure::invalid_message(
                "connection closed before hello",
            ));
        }
        Message::Ping(_bytes) | Message::Pong(_bytes) => {
            return Err(ConnectionFailure::invalid_message(
                "first application message must be hello",
            ));
        }
    };
    if text.len() > APPLICATION_MESSAGE_LIMIT {
        return Err(ConnectionFailure::message_too_large());
    }
    let message = decode_client_message(text.as_bytes()).map_err(ConnectionFailure::from)?;
    let ClientPayload::Hello(hello) = message.payload else {
        return Err(ConnectionFailure::invalid_message(
            "first application message must be hello",
        ));
    };
    if !(1..=120).contains(&hello.snapshot_hz) {
        return Err(ConnectionFailure::invalid_message(
            "snapshotHz must be between 1 and 120",
        ));
    }
    if hello.device_name.as_ref().is_some_and(|name| {
        let trimmed = name.trim();
        trimmed.is_empty() || trimmed.len() > 64 || trimmed.chars().any(char::is_control)
    }) {
        return Err(ConnectionFailure::invalid_message(
            "deviceName must be 1 to 64 bytes without control characters",
        ));
    }
    Ok(hello)
}

async fn handle_client_message(
    socket: &mut WebSocket,
    incoming: Option<Result<Message, axum::Error>>,
    snapshot_receiver: &tokio::sync::watch::Receiver<
        Arc<opensimdash_telemetry_core::TelemetrySnapshot>,
    >,
) -> bool {
    let Some(incoming) = incoming else {
        return false;
    };
    let message = match incoming {
        Ok(message) => message,
        Err(_error) => {
            send_failure_and_close(socket, ConnectionFailure::message_too_large()).await;
            return false;
        }
    };
    match message {
        Message::Text(text) if text.len() > APPLICATION_MESSAGE_LIMIT => {
            send_failure_and_close(socket, ConnectionFailure::message_too_large()).await;
            false
        }
        Message::Text(text) => match decode_client_message(text.as_bytes()) {
            Ok(message) => match message.payload {
                ClientPayload::EventAck(_acknowledgement) => true,
                ClientPayload::SnapshotRequest(_request) => {
                    let snapshot = snapshot_receiver.borrow().as_ref().clone();
                    send_snapshot(socket, snapshot).await.is_ok()
                }
                ClientPayload::Hello(_hello) => {
                    send_failure_and_close(
                        socket,
                        ConnectionFailure::invalid_message("hello may only be sent once"),
                    )
                    .await;
                    false
                }
            },
            Err(error) => {
                send_failure_and_close(socket, ConnectionFailure::from(error)).await;
                false
            }
        },
        Message::Binary(_bytes) => {
            send_failure_and_close(
                socket,
                ConnectionFailure::unsupported_message("binary messages are not supported"),
            )
            .await;
            false
        }
        Message::Close(_frame) => false,
        Message::Ping(_bytes) | Message::Pong(_bytes) => true,
    }
}

async fn send_replay(
    socket: &mut WebSocket,
    host: &HostState,
    last_event_sent: &mut u64,
) -> Result<(), axum::Error> {
    match host.replay_events_after(*last_event_sent).await {
        ReplayBatch::Events(events) => {
            for event in events {
                *last_event_sent = event.seq;
                send_payload(socket, ServerPayload::Event(event)).await?;
            }
        }
        ReplayBatch::ResyncRequired(resync) => {
            *last_event_sent = resync.newest_event_seq;
            send_payload(socket, ServerPayload::ResyncRequired(resync)).await?;
        }
    }
    Ok(())
}

async fn send_snapshot(
    socket: &mut WebSocket,
    snapshot: opensimdash_telemetry_core::TelemetrySnapshot,
) -> Result<(), axum::Error> {
    let captured_at_us = snapshot
        .meta
        .captured_at
        .map_or(0, opensimdash_telemetry_core::MonotonicTimestamp::as_micros);
    send_payload(
        socket,
        ServerPayload::Snapshot(SnapshotMessage {
            seq: snapshot.meta.sequence,
            captured_at_us,
            data: snapshot,
        }),
    )
    .await
}

async fn send_capabilities(socket: &mut WebSocket, host: &HostState) -> Result<(), axum::Error> {
    send_payload(
        socket,
        ServerPayload::Capabilities(CapabilitiesMessage {
            fields: host.capabilities().to_vec(),
            extensions: Vec::new(),
            plugins: host
                .supported_adapters()
                .iter()
                .map(|adapter| adapter.metadata().clone())
                .collect(),
        }),
    )
    .await
}

async fn send_payload(socket: &mut WebSocket, payload: ServerPayload) -> Result<(), axum::Error> {
    let message = ServerMessage::new(payload);
    let Ok(serialized) = serde_json::to_string(&message) else {
        return send_internal_error(socket).await;
    };
    socket.send(Message::Text(serialized.into())).await
}

async fn send_internal_error(socket: &mut WebSocket) -> Result<(), axum::Error> {
    let fallback = r#"{"v":1,"type":"error","code":"internal","message":"failed to encode server message","retryable":true}"#;
    socket.send(Message::Text(fallback.into())).await
}

async fn send_failure_and_close(socket: &mut WebSocket, failure: ConnectionFailure) {
    let _send_result = send_payload(
        socket,
        ServerPayload::Error(ErrorMessage {
            code: failure.error_code,
            message: failure.message.to_owned(),
            retryable: failure.retryable,
        }),
    )
    .await;
    let _close_result = socket
        .send(Message::Close(Some(CloseFrame {
            code: failure.close_code,
            reason: failure.close_reason.into(),
        })))
        .await;
}

#[derive(Debug, Clone, Copy)]
struct ConnectionFailure {
    error_code: ErrorCode,
    message: &'static str,
    retryable: bool,
    close_code: u16,
    close_reason: &'static str,
}

impl ConnectionFailure {
    const fn invalid_message(message: &'static str) -> Self {
        Self {
            error_code: ErrorCode::InvalidMessage,
            message,
            retryable: false,
            close_code: close_code::PROTOCOL,
            close_reason: "invalid_message",
        }
    }

    const fn unsupported_message(message: &'static str) -> Self {
        Self {
            error_code: ErrorCode::InvalidMessage,
            message,
            retryable: false,
            close_code: close_code::UNSUPPORTED,
            close_reason: "unsupported_message",
        }
    }

    const fn pairing_required(message: &'static str) -> Self {
        Self {
            error_code: ErrorCode::PairingRequired,
            message,
            retryable: true,
            close_code: close_code::POLICY,
            close_reason: "pairing_required",
        }
    }

    const fn message_too_large() -> Self {
        Self {
            error_code: ErrorCode::MessageTooLarge,
            message: "message exceeds the 16 KiB application limit",
            retryable: false,
            close_code: close_code::SIZE,
            close_reason: "message_too_large",
        }
    }
}

impl From<WireDecodeError> for ConnectionFailure {
    fn from(error: WireDecodeError) -> Self {
        match error {
            WireDecodeError::UnsupportedVersion { .. } => Self {
                error_code: ErrorCode::UnsupportedVersion,
                message: "unsupported protocol version",
                retryable: false,
                close_code: close_code::PROTOCOL,
                close_reason: "unsupported_version",
            },
            WireDecodeError::InvalidMessage { .. } => {
                Self::invalid_message("invalid JSON protocol message")
            }
            _ => Self::invalid_message("unsupported protocol message"),
        }
    }
}

impl From<PairingError> for ConnectionFailure {
    fn from(error: PairingError) -> Self {
        match error {
            PairingError::RandomSourceUnavailable => Self {
                error_code: ErrorCode::Internal,
                message: "could not create a secure device session",
                retryable: true,
                close_code: close_code::AWAY,
                close_reason: "internal",
            },
            PairingError::PairingRequired | PairingError::InvalidDeviceSession => {
                Self::pairing_required("pairing or a valid device session is required")
            }
            PairingError::ConflictingCredentials => {
                Self::invalid_message("provide exactly one credential")
            }
            PairingError::InvalidPairingToken => Self {
                error_code: ErrorCode::InvalidPairingToken,
                message: "pairing token is invalid or already used",
                retryable: false,
                close_code: close_code::POLICY,
                close_reason: "invalid_pairing_token",
            },
            PairingError::PairingTokenExpired => Self {
                error_code: ErrorCode::PairingTokenExpired,
                message: "pairing token has expired",
                retryable: false,
                close_code: close_code::POLICY,
                close_reason: "pairing_token_expired",
            },
            PairingError::StorageUnavailable => Self {
                error_code: ErrorCode::Internal,
                message: "paired-device storage is unavailable",
                retryable: true,
                close_code: close_code::ERROR,
                close_reason: "device_storage_unavailable",
            },
        }
    }
}
