use std::{
    collections::BTreeSet,
    error::Error,
    fmt::{self, Display, Formatter},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use opensimdash_config::{AppSettings, SettingsRepository};
use opensimdash_game_plugin_api::PluginSource;
use opensimdash_game_plugin_runtime::{
    PluginInstallReceipt, install_package, remove_installed_plugin,
};
use opensimdash_host::{
    HostConfig, HostDiagnostics, InstanceGuard, InstanceMode, PairedDevice, RunningHost, bind_host,
    dashboard_url, default_data_directory, default_runtime_directory, pairing_url, qr_svg,
};
use parking_lot::RwLock;
use serde::Serialize;
use tokio::sync::Mutex;

const PAIRING_LIFETIME: Duration = Duration::from_secs(10 * 60);

/// Process-owned desktop state. The embedded Host is the only network runtime.
#[derive(Debug)]
pub struct DesktopRuntime {
    host: Mutex<Option<RunningHost>>,
    settings: RwLock<AppSettings>,
    settings_repository: SettingsRepository,
    data_directory: PathBuf,
    recovery: RecoveryNotice,
    exiting: AtomicBool,
    tray_available: AtomicBool,
    _instance: InstanceGuard,
}

/// Safe startup failure shown by the native desktop error dialog.
#[derive(Debug)]
pub struct DesktopStartupError(String);

impl Display for DesktopStartupError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for DesktopStartupError {}

/// Non-secret configuration recovery details suitable for the control center.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryNotice {
    /// Whether a valid backup replaced the primary settings file.
    pub recovered: bool,
    /// Whether no valid backup existed and defaults were installed.
    pub reset_to_defaults: bool,
    /// Preserved invalid file path, when recovery quarantined one.
    pub quarantined_path: Option<String>,
}

/// URLs and listener addresses shown in the desktop control center.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEndpoints {
    /// LAN dashboard root.
    pub dashboard_url: String,
    /// LAN dashboard editor.
    pub editor_url: String,
    /// Local diagnostics endpoint.
    pub diagnostics_url: String,
    /// Actual bound HTTP address, including an ephemeral test port.
    pub http_address: String,
    /// Actual bound telemetry UDP address.
    pub udp_address: String,
}

/// Complete point-in-time state returned to the Preact shell.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSnapshot {
    /// Desktop binary version.
    pub version: &'static str,
    /// Valid persisted application settings.
    pub settings: AppSettings,
    /// Embedded Host counters and game detection state.
    pub diagnostics: HostDiagnostics,
    /// Browser and game listener endpoints.
    pub endpoints: RuntimeEndpoints,
    /// Remembered mobile dashboards without credentials.
    pub devices: Vec<PairedDevice>,
    /// Settings recovery action performed during this launch.
    pub recovery: RecoveryNotice,
    /// Shared user data directory.
    pub data_directory: String,
    /// Whether the operating system tray integration initialized successfully.
    pub tray_available: bool,
}

/// One-time phone pairing information. The secret remains in a URL fragment.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingTicket {
    /// One-time LAN URL.
    pub url: String,
    /// High-contrast SVG QR code for the URL.
    pub qr_svg: String,
    /// Ticket lifetime from issue time.
    pub expires_in_seconds: u64,
}

impl fmt::Debug for PairingTicket {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PairingTicket")
            .field("url", &"[REDACTED]")
            .field("qr_svg", &"[REDACTED]")
            .field("expires_in_seconds", &self.expires_in_seconds)
            .finish()
    }
}

impl DesktopRuntime {
    /// Acquires desktop ownership, loads settings, and starts the embedded Host.
    ///
    /// # Errors
    ///
    /// Returns a user-facing error for duplicate ownership, settings I/O,
    /// invalid overrides, or listener startup failure.
    pub async fn start() -> Result<Self, DesktopStartupError> {
        Self::start_at(default_data_directory(), default_runtime_directory(), true).await
    }

