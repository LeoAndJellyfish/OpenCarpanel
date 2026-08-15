//! Stable manifests, package envelopes, metadata and decoder messages shared
//! by the Host, dashboard bindings and third-party game plugins.

use std::{
    collections::BTreeSet,
    error::Error,
    fmt::{self, Display, Formatter},
    path::{Component, Path},
};

use opencarpanel_adapter_api::AdapterId;
use opencarpanel_telemetry_core::{TelemetryEvent, TelemetryField, TelemetryUpdate};
use schemars::JsonSchema;
use semver::Version;
use serde::{Deserialize, Serialize};

/// Current game plugin manifest schema.
pub const GAME_PLUGIN_MANIFEST_VERSION: u16 = 1;
/// Current sandboxed decoder ABI.
pub const GAME_PLUGIN_ABI_VERSION: u16 = 1;
/// Current distributable package envelope schema.
pub const GAME_PLUGIN_PACKAGE_VERSION: u16 = 1;
/// Largest UDP payload accepted by the platform.
pub const MAX_PLUGIN_DATAGRAM_BYTES: u32 = 65_507;
/// Maximum decoded WASM module size accepted from a package.
pub const MAX_PLUGIN_MODULE_BYTES: usize = 2 * 1024 * 1024;
/// Maximum serialized decoder response copied out of guest memory.
pub const MAX_PLUGIN_OUTPUT_BYTES: usize = 256 * 1024;
/// Maximum updates emitted for one input datagram.
pub const MAX_PLUGIN_UPDATES: usize = 8;
/// Maximum reliable events emitted for one input datagram.
pub const MAX_PLUGIN_EVENTS: usize = 16;

const MAX_SHORT_TEXT_BYTES: usize = 128;
const MAX_DESCRIPTION_BYTES: usize = 1_024;
const MAX_SETUP_STEPS: usize = 16;
const MAX_WIDGETS: usize = 32;
const MAX_PLUGIN_ID_BYTES: usize = 64;

/// Complete source-of-truth declaration for one supported game or telemetry producer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GamePluginManifest {
    /// Manifest schema version.
    pub schema_version: u16,
    /// Stable lowercase game/plugin identifier.
    pub id: String,
    /// Product-facing source name.
    pub name: String,
    /// Plugin implementation semantic version.
    pub version: String,
    /// Publisher shown before local installation.
    pub publisher: String,
    /// SPDX license expression or short license identifier.
    pub license: String,
    /// Concise purpose and compatibility summary.
    pub description: String,
    /// Decoder implementation selected by the Host.
    pub runtime: PluginRuntime,
    /// Human-readable upstream wire protocol information.
    pub protocol: PluginProtocol,
    /// Host-owned input transport declaration.
    pub ingress: PluginIngress,
    /// Canonical telemetry paths this decoder can produce.
    pub capabilities: Vec<TelemetryField>,
    /// Trusted Dashboard presentation configuration.
    pub presentation: PluginPresentation,
    /// Declarative setup workflow for the desktop control center.
    pub setup: PluginSetup,
}

/// Decoder implementation referenced by a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum PluginRuntime {
    /// Adapter compiled into the official Host.
    Builtin {
        /// Factory entrypoint resolved by the built-in registry.
        entrypoint: String,
    },
    /// Sandboxed WebAssembly adapter installed from a package.
    Wasm {
        /// Stable ABI required by the module.
        abi_version: u16,
        /// Safe package-relative module filename.
        module: String,
        /// Lowercase SHA-256 of the decoded module bytes.
        sha256: String,
    },
}

/// Upstream game or software telemetry protocol label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginProtocol {
    /// Protocol family, such as `EA UDP` or `SCS bridge`.
    pub name: String,
    /// Accepted upstream protocol versions.
    pub version: String,
}

/// Host-owned transport used to deliver bytes to a decoder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginIngress {
    /// v1 transport kind.
    pub kind: PluginIngressKind,
    /// Suggested port when configuring the game or producer.
    pub default_port: u16,
    /// Decoder-specific maximum input size.
    pub max_datagram_bytes: u32,
}

/// Supported plugin ingress transports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginIngressKind {
    /// Datagram arrives on the single UDP listener owned by the Host.
    SharedUdp,
}

