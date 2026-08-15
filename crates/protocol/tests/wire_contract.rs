use std::{error::Error, fs, io, path::Path};

use opensimdash_protocol::{
    CapabilitiesMessage, ClientHello, ClientMessage, ClientPayload, ErrorCode, ErrorMessage,
    EventAckMessage, EventMessage, ResyncRequiredMessage, ServerHello, ServerMessage,
    ServerPayload, SnapshotMessage, SnapshotRequestMessage, StaleMessage, StaleReason,
    WireDecodeError, decode_client_message, decode_server_message,
};
use opensimdash_telemetry_core::{
    MonotonicTimestamp, TelemetryEvent, TelemetryField, TelemetrySnapshot,
};
use serde_json::{Value, json};

fn encoded(value: &impl serde::Serialize) -> serde_json::Result<Value> {
    serde_json::to_value(value)
}

fn rejected<T>(result: Result<T, WireDecodeError>) -> Result<WireDecodeError, Box<dyn Error>> {
    match result {
        Ok(_) => Err(io::Error::other("message unexpectedly decoded").into()),
        Err(error) => Ok(error),
    }
}

#[test]
fn server_messages_keep_stable_external_tags() -> serde_json::Result<()> {
    let cases = [
        (
            "hello",
            ServerPayload::Hello(ServerHello {
                server_version: "0.1.0".into(),
                protocol_version: 1,
                device_session: Some("device-session".into()),
            }),
        ),
        (
            "snapshot",
            ServerPayload::Snapshot(SnapshotMessage {
                seq: 7,
                captured_at_us: 42,
                data: TelemetrySnapshot::default(),
            }),
        ),
        (
            "event",
            ServerPayload::Event(EventMessage {
                seq: 8,
                data: TelemetryEvent {
                    name: "lap.completed".into(),
                    occurred_at: MonotonicTimestamp::from_micros(41),
                    data: json!({"lap": 3}),
                },
            }),
        ),
        (
            "capabilities",
            ServerPayload::Capabilities(CapabilitiesMessage {
                fields: vec![TelemetryField::VehicleSpeed],
                extensions: vec!["f1-24.revLightsPercent".into()],
                plugins: Vec::new(),
            }),
        ),
        (
            "resync_required",
            ServerPayload::ResyncRequired(ResyncRequiredMessage {
                oldest_available_event_seq: 20,
                newest_event_seq: 40,
            }),
        ),
        (
            "stale",
            ServerPayload::Stale(StaleMessage {
                since_us: 5_000_000,
                reason: StaleReason::GameDataTimeout,
            }),
        ),
        (
            "error",
            ServerPayload::Error(ErrorMessage {
                code: ErrorCode::UnsupportedVersion,
                message: "upgrade the dashboard".into(),
                retryable: false,
            }),
        ),
    ];

    for (expected_type, payload) in cases {
        let value = encoded(&ServerMessage::new(payload))?;
        assert_eq!(value["v"], 1);
        assert_eq!(value["type"], expected_type);
    }

    Ok(())
}

#[test]
fn client_hello_and_event_ack_keep_stable_external_tags() -> serde_json::Result<()> {
    let hello = encoded(&ClientMessage::new(ClientPayload::Hello(ClientHello {
        pairing_token: Some("pair-once".into()),
        device_session: None,
        device_name: Some("Test tablet".into()),
        last_event_seq: Some(10),
        snapshot_hz: 60,
    })))?;
    assert_eq!(hello["v"], 1);
    assert_eq!(hello["type"], "hello");
    assert_eq!(hello["snapshotHz"], 60);
    assert_eq!(hello["deviceName"], "Test tablet");

    let ack = encoded(&ClientMessage::new(ClientPayload::EventAck(
        EventAckMessage { seq: 11 },
    )))?;
    assert_eq!(ack["type"], "event_ack");
    assert_eq!(ack["seq"], 11);

    let snapshot_request = encoded(&ClientMessage::new(ClientPayload::SnapshotRequest(
        SnapshotRequestMessage {},
    )))?;
    assert_eq!(snapshot_request["type"], "snapshot_request");

    Ok(())
}

#[test]
fn snapshot_data_is_flattened_beside_the_envelope_tag() -> serde_json::Result<()> {
    let value = encoded(&ServerMessage::new(ServerPayload::Snapshot(
        SnapshotMessage {
            seq: 1,
            captured_at_us: 99,
            data: TelemetrySnapshot::default(),
        },
    )))?;

    assert_eq!(value["v"], 1);
    assert_eq!(value["type"], "snapshot");
    assert!(value.get("data").is_some());
    assert!(value.get("payload").is_none());

    Ok(())
}

#[test]
fn unknown_message_type_returns_a_protocol_error() -> Result<(), Box<dyn Error>> {
    let error = rejected(decode_server_message(br#"{"v":1,"type":"future_message"}"#))?;

    assert!(matches!(error, WireDecodeError::InvalidMessage { .. }));
    Ok(())
}

#[test]
fn unsupported_version_is_rejected_before_payload_use() -> Result<(), Box<dyn Error>> {
    let error = rejected(decode_client_message(
        br#"{"v":2,"type":"event_ack","seq":1}"#,
    ))?;

    assert_eq!(
        error,
        WireDecodeError::UnsupportedVersion {
            expected: 1,
            actual: 2,
        }
    );
    Ok(())
}

#[test]
fn malformed_input_returns_a_protocol_error_without_panicking() -> Result<(), Box<dyn Error>> {
    let error = rejected(decode_server_message(br#"{"v":1,"type":"snapshot"}"#))?;

    assert!(matches!(error, WireDecodeError::InvalidMessage { .. }));
    Ok(())
}

#[test]
fn committed_web_fixtures_are_serialized_rust_messages() -> Result<(), Box<dyn Error>> {
    let fixture_root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/widget-sdk/src/fixtures");
    let cases = [
        (
            "server-hello.json",
            ServerMessage::new(ServerPayload::Hello(ServerHello {
                server_version: "0.1.0".into(),
                protocol_version: 1,
                device_session: Some("fixture-device-session".into()),
            })),
        ),
        (
            "server-snapshot.json",
            ServerMessage::new(ServerPayload::Snapshot(SnapshotMessage {
                seq: 7,
                captured_at_us: 42,
                data: TelemetrySnapshot::default(),
            })),
        ),
        (
            "server-event.json",
            ServerMessage::new(ServerPayload::Event(EventMessage {
                seq: 8,
                data: TelemetryEvent {
                    name: "lap.completed".into(),
                    occurred_at: MonotonicTimestamp::from_micros(41),
                    data: json!({"lap": 3}),
                },
            })),
        ),
    ];

    for (file, message) in cases {
        let committed: Value = serde_json::from_slice(&fs::read(fixture_root.join(file))?)?;
        assert_eq!(committed, encoded(&message)?);
    }

    Ok(())
}
