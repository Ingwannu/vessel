//! Root compaction observations, independent of native command execution.
//! Bounded retained ids prevent duplicate completion channels from reopening rows.
use serde::Deserialize;
use serde_json::Value;
use vessel_proto::{
    AgentEvent,
    timeline::{Compaction, CompactionStatus, ExternalTurnId},
};

const MAX_COMPACTION_OBSERVATIONS: usize = 128;
const MAX_COMPACTION_ITEM_BYTES: usize = 1024;
#[derive(Debug, Default)]
pub(super) struct Compactions {
    buckets: Vec<Bucket>,
    admitted: usize,
}
#[derive(Debug)]
struct Bucket {
    turn: Option<ExternalTurnId>,
    items: Vec<Item>,
    notification_completions: usize,
    item_completions: usize,
}
#[derive(Debug)]
struct Item {
    id: String,
    loading: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadFrame {
    thread_id: String,
    #[serde(default)]
    turn_id: Option<ExternalTurnId>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemFrame {
    #[serde(flatten)]
    owner: ThreadFrame,
    item: NativeItem,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum NativeItem {
    ContextCompaction {
        id: String,
    },
    #[serde(other)]
    Other,
}

impl Compactions {
    /// None delegates to the normal item reader. Child routing remains separate.
    pub(super) fn observe(
        &mut self,
        root: &str,
        method: &str,
        params: &Value,
    ) -> Option<Vec<AgentEvent>> {
        match method {
            "item/started" | "item/completed" => {
                let frame: ItemFrame = serde_json::from_value(params.clone()).ok()?;
                let NativeItem::ContextCompaction { id } = frame.item else {
                    return None;
                };
                if frame.owner.thread_id != root {
                    return Some(vec![]);
                }
                if id.is_empty()
                    || id.len() > MAX_COMPACTION_ITEM_BYTES
                    || id.chars().any(char::is_control)
                {
                    return Some(vec![]);
                }
                Some(self.item(frame.owner.turn_id, id, method == "item/started"))
            }
            "thread/compacted" => {
                let frame: ThreadFrame = serde_json::from_value(params.clone()).ok()?;
                Some(if frame.thread_id == root {
                    self.notification(frame.turn_id)
                } else {
                    vec![]
                })
            }
            _ => None,
        }
    }

    fn bucket(&mut self, turn: Option<ExternalTurnId>) -> Option<usize> {
        if let Some(index) = self.buckets.iter().position(|bucket| bucket.turn == turn) {
            return Some(index);
        }
        if self.buckets.len() >= MAX_COMPACTION_OBSERVATIONS {
            return None;
        }
        let index = self.buckets.len();
        self.buckets.push(Bucket {
            turn,
            items: vec![],
            notification_completions: 0,
            item_completions: 0,
        });
        Some(index)
    }

    fn item(&mut self, turn: Option<ExternalTurnId>, id: String, started: bool) -> Vec<AgentEvent> {
        let Some(index) = self.bucket(turn) else {
            return vec![];
        };
        let Some(bucket) = self.buckets.get_mut(index) else {
            return vec![];
        };
        let known = bucket.items.iter().position(|item| item.id == id);
        let position = if let Some(position) = known {
            position
        } else {
            if self.admitted >= MAX_COMPACTION_OBSERVATIONS {
                return vec![];
            }
            self.admitted = self.admitted.saturating_add(1);
            let position = bucket.items.len();
            bucket.items.push(Item {
                id,
                loading: started,
            });
            position
        };
        if started {
            return if known.is_none() {
                vec![event(CompactionStatus::Loading)]
            } else {
                vec![]
            };
        }
        let Some(item) = bucket.items.get_mut(position) else {
            return vec![];
        };
        if known.is_some() && !item.loading {
            return vec![];
        }
        item.loading = false;
        if bucket.notification_completions > 0 {
            bucket.notification_completions = bucket.notification_completions.saturating_sub(1);
            return vec![];
        }
        bucket.item_completions = bucket.item_completions.saturating_add(1);
        vec![event(CompactionStatus::Completed)]
    }

    fn notification(&mut self, turn: Option<ExternalTurnId>) -> Vec<AgentEvent> {
        let existing = turn
            .is_none()
            .then(|| {
                self.buckets.iter().position(|bucket| {
                    bucket.item_completions > 0 || bucket.items.iter().any(|item| item.loading)
                })
            })
            .flatten();
        let Some(index) = existing.or_else(|| self.bucket(turn)) else {
            return vec![];
        };
        let Some(bucket) = self.buckets.get_mut(index) else {
            return vec![];
        };
        if bucket.item_completions > 0 {
            bucket.item_completions = bucket.item_completions.saturating_sub(1);
            return vec![];
        }
        if let Some(item) = bucket.items.iter_mut().find(|item| item.loading) {
            item.loading = false;
        } else {
            if self.admitted >= MAX_COMPACTION_OBSERVATIONS {
                return vec![];
            }
            self.admitted = self.admitted.saturating_add(1);
            bucket.notification_completions = bucket.notification_completions.saturating_add(1);
        }
        vec![event(CompactionStatus::Completed)]
    }

    /// Some provider versions omit item completion before the actual root turn ends.
    pub(super) fn finish(&mut self, turn: &str) -> Vec<AgentEvent> {
        let mut completed = vec![];
        for bucket in &mut self.buckets {
            if bucket.turn.as_ref().is_some_and(|id| id.as_str() != turn) {
                continue;
            }
            for item in &mut bucket.items {
                if item.loading {
                    item.loading = false;
                    bucket.item_completions = bucket.item_completions.saturating_add(1);
                    completed.push(event(CompactionStatus::Completed));
                }
            }
        }
        completed
    }
}

fn event(status: CompactionStatus) -> AgentEvent {
    AgentEvent::Compaction(Compaction {
        status,
        trigger: None,
        pre_tokens: None,
    })
}

#[cfg(test)]
mod tests;
