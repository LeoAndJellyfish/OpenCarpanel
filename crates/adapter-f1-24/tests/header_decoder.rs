use opencarpanel_adapter_f1_24::{
    DecodeError, F1_24_PACKET_FORMAT, PACKET_HEADER_LEN, PacketHeader,
};

fn valid_header() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(PACKET_HEADER_LEN);
    bytes.extend_from_slice(&F1_24_PACKET_FORMAT.to_le_bytes());
    bytes.push(24);
    bytes.push(1);
    bytes.push(7);
    bytes.push(3);
    bytes.push(6);
    bytes.extend_from_slice(&0x0102_0304_0506_0708_u64.to_le_bytes());
    bytes.extend_from_slice(&12.5_f32.to_le_bytes());
    bytes.extend_from_slice(&0x1020_3040_u32.to_le_bytes());
    bytes.extend_from_slice(&0x5060_7080_u32.to_le_bytes());
    bytes.push(17);
    bytes.push(255);
    bytes
}

#[test]
fn valid_header_decodes_every_field_and_preserves_payload() -> Result<(), DecodeError> {
    let mut datagram = valid_header();
    datagram.extend_from_slice(&[0xAA, 0xBB, 0xCC]);

    let (header, payload) = PacketHeader::decode(&datagram)?;

    assert_eq!(header.packet_format, 2024);
    assert_eq!(header.game_year, 24);
    assert_eq!(header.game_major_version, 1);
    assert_eq!(header.game_minor_version, 7);
    assert_eq!(header.packet_version, 3);
    assert_eq!(header.packet_id, 6);
    assert_eq!(header.session_uid, 0x0102_0304_0506_0708);
    assert_eq!(header.session_time.to_bits(), 12.5_f32.to_bits());
    assert_eq!(header.frame_identifier, 0x1020_3040);
    assert_eq!(header.overall_frame_identifier, 0x5060_7080);
    assert_eq!(header.player_car_index, 17);
    assert_eq!(header.secondary_player_car_index, 255);
    assert_eq!(payload, [0xAA, 0xBB, 0xCC]);

    Ok(())
}

#[test]
fn every_truncated_header_returns_unexpected_end() {
    let bytes = valid_header();

    for length in 0..PACKET_HEADER_LEN {
        let result = PacketHeader::decode(&bytes[..length]);
        assert!(
            matches!(result, Err(DecodeError::UnexpectedEnd { .. })),
            "length {length} must be rejected"
        );
    }
}

#[test]
fn non_f1_24_format_is_rejected() {
    let mut bytes = valid_header();
    bytes[0..2].copy_from_slice(&2023_u16.to_le_bytes());

    assert_eq!(
        PacketHeader::decode(&bytes),
        Err(DecodeError::UnsupportedPacketFormat {
            expected: 2024,
            actual: 2023,
        })
    );
}
