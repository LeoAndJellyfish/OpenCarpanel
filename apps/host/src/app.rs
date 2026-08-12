use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
    net::{AddrParseError, IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use opencarpanel_adapter_api::AdapterError;
use opencarpanel_config::{HostSettings, LayoutRepository, ValidationError};
use opencarpanel_telemetry_core::TelemetrySnapshot;
use tokio::{
    net::{TcpListener, UdpSocket},
    sync::{Semaphore, watch},
    task::{JoinError, JoinHandle},
};

use crate::{
    HostState, PairedDevice, PairingError,
    adapters::{AdapterRegistry, AdapterSelection},
    http,
    pairing::PairingService,
    shutdown::wait_for_shutdown,
    telemetry::run_udp_ingestion,
};

/// Network endpoints used when binding the Host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostConfig {
    /// HTTP/WebSocket listen address.
    pub http_address: SocketAddr,
    /// Shared game-telemetry UDP listen address.
    pub udp_address: SocketAddr,
    /// Automatic detection or one fixed game adapter.
    pub adapter_selection: AdapterSelection,
    /// Maximum per-client latest-state publication rate.
    pub snapshot_hz_limit: u16,
    /// Persistent layouts and settings directory.
    pub data_directory: PathBuf,
}

impl Default for HostConfig {
    fn default() -> Self {
        let any_v4 = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
        Self {
            http_address: SocketAddr::new(any_v4, 20_778),
            udp_address: SocketAddr::new(any_v4, 20_777),
            adapter_selection: AdapterSelection::Auto,
            snapshot_hz_limit: 60,
            data_directory: default_data_directory(),
        }
    }
}

impl HostConfig {
    /// Builds a runtime config from validated persisted settings.
    ///
    /// # Errors
    ///
    /// Returns [`HostSettingsError`] for invalid settings, socket text, or a
    /// game selection not compiled into this Host.
    pub fn from_settings(
        settings: &HostSettings,
        data_directory: impl Into<PathBuf>,
    ) -> Result<Self, HostSettingsError> {
        settings.validate().map_err(HostSettingsError::Validation)?;
        Ok(Self {
            http_address: settings
                .http_bind
                .parse()
                .map_err(HostSettingsError::HttpAddress)?,
            udp_address: settings
                .udp_bind
                .parse()
                .map_err(HostSettingsError::UdpAddress)?,
            adapter_selection: settings
                .adapter_selection
                .parse()
                .map_err(HostSettingsError::AdapterSelection)?,
            snapshot_hz_limit: settings.snapshot_hz,
            data_directory: data_directory.into(),
        })
    }

    /// Applies the same developer and automation overrides used by both entry points.
    ///
    /// Persisted settings remain the user-facing source of truth. These environment
    /// variables are intentionally applied afterwards so integration tests can bind
    /// ephemeral ports without rewriting a real profile.
    ///
    /// # Errors
    ///
    /// Returns [`HostEnvironmentError`] when an override cannot be parsed or is
    /// outside the supported publication-rate set.
    pub fn apply_environment_overrides(&mut self) -> Result<(), HostEnvironmentError> {
        if let Ok(address) = std::env::var("OPENCARPANEL_HTTP_BIND") {
            self.http_address = address.parse().map_err(HostEnvironmentError::HttpAddress)?;
        }
        if let Ok(address) = std::env::var("OPENCARPANEL_UDP_BIND") {
            self.udp_address = address.parse().map_err(HostEnvironmentError::UdpAddress)?;
        }
        if let Ok(selection) = std::env::var("OPENCARPANEL_GAME") {
            self.adapter_selection = selection
                .parse()
                .map_err(HostEnvironmentError::AdapterSelection)?;
        }
        if let Ok(snapshot_hz) = std::env::var("OPENCARPANEL_SNAPSHOT_HZ") {
            let snapshot_hz = snapshot_hz
                .parse::<u16>()
                .map_err(HostEnvironmentError::SnapshotRate)?;
            if !matches!(snapshot_hz, 20 | 30 | 60) {
                return Err(HostEnvironmentError::UnsupportedSnapshotRate(snapshot_hz));
            }
            self.snapshot_hz_limit = snapshot_hz;
        }
        Ok(())
    }
}