/// Safe presentation values consumed by the trusted Dashboard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginPresentation {
    /// Compact tab/status label.
    pub short_name: String,
    /// Secondary label rendered next to the game name.
    pub detail: String,
    /// Broad visual family.
    pub family: PluginGameFamily,
    /// Semantics used by the built-in status widget.
    pub status_mode: PluginStatusMode,
    /// Built-in responsive placement template.
    pub layout_preset: PluginLayoutPreset,
    /// Theme values applied to a new per-game layout.
    pub theme: PluginTheme,
    /// Safe fallback for games that do not report maximum RPM.
    pub fallback_rpm_max: u16,
    /// Trusted built-in widget types offered for this game.
    pub widgets: Vec<String>,
}

/// High-level dashboard visual family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginGameFamily {
    /// Circuit/open-wheel oriented density.
    Formula,
    /// Road and hauling oriented density.
    Truck,
    /// General driving telemetry layout.
    Generic,
}

/// Built-in status widget semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginStatusMode {
    /// DRS/active-aero state.
    Drs,
    /// SCS bridge and job state.
    Scs,
    /// Transport freshness only.
    Generic,
}

/// Responsive layout template selected by a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginLayoutPreset {
    /// Six-panel formula layout.
    Formula,
    /// Five-panel trucking layout.
    Truck,
    /// Four core telemetry panels.
    Generic,
}

/// Hexadecimal colors used for a plugin's default layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginTheme {
    /// Dashboard background.
    pub background: String,
    /// Primary text and marks.
    pub foreground: String,
    /// Game accent color.
    pub accent: String,
    /// Warning color.
    pub warning: String,
}

/// Declarative setup workflow rendered by the desktop application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum PluginSetup {
    /// EA F1 in-game UDP settings.
    F1Udp {
        /// Format label the user selects in game.
        format: String,
        /// Recommended game send rate.
        send_rate_hz: u16,
    },
    /// Official SCS SDK bridge installation.
    ScsSdk {
        /// Steam application id used for discovery.
        steam_app_id: u32,
        /// Expected Steam install directory name.
        directory_name: String,
    },
    /// Generic UDP producer instructions.
    Udp {
        /// Ordered concise configuration steps.
        steps: Vec<String>,
    },
    /// No setup beyond starting the producer.
    None,
}

/// Origin of a plugin visible in diagnostics and management UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginSource {
    /// Shipped and compiled with `OpenCarpanel`.
    Builtin,
    /// Installed in the current user's data directory.
    Installed,
}

/// Non-secret manifest subset sent to desktop and dashboard clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GamePluginMetadata {
    /// Stable game/plugin identifier.
    pub id: String,
    /// Product-facing source name.
    pub name: String,
    /// Plugin implementation version.
    pub version: String,
    /// Publisher shown in diagnostics.
    pub publisher: String,
    /// Concise compatibility summary.
    pub description: String,
    /// Human-readable accepted protocol version.
    pub protocol_version: String,
    /// Host-owned transport declaration.
    pub ingress: PluginIngress,
    /// Canonical telemetry fields.
    pub capabilities: Vec<TelemetryField>,
    /// Dashboard configuration.
    pub presentation: PluginPresentation,
    /// Desktop setup workflow.
    pub setup: PluginSetup,
    /// Built-in or locally installed origin.
    pub source: PluginSource,
}

/// Decoder output copied from sandbox memory after one recognized datagram.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginDecodeOutput {
    /// Decoder output contract version.
    pub schema_version: u16,
    /// Partial canonical state updates.
    #[serde(default)]
    pub updates: Vec<TelemetryUpdate>,
    /// Reliable canonical events.
    #[serde(default)]
    pub events: Vec<TelemetryEvent>,
}

/// Single-file distributable plugin package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GamePluginPackage {
    /// Package envelope version.
    pub package_version: u16,
    /// Embedded external-plugin manifest.
    pub manifest: GamePluginManifest,
    /// Base64-encoded WASM module bytes.
    pub module_base64: String,
}

