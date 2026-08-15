use std::collections::VecDeque;

use opensimdash_protocol::{EventMessage, ResyncRequiredMessage};
use opensimdash_telemetry_core::TelemetryEvent;
use tokio::sync::{Mutex, broadcast};

/// Maximum number of reliable telemetry events retained for reconnect replay.
pub const EVENT_BUFFER_CAPACITY: usize = 64;

#[derive(Debug)]
struct EventState {
    next_sequence: u64,
    retained: VecDeque<EventMessage>,
}

/// Result of asking the bounded event history to resume after a sequence.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ReplayBatch {
    Events(Vec<EventMessage>),
    ResyncRequired(ResyncRequiredMessage),
}

/// Bounded reliable event publication lane.
#[derive(Debug)]
pub(crate) struct EventHub {
    state: Mutex<EventState>,
    sender: broadcast::Sender<EventMessage>,
}

impl EventHub {
    pub(crate) fn new() -> Self {
        let (sender, _receiver) = broadcast::channel(EVENT_BUFFER_CAPACITY);
        Self {
            state: Mutex::new(EventState {
                next_sequence: 1,
                retained: VecDeque::with_capacity(EVENT_BUFFER_CAPACITY),
            }),
            sender,
        }
    }

    pub(crate) async fn publish(&self, event: TelemetryEvent) -> EventMessage {
        let message = {
            let mut state = self.state.lock().await;
            let sequence = state.next_sequence;
            state.next_sequence = state.next_sequence.saturating_add(1);
            let message = EventMessage {
                seq: sequence,
                data: event,
            };
            if state.retained.len() == EVENT_BUFFER_CAPACITY {
                let _oldest = state.retained.pop_front();
            }
            state.retained.push_back(message.clone());
            message
        };
        let _receiver_count = self.sender.send(message.clone());
        message
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<EventMessage> {
        self.sender.subscribe()
    }

    pub(crate) async fn replay_after(&self, last_sequence: u64) -> ReplayBatch {
        let state = self.state.lock().await;
        let Some(oldest) = state.retained.front().map(|event| event.seq) else {
            return ReplayBatch::Events(Vec::new());
        };
        let newest = state.retained.back().map_or(oldest, |event| event.seq);
        if last_sequence.saturating_add(1) < oldest {
            return ReplayBatch::ResyncRequired(ResyncRequiredMessage {
                oldest_available_event_seq: oldest,
                newest_event_seq: newest,
            });
        }

        ReplayBatch::Events(
            state
                .retained
                .iter()
                .filter(|event| event.seq > last_sequence)
                .cloned()
                .collect(),
        )
    }
}
