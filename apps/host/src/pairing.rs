use std::{
    collections::VecDeque,
    error::Error,
    fmt::{self, Display, Formatter},
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use atomic_write_file::AtomicWriteFile;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::{sync::Mutex, time::Instant};
use tracing::error;

use opencarpanel_protocol::ClientHello;

const PAIRING_TOKEN_BYTES: usize = 16;
const DEVICE_SESSION_BYTES: usize = 32;
const DEVICE_ID_BYTES: usize = 12;
const MAX_DEVICE_NAME_BYTES: usize = 64;
const MAX_PENDING_PAIRINGS: usize = 16;
const MAX_DEVICE_SESSIONS: usize = 64;
const MAX_DEVICE_FILE_BYTES: usize = 128 * 1024;
const DEVICE_SCHEMA_VERSION: u16 = 1;
const DEVICE_BACKUP_LIMIT: usize = 3;
const LAST_SEEN_WRITE_INTERVAL_MS: u64 = 60_000;
static DEVICE_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct PendingPairing {
    digest: [u8; 32],
    expires_at: Instant,
}

#[derive(Debug, Clone)]
struct DeviceRecord {
    id: String,
    name: String,
    session_digest: [u8; 32],
    paired_at_unix_ms: u64,
    last_seen_unix_ms: u64,
}

#[derive(Debug, Default)]
struct PairingState {
    pending: VecDeque<PendingPairing>,
    devices: VecDeque<DeviceRecord>,
}

/// Non-secret paired-device metadata safe to show in the desktop control center.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedDevice {
    /// Random stable local device id, not a session credential.
    pub id: String,
    /// User-agent-derived bounded display label.
    pub name: String,
    /// Initial pairing time.
    pub paired_at_unix_ms: u64,
    /// Most recent authenticated WebSocket connection time.
    pub last_seen_unix_ms: u64,
}

impl From<&DeviceRecord> for PairedDevice {
    fn from(value: &DeviceRecord) -> Self {
        Self {
            id: value.id.clone(),
            name: value.name.clone(),
            paired_at_unix_ms: value.paired_at_unix_ms,
            last_seen_unix_ms: value.last_seen_unix_ms,
        }
    }
}

/// One-time pairing and persistent device-session authority.
#[derive(Debug)]
pub(crate) struct PairingService {
    state: Mutex<PairingState>,
    repository: Option<DeviceRepository>,
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
    /// The persisted device session was unknown or had been revoked.
    InvalidDeviceSession,
    /// Device metadata could not be committed safely.
    StorageUnavailable,
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
            Self::InvalidDeviceSession => "the device session is invalid or revoked",
            Self::StorageUnavailable => "paired-device storage is unavailable",
        })
    }
}

impl Error for PairingError {}

impl PairingService {
    pub(crate) fn ephemeral() -> Self {
        Self {
            state: Mutex::new(PairingState::default()),
            repository: None,
        }
    }

    pub(crate) fn load(root: impl AsRef<Path>) -> io::Result<Self> {
        let repository = DeviceRepository::new(root);
        let devices = repository.load()?;
        Ok(Self {
            state: Mutex::new(PairingState {
                pending: VecDeque::new(),
                devices,
            }),
            repository: Some(repository),
        })
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
            (Some(token), None) => {
                self.consume_pairing_token(token, hello.device_name.as_deref())
                    .await
            }
            (None, Some(session)) => self.resume_device_session(session, true).await,
        }
    }

    pub(crate) async fn authorize_device_session(&self, session: &str) -> Result<(), PairingError> {
        self.resume_device_session(session, false).await.map(|_| ())
    }

    pub(crate) async fn devices(&self) -> Vec<PairedDevice> {
        self.state
            .lock()
            .await
            .devices
            .iter()
            .rev()
            .map(PairedDevice::from)
            .collect()
    }

    pub(crate) async fn revoke_device(&self, id: &str) -> Result<bool, PairingError> {
        let mut state = self.state.lock().await;
        let Some(index) = state.devices.iter().position(|device| device.id == id) else {
            return Ok(false);
        };
        let Some(removed) = state.devices.remove(index) else {
            return Ok(false);
        };
        if let Err(error) = self.persist(&state.devices) {
            state.devices.insert(index, removed);
            error!(%error, "could not persist device revocation");
            return Err(PairingError::StorageUnavailable);
        }
        Ok(true)
    }

    pub(crate) async fn revoke_all_devices(&self) -> Result<usize, PairingError> {
        let mut state = self.state.lock().await;
        let removed = std::mem::take(&mut state.devices);
        let count = removed.len();
        if let Err(error) = self.persist(&state.devices) {
            state.devices = removed;
            error!(%error, "could not persist all-device revocation");
            return Err(PairingError::StorageUnavailable);
        }
        Ok(count)
    }

    async fn consume_pairing_token(
        &self,
        token: &str,
        device_name: Option<&str>,
    ) -> Result<Authentication, PairingError> {
        let candidate = digest(token);
        let session = random_credential::<DEVICE_SESSION_BYTES>()?;
        let device_id = random_credential::<DEVICE_ID_BYTES>()?;
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

        while state.devices.len() >= MAX_DEVICE_SESSIONS {
            let _oldest = state.devices.pop_front();
        }
        let now = unix_time_ms();
        state.devices.push_back(DeviceRecord {
            id: device_id,
            name: sanitized_device_name(device_name),
            session_digest: digest(&session),
            paired_at_unix_ms: now,
            last_seen_unix_ms: now,
        });
        if let Err(error) = self.persist(&state.devices) {
            let _uncommitted = state.devices.pop_back();
            error!(%error, "could not persist newly paired device");
            return Err(PairingError::StorageUnavailable);
        }
        Ok(Authentication {
            new_device_session: Some(session),
        })
    }

    async fn resume_device_session(
        &self,
        session: &str,
        update_last_seen: bool,
    ) -> Result<Authentication, PairingError> {
        let candidate = digest(session);
        let mut state = self.state.lock().await;
        let Some(index) = state
            .devices
            .iter()
            .position(|known| bool::from(known.session_digest.ct_eq(&candidate)))
        else {
            return Err(PairingError::InvalidDeviceSession);
        };

        let now = unix_time_ms();
        if update_last_seen
            && now.saturating_sub(state.devices[index].last_seen_unix_ms)
                >= LAST_SEEN_WRITE_INTERVAL_MS
        {
            let previous = state.devices[index].last_seen_unix_ms;
            state.devices[index].last_seen_unix_ms = now;
            if let Err(error) = self.persist(&state.devices) {
                state.devices[index].last_seen_unix_ms = previous;
                error!(%error, "could not persist paired-device last-seen time");
            }
        }
        Ok(Authentication {
            new_device_session: None,
        })
    }

    fn persist(&self, devices: &VecDeque<DeviceRecord>) -> io::Result<()> {
        self.repository
            .as_ref()
            .map_or(Ok(()), |repository| repository.save(devices))
    }
}

