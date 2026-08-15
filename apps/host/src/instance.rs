use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use fs2::FileExt;
use serde::{Deserialize, Serialize};

const INSTANCE_SCHEMA_VERSION: u16 = 1;
const MAX_LOCK_METADATA_BYTES: u64 = 8 * 1024;

/// User-facing application entry point that currently owns the Host runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceMode {
    /// Tauri desktop control center.
    Desktop,
    /// Standalone terminal Host.
    Headless,
}

impl Display for InstanceMode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Desktop => "desktop",
            Self::Headless => "headless",
        })
    }
}

/// Non-secret owner information retained in the stable instance-lock file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMetadata {
    /// Lock metadata schema version.
    pub schema_version: u16,
    /// Operating-system process identifier.
    pub pid: u32,
    /// Desktop or headless entry point.
    pub mode: InstanceMode,
    /// `OpenSimDash` version reported by the owner.
    pub version: String,
    /// Wall-clock start time for human diagnostics only.
    pub started_at_unix_ms: u64,
}

/// Failure to establish exclusive ownership of the per-user Host runtime.
#[derive(Debug)]
#[non_exhaustive]
pub enum InstanceError {
    /// Another GUI or CLI already owns the shared lock.
    AlreadyRunning {
        /// Stable lock path used by both entry points.
        path: PathBuf,
        /// Best-effort non-secret owner information.
        owner: Option<InstanceMetadata>,
    },
    /// The lock directory, file, or metadata could not be accessed safely.
    Io {
        /// Path involved in the failed operation.
        path: PathBuf,
        /// Underlying operating-system failure.
        source: io::Error,
    },
    /// Owner metadata could not be serialized.
    Metadata(serde_json::Error),
}

impl Display for InstanceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyRunning { path, owner } => {
                formatter.write_str("OpenSimDash is already running")?;
                if let Some(owner) = owner {
                    write!(
                        formatter,
                        " (PID {}, {}, version {})",
                        owner.pid, owner.mode, owner.version
                    )?;
                }
                write!(formatter, "; instance lock: {}", path.display())
            }
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "could not access instance lock {}: {source}",
                    path.display()
                )
            }
            Self::Metadata(source) => {
                write!(formatter, "could not encode instance metadata: {source}")
            }
        }
    }
}

impl Error for InstanceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Metadata(source) => Some(source),
            Self::AlreadyRunning { .. } => None,
        }
    }
}

/// Exclusive process-lifetime ownership of the user's one Host runtime.
///
/// The lock file deliberately remains on disk after exit. Deleting a lock path
/// can create two independently locked inodes on Unix; retaining it lets the OS
/// file lock be the sole authority and automatically recover after crashes.
#[derive(Debug)]
pub struct InstanceGuard {
    file: File,
    path: PathBuf,
    metadata_path: PathBuf,
    metadata: InstanceMetadata,
}

impl InstanceGuard {
    /// Acquires the default user-level runtime lock.
    ///
    /// # Errors
    ///
    /// Returns [`InstanceError::AlreadyRunning`] when another desktop or
    /// headless process owns it, or an I/O/serialization failure otherwise.
    pub fn acquire(mode: InstanceMode) -> Result<Self, InstanceError> {
        Self::acquire_in(default_runtime_directory(), mode)
    }

    /// Acquires a lock rooted at an injected directory for deterministic tests.
    ///
    /// # Errors
    ///
    /// Returns the same failures as [`Self::acquire`].
    pub fn acquire_in(
        runtime_directory: impl AsRef<Path>,
        mode: InstanceMode,
    ) -> Result<Self, InstanceError> {
        let runtime_directory = runtime_directory.as_ref();
        fs::create_dir_all(runtime_directory).map_err(|source| InstanceError::Io {
            path: runtime_directory.to_path_buf(),
            source,
        })?;
        let path = runtime_directory.join("instance.lock");
        let metadata_path = runtime_directory.join("instance-owner.json");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|source| InstanceError::Io {
                path: path.clone(),
                source,
            })?;

        if let Err(source) = file.try_lock_exclusive() {
            if is_lock_contended(&source) {
                return Err(InstanceError::AlreadyRunning {
                    owner: read_metadata(&metadata_path).ok().flatten(),
                    path,
                });
            }
            return Err(InstanceError::Io { path, source });
        }

