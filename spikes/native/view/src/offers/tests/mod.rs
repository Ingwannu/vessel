//! The store's rules, the merge of a new answer into a shown list hardest:
//! what is under the pointer never moves, the choice in effect is the
//! agent's newest, nothing is invented.

use std::time::{Duration, Instant};

use vessel_proto::{
    AgentCommand, HarnessKind, HarnessRef, OptionCategory, OptionId, OptionKind, OptionValue,
    OptionValueId, SessionOption,
};

use super::*;

fn value(id: &str) -> OptionValue {
    OptionValue {
        id: OptionValueId::new(id),
        name: id.to_uppercase(),
        description: None,
        group: None,
    }
}

fn grouped(id: &str, group: &str) -> OptionValue {
    OptionValue {
        group: Some(group.to_owned()),
        ..value(id)
    }
}

fn select(id: &str, current: &str, values: Vec<OptionValue>) -> SessionOption {
    SessionOption {
        id: OptionId::new(id),
        name: id.to_owned(),
        description: None,
        category: OptionCategory::Model,
        kind: OptionKind::Select {
            current: OptionValueId::new(current),
            values,
        },
    }
}

fn toggle(id: &str, on: bool) -> SessionOption {
    SessionOption {
        id: OptionId::new(id),
        name: id.to_owned(),
        description: None,
        category: OptionCategory::Other,
        kind: OptionKind::Toggle { on },
    }
}

fn models(current: &str, ids: &[&str]) -> SessionOption {
    select("model", current, ids.iter().map(|id| value(id)).collect())
}

/// The value ids of option `id`, in order.
fn value_ids(options: &[SessionOption], id: &str) -> Vec<String> {
    options
        .iter()
        .find(|option| option.id.as_str() == id)
        .map(|option| match &option.kind {
            OptionKind::Select { values, .. } => values
                .iter()
                .map(|value| value.id.as_str().to_owned())
                .collect(),
            OptionKind::Toggle { .. } | OptionKind::Unknown => Vec::new(),
        })
        .unwrap_or_default()
}

fn option_ids(options: &[SessionOption]) -> Vec<&str> {
    options.iter().map(|option| option.id.as_str()).collect()
}

fn current_of(options: &[SessionOption], id: &str) -> Option<String> {
    options
        .iter()
        .find(|option| option.id.as_str() == id)
        .and_then(|option| match &option.kind {
            OptionKind::Select { current, .. } => Some(current.as_str().to_owned()),
            OptionKind::Toggle { .. } | OptionKind::Unknown => None,
        })
}

/// `now`, `by` later.
fn after(now: Instant, by: Duration) -> Instant {
    now.checked_add(by).unwrap()
}

fn secs(count: u64) -> Duration {
    Duration::from_secs(count)
}

fn key() -> OfferKey {
    OfferKey::new("laptop", HarnessRef::Known(HarnessKind::ClaudeCode))
}

mod arrange;
mod store;