    async fn start_at(
        data_directory: PathBuf,
        runtime_directory: PathBuf,
        apply_environment: bool,
    ) -> Result<Self, DesktopStartupError> {
        let instance = InstanceGuard::acquire_in(runtime_directory, InstanceMode::Desktop)
            .map_err(startup_error)?;
        let settings_repository = SettingsRepository::new(&data_directory);
        let loaded = settings_repository.load().map_err(startup_error)?;
        let config = host_config(&loaded.settings, &data_directory, apply_environment)
            .map_err(startup_error)?;
        let host = bind_host(config).await.map_err(startup_error)?;
        let recovery = RecoveryNotice {
            recovered: loaded.recovered,
            reset_to_defaults: loaded.reset_to_defaults,
            quarantined_path: loaded
                .quarantined_path
                .map(|path| path.display().to_string()),
        };
        Ok(Self {
            host: Mutex::new(Some(host)),
            settings: RwLock::new(loaded.settings),
            settings_repository,
            data_directory,
            recovery,
            exiting: AtomicBool::new(false),
            tray_available: AtomicBool::new(true),
            _instance: instance,
        })
    }

    /// Returns the current validated settings without waiting on network state.
    #[must_use]
    pub fn settings(&self) -> AppSettings {
        self.settings.read().clone()
    }

    /// Returns the shared application-data directory for bounded desktop state.
    #[must_use]
    pub fn data_directory(&self) -> &Path {
        &self.data_directory
    }

    /// Whether the process has begun graceful exit.
    #[must_use]
    pub fn is_exiting(&self) -> bool {
        self.exiting.load(Ordering::Acquire)
    }

    /// Marks the first graceful-exit request. Returns false for repeated requests.
    pub fn begin_exit(&self) -> bool {
        !self.exiting.swap(true, Ordering::AcqRel)
    }

    /// Clears a temporary exit request when an operation recovered in-process.
    pub fn cancel_exit(&self) {
        self.exiting.store(false, Ordering::Release);
    }

    /// Whether close-to-tray can safely hide the main window this launch.
    #[must_use]
    pub fn tray_available(&self) -> bool {
        self.tray_available.load(Ordering::Acquire)
    }

    /// Disables close-to-tray after a recoverable operating-system tray error.
    pub fn disable_tray(&self) {
        self.tray_available.store(false, Ordering::Release);
    }

    /// Returns diagnostics, devices, URLs, and settings as one coherent UI payload.
    ///
    /// # Errors
    ///
    /// Returns an error only if the runtime is between a failed restart and recovery.
    pub async fn snapshot(&self) -> Result<RuntimeSnapshot, String> {
        let host = self.host.lock().await;
        let running = host
            .as_ref()
            .ok_or_else(|| "Host 当前未运行，请修正端口设置后重试".to_owned())?;
        let root = dashboard_url(running.http_address());
        Ok(RuntimeSnapshot {
            version: env!("CARGO_PKG_VERSION"),
            settings: self.settings(),
            diagnostics: running.diagnostics(),
            endpoints: RuntimeEndpoints {
                editor_url: format!("{root}/edit"),
                diagnostics_url: format!("{root}/api/v1/diagnostics"),
                dashboard_url: root,
                http_address: running.http_address().to_string(),
                udp_address: running.udp_address().to_string(),
            },
            devices: running.paired_devices().await,
            recovery: self.recovery.clone(),
            data_directory: self.data_directory.display().to_string(),
            tray_available: self.tray_available(),
        })
    }

    /// Issues a bounded, one-use pairing URL and matching QR code.
    ///
    /// # Errors
    ///
    /// Returns an error if secure randomness, the Host, or QR encoding is unavailable.
    pub async fn create_pairing(&self) -> Result<PairingTicket, String> {
        let host = self.host.lock().await;
        let running = host
            .as_ref()
            .ok_or_else(|| "Host 当前未运行，无法创建设备配对".to_owned())?;
        let token = running
            .issue_pairing_token(PAIRING_LIFETIME)
            .await
            .map_err(|error| error.to_string())?;
        let url = pairing_url(running.http_address(), &token);
        let qr_svg = qr_svg(&url).map_err(|error| error.to_string())?;
        Ok(PairingTicket {
            url,
            qr_svg,
            expires_in_seconds: PAIRING_LIFETIME.as_secs(),
        })
    }

