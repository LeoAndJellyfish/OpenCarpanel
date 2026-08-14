use std::{
    fs,
    io::Write,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use atomic_write_file::AtomicWriteFile;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, ipc::Channel};
use tauri_plugin_notification::NotificationExt as _;
use tauri_plugin_updater::UpdaterExt as _;

use crate::runtime::DesktopRuntime;

const AUTO_CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const AUTO_CHECK_DELAY: Duration = Duration::from_secs(8);
const UPDATE_STATE_SCHEMA: u16 = 1;

/// User-facing signed updater result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// Whether a newer version is available for this platform.
    pub available: bool,
    /// Currently installed version.
    pub current_version: String,
    /// Announced newer version.
    pub version: Option<String>,
    /// Release notes supplied by the GitHub manifest.
    pub notes: Option<String>,
    /// Publication timestamp when supplied.
    pub published_at: Option<String>,
}

/// Ordered progress events emitted while installing a signed update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum UpdateProgress {
    /// The updater is confirming that the announced release is still available.
    Preparing,
    /// A signed release artifact is being downloaded.
    #[serde(rename_all = "camelCase")]
    Downloading {
        /// Bytes received across all chunks so far.
        downloaded_bytes: u64,
        /// Server-provided artifact size when available.
        total_bytes: Option<u64>,
    },
    /// The download is complete and its embedded signature is being verified.
    Verifying,
    /// Signature verification succeeded and the installer is about to start.
    Installing,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateState {
    schema_version: u16,
    last_check_unix_ms: u64,
}

/// Checks the configured HTTPS manifest without downloading an installer.
///
/// # Errors
///
/// Returns plugin configuration, network, or manifest errors.
pub async fn check(app: &AppHandle) -> Result<UpdateInfo, String> {
    let current_version = app.package_info().version.to_string();
    let update = app
        .updater()
        .map_err(|error| error.to_string())?
        .check()
        .await
        .map_err(|error| error.to_string())?;
    Ok(match update {
        Some(update) => UpdateInfo {
            available: true,
            current_version,
            version: Some(update.version),
            notes: update.body.map(|value| bounded_notes(&value)),
            published_at: update.date.map(|value| value.to_string()),
        },
        None => UpdateInfo {
            available: false,
            current_version,
            version: None,
            notes: None,
            published_at: None,
        },
    })
}

/// Downloads, verifies, and installs the newest artifact.
///
/// The embedded Host remains live during network transfer. It is shut down
/// only after signature verification succeeds. If the platform installer then
/// fails synchronously, the current Host configuration is restored.
///
/// # Errors
///
/// Returns when no update exists or any check, download, signature, install,
/// shutdown, or recovery step fails.
pub async fn install(
    app: AppHandle,
    runtime: Arc<DesktopRuntime>,
    on_progress: Channel<UpdateProgress>,
) -> Result<(), String> {
    let _ = on_progress.send(UpdateProgress::Preparing);
    let update = app
        .updater()
        .map_err(|error| error.to_string())?
        .check()
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "当前已经是最新版本".to_owned())?;
    let download_progress = on_progress.clone();
    let verification_progress = on_progress.clone();
    let mut downloaded_bytes = 0;
    let bytes = update
        .download(
            move |chunk_length, total_bytes| {
                let progress = record_chunk(&mut downloaded_bytes, chunk_length, total_bytes);
                let _ = download_progress.send(progress);
            },
            move || {
                let _ = verification_progress.send(UpdateProgress::Verifying);
            },
        )
        .await
        .map_err(|error| format!("更新下载或签名验证失败：{error}"))?;

    let _ = on_progress.send(UpdateProgress::Installing);
    runtime.begin_exit();
    if let Err(error) = runtime.shutdown().await {
        runtime.cancel_exit();
        return Err(error);
    }
    if let Err(error) = update.install(bytes) {
        let recovery = runtime.restart_current_host().await;
        runtime.cancel_exit();
        return match recovery {
            Ok(()) => Err(format!("更新安装器启动失败：{error}；原有 Host 已恢复")),
            Err(recovery_error) => Err(format!(
                "更新安装器启动失败：{error}；Host 恢复也失败：{recovery_error}"
            )),
        };
    }

    app.restart();
}

