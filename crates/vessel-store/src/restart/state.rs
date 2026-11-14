//! A bounded per-session projection, independent of retained transcript bytes.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vessel_proto::restart::{RestartCheckpoint, RestartGeneration};
use vessel_proto::{AgentEvent, EventRecord, EventSeq, SessionEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Turn {
    pub seq: EventSeq,
    pub at: DateTime<Utc>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Creation-time recovery eligibility, checked again against current marks/origin.
pub enum Eligibility {
    #[default]
    /// Retain explicit manual control (including unattended creations).
    Never,
    /// A person created this session; structural exclusions still apply.
    Person,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct State {
    pub eligibility: Eligibility,
    pub active: Option<Turn>,
    pub pending: Option<Turn>,
    pub generation: RestartGeneration,
    pub blocked: bool,
    pub parked: bool,
    pub claimed: bool,
}
impl State {
    pub(crate) fn eligible(&self) -> bool {
        self.eligibility == Eligibility::Person
    }
    pub(crate) fn observe(&mut self, record: &EventRecord) {
        match &record.event {
            SessionEvent::Agent {
                event: AgentEvent::TurnStarted { .. },
            } => {
                // This records activity but cannot undo a person's stop.
                // Only an accepted input's generation checkpoint re-arms it.
                self.active = Some(Turn {
                    seq: record.seq,
                    at: record.at,
                });
            }
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded { .. } | AgentEvent::TurnFailed { .. },
            }
            | SessionEvent::Rewound { .. } => self.active = None,
            SessionEvent::Status { status } if !status.is_live() => self.active = None,
            SessionEvent::Parked { .. } => {
                self.parked = true;
                self.active = None;
            }
            SessionEvent::Woken => self.parked = false,
            SessionEvent::Status { .. }
            | SessionEvent::Agent { .. }
            | SessionEvent::Terminal(_)
            | SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. }
            | SessionEvent::QueueChanged { .. }
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::UsageChanged { .. }
            | SessionEvent::Forked { .. } => {}
        }
    }
    pub(crate) fn checkpoint(&mut self, change: RestartCheckpoint, enabled: bool) -> bool {
        match change {
            RestartCheckpoint::Arm { generation, turn } => {
                if generation < self.generation
                    || self.active.is_none_or(|active| active.seq != turn)
                {
                    return false;
                }
                self.generation = generation;
                self.blocked = false;
            }
            RestartCheckpoint::Suppress { generation, turn } => {
                if generation < self.generation
                    || turn.is_some_and(|turn| self.active.is_none_or(|active| active.seq != turn))
                {
                    return false;
                }
                self.generation = generation;
                self.blocked = true;
                self.pending = None;
            }
            RestartCheckpoint::Shutdown {
                generation,
                turn,
                parked,
            } => {
                if enabled
                    && self.eligible()
                    && !self.blocked
                    && !parked
                    && !self.parked
                    && generation >= self.generation
                {
                    self.pending = self.active.filter(|active| Some(active.seq) == turn);
                }
            }
        }
        true
    }
}
