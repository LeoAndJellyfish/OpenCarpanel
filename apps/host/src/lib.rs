//! Supervised local UDP and HTTP runtime for `OpenCarpanel`.

mod app;
mod diagnostics;
mod events;
mod http;
mod layout_api;
mod onboarding;
mod pairing;
mod shutdown;
mod static_assets;
mod telemetry;
mod websocket;

pub use app::{
    HostConfig, HostError, RunningHost, bind_host, spawn_host, spawn_host_with_layout_repository,
};
pub use events::EVENT_BUFFER_CAPACITY;
pub use onboarding::{pairing_url, terminal_qr};
pub use pairing::PairingError;
pub use telemetry::{HostMetrics, HostState};
pub use websocket::MAX_WEBSOCKET_CONNECTIONS;
