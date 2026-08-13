use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use atomic_write_file::AtomicWriteFile;
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use tauri::{AppHandle, Manager as _};

const BACKUP_LIMIT: usize = 3;

/// Safe status values for one selected SCS installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScsPluginState {
    /// No `OpenCarpanel` plugin exists at the derived target.
    Missing,
    /// Installed bytes match the artifact bundled with this desktop version.
    Current,
    /// A regular `OpenCarpanel` plugin exists but differs from this version.
    Outdated,
}

/// Non-secret SCS bridge inspection result shown to the user.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScsPluginStatus {
    /// Stable selected game id.
    pub game: String,
    /// Canonical game root granted through the system picker.
    pub game_directory: String,
    /// Exact derived plugin target.
    pub plugin_path: String,
    /// Current target state.
    pub state: ScsPluginState,
    /// SHA-256 of the bundled artifact, safe for verification.
    pub bundled_sha256: String,
    /// SHA-256 of an existing target, if present.
    pub installed_sha256: Option<String>,
}

/// Inspects a user-selected ETS2/ATS installation without modifying it.
///
/// # Errors
///
/// Rejects unsupported game ids, invalid roots, symlink escapes, unexpected
/// target types, missing bundle resources, and filesystem I/O failures.
pub fn inspect(
    app: &AppHandle,
    game: &str,
    selected_directory: &Path,
) -> Result<ScsPluginStatus, String> {
    let bundled = bundled_plugin(app)?;
    inspect_with_artifact(game, selected_directory, &bundled)
}

/// Installs or updates the bridge with backup, atomic replacement, and hash verification.
///
/// # Errors
///
/// Returns before replacing a valid target when validation, reading, backup,
/// writing, or final hash verification fails.
pub fn install(
    app: &AppHandle,
    game: &str,
    selected_directory: &Path,
) -> Result<ScsPluginStatus, String> {
    let bundled = bundled_plugin(app)?;
    install_with_artifact(game, selected_directory, &bundled)
}

fn inspect_with_artifact(
    game: &str,
    selected_directory: &Path,
    bundled: &Path,
) -> Result<ScsPluginStatus, String> {
    validate_game(game)?;
    let game_directory = canonical_directory(selected_directory, "游戏目录")?;
    let plugin_directory = resolve_plugin_directory(&game_directory, game, false)?;
    let plugin_path = plugin_directory.join(plugin_filename());
    let bundled_hash = file_hash(bundled)?;
    let installed_hash = existing_plugin_hash(&plugin_path)?;
    let state = match installed_hash.as_ref() {
        None => ScsPluginState::Missing,
        Some(hash) if hash == &bundled_hash => ScsPluginState::Current,
        Some(_) => ScsPluginState::Outdated,
    };
    Ok(ScsPluginStatus {
        game: game.to_owned(),
        game_directory: game_directory.display().to_string(),
        plugin_path: plugin_path.display().to_string(),
        state,
        bundled_sha256: bundled_hash,
        installed_sha256: installed_hash,
    })
}

fn install_with_artifact(
    game: &str,
    selected_directory: &Path,
    bundled: &Path,
) -> Result<ScsPluginStatus, String> {
    validate_game(game)?;
    let game_directory = canonical_directory(selected_directory, "游戏目录")?;
    let plugin_directory = resolve_plugin_directory(&game_directory, game, true)?;
    let plugin_path = plugin_directory.join(plugin_filename());
    let bundled_bytes = fs::read(bundled).map_err(|error| path_error(bundled, &error))?;
    let bundled_hash = bytes_hash(&bundled_bytes);

    if let Some(installed_hash) = existing_plugin_hash(&plugin_path)? {
        if installed_hash == bundled_hash {
            return inspect_with_artifact(game, &game_directory, bundled);
        }
        let backup_path = backup_path(&plugin_path);
        fs::copy(&plugin_path, &backup_path).map_err(|error| path_error(&backup_path, &error))?;
        sync_file(&backup_path)?;
        prune_backups(&plugin_directory, plugin_filename())?;
    }

    write_atomic(&plugin_path, &bundled_bytes)?;
    let installed_hash = file_hash(&plugin_path)?;
    if installed_hash != bundled_hash {
        return Err(format!("SCS 插件写入后校验失败：{}", plugin_path.display()));
    }
    inspect_with_artifact(game, &game_directory, bundled)
}

