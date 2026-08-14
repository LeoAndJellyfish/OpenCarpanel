use opencarpanel_adapter_scs::{
    ATS_GAME_ID, BRIDGE_JOB_TEXT_LEN, BRIDGE_MAGIC, BRIDGE_PACKET_LEN, BRIDGE_PROTOCOL_V1,
    BRIDGE_PROTOCOL_VERSION, BRIDGE_V1_PACKET_LEN, BridgeGame, BridgePacket, DecodeError,
    ETS2_GAME_ID,
};

fn packet_v1(game: u8) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(BRIDGE_V1_PACKET_LEN);
    bytes.extend_from_slice(&BRIDGE_MAGIC);
    bytes.extend_from_slice(&[BRIDGE_PROTOCOL_V1, game, 0, 0]);
    bytes.extend_from_slice(&0x0102_0304_0506_0708_u64.to_le_bytes());
    bytes.extend_from_slice(&42_u32.to_le_bytes());
    bytes.extend_from_slice(&(-12.5_f32).to_le_bytes());
    bytes.extend_from_slice(&1_350.4_f32.to_le_bytes());
    bytes.extend_from_slice(&2_500.0_f32.to_le_bytes());
    bytes.extend_from_slice(&(-1_i32).to_le_bytes());
    bytes.extend_from_slice(&0.75_f32.to_le_bytes());
    bytes.extend_from_slice(&0.25_f32.to_le_bytes());
    assert_eq!(bytes.len(), BRIDGE_V1_PACKET_LEN);
    bytes
}

fn extend_text(bytes: &mut Vec<u8>, value: &str) {
    assert!(value.len() <= BRIDGE_JOB_TEXT_LEN);
    bytes.extend_from_slice(value.as_bytes());
    bytes.resize(bytes.len() + BRIDGE_JOB_TEXT_LEN - value.len(), 0);
}

fn packet_v2(game: u8) -> Vec<u8> {
    let mut bytes = packet_v1(game);
    bytes[4] = BRIDGE_PROTOCOL_VERSION;
    bytes.reserve(BRIDGE_PACKET_LEN - BRIDGE_V1_PACKET_LEN);
    bytes.extend_from_slice(&12_345.5_f32.to_le_bytes());
    bytes.extend_from_slice(&987.25_f32.to_le_bytes());
    bytes.extend_from_slice(&22.22_f32.to_le_bytes());
    bytes.extend_from_slice(&321.5_f32.to_le_bytes());
    bytes.extend_from_slice(&500.0_f32.to_le_bytes());
    bytes.extend_from_slice(&1_234.0_f32.to_le_bytes());
    bytes.extend_from_slice(&0x01ff_u16.to_le_bytes());
    bytes.extend_from_slice(&0x000f_u16.to_le_bytes());
    bytes.extend_from_slice(&123_456_u32.to_le_bytes());
    bytes.extend_from_slice(&789_u32.to_le_bytes());
    bytes.extend_from_slice(&98_765_u64.to_le_bytes());
    bytes.extend_from_slice(&18_500.0_f32.to_le_bytes());
    extend_text(&mut bytes, "冷冻食品");
    extend_text(&mut bytes, "Berlin");
    extend_text(&mut bytes, "Praha");
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
        let decoded = BridgePacket::decode(&packet_v1(game_id))?;
        assert_eq!(decoded.protocol_version, BRIDGE_PROTOCOL_V1);
        assert_eq!(decoded.game, game);
        assert_eq!(decoded.session_nonce, 0x0102_0304_0506_0708);
        assert_eq!(decoded.frame_sequence, 42);
        assert_f32_eq(decoded.speed_mps, -12.5);
        assert_f32_eq(decoded.rpm, 1_350.4);
        assert_f32_eq(decoded.rpm_max, 2_500.0);
        assert_eq!(decoded.displayed_gear, -1);
        assert_f32_eq(decoded.throttle, 0.75);
        assert_f32_eq(decoded.brake, 0.25);
        assert!(decoded.navigation_distance_m.is_none());
        assert!(decoded.lights.is_none());
        assert!(decoded.job.is_none());
    }
    Ok(())
}

#[test]
fn exact_v2_packet_decodes_navigation_fuel_lights_and_job() -> Result<(), DecodeError> {
    let decoded = BridgePacket::decode(&packet_v2(ETS2_GAME_ID))?;
    assert_eq!(decoded.protocol_version, BRIDGE_PROTOCOL_VERSION);
    assert_eq!(decoded.navigation_distance_m, Some(12_345.5));
    assert_eq!(decoded.navigation_time_s, Some(987.25));
    assert_eq!(decoded.navigation_speed_limit_mps, Some(22.22));
    assert_eq!(decoded.fuel_liters, Some(321.5));
    assert_eq!(decoded.fuel_capacity_liters, Some(500.0));
    assert_eq!(decoded.fuel_range_km, Some(1_234.0));
    assert_eq!(decoded.fuel_warning, Some(true));

    let lights = decoded.lights;
    assert_eq!(lights.map(|value| value.parking), Some(true));
    assert_eq!(lights.map(|value| value.low_beam), Some(true));
    assert_eq!(lights.map(|value| value.high_beam), Some(true));
    assert_eq!(lights.map(|value| value.beacon), Some(true));
    assert_eq!(lights.map(|value| value.brake), Some(true));
    assert_eq!(lights.map(|value| value.reverse), Some(true));
    assert_eq!(lights.map(|value| value.left_indicator), Some(true));
    assert_eq!(lights.map(|value| value.right_indicator), Some(true));
    assert_eq!(lights.map(|value| value.hazard), Some(true));

    let job = decoded.job.as_ref();
    assert_eq!(job.map(|value| value.active), Some(true));
    assert_eq!(job.map(|value| value.cargo_loaded), Some(true));
    assert_eq!(job.map(|value| value.special), Some(true));
    assert_eq!(job.map(|value| value.delivery_time), Some(123_456));
    assert_eq!(job.map(|value| value.planned_distance_km), Some(789));
    assert_eq!(job.map(|value| value.income), Some(98_765));
    assert_f32_eq(job.map_or(f32::NAN, |value| value.cargo_mass_kg), 18_500.0);
    assert_eq!(
        job.and_then(|value| value.cargo.as_deref()),
        Some("冷冻食品")
    );
    assert_eq!(
        job.and_then(|value| value.source_city.as_deref()),
        Some("Berlin")
    );
    assert_eq!(
        job.and_then(|value| value.destination_city.as_deref()),
        Some("Praha")
    );
    Ok(())
}

