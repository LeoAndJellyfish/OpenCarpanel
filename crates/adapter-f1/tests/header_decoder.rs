use opencarpanel_adapter_f1::{
    DecodeError, F1_24_PACKET_FORMAT, F1_25_2026_PACKET_FORMAT, F1_25_PACKET_FORMAT,
    PACKET_HEADER_LEN, PacketHeader,
};

const SUPPORTED_HEADERS: [(u16, u8); 3] = [
    (F1_24_PACKET_FORMAT, 24),
    (F1_25_PACKET_FORMAT, 25),
    (F1_25_2026_PACKET_FORMAT, 26),
];

fn valid_header(packet_format: u16, game_year: u8) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(PACKET_HEADER_LEN);
    bytes.extend_from_slice(&packet_format.to_le_bytes());
    bytes.push(game_year);
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
fn valid_headers_decode_every_field_and_preserve_payload() -> Result<(), DecodeError> {
    for (packet_format, game_year) in SUPPORTED_HEADERS {
        let mut datagram = valid_header(packet_format, game_year);
        datagram.extend_from_slice(&[0xAA, 0xBB, 0xCC]);

        let (header, payload) = PacketHeader::decode(&datagram, packet_format)?;

        assert_eq!(header.packet_format, packet_format);
        assert_eq!(header.game_year, game_year);
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
    }

    Ok(())
}

#[test]
fn every_truncated_header_returns_unexpected_end() {
    for (packet_format, game_year) in SUPPORTED_HEADERS {
        let bytes = valid_header(packet_format, game_year);
        for length in 0..PACKET_HEADER_LEN {
            let result = PacketHeader::decode(&bytes[..length], packet_format);
            assert!(
                matches!(result, Err(DecodeError::UnexpectedEnd { .. })),
                "format {packet_format} length {length} must be rejected"
            );
        }
    }
}

#[test]
fn packet_formats_are_strictly_isolated() {
    let f1_24 = valid_header(F1_24_PACKET_FORMAT, 24);
    let f1_25 = valid_header(F1_25_PACKET_FORMAT, 25);
    let f1_25_2026 = valid_header(F1_25_2026_PACKET_FORMAT, 26);

    assert_eq!(
        PacketHeader::decode(&f1_25, F1_24_PACKET_FORMAT),
        Err(DecodeError::UnsupportedPacketFormat {
            expected: F1_24_PACKET_FORMAT,
            actual: F1_25_PACKET_FORMAT,
        })
    );
    assert_eq!(
        PacketHeader::decode(&f1_24, F1_25_PACKET_FORMAT),
        Err(DecodeError::UnsupportedPacketFormat {
            expected: F1_25_PACKET_FORMAT,
            actual: F1_24_PACKET_FORMAT,
        })
    );
    assert_eq!(
        PacketHeader::decode(&f1_25_2026, F1_25_PACKET_FORMAT),
        Err(DecodeError::UnsupportedPacketFormat {
            expected: F1_25_PACKET_FORMAT,
            actual: F1_25_2026_PACKET_FORMAT,
        })
    );
}
