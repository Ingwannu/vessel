//! Runtime terminal watermarks beside the Inbox cursor. The single delivery
//! worker coalesces both; neither source consumes the other's position.
use super::{Push, policy};
use crate::terminals::Terminals;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, PoisonError};
use vessel_proto::notifications::{
    NotificationSource, TerminalActivity, TerminalAttention, TerminalAttentionReason,
    TerminalAttentionRevision,
};
use vessel_proto::{SessionStatus, TerminalId};

mod spacing;

#[derive(Debug)]
pub(super) struct Observer {
    registry: Arc<Terminals>,
    seen: HashMap<TerminalId, TerminalAttentionRevision>,
    /// How soon each terminal may push again (`spacing.rs`).
    spacing: spacing::Spacing,
}
#[derive(Debug, Clone)]
pub(super) struct Burst {
    registry: Arc<Terminals>,
    candidates: Arc<[TerminalAttention]>,
}
impl Observer {
    pub(super) fn new(registry: Arc<Terminals>) -> (Self, tokio::sync::watch::Receiver<()>) {
        let changes = registry.attention_changes();
        let seen = registry
            .list(|_info| true)
            .into_iter()
            .filter_map(|info| info.attention.map(|state| (info.id, state.revision)))
            .collect();
        (
            Self {
                registry,
                seen,
                spacing: spacing::Spacing::default(),
            },
            changes,
        )
    }
    /// This burst's candidates: new attention, of terminals whose last push
    /// is far enough behind `now`. Attention inside a gap is seen and
    /// dropped, never pushed later.
    pub(super) fn capture(&mut self, now: chrono::DateTime<Utc>) -> Burst {
        let mut candidates = fresh(&mut self.seen, self.registry.list(|_info| true));
        let seen = &self.seen;
        self.spacing.retain(|terminal| seen.contains_key(&terminal));
        candidates.retain(|state| self.spacing.allows(state.terminal, now));
        if vessel_proto::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL.is_none() {
            candidates.clear();
        }
        Burst {
            registry: Arc::clone(&self.registry),
            candidates: candidates.into(),
        }
    }
}
impl Observer {
    /// The burst's deliveries pushed these terminals' attention (each once,
    /// however many devices received it).
    pub(super) fn pushed(&mut self, terminals: HashSet<TerminalId>, now: chrono::DateTime<Utc>) {
        for terminal in terminals {
            self.spacing.pushed(terminal, now);
        }
    }
}
impl Burst {
    pub(super) fn current(&self, push: &Push) -> Option<NotificationSource> {
        self.candidates.iter().find_map(|state| {
            let info = self.registry.get(state.terminal).ok()?.handle.info();
            if info.attention != Some(*state) || matches!(info.status, SessionStatus::Failed { .. })
            {
                return None;
            }
            let target = vessel_proto::push::PushFocus::Terminal {
                terminal: state.terminal,
            };
            let snapshots: Vec<_> = push
                .presence
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .values()
                .copied()
                .collect();
            matches!(
                policy::plan_for(&snapshots, target, Utc::now()),
                policy::Plan::Push
            )
            .then_some(NotificationSource::Terminal {
                terminal: state.terminal,
                revision: state.revision,
            })
        })
    }
}

fn fresh(
    seen: &mut HashMap<TerminalId, TerminalAttentionRevision>,
    infos: Vec<vessel_proto::terminals::TerminalInfo>,
) -> Vec<TerminalAttention> {
    seen.retain(|id, _revision| infos.iter().any(|info| info.id == *id));
    let mut candidates = Vec::new();
    for info in infos {
        let Some(state) = info.attention else {
            continue;
        };
        if seen
            .get(&info.id)
            .is_some_and(|revision| *revision >= state.revision)
        {
            continue;
        }
        let _previous = seen.insert(info.id, state.revision);
        if current(&info, state)
            && matches!(
                state.activity,
                TerminalActivity::Attention {
                    reason: TerminalAttentionReason::Finished | TerminalAttentionReason::NeedsInput
                }
            )
        {
            candidates.push(state);
        }
    }
    candidates
}
fn current(info: &vessel_proto::terminals::TerminalInfo, state: TerminalAttention) -> bool {
    info.id == state.terminal
        && info.attention == Some(state)
        && !matches!(info.status, SessionStatus::Failed { .. })
}

#[cfg(test)]
mod tests;
