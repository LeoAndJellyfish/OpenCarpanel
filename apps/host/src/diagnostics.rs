use axum::{Json, extract::State};
use opencarpanel_protocol::PROTOCOL_VERSION;
use serde::Serialize;

use crate::{http::HttpState, websocket::MAX_WEBSOCKET_CONNECTIONS};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnosticsResponse {
    status: &'static str,
    version: &'static str,
    protocol_version: u16,
    adapter: String,
    adapter_selection: String,
    active_adapter: Option<String>,
    supported_adapters: Vec<AdapterDiagnostics>,
    uptime_ms: u64,
    telemetry: TelemetryDiagnostics,
    connections: ConnectionDiagnostics,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TelemetryDiagnostics {
    packets_received: u64,
    packets_recognized: u64,
    packet_errors: u64,
    last_packet_age_ms: Option<u64>,
    snapshots_published: u64,
    event_resyncs: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdapterDiagnostics {
    id: String,
    display_name: String,
    protocol_version: String,
    capabilities: Vec<opencarpanel_telemetry_core::TelemetryField>,
    packets_recognized: u64,
    last_packet_age_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionDiagnostics {
    active: usize,
    limit: usize,
}

pub(crate) async fn get(State(state): State<HttpState>) -> Json<DiagnosticsResponse> {
    let metrics = state.host.metrics();
    let last_packet_age_ms = (metrics.packets_received > 0).then(|| {
        metrics
            .uptime_ms
            .saturating_sub(metrics.last_packet_at_us / 1_000)
    });
    let supported_adapters = state
        .host
        .supported_adapters()
        .iter()
        .enumerate()
        .map(|(index, adapter)| {
            let (packets_recognized, last_packet_at_us) =
                state.host.adapter_packet_metrics(index).unwrap_or((0, 0));
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
    Json(DiagnosticsResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        protocol_version: PROTOCOL_VERSION,
        adapter: state.host.adapter_id().to_owned(),
        adapter_selection: state.host.adapter_selection().as_str().to_owned(),
        active_adapter: state.host.active_adapter_id().map(str::to_owned),
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
            active: MAX_WEBSOCKET_CONNECTIONS
                .saturating_sub(state.websocket_connections.available_permits()),
            limit: MAX_WEBSOCKET_CONNECTIONS,
        },
    })
}
