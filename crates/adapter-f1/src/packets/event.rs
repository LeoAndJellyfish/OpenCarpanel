use crate::{Cursor, DecodeError, PacketHeader};

use super::{EVENT_PACKET_ID, EVENT_PACKET_LEN, F1Layout, validate_packet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EventSample {
    pub(crate) code: [u8; 4],
    pub(crate) details: [u8; 12],
}

pub(crate) fn decode_event(
    header: &PacketHeader,
    payload: &[u8],
    datagram_len: usize,
    _layout: F1Layout,
) -> Result<EventSample, DecodeError> {
    validate_packet(header, datagram_len, EVENT_PACKET_ID, EVENT_PACKET_LEN)?;
    let mut cursor = Cursor::new(payload);
    Ok(EventSample {
        code: cursor.read_array()?,
        details: cursor.read_array()?,
    })
}
