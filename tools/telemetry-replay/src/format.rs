use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, Read, Write},
};

use opensimdash_adapter_api::AdapterId;

const MAGIC: [u8; 8] = *b"OSDUDP01";
const FORMAT_VERSION: u16 = 1;

/// Maximum payload accepted from one UDP datagram.
pub const MAX_DATAGRAM_LEN: usize = 65_507;

/// Maximum bytes accepted or written for one capture file.
pub const MAX_CAPTURE_BYTES: u64 = 1_073_741_824;

/// Versioned metadata at the start of a telemetry capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureHeader {
    adapter_id: AdapterId,
    created_unix_ms: u64,
}

impl CaptureHeader {
    /// Creates capture metadata.
    #[must_use]
    pub const fn new(adapter_id: AdapterId, created_unix_ms: u64) -> Self {
        Self {
            adapter_id,
            created_unix_ms,
        }
    }

    /// Returns the adapter expected to decode these datagrams.
    #[must_use]
    pub const fn adapter_id(&self) -> &AdapterId {
        &self.adapter_id
    }

    /// Returns capture creation time in milliseconds since the Unix epoch.
    #[must_use]
    pub const fn created_unix_ms(&self) -> u64 {
        self.created_unix_ms
    }
}

/// One captured datagram and its monotonic offset from recording start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureRecord {
    delta_us: u64,
    datagram: Vec<u8>,
}

impl CaptureRecord {
    /// Creates a record. Bounds are enforced when it is written or replayed.
    #[must_use]
    pub const fn new(delta_us: u64, datagram: Vec<u8>) -> Self {
        Self { delta_us, datagram }
    }

    /// Returns microseconds elapsed since recording start.
    #[must_use]
    pub const fn delta_us(&self) -> u64 {
        self.delta_us
    }

    /// Returns the captured UDP payload.
    #[must_use]
    pub fn datagram(&self) -> &[u8] {
        &self.datagram
    }
}

/// Failure while reading or writing the bounded capture format.
#[derive(Debug)]
#[non_exhaustive]
pub enum CaptureError {
    /// Underlying stream operation failed.
    Io(io::Error),
    /// File does not begin with the `OpenSimDash` capture signature.
    InvalidMagic,
    /// File uses a capture format this build cannot read.
    UnsupportedVersion {
        /// Version implemented by this build.
        expected: u16,
        /// Version read from the file.
        actual: u16,
    },
    /// Adapter id metadata violates the shared slug contract.
    InvalidAdapterId(String),
    /// Adapter id metadata is not valid UTF-8.
    InvalidAdapterIdEncoding,
    /// One datagram exceeds the bounded UDP payload limit.
    DatagramTooLarge {
        /// Observed datagram size.
        actual: usize,
        /// Maximum accepted datagram size.
        maximum: usize,
    },
    /// Capture would exceed the bounded file size.
    CaptureTooLarge {
        /// Bytes observed or about to be written.
        actual: u64,
        /// Maximum accepted capture size.
        maximum: u64,
    },
    /// Record timestamps move backwards.
    NonMonotonicRecord {
        /// Previous record offset.
        previous_us: u64,
        /// Rejected record offset.
        actual_us: u64,
    },
}

impl Display for CaptureError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "capture I/O failed: {error}"),
            Self::InvalidMagic => formatter.write_str("not an OpenSimDash UDP capture"),
            Self::UnsupportedVersion { expected, actual } => write!(
                formatter,
                "unsupported capture format {actual}; expected {expected}"
            ),
            Self::InvalidAdapterId(value) => write!(formatter, "invalid adapter id {value:?}"),
            Self::InvalidAdapterIdEncoding => {
                formatter.write_str("capture adapter id is not valid UTF-8")
            }
            Self::DatagramTooLarge { actual, maximum } => write!(
                formatter,
                "datagram has {actual} bytes; maximum is {maximum}"
            ),
            Self::CaptureTooLarge { actual, maximum } => write!(
                formatter,
                "capture would contain {actual} bytes; maximum is {maximum}"
            ),
            Self::NonMonotonicRecord {
                previous_us,
                actual_us,
            } => write!(
                formatter,
                "record time {actual_us} us precedes {previous_us} us"
            ),
        }
    }
}

impl Error for CaptureError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for CaptureError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Streaming writer for the explicit capture format.
#[derive(Debug)]
pub struct CaptureWriter<W> {
    writer: W,
    bytes_written: u64,
    previous_delta_us: Option<u64>,
}

