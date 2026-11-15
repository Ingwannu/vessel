//! Finite provider-edge enrichment: output-only updates can retain known arguments.
//! No eviction: forgotten identities must never make a reused unscoped id look unique.
use std::collections::HashMap;
use vessel_proto::timeline::{ToolCallDetail, ToolCallScope, ToolFailure};
use vessel_proto::{AgentEvent, ContentBlock, ToolCallContent, ToolCallId, ToolKind};

const MAX_DETAIL_SNAPSHOTS: usize = 128;
const MAX_DETAIL_SNAPSHOT_BYTES: usize = 524_288;
const MAX_DETAIL_BYTES: usize = 65_536;
const SNAPSHOT_ROW_BYTES: usize = 256;

type Key = (ToolCallId, Option<Box<ToolCallScope>>);
#[derive(Debug)]
struct Snapshot {
    turn: Option<u64>,
    kind: ToolKind,
    detail: Option<ToolCallDetail>,
}
#[derive(Debug, Default)]
pub(crate) struct Snapshots {
    turn: Option<u64>,
    entries: HashMap<Key, Snapshot>,
    bytes: usize,
}

impl Snapshots {
    pub(crate) fn observe(&mut self, event: &mut AgentEvent) {
        match event {
            AgentEvent::TurnStarted { .. } => {
                self.turn = Some(self.turn.unwrap_or_default().saturating_add(1));
            }
            AgentEvent::ToolCall(call) => self.remember(call),
            AgentEvent::ToolCallUpdate(update) => self.enrich(update),
            AgentEvent::Ready { .. }
            | AgentEvent::UserMessageChunk { .. }
            | AgentEvent::MessageChunk { .. }
            | AgentEvent::ThoughtChunk { .. }
            | AgentEvent::TurnEnded { .. }
            | AgentEvent::TurnFailed { .. }
            | AgentEvent::PermissionRequested(_)
            | AgentEvent::PermissionResolved { .. }
            | AgentEvent::QuestionAsked(_)
            | AgentEvent::QuestionResolved { .. }
            | AgentEvent::Plan { .. }
            | AgentEvent::Notification(_)
            | AgentEvent::Compaction(_)
            | AgentEvent::Plugin(_)
            | AgentEvent::CommandAccepted { .. }
            | AgentEvent::Error { .. }
            | AgentEvent::Steered { .. }
            | AgentEvent::HarnessLog { .. } => {}
        }
    }

    fn remember(&mut self, call: &vessel_proto::ToolCall) {
        let Some(detail) = call.detail.as_deref() else {
            return;
        };
        let key = (call.id.clone(), call.scope.clone());
        if let Some(previous) = self.entries.get_mut(&key) {
            if key.1.is_none() && previous.turn != self.turn {
                previous.detail = None;
            }
            return;
        }
        let Ok(encoded) = serde_json::to_vec(detail) else {
            return;
        };
        let scope_bytes = call.scope.as_deref().map_or(0, |scope| match scope {
            ToolCallScope::ProviderTurn { thread, turn } => {
                thread.as_str().len().saturating_add(turn.as_str().len())
            }
            ToolCallScope::SessionTurn { .. } | ToolCallScope::Record { .. } => 0,
        });
        let Some(charge) = encoded
            .len()
            .checked_add(call.id.as_str().len())
            .and_then(|value| value.checked_add(scope_bytes))
            .and_then(|value| value.checked_add(SNAPSHOT_ROW_BYTES))
        else {
            return;
        };
        let Some(bytes) = self.bytes.checked_add(charge) else {
            return;
        };
        if encoded.len() > MAX_DETAIL_BYTES
            || self.entries.len() >= MAX_DETAIL_SNAPSHOTS
            || bytes > MAX_DETAIL_SNAPSHOT_BYTES
        {
            return;
        }
        let _prior = self.entries.insert(
            key,
            Snapshot {
                turn: self.turn,
                kind: call.kind,
                detail: Some(detail.clone()),
            },
        );
        self.bytes = bytes;
    }

    fn enrich(&self, update: &mut vessel_proto::ToolCallUpdate) {
        let previous = self
            .entries
            .get(&(update.id.clone(), update.scope.clone()))
            .filter(|previous| update.scope.is_some() || previous.turn == self.turn);
        let mut detail = match previous.and_then(|value| value.detail.clone()) {
            Some(previous) => Some(previous.updated(update.detail.as_deref().cloned())),
            None => update.detail.as_deref().cloned(),
        };
        if let Some(raw) = &update.raw_input {
            let kind = update.kind.or_else(|| previous.map(|value| value.kind));
            if let (Some(kind), Ok(input)) = (kind, serde_json::from_str(raw.as_text()))
                && let Some(next) = super::for_kind(kind, &input)
            {
                detail = Some(next);
            }
        }
        let text = update.content.as_deref().and_then(text_content);
        if let Some(text) = text.as_deref() {
            if update.status == Some(vessel_proto::ToolCallStatus::Failed)
                && update.failure.is_none()
            {
                update.failure = Some(ToolFailure {
                    message: text.to_owned(),
                });
            }
            if let Some(detail) = &mut detail {
                attach_text(detail, text);
            }
        }
        if let Some(detail) = detail
            && serde_json::to_vec(&detail).is_ok_and(|encoded| encoded.len() <= MAX_DETAIL_BYTES)
        {
            update.detail = Some(Box::new(detail));
        }
    }
}

fn text_content(content: &[ToolCallContent]) -> Option<String> {
    let mut combined = String::new();
    for content in content {
        let ToolCallContent::Content {
            content: ContentBlock::Text { text },
        } = content
        else {
            continue;
        };
        let size = combined.len().checked_add(text.len())?.checked_add(1)?;
        if size > MAX_DETAIL_BYTES {
            return None;
        }
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(text);
    }
    (!combined.is_empty()).then_some(combined)
}

fn attach_text(detail: &mut ToolCallDetail, text: &str) {
    match detail {
        ToolCallDetail::Shell(detail) => detail.output = Some(text.to_owned()),
        ToolCallDetail::Read(detail) => detail.content = Some(text.to_owned()),
        ToolCallDetail::Search(detail) => detail.content = Some(text.to_owned()),
        ToolCallDetail::Fetch(detail) => detail.result = Some(text.to_owned()),
        ToolCallDetail::PlainText(detail) => detail.text = Some(text.to_owned()),
        ToolCallDetail::Unknown(detail) => {
            detail.output = vessel_proto::RawJson::from_serialize(text).ok();
        }
        ToolCallDetail::Edit(_)
        | ToolCallDetail::Write(_)
        | ToolCallDetail::Plan(_)
        | ToolCallDetail::WorktreeSetup(_)
        | ToolCallDetail::SubAgent(_) => {}
    }
}

#[cfg(test)]
mod tests;