    /// Revokes a remembered dashboard by non-secret id.
    ///
    /// # Errors
    ///
    /// Returns an error if the Host is unavailable or persistence fails.
    pub async fn revoke_device(&self, id: &str) -> Result<bool, String> {
        let host = self.host.lock().await;
        let running = host
            .as_ref()
            .ok_or_else(|| "Host 当前未运行，无法管理设备".to_owned())?;
        running
            .revoke_device(id)
            .await
            .map_err(|error| error.to_string())
    }

    /// Validates, applies, and atomically saves settings.
    ///
    /// Host-affecting changes restart the embedded runtime. Failed binds and
    /// failed persistence both restore the previous runtime configuration.
    ///
    /// # Errors
    ///
    /// Returns a detailed error while preserving the prior valid settings.
    pub async fn update_settings(&self, next: AppSettings) -> Result<(), String> {
        next.validate().map_err(|error| error.to_string())?;
        let previous = self.settings();
        if next.host == previous.host {
            self.settings_repository
                .save(&next)
                .map_err(|error| error.to_string())?;
            *self.settings.write() = next;
            return Ok(());
        }

        let previous_config = host_config(&previous, &self.data_directory, true)
            .map_err(|error| error.to_string())?;
        let next_config =
            host_config(&next, &self.data_directory, true).map_err(|error| error.to_string())?;
        let mut host_slot = self.host.lock().await;
        let previous_host = host_slot
            .take()
            .ok_or_else(|| "Host 当前未运行，无法应用设置".to_owned())?;

        if let Err(error) = previous_host.shutdown().await {
            let recovery = bind_host(previous_config.clone()).await;
            *host_slot = recovery.ok();
            return Err(format!("停止旧 Host 失败：{error}"));
        }

        let next_host = match bind_host(next_config).await {
            Ok(running) => running,
            Err(error) => {
                return restore_after_failure(
                    &mut host_slot,
                    previous_config,
                    format!("新端口或游戏设置无法启动：{error}"),
                )
                .await;
            }
        };

        if let Err(error) = self.settings_repository.save(&next) {
            let _shutdown_result = next_host.shutdown().await;
            return restore_after_failure(
                &mut host_slot,
                previous_config,
                format!("设置未能安全写入磁盘：{error}"),
            )
            .await;
        }

        *host_slot = Some(next_host);
        *self.settings.write() = next;
        Ok(())
    }

