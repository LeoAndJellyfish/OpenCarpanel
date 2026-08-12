//! Supervised local UDP and HTTP runtime for `OpenCarpanel`.

mod adapters;
mod app;
mod diagnostics;
mod events;
mod http;
mod instance;
mod layout_api;
mod onboarding;
mod pairing;
mod shutdown;
mod static_assets;
mod telemetry;
mod websocket;

pub use adapters::{AdapterSelection, ParseAdapterSelectionError, SupportedAdapter};
pub use app::{
    HostConfig, HostError, HostSettingsError, RunningHost, bind_host, default_data_directory,
    spawn_host, spawn_host_with_adapter_selection, spawn_host_with_layout_repository,
};
pub use diagnostics::{
    AdapterDiagnostics, ConnectionDiagnostics, HostDiagnostics, TelemetryDiagnostics,
};
pub use events::EVENT_BUFFER_CAPACITY;
pub use instance::{
    InstanceError, InstanceGuard, InstanceMetadata, InstanceMode, default_runtime_directory,
};
pub use onboarding::{pairing_url, qr_svg, terminal_qr};
pub use pairing::{PairedDevice, PairingError};
pub use telemetry::{HostMetrics, HostState};
pub use websocket::MAX_WEBSOCKET_CONNECTIONS;