/// Invalid environment override shared by desktop and headless startup.
#[derive(Debug)]
#[non_exhaustive]
pub enum HostEnvironmentError {
    /// `OPENCARPANEL_HTTP_BIND` is not a socket address.
    HttpAddress(AddrParseError),
    /// `OPENCARPANEL_UDP_BIND` is not a socket address.
    UdpAddress(AddrParseError),
    /// `OPENCARPANEL_GAME` names no compiled adapter selection.
    AdapterSelection(crate::ParseAdapterSelectionError),
    /// `OPENCARPANEL_SNAPSHOT_HZ` is not an integer.
    SnapshotRate(std::num::ParseIntError),
    /// Parsed snapshot rate is not one of the bounded supported values.
    UnsupportedSnapshotRate(u16),
}

impl Display for HostEnvironmentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::HttpAddress(error) => {
                write!(formatter, "invalid OPENCARPANEL_HTTP_BIND: {error}")
            }
            Self::UdpAddress(error) => {
                write!(formatter, "invalid OPENCARPANEL_UDP_BIND: {error}")
            }
            Self::AdapterSelection(error) => {
                write!(formatter, "invalid OPENCARPANEL_GAME: {error}")
            }
            Self::SnapshotRate(error) => {
                write!(formatter, "invalid OPENCARPANEL_SNAPSHOT_HZ: {error}")
            }
            Self::UnsupportedSnapshotRate(rate) => write!(
                formatter,
                "OPENCARPANEL_SNAPSHOT_HZ must be 20, 30, or 60; got {rate}"
            ),
        }
    }
}

impl Error for HostEnvironmentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::HttpAddress(error) | Self::UdpAddress(error) => Some(error),
            Self::AdapterSelection(error) => Some(error),
            Self::SnapshotRate(error) => Some(error),
            Self::UnsupportedSnapshotRate(_) => None,
        }
    }
}

/// Failure converting persisted settings into typed Host runtime config.
#[derive(Debug)]
#[non_exhaustive]
pub enum HostSettingsError {
    /// Model-level validation failure.
    Validation(ValidationError),
    /// HTTP listener address failed to parse.
    HttpAddress(AddrParseError),
    /// UDP listener address failed to parse.
    UdpAddress(AddrParseError),
    /// Fixed adapter selection failed to parse.
    AdapterSelection(crate::ParseAdapterSelectionError),
}

impl Display for HostSettingsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => Display::fmt(error, formatter),
            Self::HttpAddress(error) => write!(formatter, "invalid HTTP bind address: {error}"),
            Self::UdpAddress(error) => write!(formatter, "invalid UDP bind address: {error}"),
            Self::AdapterSelection(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for HostSettingsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Validation(error) => Some(error),
            Self::HttpAddress(error) | Self::UdpAddress(error) => Some(error),
            Self::AdapterSelection(error) => Some(error),
        }
    }
}

/// Host startup, runtime, or supervised-task failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum HostError {
    /// A configured listener could not bind.
    Bind {
        /// Human-readable local service name.
        service: &'static str,
        /// Address that could not be bound.
        address: SocketAddr,
        /// Underlying operating-system error.
        source: io::Error,
    },
    /// A long-running network service exited with an I/O failure.
    Runtime {
        /// Service that failed.
        service: &'static str,
        /// Underlying operating-system error.
        source: io::Error,
    },
    /// Built-in adapter metadata could not initialize.
    Adapter(AdapterError),
    /// Persistent paired-device state could not be opened safely.
    DeviceStore {
        /// Application data directory containing `clients.json`.
        path: PathBuf,
        /// Underlying filesystem failure.
        source: io::Error,
    },
    /// The owned supervisor task panicked or was cancelled unexpectedly.
    Join(JoinError),
}

