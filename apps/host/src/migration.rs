use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use crate::{InstanceGuard, InstanceMode, default_data_directory};

const EXCLUDED_DIRECTORIES: [&str; 3] = ["runtime", "game-plugins", "logs"];

/// Copies the previous branded profile into the `OpenSimDash` data directory once.
///
/// Settings, paired devices, layouts, and installation discovery state are
/// retained. Runtime locks, logs, and incompatible game plugins are deliberately
/// excluded. The previous profile remains untouched as a recoverable backup.
///
/// # Errors
///
/// Returns an I/O error when the previous profile is unsafe, still in use, or
/// cannot be copied atomically into the new location.
pub fn migrate_previous_data_directory(mode: InstanceMode) -> io::Result<bool> {
    if std::env::var_os("OPENSIMDASH_DATA_DIR").is_some() {
        return Ok(false);
    }
    let Some(previous) = previous_default_data_directory() else {
        return Ok(false);
    };
    migrate_data_directory(&previous, &default_data_directory(), mode)
}

fn migrate_data_directory(
    previous: &Path,
    destination: &Path,
    mode: InstanceMode,
) -> io::Result<bool> {
    if destination.exists() || !previous.exists() {
        return Ok(false);
    }
    let previous_metadata = fs::symlink_metadata(previous)
        .map_err(|error| path_error("inspect previous profile", previous, &error))?;
    if previous_metadata.file_type().is_symlink() || !previous_metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "previous application profile must be a regular directory",
        ));
    }

    let _previous_instance = InstanceGuard::acquire_in(previous.join("runtime"), mode)
        .map_err(|error| io::Error::other(format!("previous profile is still in use: {error}")))?;
    let parent = destination.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "OpenSimDash data directory has no parent",
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| path_error("create profile parent", parent, &error))?;
    let staging = tempfile::Builder::new()
        .prefix(".opensimdash-data-migration-")
        .tempdir_in(parent)
        .map_err(|error| path_error("create migration staging directory", parent, &error))?;
    copy_directory(previous, staging.path(), true)?;

    if destination.exists() {
        return Ok(false);
    }
    fs::rename(staging.path(), destination)
        .map_err(|error| path_error("commit migrated profile", destination, &error))?;
    Ok(true)
}

fn copy_directory(source: &Path, destination: &Path, root: bool) -> io::Result<()> {
    fs::create_dir_all(destination)
        .map_err(|error| path_error("create migrated directory", destination, &error))?;
    for entry in
        fs::read_dir(source).map_err(|error| path_error("read previous profile", source, &error))?
    {
        let entry =
            entry.map_err(|error| path_error("read previous profile entry", source, &error))?;
        if root
            && EXCLUDED_DIRECTORIES
                .iter()
                .any(|excluded| entry.file_name() == OsStr::new(excluded))
        {
            continue;
        }
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|error| path_error("inspect previous profile entry", &source_path, &error))?;
        if file_type.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "previous application profile must not contain symbolic links",
            ));
        }
        if file_type.is_dir() {
            copy_directory(&source_path, &destination_path, false)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &destination_path)
                .map_err(|error| path_error("copy previous profile file", &source_path, &error))?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "previous application profile contains an unsupported file type",
            ));
        }
    }
    Ok(())
}

fn previous_default_data_directory() -> Option<PathBuf> {
    let previous_name = ["Open", "Car", "panel"].concat();

    #[cfg(target_os = "windows")]
    if let Some(local_data) = std::env::var_os("LOCALAPPDATA") {
        return Some(PathBuf::from(local_data).join(previous_name));
    }

    #[cfg(target_os = "macos")]
    if let Some(user_home) = std::env::var_os("HOME") {
        return Some(
            PathBuf::from(user_home)
                .join("Library")
                .join("Application Support")
                .join(previous_name),
        );
    }

    None
}

fn path_error(operation: &str, path: &Path, error: &io::Error) -> io::Error {
    io::Error::new(
        error.kind(),
        format!("{operation} at {}: {error}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn migrates_user_state_without_runtime_logs_or_plugins() -> Result<(), Box<dyn Error>> {
        let temporary = tempfile::tempdir()?;
        let previous = temporary.path().join("previous");
        let destination = temporary.path().join("current");
        fs::create_dir_all(previous.join("layouts"))?;
        fs::create_dir_all(previous.join("game-plugins/example"))?;
        fs::create_dir_all(previous.join("logs"))?;
        fs::write(previous.join("settings.json"), b"settings")?;
        fs::write(previous.join("devices.json"), b"devices")?;
        fs::write(previous.join("layouts/default.json"), b"layout")?;
        fs::write(
            previous.join("game-plugins/example/decoder.wasm"),
            b"plugin",
        )?;
        fs::write(previous.join("logs/previous.log"), b"log")?;

        assert!(migrate_data_directory(
            &previous,
            &destination,
            InstanceMode::Headless,
        )?);
        assert_eq!(fs::read(destination.join("settings.json"))?, b"settings");
        assert_eq!(fs::read(destination.join("devices.json"))?, b"devices");
        assert_eq!(
            fs::read(destination.join("layouts/default.json"))?,
            b"layout"
        );
        assert!(!destination.join("runtime").exists());
        assert!(!destination.join("game-plugins").exists());
        assert!(!destination.join("logs").exists());
        assert!(previous.join("settings.json").exists());
        Ok(())
    }

    #[test]
    fn never_overwrites_an_existing_new_profile() -> Result<(), Box<dyn Error>> {
        let temporary = tempfile::tempdir()?;
        let previous = temporary.path().join("previous");
        let destination = temporary.path().join("current");
        fs::create_dir_all(&previous)?;
        fs::create_dir_all(&destination)?;
        fs::write(previous.join("settings.json"), b"previous")?;
        fs::write(destination.join("settings.json"), b"current")?;

        assert!(!migrate_data_directory(
            &previous,
            &destination,
            InstanceMode::Headless,
        )?);
        assert_eq!(fs::read(destination.join("settings.json"))?, b"current");
        Ok(())
    }
}
