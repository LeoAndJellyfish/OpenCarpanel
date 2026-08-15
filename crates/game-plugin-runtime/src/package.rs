use std::{
    collections::BTreeSet,
    error::Error,
    fmt::{self, Display, Formatter},
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};

use atomic_write_file::AtomicWriteFile;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use opensimdash_game_plugin_api::{
    GamePluginManifest, MAX_PLUGIN_MODULE_BYTES, PluginRuntime, parse_manifest, parse_package,
};
use serde::Serialize;
use sha2::{Digest as _, Sha256};

const MANIFEST_FILENAME: &str = "manifest.json";
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
/// Canonical filename extension for installable game plugin packages.
pub const PLUGIN_PACKAGE_EXTENSION: &str = "osd-plugin";
/// Maximum on-disk `.osd-plugin` package size.
pub const MAX_PLUGIN_PACKAGE_BYTES: u64 = 4 * 1024 * 1024;
/// Maximum number of external decoders loaded into one Host process.
pub const MAX_INSTALLED_GAME_PLUGINS: usize = 16;
/// Maximum non-secret plugin discovery failures exposed in one diagnostic sample.
pub const MAX_PLUGIN_LOAD_ISSUES: usize = 32;

/// Fully decoded package after hash and manifest validation.
#[derive(Debug, Clone)]
pub struct VerifiedPluginPackage {
    /// Validated external-plugin manifest.
    pub manifest: GamePluginManifest,
    /// Hash-matching core-WASM module.
    pub module: Vec<u8>,
}

/// One installed manifest and its verified module path.
#[derive(Debug, Clone)]
pub struct InstalledPlugin {
    /// Validated manifest.
    pub manifest: GamePluginManifest,
    /// Canonical module path below the plugin directory.
    pub module_path: PathBuf,
}

/// Non-secret plugin discovery failure that does not prevent Host startup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginLoadIssue {
    /// Directory or parsed plugin id when safely available.
    pub plugin_id: Option<String>,
    /// Sanitized actionable reason without local paths or package bytes.
    pub message: String,
}

/// Successful installation summary returned to the desktop application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallReceipt {
    /// Installed stable id.
    pub id: String,
    /// Installed semantic version.
    pub version: String,
    /// Public publisher string.
    pub publisher: String,
}

/// Verifies one in-memory `.osd-plugin` envelope.
///
/// # Errors
///
/// Returns [`PluginPackageError`] when Base64, size, digest, or runtime fields fail.
pub fn verify_package(bytes: &[u8]) -> Result<VerifiedPluginPackage, PluginPackageError> {
    if bytes.len() > usize::try_from(MAX_PLUGIN_PACKAGE_BYTES).unwrap_or(usize::MAX) {
        return Err(package_error("package exceeds the 4 MiB limit"));
    }
    let package = parse_package(bytes).map_err(|error| package_error(error.to_string()))?;
    let maximum_base64 = MAX_PLUGIN_MODULE_BYTES.saturating_mul(4).div_ceil(3) + 4;
    if package.module_base64.len() > maximum_base64 {
        return Err(package_error("encoded WASM module exceeds the 2 MiB limit"));
    }
    let module = STANDARD
        .decode(package.module_base64.as_bytes())
        .map_err(|_| package_error("moduleBase64 is invalid"))?;
    if module.is_empty() || module.len() > MAX_PLUGIN_MODULE_BYTES {
        return Err(package_error(
            "decoded WASM module must be within 1 byte..=2 MiB",
        ));
    }
    let PluginRuntime::Wasm { sha256, .. } = &package.manifest.runtime else {
        return Err(package_error("installable package runtime must be WASM"));
    };
    if module_sha256(&module) != *sha256 {
        return Err(package_error("WASM SHA-256 does not match the manifest"));
    }
    Ok(VerifiedPluginPackage {
        manifest: package.manifest,
        module,
    })
}

