use std::{
    collections::VecDeque,
    error::Error,
    fmt::{self, Display, Formatter},
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::{sync::Mutex, time::Instant};

use opencarpanel_protocol::ClientHello;

const PAIRING_TOKEN_BYTES: usize = 16;
const DEVICE_SESSION_BYTES: usize = 32;
const MAX_PENDING_PAIRINGS: usize = 16;
const MAX_DEVICE_SESSIONS: usize = 64;

#[derive(Debug)]
struct PendingPairing {
    digest: [u8; 32],
    expires_at: Instant,
}

#[derive(Debug, Default)]
struct PairingState {
    pending: VecDeque<PendingPairing>,
    device_sessions: VecDeque<[u8; 32]>,
}

/// In-memory one-time pairing and device-session authority.
#[derive(Debug, Default)]
pub(crate) struct PairingService {
    state: Mutex<PairingState>,
}

/// Successful authentication result for one WebSocket connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Authentication {
    pub(crate) new_device_session: Option<String>,
}

/// Failure while issuing or consuming local pairing credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PairingError {
    /// The operating system could not supply cryptographically secure bytes.
    RandomSourceUnavailable,
    /// Neither a one-time token nor an existing device session was supplied.
    PairingRequired,
    /// Both mutually exclusive credential types were supplied.
    ConflictingCredentials,
    /// The one-time token was unknown or had already been consumed.
    InvalidPairingToken,
    /// The one-time token matched but had passed its expiration time.
    PairingTokenExpired,
    /// The persisted device session was unknown or had been evicted.
    InvalidDeviceSession,
}

impl Display for PairingError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RandomSourceUnavailable => "the operating-system random source is unavailable",
            Self::PairingRequired => "pairing or a valid device session is required",
            Self::ConflictingCredentials => {
                "provide either a pairing token or a device session, not both"
            }
            Self::InvalidPairingToken => "the pairing token is invalid or already used",
            Self::PairingTokenExpired => "the pairing token has expired",
            Self::InvalidDeviceSession => "the device session is invalid",
        })
    }
}

impl Error for PairingError {}

impl PairingService {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) async fn issue_token(&self, ttl: Duration) -> Result<String, PairingError> {
        let token = random_credential::<PAIRING_TOKEN_BYTES>()?;
        let pending = PendingPairing {
            digest: digest(&token),
            expires_at: Instant::now() + ttl,
        };

        let mut state = self.state.lock().await;
        let now = Instant::now();
        state.pending.retain(|entry| entry.expires_at > now);
        while state.pending.len() >= MAX_PENDING_PAIRINGS {
            let _oldest = state.pending.pop_front();
        }
        state.pending.push_back(pending);
        Ok(token)
    }

    pub(crate) async fn authenticate(
        &self,
        hello: &ClientHello,
    ) -> Result<Authentication, PairingError> {
        match (&hello.pairing_token, &hello.device_session) {
            (Some(_), Some(_)) => Err(PairingError::ConflictingCredentials),
            (None, None) => Err(PairingError::PairingRequired),
            (Some(token), None) => self.consume_pairing_token(token).await,
            (None, Some(session)) => self.resume_device_session(session).await,
        }
    }

    pub(crate) async fn authorize_device_session(&self, session: &str) -> Result<(), PairingError> {
        self.resume_device_session(session).await.map(|_| ())
    }

    async fn consume_pairing_token(&self, token: &str) -> Result<Authentication, PairingError> {
        let candidate = digest(token);
        let session = random_credential::<DEVICE_SESSION_BYTES>()?;
        let mut state = self.state.lock().await;
        let position = state
            .pending
            .iter()
            .position(|entry| bool::from(entry.digest.ct_eq(&candidate)))
            .ok_or(PairingError::InvalidPairingToken)?;
        let Some(pending) = state.pending.remove(position) else {
            return Err(PairingError::InvalidPairingToken);
        };
        if Instant::now() >= pending.expires_at {
            return Err(PairingError::PairingTokenExpired);
        }

        while state.device_sessions.len() >= MAX_DEVICE_SESSIONS {
            let _oldest = state.device_sessions.pop_front();
        }
        state.device_sessions.push_back(digest(&session));
        Ok(Authentication {
            new_device_session: Some(session),
        })
    }

    async fn resume_device_session(&self, session: &str) -> Result<Authentication, PairingError> {
        let candidate = digest(session);
        let state = self.state.lock().await;
        let is_valid = state
            .device_sessions
            .iter()
            .any(|known| bool::from(known.ct_eq(&candidate)));
        if !is_valid {
            return Err(PairingError::InvalidDeviceSession);
        }
        Ok(Authentication {
            new_device_session: None,
        })
    }
}

fn random_credential<const N: usize>() -> Result<String, PairingError> {
    let mut bytes = [0_u8; N];
    getrandom::fill(&mut bytes).map_err(|_error| PairingError::RandomSourceUnavailable)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn digest(value: &str) -> [u8; 32] {
    Sha256::digest(value.as_bytes()).into()
}