fn bundled_plugin(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_directory = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?;
    if let Some(bundled) = locate_bundled_plugin(&resource_directory) {
        return Ok(bundled);
    }

    #[cfg(debug_assertions)]
    {
        let development = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../target/scs-plugin-package")
            .join(plugin_filename());
        if development.is_file() {
            return development
                .canonicalize()
                .map_err(|error| path_error(&development, &error));
        }
    }
    Err("桌面安装包中缺少当前平台的 SCS bridge，请重新安装 OpenCarpanel".to_owned())
}

fn locate_bundled_plugin(resource_directory: &Path) -> Option<PathBuf> {
    // A glob entry such as `resources/**/*` keeps the leading `resources`
    // directory in Tauri bundles. Keep the flat candidate as a compatibility
    // fallback for development builds and future explicit resource mappings.
    [
        resource_directory
            .join("resources")
            .join("plugins")
            .join("scs")
            .join(plugin_filename()),
        resource_directory
            .join("plugins")
            .join("scs")
            .join(plugin_filename()),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
}

fn resolve_plugin_directory(
    game_directory: &Path,
    game: &str,
    create: bool,
) -> Result<PathBuf, String> {
    let binary_directory = binary_directory(game_directory, game);
    let binary_directory = canonical_directory(&binary_directory, "SCS 64 位程序目录")?;
    if !binary_directory.starts_with(game_directory) {
        return Err("SCS 程序目录逃逸出了用户选择的游戏目录".to_owned());
    }
    let plugin_directory = binary_directory.join("plugins");
    if plugin_directory.exists() {
        let metadata = fs::symlink_metadata(&plugin_directory)
            .map_err(|error| path_error(&plugin_directory, &error))?;
        if metadata.file_type().is_symlink() {
            return Err("拒绝写入符号链接形式的 SCS plugins 目录".to_owned());
        }
        let canonical = canonical_directory(&plugin_directory, "SCS plugins 目录")?;
        if !canonical.starts_with(&binary_directory) {
            return Err("SCS plugins 目录逃逸出了游戏程序目录".to_owned());
        }
        return Ok(canonical);
    }
    if !create {
        return Ok(plugin_directory);
    }
    fs::create_dir(&plugin_directory).map_err(|error| path_error(&plugin_directory, &error))?;
    let canonical = canonical_directory(&plugin_directory, "SCS plugins 目录")?;
    if !canonical.starts_with(&binary_directory) {
        return Err("创建的 SCS plugins 目录不在游戏程序目录内".to_owned());
    }
    Ok(canonical)
}

#[cfg(target_os = "windows")]
fn binary_directory(game_directory: &Path, _game: &str) -> PathBuf {
    game_directory.join("bin").join("win_x64")
}

#[cfg(target_os = "macos")]
fn binary_directory(game_directory: &Path, game: &str) -> PathBuf {
    let direct = game_directory.join("Contents").join("MacOS");
    if direct.is_dir() {
        return direct;
    }
    let app_name = match game {
        "ets2" => "Euro Truck Simulator 2.app",
        "ats" => "American Truck Simulator.app",
        _ => "OpenCarpanel Unsupported Game.app",
    };
    game_directory.join(app_name).join("Contents").join("MacOS")
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn binary_directory(game_directory: &Path, _game: &str) -> PathBuf {
    game_directory.join("bin").join("linux_x64")
}

#[cfg(target_os = "windows")]
const fn plugin_filename() -> &'static str {
    "opencarpanel-scs-telemetry.dll"
}

#[cfg(target_os = "macos")]
const fn plugin_filename() -> &'static str {
    "opencarpanel-scs-telemetry.dylib"
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const fn plugin_filename() -> &'static str {
    "opencarpanel-scs-telemetry.so"
}

fn validate_game(game: &str) -> Result<(), String> {
    if matches!(game, "ets2" | "ats") {
        Ok(())
    } else {
        Err("只允许为 ets2 或 ats 安装 SCS bridge".to_owned())
    }
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf, String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| path_error(path, &error))?;
    if !canonical.is_dir() {
        return Err(format!("{label}不是有效文件夹：{}", canonical.display()));
    }
    Ok(canonical)
}

