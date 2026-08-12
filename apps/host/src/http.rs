use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};
use opencarpanel_config::{LayoutRepository, MAX_LAYOUT_BYTES};
use opencarpanel_protocol::PROTOCOL_VERSION;
use serde::Serialize;
use tokio::sync::Semaphore;

use crate::{
    HostState, diagnostics, layout_api, pairing::PairingService, static_assets, websocket,
};

#[derive(Debug, Clone)]
pub(crate) struct HttpState {
    pub(crate) host: Arc<HostState>,
    pub(crate) pairing: Arc<PairingService>,
    pub(crate) layouts: Arc<LayoutRepository>,
    pub(crate) websocket_connections: Arc<Semaphore>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: &'static str,
    protocol_version: u16,
    adapter: String,
    adapter_selection: String,
    active_adapter: Option<String>,
    supported_adapters: Vec<String>,
}

pub(crate) fn router(
    host: Arc<HostState>,
    pairing: Arc<PairingService>,
    layouts: Arc<LayoutRepository>,
) -> Router {
    Router::new()
        .route("/", get(static_assets::root))
        .route("/api/v1/health", get(health))
        .route("/api/v1/diagnostics", get(diagnostics::get))
        .route("/api/v1/ws", get(websocket::upgrade))
        .route(
            "/api/v1/layouts/{layout_id}",
            get(layout_api::get_layout).put(layout_api::put_layout),
        )
        .route("/{*path}", get(static_assets::path))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_LAYOUT_BYTES))
        .with_state(HttpState {
            host,
            pairing,
            layouts,
            websocket_connections: Arc::new(Semaphore::new(websocket::MAX_WEBSOCKET_CONNECTIONS)),
        })
}

async fn health(State(state): State<HttpState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        protocol_version: PROTOCOL_VERSION,
        adapter: state.host.adapter_id().to_owned(),
        adapter_selection: state.host.adapter_selection().as_str().to_owned(),
        active_adapter: state.host.active_adapter_id().map(str::to_owned),
        supported_adapters: state
            .host
            .supported_adapters()
            .iter()
            .map(|adapter| adapter.id().to_owned())
            .collect(),
    })
}
