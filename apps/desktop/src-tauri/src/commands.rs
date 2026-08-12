use std::sync::Arc;

use opencarpanel_config::AppSettings;
use serde::Serialize;
use tauri::{AppHandle, Manager as _, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::DialogExt as _;
use tauri_plugin_opener::OpenerExt as _;

use crate::runtime::{PairingTicket, RuntimeSnapshot, SharedDesktopRuntime};
use crate::scs::ScsPluginStatus;
use crate::updater::UpdateInfo;

/// Bootstrap payload including operating-system integration state.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBootstrap {
    /// Embedded Host and persisted application state.
    #[serde(flatten)]
    pub runtime: RuntimeSnapshot,
    /// Actual login-item state reported by the operating system.
    pub autostart_enabled: bool,
}

#[tauri::command]
pub async fn bootstrap(
    app: AppHandle,
    runtime: State<'_, SharedDesktopRuntime>,
) -> Result<DesktopBootstrap, String> {
    let snapshot = runtime.snapshot().await?;
    let autostart_enabled = match app.autolaunch().is_enabled() {
        Ok(enabled) => enabled,
        Err(error) => {
            tracing::warn!(%error, "autostart status unavailable");
            false
        }
    };
    Ok(DesktopBootstrap {
        runtime: snapshot,
        autostart_enabled,
    })
}

#[tauri::command]
pub async fn refresh_runtime(
    runtime: State<'_, SharedDesktopRuntime>,
) -> Result<RuntimeSnapshot, String> {
    runtime.snapshot().await
}

#[tauri::command]
pub async fn create_pairing(
    runtime: State<'_, SharedDesktopRuntime>,
) -> Result<PairingTicket, String> {
    runtime.create_pairing().await
}

#[tauri::command]
pub async fn revoke_device(
    runtime: State<'_, SharedDesktopRuntime>,
    device_id: String,
) -> Result<RuntimeSnapshot, String> {
    runtime.revoke_device(&device_id).await?;
    runtime.snapshot().await
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    runtime: State<'_, SharedDesktopRuntime>,
    settings: AppSettings,
) -> Result<DesktopBootstrap, String> {
    let previous = runtime.settings();
    let autostart_changed = previous.desktop.launch_at_login != settings.desktop.launch_at_login;
    if autostart_changed {
        set_autostart(&app, settings.desktop.launch_at_login)?;
    }

    if let Err(error) = runtime.update_settings(settings).await {
        if autostart_changed {
            let _rollback_result = set_autostart(&app, previous.desktop.launch_at_login);
        }
        return Err(error);
    }
    bootstrap(app, runtime).await
}

#[tauri::command]
pub async fn open_dashboard(
    app: AppHandle,
    runtime: State<'_, SharedDesktopRuntime>,
    target: String,
) -> Result<(), String> {
    let url = runtime.dashboard_target(&target).await?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn open_logs(app: AppHandle, runtime: State<'_, SharedDesktopRuntime>) -> Result<(), String> {
    let path = runtime.data_directory().join("logs");
    std::fs::create_dir_all(&path).map_err(|error| error.to_string())?;
    app.opener()
        .open_path(path.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn choose_scs_directory(
    app: AppHandle,
    game: String,
) -> Result<Option<ScsPluginStatus>, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .set_title("选择 Euro Truck Simulator 2 或 American Truck Simulator 文件夹")
            .blocking_pick_folder()
            .map(|path| path.into_path().map_err(|error| error.to_string()))
            .transpose()
            .and_then(|path| {
                path.map(|path| crate::scs::inspect(&handle, &game, &path))
                    .transpose()
            })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn install_scs_plugin(
    app: AppHandle,
    game: String,
    selected_directory: String,
) -> Result<ScsPluginStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::scs::install(&app, &game, std::path::Path::new(&selected_directory))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateInfo, String> {
    crate::updater::check(&app).await
}

#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    runtime: State<'_, SharedDesktopRuntime>,
) -> Result<(), String> {
    crate::updater::install(app, Arc::clone(runtime.inner())).await
}

fn set_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|error| error.to_string())
}

/// Clones managed runtime state for lifecycle tasks that must outlive callbacks.
pub fn managed_runtime(app: &AppHandle) -> Option<SharedDesktopRuntime> {
    app.try_state::<SharedDesktopRuntime>()
        .map(|state| Arc::clone(state.inner()))
}
