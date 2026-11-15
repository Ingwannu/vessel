//! Only observed child routes receive separate canonical timelines and no managed rights.
use super::{mapper, protocol};
use crate::child_ids::ChildBudget;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use vessel_proto::AgentEvent;
use vessel_proto::subagents::{
    ChildActivity, ChildDescriptor, ChildDescriptors, ChildGap, ChildStatus, ProviderChildId,
    ProviderObservation,
};
mod lifecycle;
mod timeline;
mod wire;

#[derive(Debug)]
struct Child {
    descriptor: ChildDescriptor,
    streamed: HashSet<String>,
    announced: HashSet<String>,
    prompts: HashSet<String>,
}
#[derive(Debug)]
pub(super) struct Children {
    entries: HashMap<ProviderChildId, Child>,
    budget: ChildBudget,
    supported: bool,
    resumed: bool,
    reported_gaps: Vec<ChildGap>,
}
impl Children {
    pub(super) fn new(resumed: bool) -> Self {
        Self {
            entries: HashMap::new(),
            budget: ChildBudget::default(),
            supported: false,
            resumed,
            reported_gaps: Vec::new(),
        }
    }
    pub(super) fn observe(&mut self, root: &str, method: &str, params: &Value) -> Vec<AgentEvent> {
        let mut out = Vec::new();
        if !self.supported {
            self.supported = true;
            out.push(event(ProviderObservation::Supported));
            if self.resumed {
                out.push(event(ProviderObservation::Gap {
                    reason: ChildGap::Retention,
                }));
            }
        }
        if matches!(method, "item/started" | "item/completed")
            && let Ok(envelope) = serde_json::from_value::<wire::ItemEnvelope>(params.clone())
        {
            self.lifecycle(root, envelope, &mut out);
        }
        let Ok(envelope) = serde_json::from_value::<wire::ThreadEnvelope>(params.clone()) else {
            return out;
        };
        if envelope.thread_id == root {
            return out;
        }
        let Ok(id) = envelope.thread_id.parse::<ProviderChildId>() else {
            return out;
        };
        if !self.entries.contains_key(&id) {
            return out;
        }
        out.extend(self.timeline(&id, method, params));
        out
    }
    fn descriptor(&mut self, descriptor: ChildDescriptor) -> Result<AgentEvent, ChildGap> {
        let mut proposed = self
            .entries
            .values()
            .map(|entry| entry.descriptor.clone())
            .collect::<Vec<_>>();
        if let Some(row) = proposed.iter_mut().find(|row| row.id == descriptor.id) {
            *row = descriptor.clone();
        } else {
            proposed.push(descriptor.clone());
        }
        let _validated = ChildDescriptors::try_from(proposed).map_err(|error| match error {
            vessel_proto::subagents::InvalidChildCollection::Identity => ChildGap::Nesting,
            vessel_proto::subagents::InvalidChildCollection::Count
            | vessel_proto::subagents::InvalidChildCollection::Bytes
            | vessel_proto::subagents::InvalidChildCollection::Timeline => {
                ChildGap::IngestionBudget
            }
        })?;
        if let Some(entry) = self.entries.get_mut(&descriptor.id) {
            entry.descriptor = descriptor.clone();
        } else {
            let mut budget = self.budget;
            if !budget.retain(descriptor.id.as_str()) {
                return Err(ChildGap::IngestionBudget);
            }
            self.budget = budget;
            let _previous = self.entries.insert(
                descriptor.id.clone(),
                Child {
                    descriptor: descriptor.clone(),
                    streamed: HashSet::new(),
                    announced: HashSet::new(),
                    prompts: HashSet::new(),
                },
            );
        }
        Ok(event(ProviderObservation::Descriptor { descriptor }))
    }
    fn gap(&mut self, reason: ChildGap) -> Vec<AgentEvent> {
        if self.reported_gaps.contains(&reason) {
            return Vec::new();
        }
        self.reported_gaps.push(reason);
        vec![event(ProviderObservation::Gap { reason })]
    }
    fn wrap(&mut self, id: &ProviderChildId, events: Vec<AgentEvent>) -> Vec<AgentEvent> {
        let mut out = Vec::new();
        for activity in events {
            match ChildActivity::try_from(activity) {
                Ok(activity) => out.push(event(ProviderObservation::Activity {
                    child: id.clone(),
                    activity: Box::new(activity),
                })),
                Err(_invalid) => out.extend(self.gap(ChildGap::IngestionBudget)),
            }
        }
        out
    }
    fn set_status(&mut self, id: &ProviderChildId, status: ChildStatus) -> Vec<AgentEvent> {
        let Some(entry) = self.entries.get(id) else {
            return Vec::new();
        };
        if entry.descriptor.status == Some(status) {
            return Vec::new();
        }
        let mut descriptor = entry.descriptor.clone();
        descriptor.status = Some(status);
        match self.descriptor(descriptor) {
            Ok(event) => vec![event],
            Err(reason) => self.gap(reason),
        }
    }
}
fn event(observation: ProviderObservation) -> AgentEvent {
    AgentEvent::ProviderChild(observation)
}
fn identity(id: ProviderChildId, parent: Option<ProviderChildId>) -> ChildDescriptor {
    ChildDescriptor {
        id,
        parent,
        title: None,
        description: None,
        subtitle: None,
        status: None,
        created_at: None,
        updated_at: None,
    }
}
#[cfg(test)]
mod tests;