fn existing_plugin_hash(path: &Path) -> Result<Option<String>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| path_error(path, &error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("SCS 插件目标不是普通文件：{}", path.display()));
    }
    file_hash(path).map(Some)
}

fn file_hash(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| path_error(path, &error))?;
    Ok(bytes_hash(&bytes))
}

fn bytes_hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = AtomicWriteFile::open(path).map_err(|error| path_error(path, &error))?;
    file.write_all(bytes)
        .map_err(|error| path_error(path, &error))?;
    file.commit().map_err(|error| path_error(path, &error))
}

fn sync_file(path: &Path) -> Result<(), String> {
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| path_error(path, &error))
}

fn backup_path(plugin_path: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    plugin_path.with_file_name(format!("{}.bak-{timestamp}", plugin_filename()))
}

fn prune_backups(directory: &Path, filename: &str) -> Result<(), String> {
    let prefix = format!("{filename}.bak-");
    let mut backups = fs::read_dir(directory)
        .map_err(|error| path_error(directory, &error))?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(&prefix))
                && entry.file_type().is_ok_and(|kind| kind.is_file())
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    backups.sort();
    let remove_count = backups.len().saturating_sub(BACKUP_LIMIT);
    for backup in backups.into_iter().take(remove_count) {
        fs::remove_file(&backup).map_err(|error| path_error(&backup, &error))?;
    }
    Ok(())
}

fn path_error(path: &Path, error: &io::Error) -> String {
    format!("{}：{error}", path.display())
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn install_is_idempotent_and_retains_a_backup() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let game = temp.path().join("Euro Truck Simulator 2");
        let binary = game.join(platform_binary_test_path());
        fs::create_dir_all(&binary)?;
        let artifact = temp.path().join(plugin_filename());
        fs::write(&artifact, b"new-plugin")?;

        let missing = inspect_with_artifact("ets2", &game, &artifact)?;
        assert_eq!(missing.state, ScsPluginState::Missing);
        let installed = install_with_artifact("ets2", &game, &artifact)?;
        assert_eq!(installed.state, ScsPluginState::Current);
        let installed_again = install_with_artifact("ets2", &game, &artifact)?;
        assert_eq!(installed_again.state, ScsPluginState::Current);

        fs::write(&artifact, b"updated-plugin")?;
        let updated = install_with_artifact("ets2", &game, &artifact)?;
        assert_eq!(updated.state, ScsPluginState::Current);
        assert!(fs::read_dir(binary.join("plugins"))?.any(|entry| {
            entry.is_ok_and(|value| value.file_name().to_string_lossy().contains(".bak-"))
        }));
        Ok(())
    }

    #[test]
    fn rejects_a_directory_without_the_platform_binary_marker() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let artifact = temp.path().join(plugin_filename());
        fs::write(&artifact, b"plugin")?;
        let error = match inspect_with_artifact("ats", temp.path(), &artifact) {
            Ok(_status) => return Err("missing binary marker was accepted".into()),
            Err(error) => error,
        };
        assert!(error.contains("win_x64") || error.contains("MacOS"));
        Ok(())
    }

    #[test]
    fn locates_the_scs_bridge_in_tauri_glob_resource_layout() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let artifact = temp
            .path()
            .join("resources")
            .join("plugins")
            .join("scs")
            .join(plugin_filename());
        let parent = artifact.parent().ok_or("artifact path has no parent")?;
        fs::create_dir_all(parent)?;
        fs::write(&artifact, b"plugin")?;

        assert_eq!(locate_bundled_plugin(temp.path()), Some(artifact));
        Ok(())
    }

    #[test]
    fn locates_the_scs_bridge_in_flat_resource_layout() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let artifact = temp
            .path()
            .join("plugins")
            .join("scs")
            .join(plugin_filename());
        let parent = artifact.parent().ok_or("artifact path has no parent")?;
        fs::create_dir_all(parent)?;
        fs::write(&artifact, b"plugin")?;

        assert_eq!(locate_bundled_plugin(temp.path()), Some(artifact));
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn platform_binary_test_path() -> PathBuf {
        PathBuf::from("bin").join("win_x64")
    }

    #[cfg(target_os = "macos")]
    fn platform_binary_test_path() -> PathBuf {
        PathBuf::from("Contents").join("MacOS")
    }
}
