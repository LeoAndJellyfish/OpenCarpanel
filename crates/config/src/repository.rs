use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use atomic_write_file::AtomicWriteFile;

use crate::{LayoutDocument, LayoutId, MAX_LAYOUT_BYTES, ValidationError, migrate_layout_json};

const DEFAULT_BACKUP_LIMIT: usize = 3;
static QUARANTINE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Failure while migrating, validating, loading, or saving configuration.
#[derive(Debug)]
#[non_exhaustive]
pub enum ConfigError {
    /// A filesystem operation failed.
    Io {
        /// Path involved in the operation.
        path: PathBuf,
        /// Underlying operating-system error.
        source: io::Error,
    },
    /// JSON syntax or typed deserialization failed.
    Json(serde_json::Error),
    /// Runtime validation failed.
    Validation(ValidationError),
    /// Client attempted to save from an obsolete revision.
    Conflict {
        /// Latest stored revision.
        current_revision: u64,
    },
    /// Document schema is newer or otherwise unsupported.
    UnsupportedSchema {
        /// Rejected schema version.
        actual: u16,
    },
    /// `schemaVersion` is not an unsigned 16-bit integer.
    InvalidSchemaVersion,
    /// A deterministic migration precondition was not met.
    Migration(&'static str),
    /// Serialized or imported document exceeds its byte bound.
    DocumentTooLarge {
        /// Observed byte count.
        actual: usize,
        /// Maximum byte count.
        maximum: usize,
    },
    /// Primary data was invalid and no valid bounded backup remained.
    RecoveryFailed {
        /// Layout for which recovery failed.
        layout_id: String,
    },
    /// Revision counter reached its representable maximum.
    RevisionExhausted,
}

impl Display for ConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "configuration I/O failed at {}: {source}",
                    path.display()
                )
            }
            Self::Json(error) => write!(formatter, "invalid configuration JSON: {error}"),
            Self::Validation(error) => Display::fmt(error, formatter),
            Self::Conflict { current_revision } => write!(
                formatter,
                "configuration revision conflict; current revision is {current_revision}"
            ),
            Self::UnsupportedSchema { actual } => {
                write!(
                    formatter,
                    "unsupported configuration schema version {actual}"
                )
            }
            Self::InvalidSchemaVersion => formatter
                .write_str("configuration schemaVersion must be an unsigned 16-bit integer"),
            Self::Migration(message) => {
                write!(formatter, "configuration migration failed: {message}")
            }
            Self::DocumentTooLarge { actual, maximum } => write!(
                formatter,
                "configuration is {actual} bytes; maximum is {maximum}"
            ),
            Self::RecoveryFailed { layout_id } => {
                write!(
                    formatter,
                    "no valid backup remains for layout {layout_id:?}"
                )
            }
            Self::RevisionExhausted => formatter.write_str("configuration revision is exhausted"),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json(error) => Some(error),
            Self::Validation(error) => Some(error),
            _ => None,
        }
    }
}

/// Result of loading a primary or recovered layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedLayout {
    /// Valid, migrated layout.
    pub document: LayoutDocument,
    /// Whether a backup replaced an invalid primary.
    pub recovered: bool,
    /// Preserved corrupt primary when recovery occurred.
    pub quarantined_path: Option<PathBuf>,
}

/// Filesystem-backed versioned layout repository.
#[derive(Debug, Clone)]
pub struct LayoutRepository {
    root: PathBuf,
    backup_limit: usize,
}