impl GamePluginManifest {
    /// Validates every bounded and cross-field manifest invariant.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] for unsupported versions or unsafe values.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.schema_version != GAME_PLUGIN_MANIFEST_VERSION {
            return Err(invalid(format!(
                "manifest schema {} is unsupported; expected {GAME_PLUGIN_MANIFEST_VERSION}",
                self.schema_version
            )));
        }
        bounded("id", &self.id, 1, MAX_PLUGIN_ID_BYTES)?;
        AdapterId::new(self.id.clone())
            .map_err(|_| invalid("id must be a safe lowercase plugin identifier"))?;
        bounded("name", &self.name, 1, MAX_SHORT_TEXT_BYTES)?;
        bounded("version", &self.version, 1, MAX_SHORT_TEXT_BYTES)?;
        Version::parse(&self.version)
            .map_err(|error| invalid(format!("version is not semantic: {error}")))?;
        bounded("publisher", &self.publisher, 1, MAX_SHORT_TEXT_BYTES)?;
        bounded("license", &self.license, 1, MAX_SHORT_TEXT_BYTES)?;
        bounded("description", &self.description, 1, MAX_DESCRIPTION_BYTES)?;
        bounded(
            "protocol name",
            &self.protocol.name,
            1,
            MAX_SHORT_TEXT_BYTES,
        )?;
        bounded(
            "protocol version",
            &self.protocol.version,
            1,
            MAX_SHORT_TEXT_BYTES,
        )?;
        if self.ingress.default_port == 0 {
            return Err(invalid("ingress default port must be non-zero"));
        }
        if !(1..=MAX_PLUGIN_DATAGRAM_BYTES).contains(&self.ingress.max_datagram_bytes) {
            return Err(invalid(format!(
                "maxDatagramBytes must be within 1..={MAX_PLUGIN_DATAGRAM_BYTES}"
            )));
        }
        if self.capabilities.is_empty() {
            return Err(invalid("capabilities must not be empty"));
        }
        let distinct_capabilities = self.capabilities.iter().copied().collect::<BTreeSet<_>>();
        if distinct_capabilities.len() != self.capabilities.len() {
            return Err(invalid("capabilities contains duplicates"));
        }
        self.validate_runtime()?;
        self.presentation.validate()?;
        self.setup.validate()?;
        if matches!(self.setup, PluginSetup::ScsSdk { .. })
            && !matches!(self.runtime, PluginRuntime::Builtin { .. })
        {
            return Err(invalid(
                "scs_sdk setup is reserved for reviewed built-in bridge plugins",
            ));
        }
        Ok(())
    }

    /// Builds the public, non-secret metadata used by Host clients.
    #[must_use]
    pub fn metadata(&self, source: PluginSource) -> GamePluginMetadata {
        GamePluginMetadata {
            id: self.id.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            publisher: self.publisher.clone(),
            description: self.description.clone(),
            protocol_version: self.protocol.version.clone(),
            ingress: self.ingress.clone(),
            capabilities: self.capabilities.clone(),
            presentation: self.presentation.clone(),
            setup: self.setup.clone(),
            source,
        }
    }

    fn validate_runtime(&self) -> Result<(), ManifestError> {
        match &self.runtime {
            PluginRuntime::Builtin { entrypoint } => {
                bounded("builtin entrypoint", entrypoint, 1, MAX_PLUGIN_ID_BYTES)?;
                AdapterId::new(entrypoint.clone())
                    .map_err(|_| invalid("builtin entrypoint must be a safe lowercase id"))?;
            }
            PluginRuntime::Wasm {
                abi_version,
                module,
                sha256,
            } => {
                if *abi_version != GAME_PLUGIN_ABI_VERSION {
                    return Err(invalid(format!(
                        "WASM ABI {abi_version} is unsupported; expected {GAME_PLUGIN_ABI_VERSION}"
                    )));
                }
                safe_module_path(module)?;
                if sha256.len() != 64
                    || !sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                {
                    return Err(invalid(
                        "runtime sha256 must be 64 lowercase hexadecimal bytes",
                    ));
                }
            }
        }
        Ok(())
    }
}

impl PluginPresentation {
    fn validate(&self) -> Result<(), ManifestError> {
        bounded("short name", &self.short_name, 1, 32)?;
        bounded("presentation detail", &self.detail, 1, MAX_SHORT_TEXT_BYTES)?;
        for (name, value) in [
            ("background", &self.theme.background),
            ("foreground", &self.theme.foreground),
            ("accent", &self.theme.accent),
            ("warning", &self.theme.warning),
        ] {
            if !is_hex_color(value) {
                return Err(invalid(format!("theme {name} must be a #RRGGBB color")));
            }
        }
        if !(500..=30_000).contains(&self.fallback_rpm_max) {
            return Err(invalid("fallbackRpmMax must be within 500..=30000"));
        }
        if self.widgets.is_empty() || self.widgets.len() > MAX_WIDGETS {
            return Err(invalid(format!(
                "widgets must contain 1..={MAX_WIDGETS} entries"
            )));
        }
        let mut distinct = BTreeSet::new();
        for widget in &self.widgets {
            validate_component_type(widget)?;
            if !distinct.insert(widget) {
                return Err(invalid("widgets contains duplicates"));
            }
        }
        Ok(())
    }
}

