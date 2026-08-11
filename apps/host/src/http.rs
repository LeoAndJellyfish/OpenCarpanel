use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};
use opencarpanel_protocol::PROTOCOL_VERSION;
use serde::Serialize;

use crate::{HostState, pairing::PairingService, websocket};

#[derive(Debug, Clone)]
pub(crate) struct HttpState {
    pub(crate) host: Arc<HostState>,
    pub(crate) pairing: Arc<PairingService>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: &'static str,
    protocol_version: u16,
    adapter: String,
}

pub(crate) fn router(host: Arc<HostState>, pairing: Arc<PairingService>) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/ws", get(websocket::upgrade))
        .with_state(HttpState { host, pairing })
}

async fn health(State(state): State<HttpState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        protocol_version: PROTOCOL_VERSION,
        adapter: state.host.adapter_id().to_owned(),
    })
}
