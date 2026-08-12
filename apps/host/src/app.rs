use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use opencarpanel_adapter_api::AdapterError;
use opencarpanel_config::LayoutRepository;
use opencarpanel_telemetry_core::TelemetrySnapshot;
use tokio::{
    net::{TcpListener, UdpSocket},
    sync::watch,
    task::{JoinError, JoinHandle},
};

use crate::{
    HostState, PairingError,
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
            data_directory: default_data_directory(),
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
            Self::Join(error) => write!(formatter, "Host supervisor task failed: {error}"),
        }
    }
}

impl Error for HostError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Bind { source, .. } | Self::Runtime { source, .. } => Some(source),
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
        data_directory,
    } = config;
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
        adapter_selection,
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
    spawn_host_inner(
        http_listener,
        udp_socket,
        layouts,
        AdapterSelection::Auto,
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
    spawn_host_inner(
        http_listener,
        udp_socket,
        layouts,
        adapter_selection,
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
        AdapterSelection::Auto,
        None,
    )
}

fn spawn_host_inner(
    http_listener: TcpListener,
    udp_socket: UdpSocket,
    layouts: LayoutRepository,
    adapter_selection: AdapterSelection,
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
    let pairing = Arc::new(PairingService::new());
    let layouts = Arc::new(layouts);
    let (shutdown_sender, shutdown_receiver) = watch::channel(false);
    let supervisor_state = Arc::clone(&state);
    let supervisor_pairing = Arc::clone(&pairing);
    let supervisor_layouts = Arc::clone(&layouts);
    let supervisor = tokio::spawn(run_supervised(
        http_listener,
        udp_socket,
        shutdown_receiver,
        RuntimeServices {
            state: supervisor_state,
            pairing: supervisor_pairing,
            layouts: supervisor_layouts,
            adapters,
        },
    ));

    Ok(RunningHost {
        http_address,
        udp_address,
        state,
        pairing,
        shutdown_sender,
        supervisor: Some(supervisor),
        _temporary_data: temporary_data,
    })
}

struct RuntimeServices {
    state: Arc<HostState>,
    pairing: Arc<PairingService>,
    layouts: Arc<LayoutRepository>,
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
        adapters,
    } = services;
    let http_shutdown = shutdown.clone();
    let udp_shutdown = shutdown;
    let http_state = Arc::clone(&state);
    let http_service = async move {
        axum::serve(http_listener, http::router(http_state, pairing, layouts))
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

fn default_data_directory() -> PathBuf {
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