/// Installs a validated package below the per-user game plugin directory.
///
/// The module is committed first and the manifest last, so interrupted updates
/// are skipped rather than loaded as mixed versions.
///
/// # Errors
///
/// Returns [`PluginPackageError`] for unsafe paths, I/O, or validation failures.
pub fn install_package(
    plugins_root: &Path,
    package_path: &Path,
    reserved_ids: &BTreeSet<String>,
) -> Result<PluginInstallReceipt, PluginPackageError> {
    ensure_plugin_package_extension(package_path)?;
    let metadata = fs::metadata(package_path).map_err(|error| io_error("read package", &error))?;
    if !metadata.is_file() || metadata.len() > MAX_PLUGIN_PACKAGE_BYTES {
        return Err(package_error(
            "package must be a regular file no larger than 4 MiB",
        ));
    }
    let bytes = fs::read(package_path).map_err(|error| io_error("read package", &error))?;
    let verified = verify_package(&bytes)?;
    if reserved_ids.contains(&verified.manifest.id) {
        return Err(package_error(
            "an installed plugin cannot replace a built-in plugin",
        ));
    }

    // Reject modules that cannot satisfy the runtime contract before touching
    // the installed plugin directory. A failed install therefore leaves the
    // previously installed version intact.
    crate::WasmGameAdapter::from_bytes(&verified.manifest, &verified.module)
        .map_err(|error| package_error(format!("decoder cannot be loaded: {error}")))?;

    fs::create_dir_all(plugins_root).map_err(|error| io_error("create plugin root", &error))?;
    reject_symlink(plugins_root, "plugin root")?;
    let directory = plugin_directory(plugins_root, &verified.manifest.id)?;
    if directory.exists() {
        reject_symlink(&directory, "plugin directory")?;
    } else {
        fs::create_dir(&directory).map_err(|error| io_error("create plugin directory", &error))?;
    }
    let PluginRuntime::Wasm { module, .. } = &verified.manifest.runtime else {
        return Err(package_error("installable package runtime must be WASM"));
    };
    let module_path = directory.join(module);
    atomic_write(&module_path, &verified.module)?;
    let mut manifest_bytes = serde_json::to_vec_pretty(&verified.manifest)
        .map_err(|error| package_error(format!("serialize manifest: {error}")))?;
    manifest_bytes.push(b'\n');
    atomic_write(&directory.join(MANIFEST_FILENAME), &manifest_bytes)?;

    Ok(PluginInstallReceipt {
        id: verified.manifest.id,
        version: verified.manifest.version,
        publisher: verified.manifest.publisher,
    })
}

/// Requires the canonical `.osd-plugin` filename extension.
///
/// # Errors
///
/// Returns [`PluginPackageError`] for every missing, differently cased, or
/// otherwise non-canonical extension.
pub fn ensure_plugin_package_extension(path: &Path) -> Result<(), PluginPackageError> {
    if path.extension().and_then(std::ffi::OsStr::to_str) != Some(PLUGIN_PACKAGE_EXTENSION) {
        return Err(package_error(
            "plugin package filename must end in .osd-plugin",
        ));
    }
    Ok(())
}