/// Schedules at most one background check per 24 hours.
pub fn schedule_automatic_check(app: AppHandle, runtime: Arc<DesktopRuntime>) {
    if !runtime.settings().desktop.automatic_updates
        || !claim_automatic_check(runtime.data_directory())
    {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(AUTO_CHECK_DELAY).await;
        let Ok(info) = check(&app).await else {
            return;
        };
        if info.available && runtime.settings().desktop.notifications_enabled {
            let version = info.version.as_deref().unwrap_or("新版本");
            let _notification = app
                .notification()
                .builder()
                .title("OpenCarpanel 更新可用")
                .body(format!(
                    "{version} 已通过发布清单发现，可在控制中心中下载并验签。"
                ))
                .show();
        }
    });
}

fn claim_automatic_check(data_directory: &std::path::Path) -> bool {
    let path = data_directory.join("update-state.json");
    let now = unix_time_ms();
    if let Ok(bytes) = fs::read(&path)
        && let Ok(state) = serde_json::from_slice::<UpdateState>(&bytes)
        && state.schema_version == UPDATE_STATE_SCHEMA
        && now.saturating_sub(state.last_check_unix_ms)
            < u64::try_from(AUTO_CHECK_INTERVAL.as_millis()).unwrap_or(u64::MAX)
    {
        return false;
    }

    let state = UpdateState {
        schema_version: UPDATE_STATE_SCHEMA,
        last_check_unix_ms: now,
    };
    let Ok(mut bytes) = serde_json::to_vec_pretty(&state) else {
        return false;
    };
    bytes.push(b'\n');
    if fs::create_dir_all(data_directory).is_err() {
        return false;
    }
    let Ok(mut file) = AtomicWriteFile::open(&path) else {
        return false;
    };
    file.write_all(&bytes).is_ok() && file.commit().is_ok()
}

fn bounded_notes(value: &str) -> String {
    value.chars().take(4_000).collect()
}

fn record_chunk(
    downloaded_bytes: &mut u64,
    chunk_length: usize,
    total_bytes: Option<u64>,
) -> UpdateProgress {
    *downloaded_bytes =
        downloaded_bytes.saturating_add(u64::try_from(chunk_length).unwrap_or(u64::MAX));
    UpdateProgress::Downloading {
        downloaded_bytes: *downloaded_bytes,
        total_bytes,
    }
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
    use std::error::Error;

    use super::*;

    #[test]
    fn automatic_check_claim_is_bounded_to_one_day() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        assert!(claim_automatic_check(temp.path()));
        assert!(!claim_automatic_check(temp.path()));
        Ok(())
    }

    #[test]
    fn release_notes_are_bounded() {
        let input = "x".repeat(5_000);
        assert_eq!(bounded_notes(&input).len(), 4_000);
    }

    #[test]
    fn download_progress_accumulates_chunks() {
        let mut downloaded_bytes = 0;
        assert_eq!(
            record_chunk(&mut downloaded_bytes, 32, Some(96)),
            UpdateProgress::Downloading {
                downloaded_bytes: 32,
                total_bytes: Some(96),
            }
        );
        assert_eq!(
            record_chunk(&mut downloaded_bytes, 64, Some(96)),
            UpdateProgress::Downloading {
                downloaded_bytes: 96,
                total_bytes: Some(96),
            }
        );
    }

    #[test]
    fn update_progress_uses_the_frontend_json_contract() -> Result<(), Box<dyn Error>> {
        let value = serde_json::to_value(UpdateProgress::Downloading {
            downloaded_bytes: 64,
            total_bytes: Some(128),
        })?;
        assert_eq!(
            value,
            serde_json::json!({
                "phase": "downloading",
                "downloadedBytes": 64,
                "totalBytes": 128,
            })
        );
        Ok(())
    }
}
