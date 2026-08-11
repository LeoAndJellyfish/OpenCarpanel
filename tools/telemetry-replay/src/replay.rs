use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, Read},
    thread,
    time::{Duration, Instant},
};

use crate::{CaptureError, CaptureReader, CaptureRecord};

/// Clock abstraction used to make replay scheduling deterministic in tests.
pub trait ReplayClock {
    /// Waits until `elapsed` relative to replay start.
    fn wait_until(&mut self, elapsed: Duration);
}

/// Target that accepts replayed UDP payloads.
pub trait DatagramSink {
    /// Sends one payload.
    ///
    /// # Errors
    ///
    /// Returns an I/O error from the underlying target.
    fn send(&mut self, datagram: &[u8]) -> io::Result<()>;
}

/// Failure while scheduling or sending a capture.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReplayError {
    /// Speed must be finite and non-negative.
    InvalidSpeed(f64),
    /// A scaled record time cannot be represented by `Duration`.
    InvalidScaledTime {
        /// Original record offset.
        delta_us: u64,
        /// Requested speed multiplier.
        speed: f64,
    },
    /// Capture stream is invalid.
    Capture(CaptureError),
    /// Datagram target failed.
    Send(io::Error),
}

impl Display for ReplayError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpeed(speed) => {
                write!(
                    formatter,
                    "replay speed must be finite and non-negative, got {speed}"
                )
            }
            Self::InvalidScaledTime { delta_us, speed } => write!(
                formatter,
                "capture time {delta_us} us cannot be represented at speed {speed}"
            ),
            Self::Capture(error) => Display::fmt(error, formatter),
            Self::Send(error) => write!(formatter, "failed to send replay datagram: {error}"),
        }
    }
}

impl Error for ReplayError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Capture(error) => Some(error),
            Self::Send(error) => Some(error),
            Self::InvalidSpeed(_) | Self::InvalidScaledTime { .. } => None,
        }
    }
}

/// Replays in-memory records in order.
///
/// `speed = 0` disables waiting for deterministic immediate playback.
///
/// # Errors
///
/// Returns [`ReplayError`] for invalid speed/timing or sink failures.
pub fn replay_records(
    records: &[CaptureRecord],
    speed: f64,
    clock: &mut impl ReplayClock,
    sink: &mut impl DatagramSink,
) -> Result<u64, ReplayError> {
    validate_speed(speed)?;
    let mut previous_us = None;
    let mut sent = 0_u64;

    for record in records {
        if previous_us.is_some_and(|previous| record.delta_us() < previous) {
            return Err(ReplayError::Capture(CaptureError::NonMonotonicRecord {
                previous_us: previous_us.unwrap_or_default(),
                actual_us: record.delta_us(),
            }));
        }
        schedule(record, speed, clock, sink)?;
        previous_us = Some(record.delta_us());
        sent = sent.saturating_add(1);
    }
    Ok(sent)
}

/// Streams and replays records without loading the capture into memory.
///
/// # Errors
///
/// Returns [`ReplayError`] for an invalid capture, timing, speed, or target.
pub fn replay_stream<R>(
    reader: &mut CaptureReader<R>,
    speed: f64,
    clock: &mut impl ReplayClock,
    sink: &mut impl DatagramSink,
) -> Result<u64, ReplayError>
where
    R: Read,
{
    validate_speed(speed)?;
    let mut sent = 0_u64;
    while let Some(record) = reader.next_record().map_err(ReplayError::Capture)? {
        schedule(&record, speed, clock, sink)?;
        sent = sent.saturating_add(1);
    }
    Ok(sent)
}

fn validate_speed(speed: f64) -> Result<(), ReplayError> {
    if speed.is_finite() && speed >= 0.0 {
        Ok(())
    } else {
        Err(ReplayError::InvalidSpeed(speed))
    }
}

fn schedule(
    record: &CaptureRecord,
    speed: f64,
    clock: &mut impl ReplayClock,
    sink: &mut impl DatagramSink,
) -> Result<(), ReplayError> {
    if speed > 0.0 {
        let seconds = Duration::from_micros(record.delta_us()).as_secs_f64() / speed;
        let elapsed =
            Duration::try_from_secs_f64(seconds).map_err(|_| ReplayError::InvalidScaledTime {
                delta_us: record.delta_us(),
                speed,
            })?;
        clock.wait_until(elapsed);
    }
    sink.send(record.datagram()).map_err(ReplayError::Send)
}

/// Wall-clock scheduler used by the command-line replay operation.
#[derive(Debug)]
pub struct SystemReplayClock {
    started_at: Instant,
}

impl Default for SystemReplayClock {
    fn default() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }
}

impl ReplayClock for SystemReplayClock {
    fn wait_until(&mut self, elapsed: Duration) {
        if let Some(remaining) = elapsed.checked_sub(self.started_at.elapsed()) {
            thread::sleep(remaining);
        }
    }
}
