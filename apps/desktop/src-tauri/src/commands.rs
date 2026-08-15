use std::sync::Arc;

use opensimdash_config::AppSettings;
use opensimdash_game_plugin_runtime::{
    MAX_PLUGIN_PACKAGE_BYTES, ensure_plugin_package_extension, verify_package,
};
use serde::Serialize;
use tauri::{AppHandle, Manager as _, State, ipc::Channel};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::{DialogExt as _, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_opener::OpenerExt as _;

use crate::runtime::{PairingTicket, RuntimeSnapshot, SharedDesktopRuntime};
use crate::scs::ScsPluginStatus;
use crate::updater::{UpdateInfo, UpdateProgress};

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
pub async fn discover_scs_directory(
    app: AppHandle,
    runtime: State<'_, SharedDesktopRuntime>,
    game: String,
) -> Result<Option<ScsPluginStatus>, String> {
    let data_directory = runtime.data_directory().to_path_buf();
    tauri::async_runtime::spawn_blocking(move || crate::scs::discover(&app, &data_directory, &game))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn choose_scs_directory(
    app: AppHandle,
    runtime: State<'_, SharedDesktopRuntime>,
    game: String,
) -> Result<Option<ScsPluginStatus>, String> {
    let handle = app.clone();
    let data_directory = runtime.data_directory().to_path_buf();
    tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .set_title("选择 Euro Truck Simulator 2 或 American Truck Simulator 文件夹")
            .blocking_pick_folder()
            .map(|path| path.into_path().map_err(|error| error.to_string()))
            .transpose()
            .and_then(|path| {
                path.map(|path| {
                    let status = crate::scs::inspect(&handle, &game, &path)?;
                    crate::scs::remember_status(&data_directory, &status);
                    Ok(status)
                })
                .transpose()
            })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn install_scs_plugin(
    app: AppHandle,
    runtime: State<'_, SharedDesktopRuntime>,
    game: String,
    selected_directory: String,
) -> Result<ScsPluginStatus, String> {
    let data_directory = runtime.data_directory().to_path_buf();
    tauri::async_runtime::spawn_blocking(move || {
        let status = crate::scs::install(&app, &game, std::path::Path::new(&selected_directory))?;
        crate::scs::remember_status(&data_directory, &status);
        Ok(status)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn install_game_plugin(
    app: AppHandle,
    runtime: State<'_, SharedDesktopRuntime>,
) -> Result<Option<RuntimeSnapshot>, String> {
    let handle = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        let selected = handle
            .dialog()
            .file()
            .set_title("安装 OpenSimDash 游戏插件")
            .add_filter("OpenSimDash 游戏插件", &["osd-plugin"])
            .blocking_pick_file();
        let Some(selected) = selected else {
            return Ok(None);
        };
        let package_path = selected.into_path().map_err(|error| error.to_string())?;
        ensure_plugin_package_extension(&package_path).map_err(|error| error.to_string())?;
        let metadata = std::fs::metadata(&package_path).map_err(|error| error.to_string())?;
        if !metadata.is_file() || metadata.len() > MAX_PLUGIN_PACKAGE_BYTES {
            return Err("插件包必须是不超过 4 MiB 的普通文件".to_owned());
        }
        let package = std::fs::read(&package_path).map_err(|error| error.to_string())?;
        let verified = verify_package(&package).map_err(|error| error.to_string())?;
        let confirmed = handle
            .dialog()
            .message(format!(
                "{}\n版本：{}\n发布者：{}\n许可证：{}\n\n插件将在受限 WASM 沙箱中运行。",
                verified.manifest.name,
                verified.manifest.version,
                verified.manifest.publisher,
                verified.manifest.license,
            ))
            .title("确认安装游戏插件")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "安装".to_owned(),
                "取消".to_owned(),
            ))
            .blocking_show();
        Ok(confirmed.then_some(package_path))
    })
    .await
    .map_err(|error| error.to_string())??;
    let Some(package_path) = selected else {
        return Ok(None);
    };
    runtime.install_game_plugin(package_path).await?;
    runtime.snapshot().await.map(Some)
}

#[tauri::command]
pub async fn remove_game_plugin(
    runtime: State<'_, SharedDesktopRuntime>,
    plugin_id: String,
) -> Result<RuntimeSnapshot, String> {
    runtime.remove_game_plugin(&plugin_id).await?;
    runtime.snapshot().await
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateInfo, String> {
    crate::updater::check(&app).await
}

#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    runtime: State<'_, SharedDesktopRuntime>,
    on_progress: Channel<UpdateProgress>,
) -> Result<(), String> {
    crate::updater::install(app, Arc::clone(runtime.inner()), on_progress).await
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
