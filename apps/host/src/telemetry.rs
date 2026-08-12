use std::{
    collections::BTreeSet,
    io,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Instant,
};

use opencarpanel_protocol::EventMessage;
use opencarpanel_telemetry_core::{
    MonotonicTimestamp, TelemetryEvent, TelemetryField, TelemetrySnapshot,
};
use tokio::{
    net::UdpSocket,
    sync::{broadcast, watch},
};

use crate::{
    adapters::{AdapterRegistry, AdapterSelection, RegistryOutcome, SupportedAdapter},
    events::{EventHub, ReplayBatch},
};

const MAX_UDP_DATAGRAM_LEN: usize = 65_507;
const NO_ACTIVE_ADAPTER: usize = usize::MAX;

/// Read-only counters useful for local diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostMetrics {
    /// Milliseconds since the Host state was initialized.
    pub uptime_ms: u64,
    /// UDP datagrams received by the Host.
    pub packets_received: u64,
    /// UDP datagrams recognized by one enabled game adapter.
    pub packets_recognized: u64,
    /// Datagram decode failures.
    pub packet_errors: u64,
    /// Host-monotonic receipt time of the newest datagram.
    pub last_packet_at_us: u64,
    /// Latest-state snapshots published since startup.
    pub snapshots_published: u64,
    /// Reliable-event resumes that exceeded the bounded replay window.
    pub event_resyncs: u64,
}

/// Shared latest-value state published by the UDP ingestion loop.
#[derive(Debug)]
pub struct HostState {
    started_at: Instant,
    adapter_selection: AdapterSelection,
    supported_adapters: Vec<SupportedAdapter>,
    capabilities: Vec<TelemetryField>,
    active_adapter: AtomicUsize,
    adapter_packets: Vec<AtomicU64>,
    adapter_last_packet_at_us: Vec<AtomicU64>,
    packets_received: AtomicU64,
    packets_recognized: AtomicU64,
    packet_errors: AtomicU64,
    last_packet_at_us: AtomicU64,
    snapshots_published: AtomicU64,
    event_resyncs: AtomicU64,
    snapshot_sender: watch::Sender<Arc<TelemetrySnapshot>>,
    event_hub: EventHub,
}

