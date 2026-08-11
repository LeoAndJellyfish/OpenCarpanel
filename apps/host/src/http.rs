use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};
use opencarpanel_config::{LayoutRepository, MAX_LAYOUT_BYTES};
use opencarpanel_protocol::PROTOCOL_VERSION;
use serde::Serialize;

use crate::{HostState, layout_api, pairing::PairingService, websocket};

#[derive(Debug, Clone)]
pub(crate) struct HttpState {
    pub(crate) host: Arc<HostState>,
    pub(crate) pairing: Arc<PairingService>,
    pub(crate) layouts: Arc<LayoutRepository>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: &'static str,
    protocol_version: u16,
    adapter: String,
}

pub(crate) fn router(
    host: Arc<HostState>,
    pairing: Arc<PairingService>,
    layouts: Arc<LayoutRepository>,
) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/ws", get(websocket::upgrade))
        .route(
            "/api/v1/layouts/{layout_id}",
            get(layout_api::get_layout).put(layout_api::put_layout),
        )
        .layer(axum::extract::DefaultBodyLimit::max(MAX_LAYOUT_BYTES))
        .with_state(HttpState {
            host,
            pairing,
            layouts,
        })
}

async fn health(State(state): State<HttpState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        protocol_version: PROTOCOL_VERSION,
        adapter: state.host.adapter_id().to_owned(),
    })
}