#[derive(Debug, Clone)]
struct DeviceRepository {
    root: PathBuf,
}

impl DeviceRepository {
    fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    fn load(&self) -> io::Result<VecDeque<DeviceRecord>> {
        let primary = self.primary_path();
        if !primary.try_exists()? {
            return Ok(VecDeque::new());
        }
        match read_device_document(&primary) {
            Ok(devices) => Ok(devices),
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                let quarantine = self.quarantine_path();
                if let Some(parent) = quarantine.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&primary, &quarantine)?;
                if let Some(devices) = self.latest_valid_backup()? {
                    self.write_primary(&devices)?;
                    Ok(devices)
                } else {
                    let devices = VecDeque::new();
                    self.write_primary(&devices)?;
                    Ok(devices)
                }
            }
            Err(error) => Err(error),
        }
    }

    fn save(&self, devices: &VecDeque<DeviceRecord>) -> io::Result<()> {
        let bytes = serialize_devices(devices)?;
        write_atomic(&self.backup_path(), &bytes)?;
        write_atomic(&self.primary_path(), &bytes)?;
        self.prune_backups()
    }

    fn write_primary(&self, devices: &VecDeque<DeviceRecord>) -> io::Result<()> {
        write_atomic(&self.primary_path(), &serialize_devices(devices)?)
    }

    fn latest_valid_backup(&self) -> io::Result<Option<VecDeque<DeviceRecord>>> {
        let directory = self.backup_directory();
        if !directory.try_exists()? {
            return Ok(None);
        }
        let mut paths = regular_files(&directory)?;
        paths.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
        Ok(paths
            .into_iter()
            .find_map(|path| read_device_document(&path).ok()))
    }

    fn prune_backups(&self) -> io::Result<()> {
        let directory = self.backup_directory();
        let mut paths = regular_files(&directory)?;
        paths.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
        let remove_count = paths.len().saturating_sub(DEVICE_BACKUP_LIMIT);
        for path in paths.into_iter().take(remove_count) {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn primary_path(&self) -> PathBuf {
        self.root.join("clients.json")
    }

    fn backup_directory(&self) -> PathBuf {
        self.root.join("backups").join("clients")
    }

    fn backup_path(&self) -> PathBuf {
        let (time, sequence) = unique_suffix();
        self.backup_directory()
            .join(format!("clients-{time:020}-{sequence:020}.json"))
    }

    fn quarantine_path(&self) -> PathBuf {
        let (time, sequence) = unique_suffix();
        self.root
            .join("quarantine")
            .join(format!("clients-{time:020}-{sequence:020}.corrupt.json"))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredDeviceDocument {
    schema_version: u16,
    devices: Vec<StoredDevice>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredDevice {
    id: String,
    name: String,
    session_digest: String,
    paired_at_unix_ms: u64,
    last_seen_unix_ms: u64,
}

fn read_device_document(path: &Path) -> io::Result<VecDeque<DeviceRecord>> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > u64::try_from(MAX_DEVICE_FILE_BYTES).unwrap_or(u64::MAX) {
        return Err(invalid_data("paired-device file exceeds its size limit"));
    }
    let bytes = fs::read(path)?;
    let document: StoredDeviceDocument =
        serde_json::from_slice(&bytes).map_err(|error| invalid_data(error.to_string()))?;
    if document.schema_version != DEVICE_SCHEMA_VERSION
        || document.devices.len() > MAX_DEVICE_SESSIONS
    {
        return Err(invalid_data("unsupported paired-device document"));
    }
    document
        .devices
        .into_iter()
        .map(device_from_stored)
        .collect()
}

fn device_from_stored(stored: StoredDevice) -> io::Result<DeviceRecord> {
    if stored.id.is_empty()
        || stored.id.len() > 64
        || stored.name.is_empty()
        || stored.name.len() > MAX_DEVICE_NAME_BYTES
    {
        return Err(invalid_data("invalid paired-device metadata"));
    }
    let digest = URL_SAFE_NO_PAD
        .decode(stored.session_digest)
        .map_err(|error| invalid_data(error.to_string()))?;
    let session_digest: [u8; 32] = digest
        .try_into()
        .map_err(|_| invalid_data("invalid paired-device digest length"))?;
    Ok(DeviceRecord {
        id: stored.id,
        name: stored.name,
        session_digest,
        paired_at_unix_ms: stored.paired_at_unix_ms,
        last_seen_unix_ms: stored.last_seen_unix_ms,
    })
}

fn serialize_devices(devices: &VecDeque<DeviceRecord>) -> io::Result<Vec<u8>> {
    let document = StoredDeviceDocument {
        schema_version: DEVICE_SCHEMA_VERSION,
        devices: devices
            .iter()
            .map(|device| StoredDevice {
                id: device.id.clone(),
                name: device.name.clone(),
                session_digest: URL_SAFE_NO_PAD.encode(device.session_digest),
                paired_at_unix_ms: device.paired_at_unix_ms,
                last_seen_unix_ms: device.last_seen_unix_ms,
            })
            .collect(),
    };
    let mut bytes =
        serde_json::to_vec_pretty(&document).map_err(|error| invalid_data(error.to_string()))?;
    bytes.push(b'\n');
    if bytes.len() > MAX_DEVICE_FILE_BYTES {
        return Err(invalid_data("paired-device file exceeds its size limit"));
    }
    Ok(bytes)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = AtomicWriteFile::open(path)?;
    file.write_all(bytes)?;
    file.commit()
}

fn regular_files(directory: &Path) -> io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            paths.push(entry.path());
        }
    }
    Ok(paths)
}

