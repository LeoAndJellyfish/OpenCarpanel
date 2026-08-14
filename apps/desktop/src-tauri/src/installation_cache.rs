use std::{
    fs, io,
    io::Write as _,
    path::{Path, PathBuf},
};

use atomic_write_file::AtomicWriteFile;
use serde::{Deserialize, Serialize};

const CACHE_FILENAME: &str = "scs-installations.json";
const CACHE_SCHEMA_VERSION: u32 = 1;
const MAX_CACHE_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallationCache {
    schema_version: u32,
    #[serde(default)]
    ets2: Option<String>,
    #[serde(default)]
    ats: Option<String>,
}

impl Default for InstallationCache {
    fn default() -> Self {
        Self {
            schema_version: CACHE_SCHEMA_VERSION,
            ets2: None,
            ats: None,
        }
    }
}

/// Loads a previously validated game root for a supported SCS game.
///
/// # Errors
///
/// Returns an error for unsupported game ids, unreadable state, oversized
/// input, invalid JSON, or an unsupported cache schema.
pub(crate) fn load(data_directory: &Path, game: &str) -> Result<Option<PathBuf>, String> {
    let cache = load_cache(data_directory)?;
    let value = match game {
        "ets2" => cache.ets2,
        "ats" => cache.ats,
        _ => return Err("SCS 安装目录缓存只支持 ets2 或 ats".to_owned()),
    };
    Ok(value.map(PathBuf::from))
}

/// Persists a canonical game root without modifying the game installation.
///
/// # Errors
///
/// Rejects unsupported game ids and invalid directories, and reports bounded
/// JSON serialization or atomic-write failures.
pub(crate) fn remember(data_directory: &Path, game: &str, path: &Path) -> Result<(), String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| path_error(path, &error))?;
    if !canonical.is_dir() {
        return Err(format!("游戏目录不是有效文件夹：{}", canonical.display()));
    }
    let mut cache = load_cache(data_directory).unwrap_or_default();
    let value = Some(canonical.to_string_lossy().into_owned());
    match game {
        "ets2" => cache.ets2 = value,
        "ats" => cache.ats = value,
        _ => return Err("SCS 安装目录缓存只支持 ets2 或 ats".to_owned()),
    }
    write_cache(data_directory, &cache)
}

fn load_cache(data_directory: &Path) -> Result<InstallationCache, String> {
    let path = data_directory.join(CACHE_FILENAME);
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(InstallationCache::default());
        }
        Err(error) => return Err(path_error(&path, &error)),
    };
    if !metadata.is_file() || metadata.len() > MAX_CACHE_BYTES {
        return Err(format!("SCS 安装目录缓存无效：{}", path.display()));
    }
    let bytes = fs::read(&path).map_err(|error| path_error(&path, &error))?;
    let cache: InstallationCache =
        serde_json::from_slice(&bytes).map_err(|error| format!("{}：{error}", path.display()))?;
    if cache.schema_version != CACHE_SCHEMA_VERSION {
        return Err(format!(
            "SCS 安装目录缓存版本 {} 不受支持",
            cache.schema_version
        ));
    }
    Ok(cache)
}

fn write_cache(data_directory: &Path, cache: &InstallationCache) -> Result<(), String> {
    fs::create_dir_all(data_directory).map_err(|error| path_error(data_directory, &error))?;
    let path = data_directory.join(CACHE_FILENAME);
    let bytes = serde_json::to_vec_pretty(cache).map_err(|error| error.to_string())?;
    let mut file = AtomicWriteFile::open(&path).map_err(|error| path_error(&path, &error))?;
    file.write_all(&bytes)
        .map_err(|error| path_error(&path, &error))?;
    file.commit().map_err(|error| path_error(&path, &error))
}

fn path_error(path: &Path, error: &io::Error) -> String {
    format!("{}：{error}", path.display())
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn remembers_ets2_and_ats_independently() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let data = temp.path().join("data");
        let ets2 = temp.path().join("Euro Truck Simulator 2");
        let ats = temp.path().join("American Truck Simulator");
        fs::create_dir_all(&ets2)?;
        fs::create_dir_all(&ats)?;

        remember(&data, "ets2", &ets2)?;
        remember(&data, "ats", &ats)?;

        assert_eq!(load(&data, "ets2")?, Some(ets2.canonicalize()?));
        assert_eq!(load(&data, "ats")?, Some(ats.canonicalize()?));
        Ok(())
    }

    #[test]
    fn replaces_a_corrupt_recreatable_cache() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let data = temp.path().join("data");
        let game = temp.path().join("Euro Truck Simulator 2");
        fs::create_dir_all(&data)?;
        fs::create_dir_all(&game)?;
        fs::write(data.join(CACHE_FILENAME), b"not-json")?;

        remember(&data, "ets2", &game)?;

        assert_eq!(load(&data, "ets2")?, Some(game.canonicalize()?));
        assert_eq!(load(&data, "ats")?, None);
        Ok(())
    }
}