impl Display for HostError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind {
                service,
                address,
                source,
            } => write!(
                formatter,
                "failed to bind {service} at {address}: {source}; check port availability and firewall settings"
            ),
            Self::Runtime { service, source } => {
                write!(formatter, "{service} runtime failed: {source}")
            }
            Self::Adapter(error) => {
                write!(formatter, "failed to initialize game adapters: {error}")
            }
            Self::DeviceStore { path, source } => write!(
                formatter,
                "failed to load paired devices from {}: {source}",
                path.display()
            ),
            Self::Join(error) => write!(formatter, "Host supervisor task failed: {error}"),
        }
    }
}

impl Error for HostError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Bind { source, .. }
            | Self::Runtime { source, .. }
            | Self::DeviceStore { source, .. } => Some(source),
            Self::Adapter(error) => Some(error),
            Self::Join(error) => Some(error),
        }
    }
}

/// Owned running Host with explicit graceful shutdown.
#[derive(Debug)]
pub struct RunningHost {
    http_address: SocketAddr,
    udp_address: SocketAddr,
    state: Arc<HostState>,
    pairing: Arc<PairingService>,
    websocket_connections: Arc<Semaphore>,
    shutdown_sender: watch::Sender<bool>,
    supervisor: Option<JoinHandle<Result<(), HostError>>>,
    _temporary_data: Option<tempfile::TempDir>,
}

impl RunningHost {
    /// Returns the bound HTTP address, including an assigned ephemeral port.
    #[must_use]
    pub const fn http_address(&self) -> SocketAddr {
        self.http_address
    }

    /// Returns the bound shared game-telemetry UDP address.
    #[must_use]
    pub const fn udp_address(&self) -> SocketAddr {
        self.udp_address
    }

    /// Returns shared latest telemetry and local counters.
    #[must_use]
    pub const fn state(&self) -> &Arc<HostState> {
        &self.state
    }

    /// Issues a URL-safe one-time pairing token with the requested lifetime.
    ///
    /// # Errors
    ///
    /// Returns [`PairingError`] if the operating system cannot provide secure
    /// random bytes.
    pub async fn issue_pairing_token(
        &self,
        ttl: std::time::Duration,
    ) -> Result<String, PairingError> {
        self.pairing.issue_token(ttl).await
    }

    /// Returns non-secret metadata for remembered dashboard devices.
    pub async fn paired_devices(&self) -> Vec<PairedDevice> {
        self.pairing.devices().await
    }

    /// Revokes one remembered device by its non-secret local id.
    ///
    /// # Errors
    ///
    /// Returns [`PairingError::StorageUnavailable`] if the revocation cannot
    /// be committed atomically.
    pub async fn revoke_device(&self, id: &str) -> Result<bool, PairingError> {
        self.pairing.revoke_device(id).await
    }

    /// Revokes all remembered dashboard devices.
    ///
    /// # Errors
    ///
    /// Returns [`PairingError::StorageUnavailable`] if the empty registry
    /// cannot be committed atomically.
    pub async fn revoke_all_devices(&self) -> Result<usize, PairingError> {
        self.pairing.revoke_all_devices().await
    }

    /// Returns a point-in-time non-secret diagnostic view.
    #[must_use]
    pub fn diagnostics(&self) -> crate::HostDiagnostics {
        crate::diagnostics::snapshot(
            &self.state,
            crate::MAX_WEBSOCKET_CONNECTIONS
                .saturating_sub(self.websocket_connections.available_permits()),
        )
    }

    /// Signals every service and waits for the owned supervisor to exit.
    ///
    /// # Errors
    ///
    /// Returns [`HostError`] if a runtime service or the supervisor failed.
    pub async fn shutdown(mut self) -> Result<(), HostError> {
        let _receiver_count = self.shutdown_sender.send_replace(true);
        if let Some(supervisor) = self.supervisor.take() {
            supervisor.await.map_err(HostError::Join)??;
        }
        Ok(())
    }
}

