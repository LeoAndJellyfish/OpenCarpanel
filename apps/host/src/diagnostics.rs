use axum::{Json, extract::State};
use opencarpanel_protocol::PROTOCOL_VERSION;
use serde::Serialize;

use crate::{http::HttpState, websocket::MAX_WEBSOCKET_CONNECTIONS};

/// Complete non-secret runtime diagnostics shared by HTTP and desktop UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostDiagnostics {
    /// Overall local runtime status.
    pub status: &'static str,
    /// Host application version.
    pub version: &'static str,
    /// Browser wire-protocol major version.
    pub protocol_version: u16,
    /// Active adapter id, or configured selection before data arrives.
    pub adapter: String,
    /// Configured automatic or fixed source selection.
    pub adapter_selection: String,
    /// Adapter that most recently recognized a datagram.
    pub active_adapter: Option<String>,
    /// Metadata and counters for all compiled adapters.
    pub supported_adapters: Vec<AdapterDiagnostics>,
    /// Milliseconds since this Host runtime started.
    pub uptime_ms: u64,
    /// UDP and publication counters.
    pub telemetry: TelemetryDiagnostics,
    /// Paired WebSocket connection occupancy.
    pub connections: ConnectionDiagnostics,
}

/// UDP ingestion and publication counters safe for local export.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryDiagnostics {
    /// All UDP datagrams observed on the listener.
    pub packets_received: u64,
    /// Datagrams accepted by a compiled adapter.
    pub packets_recognized: u64,
    /// Recognized-protocol datagrams rejected as invalid.
    pub packet_errors: u64,
    /// Age of the newest datagram according to the Host monotonic clock.
    pub last_packet_age_ms: Option<u64>,
    /// Latest-state replacements since startup.
    pub snapshots_published: u64,
    /// Event replays that exceeded the bounded buffer.
    pub event_resyncs: u64,
}

/// Per-adapter compatibility and ingress diagnostics.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterDiagnostics {
    /// Stable adapter id.
    pub id: String,
    /// Product-facing game name.
    pub display_name: String,
    /// Game ingress protocol versions accepted by this build.
    pub protocol_version: String,
    /// Canonical telemetry fields supplied by the adapter.
    pub capabilities: Vec<opencarpanel_telemetry_core::TelemetryField>,
    /// Datagrams accepted by this adapter.
    pub packets_recognized: u64,
    /// Age of this adapter's newest accepted datagram.
    pub last_packet_age_ms: Option<u64>,
}

/// Current and maximum paired WebSocket connection count.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDiagnostics {
    /// Active dashboard `WebSocket` connections.
    pub active: usize,
    /// Hard connection limit.
    pub limit: usize,
}

pub(crate) async fn get(State(state): State<HttpState>) -> Json<HostDiagnostics> {
    Json(snapshot(
        &state.host,
        MAX_WEBSOCKET_CONNECTIONS.saturating_sub(state.websocket_connections.available_permits()),
    ))
}

pub(crate) fn snapshot(host: &crate::HostState, active_connections: usize) -> HostDiagnostics {
    let metrics = host.metrics();
    let last_packet_age_ms = (metrics.packets_received > 0).then(|| {
        metrics
            .uptime_ms
            .saturating_sub(metrics.last_packet_at_us / 1_000)
    });
    let supported_adapters = host
        .supported_adapters()
        .iter()
        .enumerate()
        .map(|(index, adapter)| {
            let (packets_recognized, last_packet_at_us) =
                host.adapter_packet_metrics(index).unwrap_or((0, 0));
            AdapterDiagnostics {
                id: adapter.id().to_owned(),
                display_name: adapter.display_name().to_owned(),
                protocol_version: adapter.protocol_version().to_owned(),
                capabilities: adapter.capabilities().to_vec(),
                packets_recognized,
                last_packet_age_ms: (packets_recognized > 0)
                    .then(|| metrics.uptime_ms.saturating_sub(last_packet_at_us / 1_000)),
            }
        })
        .collect();
    HostDiagnostics {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        protocol_version: PROTOCOL_VERSION,
        adapter: host.adapter_id().to_owned(),
        adapter_selection: host.adapter_selection().as_str().to_owned(),
        active_adapter: host.active_adapter_id().map(str::to_owned),
        supported_adapters,
        uptime_ms: metrics.uptime_ms,
        telemetry: TelemetryDiagnostics {
            packets_received: metrics.packets_received,
            packets_recognized: metrics.packets_recognized,
            packet_errors: metrics.packet_errors,
            last_packet_age_ms,
            snapshots_published: metrics.snapshots_published,
            event_resyncs: metrics.event_resyncs,
        },
        connections: ConnectionDiagnostics {
            active: active_connections.min(MAX_WEBSOCKET_CONNECTIONS),
            limit: MAX_WEBSOCKET_CONNECTIONS,
        },
    }
}