impl HostState {
    pub(crate) fn new(
        adapter_selection: AdapterSelection,
        supported_adapters: Vec<SupportedAdapter>,
        snapshot: TelemetrySnapshot,
    ) -> Self {
        let capabilities = supported_adapters
            .iter()
            .flat_map(|adapter| adapter.capabilities().iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let adapter_count = supported_adapters.len();
        let (snapshot_sender, _snapshot_receiver) = watch::channel(Arc::new(snapshot));
        Self {
            started_at: Instant::now(),
            adapter_selection,
            supported_adapters,
            capabilities,
            active_adapter: AtomicUsize::new(NO_ACTIVE_ADAPTER),
            adapter_packets: (0..adapter_count).map(|_| AtomicU64::new(0)).collect(),
            adapter_last_packet_at_us: (0..adapter_count).map(|_| AtomicU64::new(0)).collect(),
            packets_received: AtomicU64::new(0),
            packets_recognized: AtomicU64::new(0),
            packet_errors: AtomicU64::new(0),
            last_packet_at_us: AtomicU64::new(0),
            snapshots_published: AtomicU64::new(0),
            event_resyncs: AtomicU64::new(0),
            snapshot_sender,
            event_hub: EventHub::new(),
        }
    }

    /// Returns the active adapter id, or the configured selection before data arrives.
    #[must_use]
    pub fn adapter_id(&self) -> &str {
        self.active_adapter_id()
            .unwrap_or_else(|| self.adapter_selection.as_str())
    }

    /// Returns the configured automatic or fixed adapter selection.
    #[must_use]
    pub const fn adapter_selection(&self) -> AdapterSelection {
        self.adapter_selection
    }

    /// Returns the adapter that most recently won source selection.
    #[must_use]
    pub fn active_adapter_id(&self) -> Option<&str> {
        let index = self.active_adapter.load(Ordering::Relaxed);
        self.supported_adapters.get(index).map(SupportedAdapter::id)
    }

    /// Returns immutable metadata for all adapters compiled into the Host.
    #[must_use]
    pub fn supported_adapters(&self) -> &[SupportedAdapter] {
        &self.supported_adapters
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
        self.snapshots_published.fetch_add(1, Ordering::Relaxed);
    }

    /// Publishes and retains one ordered reliable telemetry event.
    pub async fn publish_event(&self, event: TelemetryEvent) -> EventMessage {
        self.event_hub.publish(event).await
    }

    pub(crate) fn subscribe_events(&self) -> broadcast::Receiver<EventMessage> {
        self.event_hub.subscribe()
    }

    pub(crate) async fn replay_events_after(&self, sequence: u64) -> ReplayBatch {
        let replay = self.event_hub.replay_after(sequence).await;
        if matches!(replay, ReplayBatch::ResyncRequired(_)) {
            self.event_resyncs.fetch_add(1, Ordering::Relaxed);
        }
        replay
    }

    /// Returns a point-in-time local metrics view.
    #[must_use]
    pub fn metrics(&self) -> HostMetrics {
        HostMetrics {
            uptime_ms: self.elapsed_micros() / 1_000,
            packets_received: self.packets_received.load(Ordering::Relaxed),
            packets_recognized: self.packets_recognized.load(Ordering::Relaxed),
            packet_errors: self.packet_errors.load(Ordering::Relaxed),
            last_packet_at_us: self.last_packet_at_us.load(Ordering::Relaxed),
            snapshots_published: self.snapshots_published.load(Ordering::Relaxed),
            event_resyncs: self.event_resyncs.load(Ordering::Relaxed),
        }
    }

    fn elapsed_micros(&self) -> u64 {
        u64::try_from(self.started_at.elapsed().as_micros()).unwrap_or(u64::MAX)
    }

    pub(crate) fn adapter_packet_metrics(&self, index: usize) -> Option<(u64, u64)> {
        Some((
            self.adapter_packets.get(index)?.load(Ordering::Relaxed),
            self.adapter_last_packet_at_us
                .get(index)?
                .load(Ordering::Relaxed),
        ))
    }

    fn record_recognized_packet(&self, adapter_index: usize, received_at_us: u64) {
        self.packets_recognized.fetch_add(1, Ordering::Relaxed);
        if let (Some(packets), Some(last_packet)) = (
            self.adapter_packets.get(adapter_index),
            self.adapter_last_packet_at_us.get(adapter_index),
        ) {
            packets.fetch_add(1, Ordering::Relaxed);
            last_packet.store(received_at_us, Ordering::Relaxed);
        }
    }

    fn set_active_adapter(&self, index: usize) {
        if index < self.supported_adapters.len() {
            self.active_adapter.store(index, Ordering::Relaxed);
        }
    }
}

pub(crate) async fn run_udp_ingestion(
    socket: UdpSocket,
    mut shutdown: watch::Receiver<bool>,
    state: Arc<HostState>,
    mut adapters: AdapterRegistry,
) -> io::Result<()> {
    let mut buffer = vec![0_u8; MAX_UDP_DATAGRAM_LEN].into_boxed_slice();

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
                let received_at_us = state.elapsed_micros();
                state.packets_received.fetch_add(1, Ordering::Relaxed);
                state.last_packet_at_us.store(received_at_us, Ordering::Relaxed);
                let received_at = MonotonicTimestamp::from_micros(received_at_us);
                match adapters.decode(&buffer[..length], received_at) {
                    RegistryOutcome::Rejected => {
                        state.packet_errors.fetch_add(1, Ordering::Relaxed);
                    }
                    RegistryOutcome::Recognized {
                        adapter_index,
                        active_index,
                        snapshot,
                    } => {
                        state.record_recognized_packet(adapter_index, received_at_us);
                        if let Some(active_index) = active_index {
                            state.set_active_adapter(active_index);
                        }
                        if let Some(snapshot) = snapshot {
                            state.replace_snapshot(*snapshot);
                        }
                        for event in adapters.drain_events() {
                            let _published = state.publish_event(event).await;
                        }
                    }
                }
            }
        }
    }
}