        let metadata = InstanceMetadata {
            schema_version: INSTANCE_SCHEMA_VERSION,
            pid: std::process::id(),
            mode,
            version: env!("CARGO_PKG_VERSION").to_owned(),
            started_at_unix_ms: unix_time_ms(),
        };
        if let Err(error) = write_metadata(&metadata_path, &metadata) {
            let _unlock_result = FileExt::unlock(&file);
            return Err(error);
        }
        Ok(Self {
            file,
            path,
            metadata_path,
            metadata,
        })
    }

    /// Returns the stable lock path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns metadata written by this process.
    #[must_use]
    pub const fn metadata(&self) -> &InstanceMetadata {
        &self.metadata
    }

    /// Returns the companion non-secret owner metadata path.
    #[must_use]
    pub fn metadata_path(&self) -> &Path {
        &self.metadata_path
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        let _unlock_result = FileExt::unlock(&self.file);
    }
}

/// Returns the stable per-user runtime directory shared by GUI and CLI.
#[must_use]
pub fn default_runtime_directory() -> PathBuf {
    if let Some(configured) = std::env::var_os("OPENSIMDASH_RUNTIME_DIR") {
        return PathBuf::from(configured);
    }

    #[cfg(target_os = "windows")]
    if let Some(local_data) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(local_data)
            .join("OpenSimDash")
            .join("runtime");
    }

    #[cfg(target_os = "macos")]
    if let Some(user_home) = std::env::var_os("HOME") {
        return PathBuf::from(user_home)
            .join("Library")
            .join("Application Support")
            .join("OpenSimDash")
            .join("runtime");
    }

    std::env::temp_dir().join("OpenSimDash").join("runtime")
}

fn read_metadata(path: &Path) -> Result<Option<InstanceMetadata>, InstanceError> {
    let metadata = fs::metadata(path).map_err(|source| InstanceError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.len() == 0 || metadata.len() > MAX_LOCK_METADATA_BYTES {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(|source| InstanceError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(serde_json::from_slice(&bytes).ok())
}

fn write_metadata(path: &Path, metadata: &InstanceMetadata) -> Result<(), InstanceError> {
    let mut bytes = serde_json::to_vec_pretty(metadata).map_err(InstanceError::Metadata)?;
    bytes.push(b'\n');
    OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .and_then(|mut file| {
            file.write_all(&bytes)?;
            file.sync_data()
        })
        .map_err(|source| InstanceError::Io {
            path: path.to_path_buf(),
            source,
        })
}

fn is_lock_contended(source: &io::Error) -> bool {
    source.kind() == io::ErrorKind::WouldBlock
        || cfg!(windows) && matches!(source.raw_os_error(), Some(32 | 33))
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_and_headless_share_one_os_lock() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let desktop = InstanceGuard::acquire_in(temp.path(), InstanceMode::Desktop)?;
        let error = match InstanceGuard::acquire_in(temp.path(), InstanceMode::Headless) {
            Ok(_unexpected) => return Err("the second entry point acquired the same lock".into()),
            Err(error) => error,
        };
        let InstanceError::AlreadyRunning { owner, .. } = error else {
            return Err(format!("unexpected error: {error}").into());
        };
        assert_eq!(
            owner.as_ref().map(|value| value.mode),
            Some(InstanceMode::Desktop)
        );
        assert_eq!(
            owner.as_ref().map(|value| value.pid),
            Some(std::process::id())
        );

        drop(desktop);
        let headless = InstanceGuard::acquire_in(temp.path(), InstanceMode::Headless)?;
        assert_eq!(headless.metadata().mode, InstanceMode::Headless);
        Ok(())
    }

    #[test]
    fn stale_metadata_does_not_prevent_reacquisition() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let first = InstanceGuard::acquire_in(temp.path(), InstanceMode::Desktop)?;
        let path = first.path().to_path_buf();
        drop(first);
        assert!(path.exists());

        let second = InstanceGuard::acquire_in(temp.path(), InstanceMode::Headless)?;
        assert_eq!(second.metadata().mode, InstanceMode::Headless);
        Ok(())
    }
}