/// Loads every valid external plugin in stable id order.
#[must_use]
pub fn load_installed_plugins(
    plugins_root: &Path,
    reserved_ids: &BTreeSet<String>,
) -> (Vec<InstalledPlugin>, Vec<PluginLoadIssue>) {
    let mut installed = Vec::new();
    let mut issues = Vec::new();
    if let Err(error) = reject_symlink(plugins_root, "plugin root") {
        push_issue(
            &mut issues,
            PluginLoadIssue {
                plugin_id: None,
                message: error.to_string(),
            },
        );
        return (installed, issues);
    }
    let Ok(entries) = fs::read_dir(plugins_root) else {
        return (installed, issues);
    };
    let mut directories = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .is_ok_and(|kind| kind.is_dir() && !kind.is_symlink())
        })
        .collect::<Vec<_>>();
    directories.sort_by_key(fs::DirEntry::file_name);
    for entry in directories {
        if installed.len() >= MAX_INSTALLED_GAME_PLUGINS {
            push_issue(
                &mut issues,
                PluginLoadIssue {
                    plugin_id: None,
                    message: format!(
                        "installed plugin limit of {MAX_INSTALLED_GAME_PLUGINS} was reached"
                    ),
                },
            );
            break;
        }
        let directory_name = entry.file_name().to_string_lossy().into_owned();
        match load_one(&entry.path()) {
            Ok(plugin) if reserved_ids.contains(&plugin.manifest.id) => {
                push_issue(
                    &mut issues,
                    PluginLoadIssue {
                        plugin_id: Some(plugin.manifest.id),
                        message: "installed plugin conflicts with a built-in id".to_owned(),
                    },
                );
            }
            Ok(plugin) if plugin.manifest.id != directory_name => {
                push_issue(
                    &mut issues,
                    PluginLoadIssue {
                        plugin_id: Some(directory_name),
                        message: "plugin directory does not match manifest id".to_owned(),
                    },
                );
            }
            Ok(plugin) => installed.push(plugin),
            Err(error) => push_issue(
                &mut issues,
                PluginLoadIssue {
                    plugin_id: safe_directory_id(&directory_name),
                    message: error.to_string(),
                },
            ),
        }
    }
    (installed, issues)
}

fn push_issue(issues: &mut Vec<PluginLoadIssue>, issue: PluginLoadIssue) {
    if issues.len() < MAX_PLUGIN_LOAD_ISSUES {
        issues.push(issue);
    }
}

/// Removes exactly one validated external plugin directory.
///
/// # Errors
///
/// Returns [`PluginPackageError`] for an invalid id, symlink, or filesystem failure.
pub fn remove_installed_plugin(
    plugins_root: &Path,
    plugin_id: &str,
) -> Result<bool, PluginPackageError> {
    if plugins_root.exists() {
        reject_symlink(plugins_root, "plugin root")?;
    }
    let directory = plugin_directory(plugins_root, plugin_id)?;
    if !directory.exists() {
        return Ok(false);
    }
    reject_symlink(&directory, "plugin directory")?;
    fs::remove_dir_all(&directory).map_err(|error| io_error("remove plugin directory", &error))?;
    Ok(true)
}

/// Resolves a validated direct child below the plugin root.
///
/// # Errors
///
/// Returns [`PluginPackageError`] when `plugin_id` is not a valid adapter slug.
pub fn plugin_directory(
    plugins_root: &Path,
    plugin_id: &str,
) -> Result<PathBuf, PluginPackageError> {
    opensimdash_adapter_api::AdapterId::new(plugin_id.to_owned())
        .map_err(|_| package_error("plugin id is invalid"))?;
    Ok(plugins_root.join(plugin_id))
}

fn load_one(directory: &Path) -> Result<InstalledPlugin, PluginPackageError> {
    reject_symlink(directory, "plugin directory")?;
    let manifest_path = directory.join(MANIFEST_FILENAME);
    let metadata =
        fs::metadata(&manifest_path).map_err(|error| io_error("read manifest", &error))?;
    if !metadata.is_file() || metadata.len() > MAX_MANIFEST_BYTES {
        return Err(package_error(
            "manifest must be a regular file no larger than 64 KiB",
        ));
    }
    reject_symlink(&manifest_path, "manifest")?;
    let bytes = fs::read(&manifest_path).map_err(|error| io_error("read manifest", &error))?;
    let manifest = parse_manifest(&bytes).map_err(|error| package_error(error.to_string()))?;
    let PluginRuntime::Wasm { module, sha256, .. } = &manifest.runtime else {
        return Err(package_error("installed manifest must use a WASM runtime"));
    };
    let module_path = directory.join(module);
    reject_symlink(&module_path, "WASM module")?;
    let module_metadata =
        fs::metadata(&module_path).map_err(|error| io_error("read module", &error))?;
    if !module_metadata.is_file()
        || module_metadata.len() == 0
        || module_metadata.len() > u64::try_from(MAX_PLUGIN_MODULE_BYTES).unwrap_or(u64::MAX)
    {
        return Err(package_error(
            "WASM module must be a regular file no larger than 2 MiB",
        ));
    }
    let module_bytes = fs::read(&module_path).map_err(|error| io_error("read module", &error))?;
    if module_sha256(&module_bytes) != *sha256 {
        return Err(package_error("WASM SHA-256 does not match the manifest"));
    }
    Ok(InstalledPlugin {
        manifest,
        module_path,
    })
}

