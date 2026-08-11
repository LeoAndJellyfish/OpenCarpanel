//! Supervised local UDP and HTTP runtime for `OpenCarpanel`.

mod app;
mod events;
mod http;
mod pairing;
mod shutdown;
mod telemetry;
mod websocket;

pub use app::{HostConfig, HostError, RunningHost, bind_host, spawn_host};
pub use events::EVENT_BUFFER_CAPACITY;
pub use pairing::PairingError;
pub use telemetry::{HostMetrics, HostState};