impl Drop for RunningHost {
    fn drop(&mut self) {
        let _receiver_count = self.shutdown_sender.send_replace(true);
        if let Some(supervisor) = self.supervisor.take() {
            supervisor.abort();
        }
    }
}

/// Binds configured sockets and starts the supervised Host.
///
/// # Errors
///
/// Returns [`HostError::Bind`] with actionable address context or an adapter
/// initialization error.
pub async fn bind_host(config: HostConfig) -> Result<RunningHost, HostError> {
    let HostConfig {
        http_address,
        udp_address,
        adapter_selection,
        snapshot_hz_limit,
        data_directory,
    } = config;
    let pairing =
        PairingService::load(&data_directory).map_err(|source| HostError::DeviceStore {
            path: data_directory.clone(),
            source,
        })?;
    let http_listener =
        TcpListener::bind(http_address)
            .await
            .map_err(|source| HostError::Bind {
                service: "HTTP",
                address: http_address,
                source,
            })?;
    let udp_socket = UdpSocket::bind(udp_address)
        .await
        .map_err(|source| HostError::Bind {
            service: "game telemetry UDP",
            address: udp_address,
            source,
        })?;
    spawn_host_inner(
        http_listener,
        udp_socket,
        LayoutRepository::new(data_directory),
        pairing,
        adapter_selection,
        snapshot_hz_limit,
        None,
    )
}

/// Starts the Host from caller-owned, pre-bound sockets.
///
/// This injection point keeps integration tests deterministic and port-safe.
///
/// # Errors
///
/// Returns [`HostError`] if socket addresses or adapter initialization fail.
pub fn spawn_host(
    http_listener: TcpListener,
    udp_socket: UdpSocket,
) -> Result<RunningHost, HostError> {
    let temporary_data = tempfile::tempdir().map_err(|source| HostError::Runtime {
        service: "temporary Host data directory",
        source,
    })?;
    let layouts = LayoutRepository::new(temporary_data.path());
    let pairing =
        PairingService::load(temporary_data.path()).map_err(|source| HostError::DeviceStore {
            path: temporary_data.path().to_path_buf(),
            source,
        })?;
    spawn_host_inner(
        http_listener,
        udp_socket,
        layouts,
        pairing,
        AdapterSelection::Auto,
        60,
        Some(temporary_data),
    )
}

/// Starts the Host from pre-bound sockets with an explicit game selection.
///
/// # Errors
///
/// Returns [`HostError`] if socket addresses or adapter initialization fail.
pub fn spawn_host_with_adapter_selection(
    http_listener: TcpListener,
    udp_socket: UdpSocket,
    adapter_selection: AdapterSelection,
) -> Result<RunningHost, HostError> {
    let temporary_data = tempfile::tempdir().map_err(|source| HostError::Runtime {
        service: "temporary Host data directory",
        source,
    })?;
    let layouts = LayoutRepository::new(temporary_data.path());
    let pairing =
        PairingService::load(temporary_data.path()).map_err(|source| HostError::DeviceStore {
            path: temporary_data.path().to_path_buf(),
            source,
        })?;
    spawn_host_inner(
        http_listener,
        udp_socket,
        layouts,
        pairing,
        adapter_selection,
        60,
        Some(temporary_data),
    )
}

/// Starts the Host with an injected persistent layout repository.
///
/// # Errors
///
/// Returns [`HostError`] if socket addresses or adapter initialization fail.
pub fn spawn_host_with_layout_repository(
    http_listener: TcpListener,
    udp_socket: UdpSocket,
    layouts: LayoutRepository,
) -> Result<RunningHost, HostError> {
    spawn_host_inner(
        http_listener,
        udp_socket,
        layouts,
        PairingService::ephemeral(),
        AdapterSelection::Auto,
        60,
        None,
    )
}