impl LayoutRepository {
    /// Creates a repository rooted at an injected application-data directory.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            backup_limit: DEFAULT_BACKUP_LIMIT,
        }
    }

    /// Returns the primary JSON path for a validated layout id.
    #[must_use]
    pub fn layout_path(&self, id: &LayoutId) -> PathBuf {
        self.root
            .join("layouts")
            .join(format!("{}.json", id.as_str()))
    }

    /// Loads a layout when it exists, recovering from a bounded backup when needed.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] for I/O, migration, validation, or exhausted recovery.
    pub fn load(&self, id: &LayoutId) -> Result<Option<LoadedLayout>, ConfigError> {
        let primary = self.layout_path(id);
        if !primary
            .try_exists()
            .map_err(|source| io_error(&primary, source))?
        {
            return Ok(None);
        }

        match read_and_migrate(&primary) {
            Ok(document) => Ok(Some(LoadedLayout {
                document,
                recovered: false,
                quarantined_path: None,
            })),
            Err(error) if error.is_recoverable_content_error() => {
                self.recover(id, &primary).map(Some)
            }
            Err(error) => Err(error),
        }
    }

    /// Loads an existing layout.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::RecoveryFailed`] when no primary layout exists,
    /// or another load/recovery error.
    pub fn load_required(&self, id: &LayoutId) -> Result<LoadedLayout, ConfigError> {
        self.load(id)?.ok_or_else(|| ConfigError::RecoveryFailed {
            layout_id: id.as_str().to_owned(),
        })
    }

    /// Validates and atomically stores the next document revision.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Conflict`] for stale optimistic revisions and
    /// leaves the current primary unchanged for all pre-commit failures.
    pub fn save(
        &self,
        document: &LayoutDocument,
        expected_revision: u64,
    ) -> Result<LayoutDocument, ConfigError> {
        document.validate()?;
        let current_revision = self
            .load(document.id())?
            .map_or(0, |loaded| loaded.document.revision());
        if current_revision != expected_revision {
            return Err(ConfigError::Conflict { current_revision });
        }

        let next_revision = current_revision
            .checked_add(1)
            .ok_or(ConfigError::RevisionExhausted)?;
        let mut next = document.clone();
        next.set_revision(next_revision);
        next.validate()?;
        let bytes = serialize_document(&next)?;

        let backup = self.backup_path(next.id(), next_revision);
        write_atomic(&backup, &bytes)?;
        write_atomic(&self.layout_path(next.id()), &bytes)?;
        self.prune_backups(next.id())?;
        Ok(next)
    }

    fn backup_path(&self, id: &LayoutId, revision: u64) -> PathBuf {
        self.root
            .join("backups")
            .join(id.as_str())
            .join(format!("revision-{revision:020}.json"))
    }

    fn recover(&self, id: &LayoutId, primary: &Path) -> Result<LoadedLayout, ConfigError> {
        let quarantine = self.quarantine_path(id);
        if let Some(parent) = quarantine.parent() {
            create_dir(parent)?;
        }
        fs::rename(primary, &quarantine).map_err(|source| io_error(primary, source))?;

        let document =
            self.latest_valid_backup(id)?
                .ok_or_else(|| ConfigError::RecoveryFailed {
                    layout_id: id.as_str().to_owned(),
                })?;
        let bytes = serialize_document(&document)?;
        write_atomic(primary, &bytes)?;
        Ok(LoadedLayout {
            document,
            recovered: true,
            quarantined_path: Some(quarantine),
        })
    }

    fn latest_valid_backup(&self, id: &LayoutId) -> Result<Option<LayoutDocument>, ConfigError> {
        let directory = self.root.join("backups").join(id.as_str());
        if !directory
            .try_exists()
            .map_err(|source| io_error(&directory, source))?
        {
            return Ok(None);
        }
        let mut paths = read_file_paths(&directory)?;
        paths.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
        for path in paths {
            if let Ok(document) = read_and_migrate(&path) {
                return Ok(Some(document));
            }
        }
        Ok(None)
    }

    fn prune_backups(&self, id: &LayoutId) -> Result<(), ConfigError> {
        let directory = self.root.join("backups").join(id.as_str());
        let mut paths = read_file_paths(&directory)?;
        paths.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
        let remove_count = paths.len().saturating_sub(self.backup_limit);
        for path in paths.into_iter().take(remove_count) {
            fs::remove_file(&path).map_err(|source| io_error(&path, source))?;
        }
        Ok(())
    }

    fn quarantine_path(&self, id: &LayoutId) -> PathBuf {
        let epoch_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis());
        let sequence = QUARANTINE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        self.root.join("quarantine").join(format!(
            "{}-{epoch_ms}-{sequence}.corrupt.json",
            id.as_str()
        ))
    }
}

impl ConfigError {
    fn is_recoverable_content_error(&self) -> bool {
        matches!(
            self,
            Self::Json(_)
                | Self::Validation(_)
                | Self::UnsupportedSchema { .. }
                | Self::InvalidSchemaVersion
                | Self::Migration(_)
                | Self::DocumentTooLarge { .. }
        )
    }
}

fn read_and_migrate(path: &Path) -> Result<LayoutDocument, ConfigError> {
    let metadata = fs::metadata(path).map_err(|source| io_error(path, source))?;
    let file_len = usize::try_from(metadata.len()).map_err(|_| ConfigError::DocumentTooLarge {
        actual: usize::MAX,
        maximum: MAX_LAYOUT_BYTES,
    })?;
    if file_len > MAX_LAYOUT_BYTES {
        return Err(ConfigError::DocumentTooLarge {
            actual: file_len,
            maximum: MAX_LAYOUT_BYTES,
        });
    }
    let bytes = fs::read(path).map_err(|source| io_error(path, source))?;
    migrate_layout_json(&bytes)
}

fn serialize_document(document: &LayoutDocument) -> Result<Vec<u8>, ConfigError> {
    let mut bytes = serde_json::to_vec_pretty(document).map_err(ConfigError::Json)?;
    bytes.push(b'\n');
    if bytes.len() > MAX_LAYOUT_BYTES {
        return Err(ConfigError::DocumentTooLarge {
            actual: bytes.len(),
            maximum: MAX_LAYOUT_BYTES,
        });
    }
    Ok(bytes)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        create_dir(parent)?;
    }
    let mut file = AtomicWriteFile::open(path).map_err(|source| io_error(path, source))?;
    file.write_all(bytes)
        .map_err(|source| io_error(path, source))?;
    file.commit().map_err(|source| io_error(path, source))
}

fn create_dir(path: &Path) -> Result<(), ConfigError> {
    fs::create_dir_all(path).map_err(|source| io_error(path, source))
}

fn read_file_paths(directory: &Path) -> Result<Vec<PathBuf>, ConfigError> {
    let entries = fs::read_dir(directory).map_err(|source| io_error(directory, source))?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| io_error(directory, source))?;
        let file_type = entry
            .file_type()
            .map_err(|source| io_error(directory, source))?;
        let path = entry.path();
        if file_type.is_file() {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn io_error(path: &Path, source: io::Error) -> ConfigError {
    ConfigError::Io {
        path: path.to_path_buf(),
        source,
    }
}
