use std::{error::Error, sync::Arc};

use tauri::Manager as _;
use tauri_plugin_dialog::{DialogExt as _, MessageDialogKind};
use tracing_subscriber::EnvFilter;

mod commands;
mod installation_cache;
mod notifications;
mod runtime;
mod scs;
mod steam;
mod tray;
mod updater;

/// Starts the desktop shell and its single embedded Host runtime.
///
/// # Errors
///
/// Returns a Tauri setup or event-loop failure.
pub fn run() -> Result<(), Box<dyn Error>> {
    let _log_guard = match setup_logging() {
        Ok(guard) => Some(guard),
        Err(error) => {
            eprintln!("OpenCarpanel file logging is unavailable: {error}");
            None
        }
    };
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::refresh_runtime,
            commands::create_pairing,
            commands::revoke_device,
            commands::save_settings,
            commands::open_dashboard,
            commands::open_logs,
            commands::discover_scs_directory,
            commands::choose_scs_directory,
            commands::install_scs_plugin,
            commands::install_game_plugin,
            commands::remove_game_plugin,
            commands::check_for_updates,
            commands::install_update,
        ])
        .setup(|app| {
            match tauri::async_runtime::block_on(runtime::DesktopRuntime::start()) {
                Ok(runtime) => {
                    let runtime = Arc::new(runtime);
                    app.manage(Arc::clone(&runtime));
                    if let Err(error) = tray::setup(app) {
                        runtime.disable_tray();
                        tracing::warn!(%error, "system tray unavailable; close-to-tray disabled");
                    }
                    tray::show_main_window(app.handle());
                    notifications::start(app.handle().clone(), Arc::clone(&runtime));
                    updater::schedule_automatic_check(app.handle().clone(), runtime);
                }
                Err(error) => {
                    let handle = app.handle().clone();
                    app.dialog()
                        .message(format!("OpenCarpanel 无法启动。\n\n{error}"))
                        .title("OpenCarpanel 启动失败")
                        .kind(MessageDialogKind::Error)
                        .show(move |_| handle.exit(1));
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            let tauri::WindowEvent::CloseRequested { api, .. } = event else {
                return;
            };
            let Some(runtime) = window.try_state::<runtime::SharedDesktopRuntime>() else {
                return;
            };
            if runtime.settings().desktop.close_to_tray
                && runtime.tray_available()
                && !runtime.is_exiting()
            {
                api.prevent_close();
                let _hide_result = window.hide();
            }
        })
        .build(tauri::generate_context!())?;
    app.run(|app, event| {
        let tauri::RunEvent::ExitRequested { api, .. } = event else {
            return;
        };
        let Some(runtime) = commands::managed_runtime(app) else {
            return;
        };
        if runtime.begin_exit() {
            api.prevent_exit();
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let _shutdown_result = runtime.shutdown().await;
                handle.exit(0);
            });
        }
    });
    Ok(())
}

fn setup_logging() -> Result<tracing_appender::non_blocking::WorkerGuard, Box<dyn Error>> {
    let log_directory = opencarpanel_host::default_data_directory().join("logs");
    std::fs::create_dir_all(&log_directory)?;
    let file = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("opencarpanel")
        .filename_suffix("log")
        .max_log_files(7)
        .build(log_directory)?;
    let (writer, guard) = tracing_appender::non_blocking(file);
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_ansi(false)
        .with_writer(writer)
        .try_init()
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    Ok(guard)
}
