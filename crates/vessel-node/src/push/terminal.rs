//! Runtime terminal watermarks beside the Inbox cursor. The single delivery
//! worker coalesces both; neither source consumes the other's position.
use super::{Push, policy};
use crate::terminals::Terminals;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, PoisonError};
use vessel_proto::notifications::{
    NotificationSource, TerminalActivity, TerminalAttention, TerminalAttentionReason,
    TerminalAttentionRevision,
};
use vessel_proto::{SessionStatus, TerminalId};

#[derive(Debug)]
pub(super) struct Observer {
    registry: Arc<Terminals>,
    seen: HashMap<TerminalId, TerminalAttentionRevision>,
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
        (Self { registry, seen }, changes)
    }
    pub(super) fn capture(&mut self) -> Burst {
        let mut candidates = fresh(&mut self.seen, self.registry.list(|_info| true));
        if vessel_proto::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL.is_none() {
            candidates.clear();
        }
        Burst {
            registry: Arc::clone(&self.registry),
            candidates: candidates.into(),
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