fn spawn_host_inner(
    http_listener: TcpListener,
    udp_socket: UdpSocket,
    layouts: LayoutRepository,
    pairing: PairingService,
    adapter_selection: AdapterSelection,
    snapshot_hz_limit: u16,
    temporary_data: Option<tempfile::TempDir>,
) -> Result<RunningHost, HostError> {
    let http_address = http_listener
        .local_addr()
        .map_err(|source| HostError::Runtime {
            service: "HTTP local address",
            source,
        })?;
    let udp_address = udp_socket
        .local_addr()
        .map_err(|source| HostError::Runtime {
            service: "game telemetry UDP local address",
            source,
        })?;
    let adapters = AdapterRegistry::new(adapter_selection).map_err(HostError::Adapter)?;
    let state = Arc::new(HostState::new(
        adapter_selection,
        adapters.supported_adapters(),
        TelemetrySnapshot::default(),
    ));
    let pairing = Arc::new(pairing);
    let layouts = Arc::new(layouts);
    let websocket_connections = Arc::new(Semaphore::new(crate::MAX_WEBSOCKET_CONNECTIONS));
    let (shutdown_sender, shutdown_receiver) = watch::channel(false);
    let supervisor_state = Arc::clone(&state);
    let supervisor_pairing = Arc::clone(&pairing);
    let supervisor_layouts = Arc::clone(&layouts);
    let supervisor_connections = Arc::clone(&websocket_connections);
    let supervisor = tokio::spawn(run_supervised(
        http_listener,
        udp_socket,
        shutdown_receiver,
        RuntimeServices {
            state: supervisor_state,
            pairing: supervisor_pairing,
            layouts: supervisor_layouts,
            websocket_connections: supervisor_connections,
            snapshot_hz_limit,
            adapters,
        },
    ));

    Ok(RunningHost {
        http_address,
        udp_address,
        state,
        pairing,
        websocket_connections,
        shutdown_sender,
        supervisor: Some(supervisor),
        _temporary_data: temporary_data,
    })
}

struct RuntimeServices {
    state: Arc<HostState>,
    pairing: Arc<PairingService>,
    layouts: Arc<LayoutRepository>,
    websocket_connections: Arc<Semaphore>,
    snapshot_hz_limit: u16,
    adapters: AdapterRegistry,
}

async fn run_supervised(
    http_listener: TcpListener,
    udp_socket: UdpSocket,
    shutdown: watch::Receiver<bool>,
    services: RuntimeServices,
) -> Result<(), HostError> {
    let RuntimeServices {
        state,
        pairing,
        layouts,
        websocket_connections,
        snapshot_hz_limit,
        adapters,
    } = services;
    let http_shutdown = shutdown.clone();
    let udp_shutdown = shutdown;
    let http_state = Arc::clone(&state);
    let http_service = async move {
        axum::serve(
            http_listener,
            http::router(
                http_state,
                pairing,
                layouts,
                websocket_connections,
                snapshot_hz_limit,
            ),
        )
        .with_graceful_shutdown(wait_for_shutdown(http_shutdown))
        .await
        .map_err(|source| HostError::Runtime {
            service: "HTTP",
            source,
        })
    };
    let udp_service = async move {
        run_udp_ingestion(udp_socket, udp_shutdown, state, adapters)
            .await
            .map_err(|source| HostError::Runtime {
                service: "game telemetry UDP",
                source,
            })
    };

    tokio::try_join!(http_service, udp_service)?;
    Ok(())
}

/// Returns the shared per-user application-data directory.
#[must_use]
pub fn default_data_directory() -> PathBuf {
    if let Some(configured) = std::env::var_os("OPENCARPANEL_DATA_DIR") {
        return PathBuf::from(configured);
    }

    #[cfg(target_os = "windows")]
    if let Some(local_data) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(local_data).join("OpenCarpanel");
    }

    #[cfg(target_os = "macos")]
    if let Some(user_home) = std::env::var_os("HOME") {
        return PathBuf::from(user_home)
            .join("Library")
            .join("Application Support")
            .join("OpenCarpanel");
    }

    std::env::temp_dir().join("OpenCarpanel")
}
