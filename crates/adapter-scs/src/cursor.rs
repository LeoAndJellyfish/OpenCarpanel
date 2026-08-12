use crate::DecodeError;

#[derive(Debug, Clone)]
pub(crate) struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub(crate) fn read_array<const LENGTH: usize>(&mut self) -> Result<[u8; LENGTH], DecodeError> {
        self.take()
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take::<1>()?[0])
    }

    pub(crate) fn read_u32_le(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.take()?))
    }

    pub(crate) fn read_i32_le(&mut self) -> Result<i32, DecodeError> {
        Ok(i32::from_le_bytes(self.take()?))
    }

    pub(crate) fn read_u64_le(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(self.take()?))
    }

    pub(crate) fn read_f32_le(&mut self) -> Result<f32, DecodeError> {
        Ok(f32::from_le_bytes(self.take()?))
    }

    fn take<const LENGTH: usize>(&mut self) -> Result<[u8; LENGTH], DecodeError> {
        let remaining = self.bytes.len().saturating_sub(self.offset);
        if remaining < LENGTH {
            return Err(DecodeError::UnexpectedEnd {
                offset: self.offset,
                needed: LENGTH,
                remaining,
            });
        }

        let end = self.offset + LENGTH;
        let mut value = [0; LENGTH];
        value.copy_from_slice(&self.bytes[self.offset..end]);
        self.offset = end;
        Ok(value)
    }
}