impl PluginSetup {
    fn validate(&self) -> Result<(), ManifestError> {
        match self {
            Self::F1Udp {
                format,
                send_rate_hz,
            } => {
                bounded("F1 UDP format", format, 1, MAX_SHORT_TEXT_BYTES)?;
                if !(1..=120).contains(send_rate_hz) {
                    return Err(invalid("F1 sendRateHz must be within 1..=120"));
                }
            }
            Self::ScsSdk {
                steam_app_id,
                directory_name,
            } => {
                if *steam_app_id == 0 {
                    return Err(invalid("Steam app id must be non-zero"));
                }
                bounded(
                    "SCS directory name",
                    directory_name,
                    1,
                    MAX_SHORT_TEXT_BYTES,
                )?;
            }
            Self::Udp { steps } => {
                if steps.is_empty() || steps.len() > MAX_SETUP_STEPS {
                    return Err(invalid(format!(
                        "UDP setup steps must contain 1..={MAX_SETUP_STEPS} entries"
                    )));
                }
                for step in steps {
                    bounded("UDP setup step", step, 1, 512)?;
                }
            }
            Self::None => {}
        }
        Ok(())
    }
}

impl PluginDecodeOutput {
    /// Checks the decoder response version and bounded collection sizes.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] for an incompatible or oversized response.
    pub fn validate_bounds(&self) -> Result<(), ManifestError> {
        if self.schema_version != GAME_PLUGIN_ABI_VERSION {
            return Err(invalid(format!(
                "decoder output schema {} is unsupported; expected {GAME_PLUGIN_ABI_VERSION}",
                self.schema_version
            )));
        }
        if self.updates.len() > MAX_PLUGIN_UPDATES {
            return Err(invalid(format!(
                "decoder emitted more than {MAX_PLUGIN_UPDATES} updates"
            )));
        }
        if self.events.len() > MAX_PLUGIN_EVENTS {
            return Err(invalid(format!(
                "decoder emitted more than {MAX_PLUGIN_EVENTS} events"
            )));
        }
        Ok(())
    }
}

/// Parses and validates a JSON manifest.
///
/// # Errors
///
/// Returns [`ManifestError`] for malformed JSON or a violated invariant.
pub fn parse_manifest(bytes: &[u8]) -> Result<GamePluginManifest, ManifestError> {
    let manifest: GamePluginManifest =
        serde_json::from_slice(bytes).map_err(|error| invalid_json("manifest", &error))?;
    manifest.validate()?;
    Ok(manifest)
}

/// Parses a package envelope and validates its manifest-level fields.
///
/// Module Base64 and digest verification are intentionally performed by the
/// package installer so they can remain bounded before allocation.
///
/// # Errors
///
/// Returns [`ManifestError`] for malformed JSON, versions, or a built-in runtime.
pub fn parse_package(bytes: &[u8]) -> Result<GamePluginPackage, ManifestError> {
    let package: GamePluginPackage =
        serde_json::from_slice(bytes).map_err(|error| invalid_json("package", &error))?;
    if package.package_version != GAME_PLUGIN_PACKAGE_VERSION {
        return Err(invalid(format!(
            "package version {} is unsupported; expected {GAME_PLUGIN_PACKAGE_VERSION}",
            package.package_version
        )));
    }
    package.manifest.validate()?;
    if !matches!(package.manifest.runtime, PluginRuntime::Wasm { .. }) {
        return Err(invalid("installable packages must use a WASM runtime"));
    }
    Ok(package)
}

/// Manifest validation failure with no untrusted input echo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestError {
    message: String,
}

