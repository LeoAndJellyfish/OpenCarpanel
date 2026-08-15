use std::{
    collections::BTreeSet,
    error::Error,
    fmt::{self, Display, Formatter},
    path::Path,
    str::FromStr,
};

use opencarpanel_adapter_api::{AdapterError, AdapterId, AdapterOutput, GameAdapter};
use opencarpanel_adapter_f1::{F1_24Adapter, F1_25Adapter};
use opencarpanel_adapter_scs::{AtsAdapter, Ets2Adapter};
use opencarpanel_game_plugin_api::{
    GamePluginManifest, GamePluginMetadata, PluginRuntime, PluginSource, parse_manifest,
};
use opencarpanel_game_plugin_runtime::{
    MAX_PLUGIN_LOAD_ISSUES, PluginLoadIssue, WasmGameAdapter, load_installed_plugins,
};
use opencarpanel_telemetry_core::{
    MonotonicTimestamp, TelemetryEvent, TelemetryField, TelemetryReducer, TelemetrySnapshot,
};

const ACTIVE_SOURCE_TIMEOUT_US: u64 = 2_000_000;

const BUILTIN_MANIFESTS: &[&[u8]] = &[
    include_bytes!("../../../plugins/games/f1-24/manifest.json"),
    include_bytes!("../../../plugins/games/f1-25/manifest.json"),
    include_bytes!("../../../plugins/games/ets2/manifest.json"),
    include_bytes!("../../../plugins/games/ats/manifest.json"),
];

/// Automatic detection or one stable plugin id selected by the Host.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum AdapterSelection {
    /// Detect a supported protocol and keep the current source sticky while it is active.
    #[default]
    Auto,
    /// Accept only F1 24 original-format UDP packets.
    F1_24,
    /// Accept F1 25 original 2025-format and 2026 Season Pack UDP packets.
    F1_25,
    /// Accept only the Euro Truck Simulator 2 bridge protocol.
    Ets2,
    /// Accept only the American Truck Simulator bridge protocol.
    Ats,
    /// Accept only a valid dynamically installed plugin id.
    Plugin(AdapterId),
}

impl AdapterSelection {
    /// Returns the stable persisted configuration value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Auto => "auto",
            Self::F1_24 => "f1-24",
            Self::F1_25 => "f1-25",
            Self::Ets2 => "ets2",
            Self::Ats => "ats",
            Self::Plugin(id) => id.as_str(),
        }
    }

    fn fixed_adapter_id(&self) -> Option<&str> {
        (!matches!(self, Self::Auto)).then(|| self.as_str())
    }
}

impl Display for AdapterSelection {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for AdapterSelection {
    type Err = ParseAdapterSelectionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto" => Ok(Self::Auto),
            "f1-24" => Ok(Self::F1_24),
            "f1-25" => Ok(Self::F1_25),
            "ets2" => Ok(Self::Ets2),
            "ats" => Ok(Self::Ats),
            _ => AdapterId::new(value.to_owned())
                .map(Self::Plugin)
                .map_err(|_| ParseAdapterSelectionError {
                    value: value.to_owned(),
                }),
        }
    }
}

/// Error returned for an unsafe Host plugin-selection value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseAdapterSelectionError {
    value: String,
}

impl Display for ParseAdapterSelectionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid game plugin selection {:?}; use auto or a lowercase plugin id",
            self.value
        )
    }
}

impl Error for ParseAdapterSelectionError {}

/// Immutable public metadata for one registered game plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportedAdapter {
    metadata: GamePluginMetadata,
}

impl SupportedAdapter {
    /// Returns the stable plugin id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.metadata.id
    }

    /// Returns the human-readable game or producer name.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.metadata.name
    }

    /// Returns the accepted upstream protocol version.
    #[must_use]
    pub fn protocol_version(&self) -> &str {
        &self.metadata.protocol_version
    }

    /// Returns canonical telemetry fields supplied by this plugin.
    #[must_use]
    pub fn capabilities(&self) -> &[TelemetryField] {
        &self.metadata.capabilities
    }

    /// Returns complete client-safe plugin metadata.
    #[must_use]
    pub const fn metadata(&self) -> &GamePluginMetadata {
        &self.metadata
    }
}

struct AdapterPipeline {
    adapter: Box<dyn GameAdapter>,
    reducer: TelemetryReducer,
    metadata: SupportedAdapter,
    last_seen_us: Option<u64>,
}