fn module_sha256(module: &[u8]) -> String {
    format!("{:x}", Sha256::digest(module))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), PluginPackageError> {
    let mut file = AtomicWriteFile::options()
        .open(path)
        .map_err(|error| io_error("open atomic plugin file", &error))?;
    file.write_all(bytes)
        .map_err(|error| io_error("write atomic plugin file", &error))?;
    file.commit()
        .map_err(|error| io_error("commit atomic plugin file", &error))
}

fn reject_symlink(path: &Path, kind: &str) -> Result<(), PluginPackageError> {
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(package_error(format!("{kind} must not be a symbolic link")));
    }
    Ok(())
}

fn safe_directory_id(value: &str) -> Option<String> {
    opensimdash_adapter_api::AdapterId::new(value.to_owned())
        .ok()
        .map(|id| id.as_str().to_owned())
}

/// Package validation or filesystem error with sanitized context.
#[derive(Debug)]
pub struct PluginPackageError {
    message: String,
}

impl Display for PluginPackageError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for PluginPackageError {}

fn package_error(message: impl Into<String>) -> PluginPackageError {
    PluginPackageError {
        message: message.into(),
    }
}

fn io_error(operation: &str, error: &std::io::Error) -> PluginPackageError {
    package_error(format!("{operation}: {}", error.kind()))
}

#[cfg(test)]
mod tests {
    use base64::engine::general_purpose::STANDARD;
    use opensimdash_game_plugin_api::{
        GAME_PLUGIN_PACKAGE_VERSION, GamePluginPackage, PluginRuntime,
    };

    use super::*;

    #[test]
    fn rejects_a_package_whose_module_digest_does_not_match() -> Result<(), Box<dyn Error>> {
        let manifest_bytes = include_bytes!("../../../plugins/games/f1-24/manifest.json");
        let mut manifest = parse_manifest(manifest_bytes)?;
        manifest.id = "example-game".to_owned();
        manifest.runtime = PluginRuntime::Wasm {
            abi_version: 1,
            module: "decoder.wasm".to_owned(),
            sha256: "0".repeat(64),
        };
        let bytes = serde_json::to_vec(&GamePluginPackage {
            package_version: GAME_PLUGIN_PACKAGE_VERSION,
            manifest,
            module_base64: STANDARD.encode(b"not-wasm"),
        })?;
        assert!(verify_package(&bytes).is_err());
        Ok(())
    }

    #[test]
    fn plugin_directory_never_accepts_path_components() {
        assert!(plugin_directory(Path::new("plugins"), "../escape").is_err());
        assert!(plugin_directory(Path::new("plugins"), "safe-game").is_ok());
    }

    #[test]
    fn package_extension_is_strict_and_canonical() {
        assert!(ensure_plugin_package_extension(Path::new("example.osd-plugin")).is_ok());
        assert!(ensure_plugin_package_extension(Path::new("example.plugin")).is_err());
        assert!(ensure_plugin_package_extension(Path::new("example.OSD-PLUGIN")).is_err());
        assert!(ensure_plugin_package_extension(Path::new("example")).is_err());
    }

    #[test]
    fn discovery_issue_collection_is_bounded() {
        let mut issues = Vec::new();
        for index in 0..=MAX_PLUGIN_LOAD_ISSUES {
            push_issue(
                &mut issues,
                PluginLoadIssue {
                    plugin_id: Some(format!("plugin-{index}")),
                    message: "invalid".to_owned(),
                },
            );
        }
        assert_eq!(issues.len(), MAX_PLUGIN_LOAD_ISSUES);
    }
}