impl<W> CaptureWriter<W>
where
    W: Write,
{
    /// Writes a new capture header to `writer`.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError`] if metadata cannot be represented or written.
    pub fn new(mut writer: W, header: &CaptureHeader) -> Result<Self, CaptureError> {
        let adapter_bytes = header.adapter_id.as_str().as_bytes();
        let adapter_len = u8::try_from(adapter_bytes.len())
            .map_err(|_| CaptureError::InvalidAdapterId(header.adapter_id.as_str().to_owned()))?;

        writer.write_all(&MAGIC)?;
        writer.write_all(&FORMAT_VERSION.to_le_bytes())?;
        writer.write_all(&[adapter_len])?;
        writer.write_all(adapter_bytes)?;
        writer.write_all(&header.created_unix_ms.to_le_bytes())?;

        let bytes_written = 8_u64 + 2 + 1 + u64::from(adapter_len) + 8;
        Ok(Self {
            writer,
            bytes_written,
            previous_delta_us: None,
        })
    }

    /// Appends one bounded record.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError`] for oversized data, backwards timestamps, a
    /// capture-size overflow, or a failed write.
    pub fn write_record(&mut self, record: &CaptureRecord) -> Result<(), CaptureError> {
        validate_record_size(record.datagram.len())?;
        if let Some(previous_us) = self.previous_delta_us
            && record.delta_us < previous_us
        {
            return Err(CaptureError::NonMonotonicRecord {
                previous_us,
                actual_us: record.delta_us,
            });
        }

        let datagram_len =
            u16::try_from(record.datagram.len()).map_err(|_| CaptureError::DatagramTooLarge {
                actual: record.datagram.len(),
                maximum: MAX_DATAGRAM_LEN,
            })?;
        let record_len = 8_u64 + 2 + u64::from(datagram_len);
        let next_size =
            self.bytes_written
                .checked_add(record_len)
                .ok_or(CaptureError::CaptureTooLarge {
                    actual: u64::MAX,
                    maximum: MAX_CAPTURE_BYTES,
                })?;
        if next_size > MAX_CAPTURE_BYTES {
            return Err(CaptureError::CaptureTooLarge {
                actual: next_size,
                maximum: MAX_CAPTURE_BYTES,
            });
        }

        self.writer.write_all(&record.delta_us.to_le_bytes())?;
        self.writer.write_all(&datagram_len.to_le_bytes())?;
        self.writer.write_all(&record.datagram)?;
        self.bytes_written = next_size;
        self.previous_delta_us = Some(record.delta_us);
        Ok(())
    }

    /// Flushes buffered output.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError`] if the underlying writer cannot flush.
    pub fn flush(&mut self) -> Result<(), CaptureError> {
        self.writer.flush().map_err(CaptureError::Io)
    }
}

/// Streaming reader that allocates at most one bounded datagram at a time.
#[derive(Debug)]
pub struct CaptureReader<R> {
    reader: R,
    header: CaptureHeader,
    bytes_read: u64,
    previous_delta_us: Option<u64>,
}

impl<R> CaptureReader<R>
where
    R: Read,
{
    /// Reads and validates a capture header.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError`] for truncated, invalid, or unsupported metadata.
    pub fn new(mut reader: R) -> Result<Self, CaptureError> {
        let mut magic = [0; 8];
        reader.read_exact(&mut magic)?;
        if magic != MAGIC {
            return Err(CaptureError::InvalidMagic);
        }

        let version = read_u16(&mut reader)?;
        if version != FORMAT_VERSION {
            return Err(CaptureError::UnsupportedVersion {
                expected: FORMAT_VERSION,
                actual: version,
            });
        }

        let mut adapter_len = [0];
        reader.read_exact(&mut adapter_len)?;
        let mut adapter_bytes = vec![0; usize::from(adapter_len[0])];
        reader.read_exact(&mut adapter_bytes)?;
        let adapter_value =
            String::from_utf8(adapter_bytes).map_err(|_| CaptureError::InvalidAdapterIdEncoding)?;
        let adapter_id = AdapterId::new(adapter_value.clone())
            .map_err(|_| CaptureError::InvalidAdapterId(adapter_value))?;
        let created_unix_ms = read_u64(&mut reader)?;
        let bytes_read = 8_u64 + 2 + 1 + u64::from(adapter_len[0]) + 8;

        Ok(Self {
            reader,
            header: CaptureHeader::new(adapter_id, created_unix_ms),
            bytes_read,
            previous_delta_us: None,
        })
    }

    /// Returns validated capture metadata.
    #[must_use]
    pub const fn header(&self) -> &CaptureHeader {
        &self.header
    }

    /// Reads the next record, returning `None` only at a clean record boundary.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError`] for truncated, oversized, or non-monotonic data.
    pub fn next_record(&mut self) -> Result<Option<CaptureRecord>, CaptureError> {
        let mut delta_bytes = [0; 8];
        let first_count = self.reader.read(&mut delta_bytes[..1])?;
        if first_count == 0 {
            return Ok(None);
        }
        self.reader.read_exact(&mut delta_bytes[1..])?;
        let delta_us = u64::from_le_bytes(delta_bytes);
        let datagram_len = usize::from(read_u16(&mut self.reader)?);
        validate_record_size(datagram_len)?;

        let record_len = 8_u64
            + 2
            + u64::try_from(datagram_len).map_err(|_| CaptureError::DatagramTooLarge {
                actual: datagram_len,
                maximum: MAX_DATAGRAM_LEN,
            })?;
        let next_size =
            self.bytes_read
                .checked_add(record_len)
                .ok_or(CaptureError::CaptureTooLarge {
                    actual: u64::MAX,
                    maximum: MAX_CAPTURE_BYTES,
                })?;
        if next_size > MAX_CAPTURE_BYTES {
            return Err(CaptureError::CaptureTooLarge {
                actual: next_size,
                maximum: MAX_CAPTURE_BYTES,
            });
        }
        if let Some(previous_us) = self.previous_delta_us
            && delta_us < previous_us
        {
            return Err(CaptureError::NonMonotonicRecord {
                previous_us,
                actual_us: delta_us,
            });
        }

        let mut datagram = vec![0; datagram_len];
        self.reader.read_exact(&mut datagram)?;
        self.bytes_read = next_size;
        self.previous_delta_us = Some(delta_us);
        Ok(Some(CaptureRecord::new(delta_us, datagram)))
    }
}

fn validate_record_size(datagram_len: usize) -> Result<(), CaptureError> {
    if datagram_len > MAX_DATAGRAM_LEN {
        return Err(CaptureError::DatagramTooLarge {
            actual: datagram_len,
            maximum: MAX_DATAGRAM_LEN,
        });
    }
    Ok(())
}

fn read_u16(reader: &mut impl Read) -> Result<u16, io::Error> {
    let mut bytes = [0; 2];
    reader.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u64(reader: &mut impl Read) -> Result<u64, io::Error> {
    let mut bytes = [0; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}
