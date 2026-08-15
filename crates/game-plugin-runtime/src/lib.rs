//! Sandboxed decoder execution and persistent `.ocp-plugin` package handling.

mod package;
mod wasm;

pub use package::{
    InstalledPlugin, MAX_INSTALLED_GAME_PLUGINS, MAX_PLUGIN_LOAD_ISSUES, MAX_PLUGIN_PACKAGE_BYTES,
    PluginInstallReceipt, PluginLoadIssue, VerifiedPluginPackage, install_package,
    load_installed_plugins, plugin_directory, remove_installed_plugin, verify_package,
};
pub use wasm::WasmGameAdapter;