    /// Gracefully stops the one embedded Host. Safe to call more than once.
    pub async fn shutdown(&self) -> Result<(), String> {
        let mut host = self.host.lock().await;
        if let Some(running) = host.take() {
            running
                .shutdown()
                .await
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// Starts the current valid Host configuration when it is stopped.
    ///
    /// Used only to recover after an updater installer itself fails after the
    /// signed artifact was downloaded and verified.
    pub async fn restart_current_host(&self) -> Result<(), String> {
        let mut host = self.host.lock().await;
        if host.is_some() {
            return Ok(());
        }
        let config = host_config(&self.settings(), &self.data_directory, true)
            .map_err(|error| error.to_string())?;
        *host = Some(bind_host(config).await.map_err(|error| error.to_string())?);
        Ok(())
    }

    /// Installs or upgrades one verified game plugin, then reloads the Host registry.
    ///
    /// # Errors
    ///
    /// Returns a sanitized package, filesystem, or Host restart error.
    pub async fn install_game_plugin(
        &self,
        package_path: PathBuf,
    ) -> Result<PluginInstallReceipt, String> {
        let reserved_ids = self.builtin_plugin_ids().await?;
        let plugins_root = self.plugins_root();
        let receipt = tokio::task::spawn_blocking(move || {
            install_package(&plugins_root, &package_path, &reserved_ids)
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())??;
        self.reload_current_host().await?;
        Ok(receipt)
    }

    /// Removes one installed game plugin and reloads the Host registry.
    ///
    /// If the removed plugin is the fixed source, selection is safely changed
    /// to automatic detection before its files are removed.
    ///
    /// # Errors
    ///
    /// Returns an error for built-ins, unknown ids, filesystem failures, or a
    /// failed Host restart.
    pub async fn remove_game_plugin(&self, plugin_id: &str) -> Result<(), String> {
        let source = self.plugin_source(plugin_id).await?;
        if source != PluginSource::Installed {
            return Err("只能卸载用户安装的游戏插件".to_owned());
        }

        let mut settings = self.settings();
        if settings.host.adapter_selection == plugin_id {
            "auto".clone_into(&mut settings.host.adapter_selection);
            self.update_settings(settings).await?;
        }

        let plugins_root = self.plugins_root();
        let plugin_id = plugin_id.to_owned();
        let removed = tokio::task::spawn_blocking(move || {
            remove_installed_plugin(&plugins_root, &plugin_id).map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())??;
        if !removed {
            return Err("游戏插件已经不存在".to_owned());
        }
        self.reload_current_host().await
    }

    /// Returns a cheap diagnostic sample for background desktop integrations.
    pub async fn diagnostics(&self) -> Option<HostDiagnostics> {
        self.host
            .lock()
            .await
            .as_ref()
            .map(RunningHost::diagnostics)
    }

    /// Returns a browser URL for a fixed, allowlisted dashboard surface.
    ///
    /// # Errors
    ///
    /// Returns an error for unknown targets or an unavailable Host.
    pub async fn dashboard_target(&self, target: &str) -> Result<String, String> {
        let host = self.host.lock().await;
        let running = host.as_ref().ok_or_else(|| "Host 当前未运行".to_owned())?;
        let root = dashboard_url(running.http_address());
        match target {
            "dashboard" => Ok(root),
            "editor" => Ok(format!("{root}/edit")),
            "diagnostics" => Ok(format!("{root}/api/v1/diagnostics")),
            _ => Err("不允许打开未知的本地页面".to_owned()),
        }
    }

    fn plugins_root(&self) -> PathBuf {
        self.data_directory.join("game-plugins")
    }

    async fn builtin_plugin_ids(&self) -> Result<BTreeSet<String>, String> {
        let host = self.host.lock().await;
        let running = host
            .as_ref()
            .ok_or_else(|| "Host 当前未运行，无法校验游戏插件".to_owned())?;
        Ok(running
            .state()
            .supported_adapters()
            .iter()
            .filter(|adapter| adapter.metadata().source == PluginSource::Builtin)
            .map(|adapter| adapter.id().to_owned())
            .collect())
    }

    async fn plugin_source(&self, plugin_id: &str) -> Result<PluginSource, String> {
        let host = self.host.lock().await;
        let running = host
            .as_ref()
            .ok_or_else(|| "Host 当前未运行，无法管理游戏插件".to_owned())?;
        running
            .state()
            .supported_adapters()
            .iter()
            .find(|adapter| adapter.id() == plugin_id)
            .map(|adapter| adapter.metadata().source)
            .ok_or_else(|| "未找到这个游戏插件".to_owned())
    }

    async fn reload_current_host(&self) -> Result<(), String> {
        let config = host_config(&self.settings(), &self.data_directory, true)
            .map_err(|error| error.to_string())?;
        let mut host_slot = self.host.lock().await;
        let current = host_slot
            .take()
            .ok_or_else(|| "Host 当前未运行，无法重载游戏插件".to_owned())?;
        if let Err(error) = current.shutdown().await {
            *host_slot = bind_host(config).await.ok();
            return Err(format!("重载前停止 Host 失败：{error}"));
        }
        let running = bind_host(config)
            .await
            .map_err(|error| format!("游戏插件已变更，但 Host 重启失败：{error}"))?;
        *host_slot = Some(running);
        Ok(())
    }
}

fn host_config(
    settings: &AppSettings,
    data_directory: &Path,
    apply_environment: bool,
) -> Result<HostConfig, Box<dyn Error + Send + Sync>> {
    let mut config = HostConfig::from_settings(&settings.host, data_directory)?;
    if apply_environment {
        config.apply_environment_overrides()?;
    }
    Ok(config)
}

async fn restore_after_failure(
    host_slot: &mut Option<RunningHost>,
    previous_config: HostConfig,
    cause: String,
) -> Result<(), String> {
    match bind_host(previous_config).await {
        Ok(previous_host) => {
            *host_slot = Some(previous_host);
            Err(format!("{cause}；已恢复原有 Host"))
        }
        Err(recovery_error) => Err(format!(
            "{cause}；原有 Host 也无法恢复：{recovery_error}。请修正冲突后重新应用设置"
        )),
    }
}

fn startup_error(error: impl Display) -> DesktopStartupError {
    DesktopStartupError(error.to_string())
}

/// Managed state type shared by Tauri commands and lifecycle handlers.
pub type SharedDesktopRuntime = Arc<DesktopRuntime>;

#[cfg(test)]
mod tests {
    use std::{
        fs,
        net::{TcpListener, TcpStream},
    };

    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use opensimdash_game_plugin_api::{
        GAME_PLUGIN_ABI_VERSION, GAME_PLUGIN_PACKAGE_VERSION, GamePluginPackage, PluginRuntime,
        parse_manifest,
    };
    use sha2::{Digest as _, Sha256};

    use super::*;

    #[tokio::test]
    async fn failed_listener_change_restores_previous_host() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let data_directory = temp.path().join("data");
        let runtime_directory = temp.path().join("runtime");
        let available = TcpListener::bind("127.0.0.1:0")?;
        let initial_http_address = available.local_addr()?;
        drop(available);
        let mut initial = AppSettings::default();
        initial.host.http_bind = initial_http_address.to_string();
        initial.host.udp_bind = "127.0.0.1:0".to_owned();
        SettingsRepository::new(&data_directory).save(&initial)?;

        let runtime =
            DesktopRuntime::start_at(data_directory.clone(), runtime_directory, false).await?;
        let before = runtime.snapshot().await?;
        let occupied = TcpListener::bind("127.0.0.1:0")?;
        let mut rejected = initial.clone();
        rejected.host.http_bind = occupied.local_addr()?.to_string();

        let error = match runtime.update_settings(rejected).await {
            Ok(()) => return Err("occupied HTTP listener accepted the settings".into()),
            Err(error) => error,
        };
        assert!(error.contains("已恢复原有 Host"));
        assert!(error.contains("failed to bind HTTP"));
        let occupied_address = occupied.local_addr()?.to_string();
        assert!(error.contains(occupied_address.as_str()));
        let after = runtime.snapshot().await?;
        assert_eq!(after.settings, initial);
        assert_eq!(after.diagnostics.status, "ok");
        assert_eq!(after.endpoints.http_address, before.endpoints.http_address);
        assert_eq!(
            SettingsRepository::new(&data_directory).load()?.settings,
            initial
        );
        let probe = TcpStream::connect(&after.endpoints.http_address)?;
        drop(probe);
        runtime.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn corrupt_settings_recover_during_desktop_start() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let data_directory = temp.path().join("data");
        let runtime_directory = temp.path().join("runtime");
        let repository = SettingsRepository::new(&data_directory);
        let mut expected = AppSettings::default();
        expected.host.http_bind = "127.0.0.1:0".to_owned();
        expected.host.udp_bind = "127.0.0.1:0".to_owned();
        expected.host.snapshot_hz = 30;
        repository.save(&expected)?;
        fs::write(repository.settings_path(), b"{broken")?;

        let runtime = DesktopRuntime::start_at(data_directory, runtime_directory, false).await?;
        let snapshot = runtime.snapshot().await?;
        assert_eq!(snapshot.settings, expected);
        assert_eq!(snapshot.diagnostics.status, "ok");
        assert!(snapshot.recovery.recovered);
        assert!(!snapshot.recovery.reset_to_defaults);
        let quarantined = snapshot
            .recovery
            .quarantined_path
            .as_deref()
            .ok_or("desktop did not report the quarantined settings file")?;
        assert!(Path::new(quarantined).is_file());
        assert_eq!(repository.load()?.settings, expected);
        runtime.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn pairing_ticket_debug_output_redacts_the_secret() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let data_directory = temp.path().join("data");
        let runtime_directory = temp.path().join("runtime");
        let mut settings = AppSettings::default();
        settings.host.http_bind = "127.0.0.1:0".to_owned();
        settings.host.udp_bind = "127.0.0.1:0".to_owned();
        SettingsRepository::new(&data_directory).save(&settings)?;
        let runtime = DesktopRuntime::start_at(data_directory, runtime_directory, false).await?;

        let ticket = runtime.create_pairing().await?;
        let (_, secret) = ticket
            .url
            .split_once("#pair=")
            .ok_or("pairing URL did not contain a fragment credential")?;
        let debug = format!("{ticket:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains(secret));
        assert!(!debug.contains(ticket.url.as_str()));
        assert!(!debug.contains(ticket.qr_svg.as_str()));
        runtime.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn installed_plugin_reloads_and_uninstall_resets_a_fixed_selection()
    -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let data_directory = temp.path().join("data");
        let runtime_directory = temp.path().join("runtime");
        let mut initial = AppSettings::default();
        initial.host.http_bind = "127.0.0.1:0".to_owned();
        initial.host.udp_bind = "127.0.0.1:0".to_owned();
        SettingsRepository::new(&data_directory).save(&initial)?;
        let runtime = DesktopRuntime::start_at(data_directory, runtime_directory, false).await?;

        let module = wat::parse_str(
            r#"(module
              (memory (export "memory") 6 64)
              (func (export "osd_plugin_abi_version") (result i32) i32.const 1)
              (func (export "osd_input_ptr") (result i32) i32.const 0)
              (func (export "osd_input_capacity") (result i32) i32.const 65536)
              (func (export "osd_output_ptr") (result i32) i32.const 65536)
              (func (export "osd_output_capacity") (result i32) i32.const 262144)
              (func (export "osd_decode") (param i32 i64) (result i32) i32.const 0)
            )"#,
        )?;
        let mut manifest = parse_manifest(include_bytes!(
            "../../../../plugins/games/f1-24/manifest.json"
        ))?;
        manifest.id = "desktop-test-sim".to_owned();
        manifest.name = "Desktop Test Sim".to_owned();
        manifest.runtime = PluginRuntime::Wasm {
            abi_version: GAME_PLUGIN_ABI_VERSION,
            module: "decoder.wasm".to_owned(),
            sha256: format!("{:x}", Sha256::digest(&module)),
        };
        let package_path = temp.path().join("desktop-test-sim.osd-plugin");
        fs::write(
            &package_path,
            serde_json::to_vec(&GamePluginPackage {
                package_version: GAME_PLUGIN_PACKAGE_VERSION,
                manifest,
                module_base64: STANDARD.encode(module),
            })?,
        )?;

        let receipt = runtime.install_game_plugin(package_path).await?;
        assert_eq!(receipt.id, "desktop-test-sim");
        assert!(
            runtime
                .snapshot()
                .await?
                .diagnostics
                .supported_adapters
                .iter()
                .any(|adapter| adapter.id == "desktop-test-sim")
        );

        let mut fixed = runtime.settings();
        fixed.host.adapter_selection = "desktop-test-sim".to_owned();
        runtime.update_settings(fixed).await?;
        runtime.remove_game_plugin("desktop-test-sim").await?;
        let after = runtime.snapshot().await?;
        assert_eq!(after.settings.host.adapter_selection, "auto");
        assert!(
            after
                .diagnostics
                .supported_adapters
                .iter()
                .all(|adapter| adapter.id != "desktop-test-sim")
        );

        runtime.shutdown().await?;
        Ok(())
    }
}
