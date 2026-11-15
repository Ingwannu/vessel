//! Child activity uses the same canonical mapper as the managed parent.
use super::{Children, mapper, protocol};
use serde_json::Value;
use vessel_proto::subagents::{ChildGap, ChildStatus, ProviderChildId};
use vessel_proto::{AgentEvent, ContentBlock};
impl Children {
    pub(super) fn timeline(
        &mut self,
        id: &ProviderChildId,
        method: &str,
        params: &Value,
    ) -> Vec<AgentEvent> {
        let events = match method {
            "item/started" | "item/completed" => {
                let Ok(item) = serde_json::from_value::<protocol::ItemNotification>(params.clone())
                else {
                    return self.gap(ChildGap::IngestionBudget);
                };
                self.item(id, method == "item/started", &item.item)
            }
            "item/agentMessage/delta" | "item/reasoning/summaryTextDelta" => {
                let Ok(delta) =
                    serde_json::from_value::<protocol::DeltaNotification>(params.clone())
                else {
                    return self.gap(ChildGap::IngestionBudget);
                };
                if !self.reserve(id, &delta.item_id, Cache::Streamed) {
                    return self.gap(ChildGap::IngestionBudget);
                }
                let content = ContentBlock::Text { text: delta.delta };
                vec![if method == "item/agentMessage/delta" {
                    AgentEvent::MessageChunk { content }
                } else {
                    AgentEvent::ThoughtChunk { content }
                }]
            }
            "turn/plan/updated" => {
                let Ok(plan) = serde_json::from_value::<protocol::PlanNotification>(params.clone())
                else {
                    return self.gap(ChildGap::IngestionBudget);
                };
                vec![mapper::plan(&plan.plan)]
            }
            "turn/started" | "turn/completed" => return self.turn(id, method, params),
            _ => Vec::new(),
        };
        self.wrap(id, events)
    }
    fn reserve(&mut self, id: &ProviderChildId, key: &str, cache: Cache) -> bool {
        let Some(child) = self.entries.get_mut(id) else {
            return false;
        };
        let set = match cache {
            Cache::Streamed => &mut child.streamed,
            Cache::Announced => &mut child.announced,
        };
        if set.contains(key) {
            return true;
        }
        if !self.budget.retain(key) {
            return false;
        }
        set.insert(key.to_owned());
        true
    }
    fn item(
        &mut self,
        id: &ProviderChildId,
        started: bool,
        item: &protocol::ThreadItem,
    ) -> Vec<AgentEvent> {
        let mut out = Vec::new();
        if started {
            if let Some(call) = mapper::tool_call(item) {
                if self
                    .entries
                    .get(id)
                    .is_some_and(|row| row.announced.contains(call.id.as_str()))
                {
                    return out;
                }
                if !self.reserve(id, call.id.as_str(), Cache::Announced) {
                    return self.gap(ChildGap::IngestionBudget);
                }
                out.push(AgentEvent::ToolCall(call));
            }
            return out;
        }
        if let Some(update) = mapper::tool_call_done(item) {
            out.push(AgentEvent::ToolCallUpdate(update));
        }
        if let Some((key, whole)) = mapper::whole_text(item)
            && !self
                .entries
                .get(id)
                .is_some_and(|row| row.streamed.contains(key))
        {
            out.push(whole);
        }
        out
    }
    fn turn(&mut self, id: &ProviderChildId, method: &str, params: &Value) -> Vec<AgentEvent> {
        let Ok(turn) = serde_json::from_value::<protocol::TurnNotification>(params.clone()) else {
            return self.gap(ChildGap::IngestionBudget);
        };
        let status = if method == "turn/started" {
            Some(ChildStatus::Running)
        } else {
            match turn.turn.status {
                protocol::TurnStatus::Completed => Some(ChildStatus::Completed),
                protocol::TurnStatus::Interrupted => Some(ChildStatus::Canceled),
                protocol::TurnStatus::Failed => Some(ChildStatus::Failed),
                protocol::TurnStatus::InProgress | protocol::TurnStatus::Other => None,
            }
        };
        let mut out = status.map_or_else(Vec::new, |status| self.set_status(id, status));
        if method == "turn/completed" && status.is_some() {
            out.extend(self.wrap(id, vec![mapper::turn_end(&turn.turn, None)]));
        }
        out
    }
}
#[derive(Clone, Copy)]
enum Cache {
    Streamed,
    Announced,
}
