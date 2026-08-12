use opencarpanel_adapter_scs::{
    ATS_GAME_ID, BRIDGE_MAGIC, BRIDGE_PACKET_LEN, BRIDGE_PROTOCOL_VERSION, BridgeGame,
    BridgePacket, DecodeError, ETS2_GAME_ID,
};

fn packet(game: u8) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(BRIDGE_PACKET_LEN);
    bytes.extend_from_slice(&BRIDGE_MAGIC);
    bytes.extend_from_slice(&[BRIDGE_PROTOCOL_VERSION, game, 0, 0]);
    bytes.extend_from_slice(&0x0102_0304_0506_0708_u64.to_le_bytes());
    bytes.extend_from_slice(&42_u32.to_le_bytes());
    bytes.extend_from_slice(&(-12.5_f32).to_le_bytes());
    bytes.extend_from_slice(&1_350.4_f32.to_le_bytes());
    bytes.extend_from_slice(&2_500.0_f32.to_le_bytes());
    bytes.extend_from_slice(&(-1_i32).to_le_bytes());
    bytes.extend_from_slice(&0.75_f32.to_le_bytes());
    bytes.extend_from_slice(&0.25_f32.to_le_bytes());
    assert_eq!(bytes.len(), BRIDGE_PACKET_LEN);
    bytes
}

fn write_f32(bytes: &mut [u8], offset: usize, value: f32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn assert_f32_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= f32::EPSILON,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn exact_v1_packet_decodes_both_game_ids() -> Result<(), DecodeError> {
    for (game_id, game) in [
        (ETS2_GAME_ID, BridgeGame::Ets2),
        (ATS_GAME_ID, BridgeGame::Ats),
    ] {
        let decoded = BridgePacket::decode(&packet(game_id))?;
        assert_eq!(decoded.game, game);
        assert_eq!(decoded.session_nonce, 0x0102_0304_0506_0708);
        assert_eq!(decoded.frame_sequence, 42);
        assert_f32_eq(decoded.speed_mps, -12.5);
        assert_f32_eq(decoded.rpm, 1_350.4);
        assert_f32_eq(decoded.rpm_max, 2_500.0);
        assert_eq!(decoded.displayed_gear, -1);
        assert_f32_eq(decoded.throttle, 0.75);
        assert_f32_eq(decoded.brake, 0.25);
    }
    Ok(())
}

#[test]
fn every_truncation_and_extension_is_rejected() {
    let bytes = packet(ETS2_GAME_ID);
    for length in 0..BRIDGE_PACKET_LEN {
        assert!(
            BridgePacket::decode(&bytes[..length]).is_err(),
            "length {length} must be rejected"
        );
    }

    let mut extended = bytes;
    extended.push(0);
    assert_eq!(
        BridgePacket::decode(&extended),
        Err(DecodeError::InvalidLength {
            expected: BRIDGE_PACKET_LEN,
            actual: BRIDGE_PACKET_LEN + 1,
        })
    );
}

#[test]
fn identity_version_flags_and_reserved_bytes_are_strict() {
    let mut bytes = packet(ETS2_GAME_ID);
    bytes[0] ^= 0xff;
    assert!(matches!(
        BridgePacket::decode(&bytes),
        Err(DecodeError::UnsupportedMagic { .. })
    ));

    let mut bytes = packet(ETS2_GAME_ID);
    bytes[4] = 2;
    assert_eq!(
        BridgePacket::decode(&bytes),
        Err(DecodeError::UnsupportedVersion {
            expected: BRIDGE_PROTOCOL_VERSION,
            actual: 2,
        })
    );

    let mut bytes = packet(ETS2_GAME_ID);
    bytes[5] = 3;
    assert_eq!(
        BridgePacket::decode(&bytes),
        Err(DecodeError::UnsupportedGame {
            expected: 0,
            actual: 3,
        })
    );

    for (offset, flags, reserved) in [(6, 1, 0), (7, 0, 1)] {
        let mut bytes = packet(ETS2_GAME_ID);
        bytes[offset] = 1;
        assert_eq!(
            BridgePacket::decode(&bytes),
            Err(DecodeError::UnsupportedFlags { flags, reserved })
        );
    }
}

#[test]
fn invalid_numeric_values_are_rejected_without_clamping() {
    const SPEED: usize = 20;
    const RPM: usize = 24;
    const RPM_MAX: usize = 28;
    const THROTTLE: usize = 36;
    const BRAKE: usize = 40;

    for (field, offset, value) in [
        ("speed_mps", SPEED, f32::NAN),
        ("rpm", RPM, f32::INFINITY),
        ("rpm_max", RPM_MAX, f32::NEG_INFINITY),
        ("throttle", THROTTLE, f32::NAN),
        ("brake", BRAKE, f32::INFINITY),
    ] {
        let mut bytes = packet(ETS2_GAME_ID);
        write_f32(&mut bytes, offset, value);
        assert_eq!(
            BridgePacket::decode(&bytes),
            Err(DecodeError::NonFiniteValue { field })
        );
    }

    for (field, offset, value) in [
        ("rpm", RPM, -1.0),
        ("rpm", RPM, 65_536.0),
        ("rpm_max", RPM_MAX, -1.0),
        ("throttle", THROTTLE, -0.01),
        ("throttle", THROTTLE, 1.01),
        ("brake", BRAKE, -0.01),
        ("brake", BRAKE, 1.01),
    ] {
        let mut bytes = packet(ETS2_GAME_ID);
        write_f32(&mut bytes, offset, value);
        let error = BridgePacket::decode(&bytes).err();
        assert!(
            matches!(
                error,
                Some(
                    DecodeError::InvalidRpm { field: actual, .. }
                        | DecodeError::InvalidNormalizedValue { field: actual, .. }
                )
                    if actual == field
            ),
            "{field} accepted {value}"
        );
    }
}
