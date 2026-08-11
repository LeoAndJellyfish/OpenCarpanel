use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use opencarpanel_adapter_api::{AdapterOutput, GameAdapter};
use opencarpanel_adapter_f1_24::F1_24Adapter;
use opencarpanel_protocol::EventMessage;
use opencarpanel_telemetry_core::{
    MonotonicTimestamp, TelemetryEvent, TelemetryField, TelemetryReducer, TelemetrySnapshot,
};
use tokio::{
    net::UdpSocket,
    sync::{broadcast, watch},
};

use crate::events::{EventHub, ReplayBatch};

const MAX_UDP_DATAGRAM_LEN: usize = 65_507;

/// Read-only counters useful for local diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostMetrics {
    /// UDP datagrams received by the Host.
    pub packets_received: u64,
    /// Datagram decode failures.
    pub packet_errors: u64,
    /// Host-monotonic receipt time of the newest datagram.
    pub last_packet_at_us: u64,
}

/// Shared latest-value state published by the UDP ingestion loop.
#[derive(Debug)]
pub struct HostState {
    adapter_id: String,
    capabilities: Vec<TelemetryField>,
    packets_received: AtomicU64,
    packet_errors: AtomicU64,
    last_packet_at_us: AtomicU64,
    snapshot_sender: watch::Sender<Arc<TelemetrySnapshot>>,
    event_hub: EventHub,
}

impl HostState {
    pub(crate) fn new(
        adapter_id: impl Into<String>,
        capabilities: Vec<TelemetryField>,
        snapshot: TelemetrySnapshot,
    ) -> Self {
        let (snapshot_sender, _snapshot_receiver) = watch::channel(Arc::new(snapshot));
        Self {
            adapter_id: adapter_id.into(),
            capabilities,
            packets_received: AtomicU64::new(0),
            packet_errors: AtomicU64::new(0),
            last_packet_at_us: AtomicU64::new(0),
            snapshot_sender,
            event_hub: EventHub::new(),
        }
    }

    /// Returns the stable active adapter id.
    #[must_use]
    pub fn adapter_id(&self) -> &str {
        &self.adapter_id
    }

    /// Returns the active adapter's stable canonical capabilities.
    #[must_use]
    pub fn capabilities(&self) -> &[TelemetryField] {
        &self.capabilities
    }

    /// Subscribes to replaceable latest snapshots without building a backlog.
    #[must_use]
    pub fn subscribe_snapshots(&self) -> watch::Receiver<Arc<TelemetrySnapshot>> {
        self.snapshot_sender.subscribe()
    }

    /// Replaces the latest snapshot without retaining intermediate values.
    pub fn replace_snapshot(&self, snapshot: TelemetrySnapshot) {
        self.snapshot_sender.send_replace(Arc::new(snapshot));
    }

    /// Publishes and retains one ordered reliable telemetry event.
    pub async fn publish_event(&self, event: TelemetryEvent) -> EventMessage {
        self.event_hub.publish(event).await
    }

    pub(crate) fn subscribe_events(&self) -> broadcast::Receiver<EventMessage> {
        self.event_hub.subscribe()
    }

    pub(crate) async fn replay_events_after(&self, sequence: u64) -> ReplayBatch {
        self.event_hub.replay_after(sequence).await
    }

    /// Returns a point-in-time local metrics view.
    #[must_use]
    pub fn metrics(&self) -> HostMetrics {
        HostMetrics {
            packets_received: self.packets_received.load(Ordering::Relaxed),
            packet_errors: self.packet_errors.load(Ordering::Relaxed),
            last_packet_at_us: self.last_packet_at_us.load(Ordering::Relaxed),
        }
    }
}

pub(crate) async fn run_udp_ingestion(
    socket: UdpSocket,
    mut shutdown: watch::Receiver<bool>,
    state: Arc<HostState>,
    mut adapter: F1_24Adapter,
    mut reducer: TelemetryReducer,
) -> io::Result<()> {
    let started_at = Instant::now();
    let mut buffer = vec![0_u8; MAX_UDP_DATAGRAM_LEN].into_boxed_slice();
    let mut output = AdapterOutput::with_capacity(1, 4);

    loop {
        if *shutdown.borrow() {
            return Ok(());
        }
        tokio::select! {
            biased;
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return Ok(());
                }
            }
            received = socket.recv_from(&mut buffer) => {
                let (length, _source) = received?;
                let received_at_us = u64::try_from(started_at.elapsed().as_micros())
                    .unwrap_or(u64::MAX);
                state.packets_received.fetch_add(1, Ordering::Relaxed);
                state.last_packet_at_us.store(received_at_us, Ordering::Relaxed);
                output.clear();
                let received_at = MonotonicTimestamp::from_micros(received_at_us);
                if adapter
                    .decode(&buffer[..length], received_at, &mut output)
                    .is_err()
                {
                    state.packet_errors.fetch_add(1, Ordering::Relaxed);
                    continue;
                }

                let mut changed = false;
                for update in output.updates.drain(..) {
                    changed |= reducer.apply(update);
                }
                if changed {
                    state.replace_snapshot(reducer.snapshot().clone());
                }
                for event in output.events.drain(..) {
                    let _published = state.publish_event(event).await;
                }
            }
        }
    }
}
