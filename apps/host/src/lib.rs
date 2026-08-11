//! Supervised local UDP and HTTP runtime for `OpenCarpanel`.

mod app;
mod http;
mod shutdown;
mod telemetry;

pub use app::{HostConfig, HostError, RunningHost, bind_host, spawn_host};
pub use telemetry::{HostMetrics, HostState};