impl AdapterPipeline {
    fn new(
        adapter: Box<dyn GameAdapter>,
        manifest: &GamePluginManifest,
        source: PluginSource,
    ) -> Result<Self, AdapterError> {
        validate_descriptor(adapter.as_ref(), manifest)?;
        let metadata = SupportedAdapter {
            metadata: manifest.metadata(source),
        };
        Ok(Self {
            reducer: TelemetryReducer::with_game_id(metadata.id().to_owned()),
            adapter,
            metadata,
            last_seen_us: None,
        })
    }
}

pub(crate) struct AdapterRegistry {
    selection: AdapterSelection,
    selected_index: Option<usize>,
    active_index: Option<usize>,
    pipelines: Vec<AdapterPipeline>,
    load_issues: Vec<PluginLoadIssue>,
    output: AdapterOutput,
    pending_events: Vec<TelemetryEvent>,
}

impl AdapterRegistry {
    #[cfg(test)]
    pub(crate) fn new(selection: AdapterSelection) -> Result<Self, AdapterError> {
        Self::new_with_plugins(selection, None)
    }

    pub(crate) fn new_with_plugins(
        selection: AdapterSelection,
        plugins_root: Option<&Path>,
    ) -> Result<Self, AdapterError> {
        let mut pipelines = Vec::new();
        let mut reserved_ids = BTreeSet::new();
        for manifest_bytes in BUILTIN_MANIFESTS {
            let manifest = parse_manifest(manifest_bytes)
                .map_err(|error| AdapterError::invalid_configuration(error.to_string()))?;
            reserved_ids.insert(manifest.id.clone());
            let adapter = builtin_adapter(&manifest)?;
            pipelines.push(AdapterPipeline::new(
                adapter,
                &manifest,
                PluginSource::Builtin,
            )?);
        }

        let (installed, mut load_issues) = plugins_root.map_or_else(
            || (Vec::new(), Vec::new()),
            |root| load_installed_plugins(root, &reserved_ids),
        );
        for plugin in installed {
            let pipeline = WasmGameAdapter::from_file(&plugin.manifest, &plugin.module_path)
                .map_err(|error| error.to_string())
                .and_then(|adapter| {
                    AdapterPipeline::new(
                        Box::new(adapter),
                        &plugin.manifest,
                        PluginSource::Installed,
                    )
                    .map_err(|error| error.to_string())
                });
            match pipeline {
                Ok(pipeline) => pipelines.push(pipeline),
                Err(message) if load_issues.len() < MAX_PLUGIN_LOAD_ISSUES => {
                    load_issues.push(PluginLoadIssue {
                        plugin_id: Some(plugin.manifest.id),
                        message,
                    });
                }
                Err(_message) => {}
            }
        }

        let mut selection = selection;
        let selected_index = selection.fixed_adapter_id().and_then(|selected| {
            pipelines
                .iter()
                .position(|pipeline| pipeline.metadata.id() == selected)
        });
        if selection != AdapterSelection::Auto && selected_index.is_none() {
            if load_issues.len() == MAX_PLUGIN_LOAD_ISSUES {
                load_issues.pop();
            }
            load_issues.push(PluginLoadIssue {
                plugin_id: Some(selection.as_str().to_owned()),
                message: "fixed plugin is unavailable; automatic detection is active".to_owned(),
            });
            selection = AdapterSelection::Auto;
        }

        Ok(Self {
            selection,
            selected_index,
            active_index: None,
            pipelines,
            load_issues,
            output: AdapterOutput::with_capacity(1, 4),
            pending_events: Vec::with_capacity(4),
        })
    }

    pub(crate) fn supported_adapters(&self) -> Vec<SupportedAdapter> {
        self.pipelines
            .iter()
            .map(|pipeline| pipeline.metadata.clone())
            .collect()
    }

    pub(crate) fn load_issues(&self) -> Vec<PluginLoadIssue> {
        self.load_issues.clone()
    }

    pub(crate) const fn selection(&self) -> &AdapterSelection {
        &self.selection
    }

    pub(crate) fn decode(
        &mut self,
        datagram: &[u8],
        received_at: MonotonicTimestamp,
    ) -> RegistryOutcome {
        self.pending_events.clear();
        if let Some(index) = self.selected_index {
            return self
                .try_pipeline(index, datagram, received_at)
                .unwrap_or(RegistryOutcome::Rejected);
        }

        for index in 0..self.pipelines.len() {
            if let Some(outcome) = self.try_pipeline(index, datagram, received_at) {
                return outcome;
            }
        }
        RegistryOutcome::Rejected
    }

