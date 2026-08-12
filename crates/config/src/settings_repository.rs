use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use atomic_write_file::AtomicWriteFile;

use crate::{AppSettings, ConfigError};

const MAX_SETTINGS_BYTES: usize = 64 * 1024;
const SETTINGS_BACKUP_LIMIT: usize = 3;
static SETTINGS_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Valid settings together with any recovery action performed while loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSettings {
    /// Valid settings safe to apply.
    pub settings: AppSettings,
    /// Whether a backup replaced an invalid primary.
    pub recovered: bool,
    /// Whether no valid backup existed and safe defaults were installed.
    pub reset_to_defaults: bool,
    /// Preserved invalid primary, when one was found.
    pub quarantined_path: Option<PathBuf>,
}

/// Atomic, bounded-backup repository for shared application settings.
#[derive(Debug, Clone)]
pub struct SettingsRepository {
    root: PathBuf,
}

impl SettingsRepository {
    /// Creates a repository rooted at the application data directory.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Returns the primary settings path.
    #[must_use]
    pub fn settings_path(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    /// Loads settings, recovering from a valid backup or installing defaults.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] only when the filesystem prevents a safe load,
    /// quarantine, recovery, or default write.
    pub fn load(&self) -> Result<LoadedSettings, ConfigError> {
        let primary = self.settings_path();
        if !primary
            .try_exists()
            .map_err(|source| io_error(&primary, source))?
        {
            let settings = AppSettings::default();
            self.write_primary(&settings)?;
            return Ok(LoadedSettings {
                settings,
                recovered: false,
                reset_to_defaults: false,
                quarantined_path: None,
            });
        }

        match read_settings(&primary) {
            Ok(settings) => Ok(LoadedSettings {
                settings,
                recovered: false,
                reset_to_defaults: false,
                quarantined_path: None,
            }),
            Err(error) if is_content_error(&error) => self.recover(&primary),
            Err(error) => Err(error),
        }
    }

    /// Validates and atomically persists settings, retaining three valid snapshots.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] without replacing the primary on validation,
    /// serialization, or pre-commit I/O failure.
    pub fn save(&self, settings: &AppSettings) -> Result<(), ConfigError> {
        settings.validate()?;
        let bytes = serialize_settings(settings)?;
        write_atomic(&self.backup_path(), &bytes)?;
        write_atomic(&self.settings_path(), &bytes)?;
        self.prune_backups()
    }

    fn recover(&self, primary: &Path) -> Result<LoadedSettings, ConfigError> {
        let quarantine = self.quarantine_path();
        if let Some(parent) = quarantine.parent() {
            create_dir(parent)?;
        }
        fs::rename(primary, &quarantine).map_err(|source| io_error(primary, source))?;

        if let Some(settings) = self.latest_valid_backup()? {
            self.write_primary(&settings)?;
            return Ok(LoadedSettings {
                settings,
                recovered: true,
                reset_to_defaults: false,
                quarantined_path: Some(quarantine),
            });
        }

        let settings = AppSettings::default();
        self.write_primary(&settings)?;
        Ok(LoadedSettings {
            settings,
            recovered: false,
            reset_to_defaults: true,
            quarantined_path: Some(quarantine),
        })
    }

    fn write_primary(&self, settings: &AppSettings) -> Result<(), ConfigError> {
        let bytes = serialize_settings(settings)?;
        write_atomic(&self.settings_path(), &bytes)
    }

    fn latest_valid_backup(&self) -> Result<Option<AppSettings>, ConfigError> {
        let directory = self.backup_directory();
        if !directory
            .try_exists()
            .map_err(|source| io_error(&directory, source))?
        {
            return Ok(None);
        }
        let mut paths = read_file_paths(&directory)?;
        paths.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
        Ok(paths.into_iter().find_map(|path| read_settings(&path).ok()))
    }

    fn prune_backups(&self) -> Result<(), ConfigError> {
        let directory = self.backup_directory();
        let mut paths = read_file_paths(&directory)?;
        paths.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
        let remove_count = paths.len().saturating_sub(SETTINGS_BACKUP_LIMIT);
        for path in paths.into_iter().take(remove_count) {
            fs::remove_file(&path).map_err(|source| io_error(&path, source))?;
        }
        Ok(())
    }

    fn backup_directory(&self) -> PathBuf {
        self.root.join("backups").join("settings")
    }

    fn backup_path(&self) -> PathBuf {
        let (epoch_ms, sequence) = unique_suffix();
        self.backup_directory()
            .join(format!("settings-{epoch_ms:020}-{sequence:020}.json"))
    }

    fn quarantine_path(&self) -> PathBuf {
        let (epoch_ms, sequence) = unique_suffix();
        self.root.join("quarantine").join(format!(
            "settings-{epoch_ms:020}-{sequence:020}.corrupt.json"
        ))
    }
}

fn unique_suffix() -> (u128, u64) {
    let epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    let sequence = SETTINGS_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    (epoch_ms, sequence)
}

fn read_settings(path: &Path) -> Result<AppSettings, ConfigError> {
    let metadata = fs::metadata(path).map_err(|source| io_error(path, source))?;
    let length = usize::try_from(metadata.len()).map_err(|_| ConfigError::DocumentTooLarge {
        actual: usize::MAX,
        maximum: MAX_SETTINGS_BYTES,
    })?;
    if length > MAX_SETTINGS_BYTES {
        return Err(ConfigError::DocumentTooLarge {
            actual: length,
            maximum: MAX_SETTINGS_BYTES,
        });
    }
    let bytes = fs::read(path).map_err(|source| io_error(path, source))?;
    let settings: AppSettings = serde_json::from_slice(&bytes).map_err(ConfigError::Json)?;
    settings.validate()?;
    Ok(settings)
}

fn serialize_settings(settings: &AppSettings) -> Result<Vec<u8>, ConfigError> {
    settings.validate()?;
    let mut bytes = serde_json::to_vec_pretty(settings).map_err(ConfigError::Json)?;
    bytes.push(b'\n');
    if bytes.len() > MAX_SETTINGS_BYTES {
        return Err(ConfigError::DocumentTooLarge {
            actual: bytes.len(),
            maximum: MAX_SETTINGS_BYTES,
        });
    }
    Ok(bytes)
}

fn is_content_error(error: &ConfigError) -> bool {
    matches!(
        error,
        ConfigError::Json(_)
            | ConfigError::Validation(_)
            | ConfigError::UnsupportedSchema { .. }
            | ConfigError::InvalidSchemaVersion
            | ConfigError::Migration(_)
            | ConfigError::DocumentTooLarge { .. }
    )
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
        if file_type.is_file() {
            paths.push(entry.path());
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
