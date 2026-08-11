use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};
use opencarpanel_protocol::PROTOCOL_VERSION;
use serde::Serialize;

use crate::HostState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: &'static str,
    protocol_version: u16,
    adapter: String,
}

pub(crate) fn router(state: Arc<HostState>) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .with_state(state)
}

async fn health(State(state): State<Arc<HostState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        protocol_version: PROTOCOL_VERSION,
        adapter: state.adapter_id().to_owned(),
    })
}
