use std::{sync::Arc, time::Duration};

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt as _;

use crate::runtime::DesktopRuntime;

/// Watches low-frequency game identity changes and emits optional OS notifications.
pub fn start(app: AppHandle, runtime: Arc<DesktopRuntime>) {
    tauri::async_runtime::spawn(async move {
        let mut previous: Option<String> = None;
        let mut initialized = false;
        loop {
            tokio::time::sleep(Duration::from_millis(750)).await;
            if runtime.is_exiting() {
                return;
            }
            let Some(diagnostics) = runtime.diagnostics().await else {
                continue;
            };
            let current = diagnostics.active_adapter;
            if initialized
                && current != previous
                && runtime.settings().desktop.notifications_enabled
            {
                let (title, body) = match (&previous, &current) {
                    (Some(_), Some(game)) => {
                        ("OpenSimDash 已切换游戏", format!("当前遥测来源：{game}"))
                    }
                    (None, Some(game)) => {
                        ("OpenSimDash 已连接游戏", format!("已开始接收 {game} 遥测"))
                    }
                    (Some(game), None) => (
                        "OpenSimDash 游戏数据已暂停",
                        format!("{game} 已停止发送遥测，Host 仍在运行"),
                    ),
                    (None, None) => ("OpenSimDash", "Host 正在等待游戏数据".to_owned()),
                };
                let _notification = app.notification().builder().title(title).body(body).show();
            }
            if current != previous {
                previous = current;
            }
            initialized = true;
        }
    });
}
