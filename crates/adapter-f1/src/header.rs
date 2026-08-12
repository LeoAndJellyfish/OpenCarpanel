use crate::{Cursor, DecodeError};

/// Packed byte length of the F1 24/25 packet header.
pub const PACKET_HEADER_LEN: usize = 29;

/// Common header at the start of every supported F1 UDP datagram.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PacketHeader {
    /// F1 game-year packet format.
    pub packet_format: u16,
    /// Last two digits of the game year.
    pub game_year: u8,
    /// Running game's major version.
    pub game_major_version: u8,
    /// Running game's minor version.
    pub game_minor_version: u8,
    /// Version of the packet type identified by `packet_id`.
    pub packet_version: u8,
    /// Packet type identifier defined by the official specification.
    pub packet_id: u8,
    /// Game-provided opaque session identifier.
    pub session_uid: u64,
    /// Game session timestamp in seconds.
    pub session_time: f32,
    /// Frame identifier that can move backwards after a flashback.
    pub frame_identifier: u32,
    /// Frame identifier that remains monotonic across flashbacks.
    pub overall_frame_identifier: u32,
    /// Index of the player's car in packet arrays.
    pub player_car_index: u8,
    /// Split-screen player's car index, or 255 when absent.
    pub secondary_player_car_index: u8,
}

impl PacketHeader {
    /// Decodes the common header for one exact packet format.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::UnexpectedEnd`] for a truncated header and
    /// [`DecodeError::UnsupportedPacketFormat`] for another game-year format.
    pub fn decode(
        datagram: &[u8],
        expected_packet_format: u16,
    ) -> Result<(Self, &[u8]), DecodeError> {
        let (header, payload) = Self::decode_any_format(datagram)?;
        if header.packet_format != expected_packet_format {
            return Err(DecodeError::UnsupportedPacketFormat {
                expected: expected_packet_format,
                actual: header.packet_format,
            });
        }
        Ok((header, payload))
    }

    pub(crate) fn decode_any_format(datagram: &[u8]) -> Result<(Self, &[u8]), DecodeError> {
        let mut cursor = Cursor::new(datagram);
        let packet_format = cursor.read_u16_le()?;

        let header = Self {
            packet_format,
            game_year: cursor.read_u8()?,
            game_major_version: cursor.read_u8()?,
            game_minor_version: cursor.read_u8()?,
            packet_version: cursor.read_u8()?,
            packet_id: cursor.read_u8()?,
            session_uid: cursor.read_u64_le()?,
            session_time: cursor.read_f32_le()?,
            frame_identifier: cursor.read_u32_le()?,
            overall_frame_identifier: cursor.read_u32_le()?,
            player_car_index: cursor.read_u8()?,
            secondary_player_car_index: cursor.read_u8()?,
        };

        debug_assert_eq!(datagram.len() - cursor.remaining().len(), PACKET_HEADER_LEN);
        Ok((header, cursor.remaining()))
    }
}