    pub(crate) fn drain_events(&mut self) -> impl Iterator<Item = TelemetryEvent> + '_ {
        self.pending_events.drain(..)
    }

    fn try_pipeline(
        &mut self,
        index: usize,
        datagram: &[u8],
        received_at: MonotonicTimestamp,
    ) -> Option<RegistryOutcome> {
        self.output.clear();
        if self.pipelines[index]
            .adapter
            .decode(datagram, received_at, &mut self.output)
            .is_err()
        {
            return None;
        }

        let received_at_us = received_at.as_micros();
        let should_activate = self.should_activate(index, received_at_us);
        let switched = should_activate && self.active_index != Some(index);
        let pipeline = &mut self.pipelines[index];
        pipeline.last_seen_us = Some(received_at_us);

        let mut changed = false;
        for update in self.output.updates.drain(..) {
            changed |= pipeline.reducer.apply(update);
        }

        if should_activate {
            self.active_index = Some(index);
            self.pending_events.append(&mut self.output.events);
        }
        let snapshot = (should_activate && (switched || changed))
            .then(|| Box::new(pipeline.reducer.snapshot().clone()));

        Some(RegistryOutcome::Recognized {
            adapter_index: index,
            active_index: self.active_index,
            snapshot,
        })
    }

    fn should_activate(&self, candidate: usize, now_us: u64) -> bool {
        if self.selection != AdapterSelection::Auto {
            return true;
        }
        let Some(active) = self.active_index else {
            return true;
        };
        if active == candidate {
            return true;
        }

        self.pipelines[active]
            .last_seen_us
            .is_none_or(|last_seen| now_us.saturating_sub(last_seen) >= ACTIVE_SOURCE_TIMEOUT_US)
    }
}

fn builtin_adapter(manifest: &GamePluginManifest) -> Result<Box<dyn GameAdapter>, AdapterError> {
    let PluginRuntime::Builtin { entrypoint } = &manifest.runtime else {
        return Err(AdapterError::invalid_configuration(format!(
            "built-in manifest {} does not declare a built-in runtime",
            manifest.id
        )));
    };
    match entrypoint.as_str() {
        "f1-24" => Ok(Box::new(F1_24Adapter::new()?)),
        "f1-25" => Ok(Box::new(F1_25Adapter::new()?)),
        "ets2" => Ok(Box::new(Ets2Adapter::new()?)),
        "ats" => Ok(Box::new(AtsAdapter::new()?)),
        _ => Err(AdapterError::invalid_configuration(format!(
            "built-in entrypoint {entrypoint} has no compiled adapter factory"
        ))),
    }
}