fn random_credential<const N: usize>() -> Result<String, PairingError> {
    let mut bytes = [0_u8; N];
    getrandom::fill(&mut bytes).map_err(|_error| PairingError::RandomSourceUnavailable)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn digest(value: &str) -> [u8; 32] {
    Sha256::digest(value.as_bytes()).into()
}

fn sanitized_device_name(value: Option<&str>) -> String {
    let value = value.unwrap_or("Mobile dashboard").trim();
    let mut name = String::with_capacity(value.len().min(MAX_DEVICE_NAME_BYTES));
    for character in value.chars().filter(|character| !character.is_control()) {
        if name.len() + character.len_utf8() > MAX_DEVICE_NAME_BYTES {
            break;
        }
        name.push(character);
    }
    if name.is_empty() {
        "Mobile dashboard".to_owned()
    } else {
        name
    }
}

fn unique_suffix() -> (u128, u64) {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    (time, DEVICE_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed))
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello_with_token(token: String, name: &str) -> ClientHello {
        ClientHello {
            pairing_token: Some(token),
            device_session: None,
            device_name: Some(name.to_owned()),
            last_event_seq: None,
            snapshot_hz: 60,
        }
    }

    fn hello_with_session(session: String) -> ClientHello {
        ClientHello {
            pairing_token: None,
            device_session: Some(session),
            device_name: None,
            last_event_seq: None,
            snapshot_hz: 60,
        }
    }

    #[tokio::test]
    async fn device_sessions_survive_restart_and_can_be_revoked() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let service = PairingService::load(temp.path())?;
        let token = service.issue_token(Duration::from_secs(30)).await?;
        let paired = service
            .authenticate(&hello_with_token(token, "  Driver's iPad  "))
            .await?;
        let session = paired
            .new_device_session
            .ok_or("pairing did not issue a device session")?;
        let devices = service.devices().await;
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "Driver's iPad");
        let id = devices[0].id.clone();
        drop(service);

        let restarted = PairingService::load(temp.path())?;
        restarted
            .authenticate(&hello_with_session(session.clone()))
            .await?;
        assert!(restarted.revoke_device(&id).await?);
        assert_eq!(
            restarted.authenticate(&hello_with_session(session)).await,
            Err(PairingError::InvalidDeviceSession)
        );
        Ok(())
    }

    #[tokio::test]
    async fn corrupt_device_file_is_quarantined_without_crashing_host() -> Result<(), Box<dyn Error>>
    {
        let temp = tempfile::tempdir()?;
        fs::write(temp.path().join("clients.json"), b"{broken")?;
        let service = PairingService::load(temp.path())?;
        assert!(service.devices().await.is_empty());
        let quarantine = temp.path().join("quarantine");
        assert_eq!(regular_files(&quarantine)?.len(), 1);
        Ok(())
    }
}