#[test]
fn every_v1_and_v2_truncation_and_extension_is_rejected() {
    for bytes in [packet_v1(ETS2_GAME_ID), packet_v2(ETS2_GAME_ID)] {
        for length in 0..bytes.len() {
            assert!(
                BridgePacket::decode(&bytes[..length]).is_err(),
                "length {length} must be rejected"
            );
        }

        let expected = bytes.len();
        let mut extended = bytes;
        extended.push(0);
        assert_eq!(
            BridgePacket::decode(&extended),
            Err(DecodeError::InvalidLength {
                expected,
                actual: expected + 1,
            })
        );
    }
}

#[test]
fn identity_version_flags_and_reserved_bytes_are_strict() {
    let mut bytes = packet_v1(ETS2_GAME_ID);
    bytes[0] ^= 0xff;
    assert!(matches!(
        BridgePacket::decode(&bytes),
        Err(DecodeError::UnsupportedMagic { .. })
    ));

    let mut bytes = packet_v1(ETS2_GAME_ID);
    bytes[4] = 3;
    assert_eq!(
        BridgePacket::decode(&bytes),
        Err(DecodeError::UnsupportedVersion {
            expected: BRIDGE_PROTOCOL_VERSION,
            actual: 3,
        })
    );

    let mut bytes = packet_v1(ETS2_GAME_ID);
    bytes[5] = 3;
    assert_eq!(
        BridgePacket::decode(&bytes),
        Err(DecodeError::UnsupportedGame {
            expected: 0,
            actual: 3,
        })
    );

    for (offset, flags, reserved) in [(6, 1, 0), (7, 0, 1)] {
        let mut bytes = packet_v1(ETS2_GAME_ID);
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
        let mut bytes = packet_v1(ETS2_GAME_ID);
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
        let mut bytes = packet_v1(ETS2_GAME_ID);
        write_f32(&mut bytes, offset, value);
        let error = BridgePacket::decode(&bytes).err();
        assert!(
            matches!(
                error,
                Some(
                    DecodeError::InvalidRpm { field: actual, .. }
                        | DecodeError::InvalidNormalizedValue { field: actual, .. }
                ) if actual == field
            ),
            "{field} accepted {value}"
        );
    }

    for (field, offset, value) in [
        ("navigation_distance_m", 44, -1.0),
        ("navigation_time_s", 48, f32::NAN),
        ("fuel_liters", 56, -1.0),
        ("cargo_mass_kg", 88, f32::INFINITY),
    ] {
        let mut bytes = packet_v2(ETS2_GAME_ID);
        write_f32(&mut bytes, offset, value);
        let error = BridgePacket::decode(&bytes).err();
        assert!(
            matches!(
                error,
                Some(
                    DecodeError::NegativeValue { field: actual, .. }
                        | DecodeError::NonFiniteValue { field: actual }
                ) if actual == field
            ),
            "{field} accepted {value}"
        );
    }
}

#[test]
fn non_positive_navigation_speed_limit_is_a_valid_scs_special_state() -> Result<(), DecodeError> {
    for value in [0.0, -1.0, -25.0] {
        let mut bytes = packet_v2(ETS2_GAME_ID);
        write_f32(&mut bytes, 52, value);
        assert_eq!(
            BridgePacket::decode(&bytes)?.navigation_speed_limit_mps,
            Some(value)
        );
    }
    Ok(())
}

#[test]
fn v2_rejects_unknown_bits_and_invalid_text() {
    let mut bytes = packet_v2(ETS2_GAME_ID);
    bytes[69] |= 0x80;
    assert!(matches!(
        BridgePacket::decode(&bytes),
        Err(DecodeError::UnsupportedLightBits { .. })
    ));

    let mut bytes = packet_v2(ETS2_GAME_ID);
    bytes[71] |= 0x80;
    assert!(matches!(
        BridgePacket::decode(&bytes),
        Err(DecodeError::UnsupportedStateBits { .. })
    ));

    let mut bytes = packet_v2(ETS2_GAME_ID);
    bytes[92] = 0xff;
    assert_eq!(
        BridgePacket::decode(&bytes),
        Err(DecodeError::InvalidUtf8 { field: "cargo" })
    );

    let mut bytes = packet_v2(ETS2_GAME_ID);
    bytes[120] = b'x';
    assert_eq!(
        BridgePacket::decode(&bytes),
        Err(DecodeError::InvalidTextPadding { field: "cargo" })
    );

    let mut bytes = packet_v2(ETS2_GAME_ID);
    bytes[92..124].fill(b'x');
    assert_eq!(
        BridgePacket::decode(&bytes),
        Err(DecodeError::InvalidTextPadding { field: "cargo" })
    );
}
