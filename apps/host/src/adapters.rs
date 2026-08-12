use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use opencarpanel_adapter_api::{AdapterError, AdapterOutput, GameAdapter};
use opencarpanel_adapter_f1::{F1_24Adapter, F1_25Adapter};
use opencarpanel_adapter_scs::{AtsAdapter, Ets2Adapter};
use opencarpanel_telemetry_core::{
    MonotonicTimestamp, TelemetryEvent, TelemetryField, TelemetryReducer, TelemetrySnapshot,
};

const ACTIVE_SOURCE_TIMEOUT_US: u64 = 2_000_000;

/// Game-adapter selection used by the Host telemetry receiver.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AdapterSelection {
    /// Detect a supported protocol and keep the current source sticky while it is active.
    #[default]
    Auto,
    /// Accept only F1 24 original-format UDP packets.
    F1_24,
    /// Accept only F1 25 original 2025-format UDP packets.
    F1_25,
    /// Accept only the Euro Truck Simulator 2 bridge protocol.
    Ets2,
    /// Accept only the American Truck Simulator bridge protocol.
    Ats,
}

impl AdapterSelection {
    /// Returns the stable configuration value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::F1_24 => "f1-24",
            Self::F1_25 => "f1-25",
            Self::Ets2 => "ets2",
            Self::Ats => "ats",
        }
    }

    const fn fixed_adapter_id(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::F1_24 => Some("f1-24"),
            Self::F1_25 => Some("f1-25"),
            Self::Ets2 => Some("ets2"),
            Self::Ats => Some("ats"),
        }
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
            _ => Err(ParseAdapterSelectionError {
                value: value.to_owned(),
            }),
        }
    }
}

/// Error returned for an unsupported Host game-selection value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseAdapterSelectionError {
    value: String,
}

impl Display for ParseAdapterSelectionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "unsupported game selection {:?}; expected auto, f1-24, f1-25, ets2, or ats",
            self.value
        )
    }
}

impl Error for ParseAdapterSelectionError {}

/// Immutable metadata for one adapter compiled into the Host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportedAdapter {
    id: String,
    display_name: String,
    protocol_version: String,
    capabilities: Vec<TelemetryField>,
}

impl SupportedAdapter {
    /// Returns the stable adapter id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the human-readable game name.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Returns the accepted game-input protocol version.
    #[must_use]
    pub fn protocol_version(&self) -> &str {
        &self.protocol_version
    }

    /// Returns the canonical telemetry fields supplied by this adapter.
    #[must_use]
    pub fn capabilities(&self) -> &[TelemetryField] {
        &self.capabilities
    }
}

struct AdapterPipeline {
    adapter: Box<dyn GameAdapter>,
    reducer: TelemetryReducer,
    metadata: SupportedAdapter,
    last_seen_us: Option<u64>,
}

impl AdapterPipeline {
    fn new(adapter: Box<dyn GameAdapter>) -> Self {
        let descriptor = adapter.descriptor();
        let metadata = SupportedAdapter {
            id: descriptor.id.as_str().to_owned(),
            display_name: descriptor.display_name.clone(),
            protocol_version: descriptor.protocol_version.clone(),
            capabilities: descriptor.capabilities.iter().copied().collect(),
        };
        Self {
            reducer: TelemetryReducer::with_game_id(metadata.id.clone()),
            adapter,
            metadata,
            last_seen_us: None,
        }
    }
}

pub(crate) struct AdapterRegistry {
    selection: AdapterSelection,
    selected_index: Option<usize>,
    active_index: Option<usize>,
    pipelines: Vec<AdapterPipeline>,
    output: AdapterOutput,
    pending_events: Vec<TelemetryEvent>,
}

impl AdapterRegistry {
    pub(crate) fn new(selection: AdapterSelection) -> Result<Self, AdapterError> {
        let adapters: Vec<Box<dyn GameAdapter>> = vec![
            Box::new(F1_24Adapter::new()?),
            Box::new(F1_25Adapter::new()?),
            Box::new(Ets2Adapter::new()?),
            Box::new(AtsAdapter::new()?),
        ];
        let pipelines: Vec<_> = adapters.into_iter().map(AdapterPipeline::new).collect();
        let selected_index = selection.fixed_adapter_id().and_then(|selected| {
            pipelines
                .iter()
                .position(|pipeline| pipeline.metadata.id == selected)
        });
        if selection != AdapterSelection::Auto && selected_index.is_none() {
            return Err(AdapterError::invalid_configuration(format!(
                "adapter {} is not compiled into this Host",
                selection.as_str()
            )));
        }

        Ok(Self {
            selection,
            selected_index,
            active_index: None,
            pipelines,
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
        BRIDGE_MAGIC, BRIDGE_PACKET_LEN, BRIDGE_PROTOCOL_VERSION, ETS2_GAME_ID,
    };

    use super::*;

    #[test]
    fn selection_values_are_strict_and_actionable() {
        for value in ["auto", "f1-24", "f1-25", "ets2", "ats"] {
            let parsed = value.parse::<AdapterSelection>();
            assert!(parsed.is_ok(), "rejected {value}");
            assert_eq!(parsed.map(AdapterSelection::as_str), Ok(value));
        }

        let error = "F1-25"
            .parse::<AdapterSelection>()
            .err()
            .map(|error| error.to_string());
        assert!(error.is_some_and(|message| message.contains("expected auto, f1-24, f1-25")));
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
        let mut packet = Vec::with_capacity(BRIDGE_PACKET_LEN);
        packet.extend_from_slice(&BRIDGE_MAGIC);
        packet.extend_from_slice(&[BRIDGE_PROTOCOL_VERSION, ETS2_GAME_ID, 0, 0]);
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
