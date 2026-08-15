use std::{error::Error, io, time::Duration};

use opensimdash_adapter_api::AdapterId;
use opensimdash_telemetry_replay::{
    CaptureHeader, CaptureReader, CaptureRecord, CaptureWriter, DatagramSink, ReplayClock,
    replay_records,
};

#[derive(Debug, Default)]
struct FakeClock {
    waits: Vec<Duration>,
}

impl ReplayClock for FakeClock {
    fn wait_until(&mut self, elapsed: Duration) {
        self.waits.push(elapsed);
    }
}

#[derive(Debug, Default)]
struct FakeSink {
    datagrams: Vec<Vec<u8>>,
}

impl DatagramSink for FakeSink {
    fn send(&mut self, datagram: &[u8]) -> io::Result<()> {
        self.datagrams.push(datagram.to_vec());
        Ok(())
    }
}

fn records() -> Vec<CaptureRecord> {
    vec![
        CaptureRecord::new(0, vec![1]),
        CaptureRecord::new(16_667, vec![2, 3]),
        CaptureRecord::new(33_334, vec![4]),
    ]
}

#[test]
fn capture_round_trip_preserves_header_and_records() -> Result<(), Box<dyn Error>> {
    let header = CaptureHeader::new(AdapterId::new("f1-24")?, 1_723_000_000_000);
    let mut bytes = Vec::new();
    {
        let mut writer = CaptureWriter::new(&mut bytes, &header)?;
        for record in records() {
            writer.write_record(&record)?;
        }
        writer.flush()?;
    }

    let mut reader = CaptureReader::new(bytes.as_slice())?;
    assert_eq!(reader.header(), &header);
    let mut decoded = Vec::new();
    while let Some(record) = reader.next_record()? {
        decoded.push(record);
    }
    assert_eq!(decoded, records());
    Ok(())
}

#[test]
fn replay_waits_for_monotonic_capture_times_and_preserves_order() -> Result<(), Box<dyn Error>> {
    let mut clock = FakeClock::default();
    let mut sink = FakeSink::default();

    let sent = replay_records(&records(), 1.0, &mut clock, &mut sink)?;

    assert_eq!(sent, 3);
    assert_eq!(
        clock.waits,
        [
            Duration::from_micros(0),
            Duration::from_micros(16_667),
            Duration::from_micros(33_334),
        ]
    );
    assert_eq!(sink.datagrams, [vec![1], vec![2, 3], vec![4]]);
    Ok(())
}

#[test]
fn zero_speed_replay_is_immediate_and_deterministic() -> Result<(), Box<dyn Error>> {
    let mut clock = FakeClock::default();
    let mut sink = FakeSink::default();

    replay_records(&records(), 0.0, &mut clock, &mut sink)?;

    assert!(clock.waits.is_empty());
    assert_eq!(sink.datagrams, [vec![1], vec![2, 3], vec![4]]);
    Ok(())
}

#[test]
fn writer_rejects_non_monotonic_records() -> Result<(), Box<dyn Error>> {
    let header = CaptureHeader::new(AdapterId::new("f1-24")?, 0);
    let mut bytes = Vec::new();
    let mut writer = CaptureWriter::new(&mut bytes, &header)?;
    writer.write_record(&CaptureRecord::new(20, vec![1]))?;

    assert!(
        writer
            .write_record(&CaptureRecord::new(19, vec![2]))
            .is_err()
    );
    Ok(())
}
