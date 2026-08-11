use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use opencarpanel_adapter_api::{AdapterError, GameAdapter};
use opencarpanel_adapter_f1_24::{ADAPTER_ID, F1_24Adapter};
use opencarpanel_telemetry_core::TelemetryReducer;
use tokio::{
    net::{TcpListener, UdpSocket},
    sync::watch,
    task::{JoinError, JoinHandle},
};

use crate::{HostState, http, shutdown::wait_for_shutdown, telemetry::run_udp_ingestion};

/// Network endpoints used when binding the Host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostConfig {
    /// HTTP/WebSocket listen address.
    pub http_address: SocketAddr,
    /// F1 UDP telemetry listen address.
    pub udp_address: SocketAddr,
}

impl Default for HostConfig {
    fn default() -> Self {
        let any_v4 = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
        Self {
            http_address: SocketAddr::new(any_v4, 20_778),
            udp_address: SocketAddr::new(any_v4, 20_777),
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
                write!(formatter, "failed to initialize F1 24 adapter: {error}")
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
    shutdown_sender: watch::Sender<bool>,
    supervisor: Option<JoinHandle<Result<(), HostError>>>,
}

impl RunningHost {
    /// Returns the bound HTTP address, including an assigned ephemeral port.
    #[must_use]
    pub const fn http_address(&self) -> SocketAddr {
        self.http_address
    }

    /// Returns the bound F1 UDP address.
    #[must_use]
    pub const fn udp_address(&self) -> SocketAddr {
        self.udp_address
    }

    /// Returns shared latest telemetry and local counters.
    #[must_use]
    pub const fn state(&self) -> &Arc<HostState> {
        &self.state
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
    let http_listener = TcpListener::bind(config.http_address)
        .await
        .map_err(|source| HostError::Bind {
            service: "HTTP",
            address: config.http_address,
            source,
        })?;
    let udp_socket = UdpSocket::bind(config.udp_address)
        .await
        .map_err(|source| HostError::Bind {
            service: "F1 UDP",
            address: config.udp_address,
            source,
        })?;
    spawn_host(http_listener, udp_socket).await
}

/// Starts the Host from caller-owned, pre-bound sockets.
///
/// This injection point keeps integration tests deterministic and port-safe.
///
/// # Errors
///
/// Returns [`HostError`] if socket addresses or adapter initialization fail.
pub async fn spawn_host(
    http_listener: TcpListener,
    udp_socket: UdpSocket,
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
            service: "F1 UDP local address",
            source,
        })?;
    let adapter = F1_24Adapter::new().map_err(HostError::Adapter)?;
    let reducer = TelemetryReducer::with_game_id(ADAPTER_ID);
    let state = Arc::new(HostState::new(
        adapter.descriptor().id.as_str(),
        reducer.snapshot().clone(),
    ));
    let (shutdown_sender, shutdown_receiver) = watch::channel(false);
    let supervisor_state = Arc::clone(&state);
    let supervisor = tokio::spawn(run_supervised(
        http_listener,
        udp_socket,
        shutdown_receiver,
        supervisor_state,
        adapter,
        reducer,
    ));

    Ok(RunningHost {
        http_address,
        udp_address,
        state,
        shutdown_sender,
        supervisor: Some(supervisor),
    })
}

async fn run_supervised(
    http_listener: TcpListener,
    udp_socket: UdpSocket,
    shutdown: watch::Receiver<bool>,
    state: Arc<HostState>,
    adapter: F1_24Adapter,
    reducer: TelemetryReducer,
) -> Result<(), HostError> {
    let http_shutdown = shutdown.clone();
    let udp_shutdown = shutdown;
    let http_state = Arc::clone(&state);
    let http_service = async move {
        axum::serve(http_listener, http::router(http_state))
            .with_graceful_shutdown(wait_for_shutdown(http_shutdown))
            .await
            .map_err(|source| HostError::Runtime {
                service: "HTTP",
                source,
            })
    };
    let udp_service = async move {
        run_udp_ingestion(udp_socket, udp_shutdown, state, adapter, reducer)
            .await
            .map_err(|source| HostError::Runtime {
                service: "F1 UDP",
                source,
            })
    };

    tokio::try_join!(http_service, udp_service)?;
    Ok(())
}
