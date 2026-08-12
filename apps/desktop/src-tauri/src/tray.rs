use crate::commands::managed_runtime;
use tauri::{
    App, AppHandle, Manager as _,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_opener::OpenerExt as _;

const MENU_SHOW: &str = "show-control-center";
const MENU_DASHBOARD: &str = "open-dashboard";
const MENU_PAIR: &str = "pair-device";
const MENU_QUIT: &str = "quit";

/// Creates the process-lifetime system tray and its fixed local actions.
///
/// # Errors
///
/// Returns a Tauri menu, image, or tray registration error.
pub fn setup(app: &App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, MENU_SHOW, "打开控制中心", true, None::<&str>)?;
    let dashboard = MenuItem::with_id(app, MENU_DASHBOARD, "打开手机仪表盘", true, None::<&str>)?;
    let pair = MenuItem::with_id(app, MENU_PAIR, "配对新设备", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "退出 OpenCarpanel", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &dashboard, &pair, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("OpenCarpanel · Host 正在运行")
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_SHOW => show_main_window(app),
            MENU_DASHBOARD => open_dashboard(app),
            MENU_PAIR => open_pairing(app),
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    let _tray = tray.build(app)?;
    Ok(())
}

/// Shows and focuses the singleton control-center window.
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _show_result = window.show();
        let _focus_result = window.set_focus();
    }
}

fn open_dashboard(app: &AppHandle) {
    let Some(runtime) = managed_runtime(app) else {
        return;
    };
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(url) = runtime.dashboard_target("dashboard").await {
            let _open_result = handle.opener().open_url(url, None::<&str>);
        }
    });
}

fn open_pairing(app: &AppHandle) {
    show_main_window(app);
    if let Some(window) = app.get_webview_window("main") {
        let _navigation = window.eval("window.location.search='?section=pairing'");
    }
}