impl ManifestError {
    /// Returns the actionable bounded validation reason.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for ManifestError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ManifestError {}

fn invalid(message: impl Into<String>) -> ManifestError {
    ManifestError {
        message: message.into(),
    }
}

fn invalid_json(kind: &str, error: &serde_json::Error) -> ManifestError {
    invalid(format!(
        "{kind} JSON is invalid ({:?}) at line {} column {}",
        error.classify(),
        error.line(),
        error.column()
    ))
}

fn bounded(field: &str, value: &str, minimum: usize, maximum: usize) -> Result<(), ManifestError> {
    let length = value.len();
    if value.trim() != value || !(minimum..=maximum).contains(&length) {
        return Err(invalid(format!(
            "{field} must contain {minimum}..={maximum} UTF-8 bytes without surrounding whitespace"
        )));
    }
    if value.chars().any(char::is_control) {
        return Err(invalid(format!(
            "{field} must not contain control characters"
        )));
    }
    Ok(())
}

fn safe_module_path(value: &str) -> Result<(), ManifestError> {
    bounded("WASM module path", value, 6, MAX_SHORT_TEXT_BYTES)?;
    let path = Path::new(value);
    let mut components = path.components();
    let safe = matches!(components.next(), Some(Component::Normal(_)))
        && components.next().is_none()
        && path
            .extension()
            .is_some_and(|extension| extension == "wasm");
    if !safe || value.contains(['/', '\\']) {
        return Err(invalid("WASM module must be one safe .wasm filename"));
    }
    Ok(())
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_component_type(value: &str) -> Result<(), ManifestError> {
    bounded("widget type", value, 3, 96)?;
    let valid = value.split('.').all(|segment| {
        let bytes = segment.as_bytes();
        !bytes.is_empty()
            && bytes.first().is_some_and(u8::is_ascii_alphanumeric)
            && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
            && bytes
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
            && !segment.contains("--")
    }) && value.contains('.');
    if valid {
        Ok(())
    } else {
        Err(invalid(
            "widget type must be a safe dotted lowercase identifier",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> GamePluginManifest {
        GamePluginManifest {
            schema_version: GAME_PLUGIN_MANIFEST_VERSION,
            id: "example-game".to_owned(),
            name: "Example Game".to_owned(),
            version: "1.0.0".to_owned(),
            publisher: "Example Publisher".to_owned(),
            license: "Apache-2.0".to_owned(),
            description: "Example UDP telemetry decoder.".to_owned(),
            runtime: PluginRuntime::Wasm {
                abi_version: GAME_PLUGIN_ABI_VERSION,
                module: "decoder.wasm".to_owned(),
                sha256: "a".repeat(64),
            },
            protocol: PluginProtocol {
                name: "Example UDP".to_owned(),
                version: "1".to_owned(),
            },
            ingress: PluginIngress {
                kind: PluginIngressKind::SharedUdp,
                default_port: 20_777,
                max_datagram_bytes: 1_024,
            },
            capabilities: vec![TelemetryField::VehicleSpeed],
            presentation: PluginPresentation {
                short_name: "EXAMPLE".to_owned(),
                detail: "UDP / V1".to_owned(),
                family: PluginGameFamily::Generic,
                status_mode: PluginStatusMode::Generic,
                layout_preset: PluginLayoutPreset::Generic,
                theme: PluginTheme {
                    background: "#07090c".to_owned(),
                    foreground: "#f2f0e9".to_owned(),
                    accent: "#d9ff43".to_owned(),
                    warning: "#ff4b3e".to_owned(),
                },
                fallback_rpm_max: 8_000,
                widgets: vec!["core.speed".to_owned()],
            },
            setup: PluginSetup::Udp {
                steps: vec!["Enable UDP telemetry.".to_owned()],
            },
        }
    }

    #[test]
    fn validates_external_manifest_and_public_metadata() -> Result<(), ManifestError> {
        let manifest = manifest();
        manifest.validate()?;
        let metadata = manifest.metadata(PluginSource::Installed);
        assert_eq!(metadata.id, "example-game");
        assert_eq!(metadata.source, PluginSource::Installed);
        Ok(())
    }

    #[test]
    fn rejects_paths_duplicates_and_unsupported_abi() {
        let mut invalid = manifest();
        invalid.runtime = PluginRuntime::Wasm {
            abi_version: GAME_PLUGIN_ABI_VERSION + 1,
            module: "../decoder.wasm".to_owned(),
            sha256: "A".repeat(64),
        };
        invalid.capabilities.push(TelemetryField::VehicleSpeed);
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn external_plugins_cannot_request_the_native_scs_installer() {
        let mut invalid = manifest();
        invalid.setup = PluginSetup::ScsSdk {
            steam_app_id: 227_300,
            directory_name: "Euro Truck Simulator 2".to_owned(),
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn installable_package_cannot_claim_a_builtin_entrypoint() -> Result<(), serde_json::Error> {
        let mut manifest = manifest();
        manifest.runtime = PluginRuntime::Builtin {
            entrypoint: "example-game".to_owned(),
        };
        let bytes = serde_json::to_vec(&GamePluginPackage {
            package_version: GAME_PLUGIN_PACKAGE_VERSION,
            manifest,
            module_base64: String::new(),
        })?;
        assert!(parse_package(&bytes).is_err());
        Ok(())
    }

    #[test]
    fn malformed_json_diagnostics_do_not_echo_untrusted_fields() {
        let untrusted = "x".repeat(1_024);
        let bytes = format!(r#"{{"{untrusted}":true}}"#);
        let result = parse_manifest(bytes.as_bytes());
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(!error.message().contains(&untrusted));
            assert!(error.message().len() < 160);
        }
    }
}