fn validate_descriptor(
    adapter: &dyn GameAdapter,
    manifest: &GamePluginManifest,
) -> Result<(), AdapterError> {
    let descriptor = adapter.descriptor();
    let descriptor_capabilities = descriptor.capabilities.iter().copied().collect::<Vec<_>>();
    if descriptor.id.as_str() != manifest.id
        || descriptor.display_name != manifest.name
        || descriptor.protocol_version != manifest.protocol.version
        || descriptor_capabilities != manifest.capabilities
    {
        return Err(AdapterError::invalid_configuration(format!(
            "adapter descriptor for {} has drifted from its plugin manifest",
            manifest.id
        )));
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) enum RegistryOutcome {
    Rejected,
    Recognized {
        adapter_index: usize,
        active_index: Option<usize>,
        snapshot: Option<Box<TelemetrySnapshot>>,
    },
}

#[cfg(test)]
mod tests {
    use opencarpanel_adapter_f1::{CAR_TELEMETRY_PACKET_LEN, PACKET_HEADER_LEN};
    use opencarpanel_adapter_scs::{
        BRIDGE_MAGIC, BRIDGE_PROTOCOL_V1, BRIDGE_V1_PACKET_LEN, ETS2_GAME_ID,
    };

    use super::*;

    #[test]
    fn selection_accepts_any_safe_plugin_id() {
        for value in ["auto", "f1-24", "f1-25", "ets2", "ats", "community-sim"] {
            let parsed = value.parse::<AdapterSelection>();
            assert!(parsed.is_ok(), "rejected {value}");
            assert_eq!(
                parsed.map(|selection| selection.as_str().to_owned()),
                Ok(value.to_owned())
            );
        }

        for value in ["F1-25", "../escape", "bad--id", ""] {
            assert!(
                value.parse::<AdapterSelection>().is_err(),
                "accepted {value}"
            );
        }
    }

    #[test]
    fn builtin_manifests_match_native_adapter_descriptors() -> Result<(), AdapterError> {
        let registry = AdapterRegistry::new(AdapterSelection::Auto)?;
        assert_eq!(
            registry
                .supported_adapters()
                .iter()
                .map(SupportedAdapter::id)
                .collect::<Vec<_>>(),
            ["f1-24", "f1-25", "ets2", "ats"]
        );
        Ok(())
    }

    #[test]
    fn unavailable_fixed_plugin_falls_back_to_auto_with_a_load_issue() -> Result<(), AdapterError> {
        let selection = "missing-plugin"
            .parse::<AdapterSelection>()
            .map_err(|error| AdapterError::invalid_configuration(error.to_string()))?;
        let registry = AdapterRegistry::new(selection)?;
        assert_eq!(registry.selection(), &AdapterSelection::Auto);
        assert_eq!(registry.load_issues().len(), 1);
        assert_eq!(
            registry.load_issues()[0].plugin_id.as_deref(),
            Some("missing-plugin")
        );
        Ok(())
    }

    #[test]
    fn auto_selection_does_not_flap_while_the_active_source_is_fresh() -> Result<(), AdapterError> {
        let mut registry = AdapterRegistry::new(AdapterSelection::Auto)?;
        let f1 = f1_packet();
        let ets2 = scs_packet();

        let first = registry.decode(&f1, MonotonicTimestamp::from_micros(1));
        assert!(matches!(
            first,
            RegistryOutcome::Recognized {
                active_index: Some(0),
                snapshot: Some(_),
                ..
            }
        ));

        let competing = registry.decode(&ets2, MonotonicTimestamp::from_micros(1_000_000));
        assert!(matches!(
            competing,
            RegistryOutcome::Recognized {
                adapter_index: 2,
                active_index: Some(0),
                snapshot: None,
            }
        ));

        let refreshed = registry.decode(&f1, MonotonicTimestamp::from_micros(1_500_000));
        assert!(matches!(
            refreshed,
            RegistryOutcome::Recognized {
                active_index: Some(0),
                ..
            }
        ));
        let still_fresh = registry.decode(&ets2, MonotonicTimestamp::from_micros(3_499_999));
        assert!(matches!(
            still_fresh,
            RegistryOutcome::Recognized {
                active_index: Some(0),
                snapshot: None,
                ..
            }
        ));

        let switched = registry.decode(&ets2, MonotonicTimestamp::from_micros(3_500_000));
        let RegistryOutcome::Recognized {
            active_index: Some(2),
            snapshot: Some(snapshot),
            ..
        } = switched
        else {
            return Err(AdapterError::malformed_packet(
                "ETS2 did not become active after the sticky timeout",
            ));
        };
        assert_eq!(snapshot.meta.game_id.as_deref(), Some("ets2"));
        Ok(())
    }

    fn f1_packet() -> Vec<u8> {
        let mut packet = Vec::with_capacity(CAR_TELEMETRY_PACKET_LEN);
        packet.extend_from_slice(&2024_u16.to_le_bytes());
        packet.extend_from_slice(&[24, 1, 0, 1, 6]);
        packet.extend_from_slice(&1_u64.to_le_bytes());
        packet.extend_from_slice(&1.0_f32.to_le_bytes());
        packet.extend_from_slice(&1_u32.to_le_bytes());
        packet.extend_from_slice(&1_u32.to_le_bytes());
        packet.extend_from_slice(&[0, 255]);
        packet.resize(CAR_TELEMETRY_PACKET_LEN, 0);
        packet[PACKET_HEADER_LEN..PACKET_HEADER_LEN + 2].copy_from_slice(&100_u16.to_le_bytes());
        packet
    }

    fn scs_packet() -> Vec<u8> {
        let mut packet = Vec::with_capacity(BRIDGE_V1_PACKET_LEN);
        packet.extend_from_slice(&BRIDGE_MAGIC);
        packet.extend_from_slice(&[BRIDGE_PROTOCOL_V1, ETS2_GAME_ID, 0, 0]);
        packet.extend_from_slice(&2_u64.to_le_bytes());
        packet.extend_from_slice(&2_u32.to_le_bytes());
        packet.extend_from_slice(&10.0_f32.to_le_bytes());
        packet.extend_from_slice(&1_000.0_f32.to_le_bytes());
        packet.extend_from_slice(&2_500.0_f32.to_le_bytes());
        packet.extend_from_slice(&4_i32.to_le_bytes());
        packet.extend_from_slice(&0.5_f32.to_le_bytes());
        packet.extend_from_slice(&0.0_f32.to_le_bytes());
        packet
    }
}
