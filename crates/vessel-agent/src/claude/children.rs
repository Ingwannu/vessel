//! Actual task announcements and isolated sidechains, never main-chat replies.
//! No provider task is promoted into a managed session or given lifecycle rights.
use super::{
    map::{ChildBudget, Translator},
    wire::{Frame, System},
};
use std::collections::HashMap;
use vessel_proto::AgentEvent;
use vessel_proto::subagents::{
    ChildActivity, ChildDescriptor, ChildDescriptors, ChildGap, ProviderChildId,
    ProviderObservation,
};

mod tasks;
mod traffic;

#[derive(Debug)]
struct Child {
    descriptor: ChildDescriptor,
    translator: Translator,
    background: Option<bool>,
}
#[derive(Debug)]
pub(super) struct Children {
    entries: Vec<Child>,
    tasks: HashMap<ProviderChildId, ProviderChildId>,
    aliases: HashMap<ProviderChildId, ProviderChildId>,
    owners: HashMap<ProviderChildId, ProviderChildId>,
    budget: ChildBudget,
    saw_tasks: bool,
    supported: bool,
    resumed: bool,
    reported_gaps: Vec<ChildGap>,
}
impl Children {
    pub(super) fn new(resumed: bool) -> Self {
        Self {
            entries: Vec::new(),
            tasks: HashMap::new(),
            aliases: HashMap::new(),
            owners: HashMap::new(),
            budget: ChildBudget::default(),
            saw_tasks: false,
            supported: false,
            resumed,
            reported_gaps: Vec::new(),
        }
    }
    pub(super) fn observe(&mut self, frame: &Frame) -> Vec<AgentEvent> {
        match frame {
            Frame::System(System::Init(_)) => self.support(),
            Frame::System(System::TaskStarted(task)) => self.started(task),
            Frame::System(System::TaskUpdated(task)) => self.updated(task),
            Frame::System(System::TaskNotification(task)) => self.notified(task),
            Frame::System(System::TaskProgress(task)) => self.progress(task),
            Frame::Assistant(message) => self.assistant(message),
            Frame::StreamEvent(event) => self.stream(event),
            Frame::User(message) => self.user(message),
            Frame::System(System::ApiRetry(_) | System::PermissionDenied(_) | System::Unknown)
            | Frame::Result(_)
            | Frame::ControlRequest(_)
            | Frame::ControlResponse(_)
            | Frame::ControlCancelRequest(_)
            | Frame::KeepAlive
            | Frame::Unknown => Vec::new(),
        }
    }
    fn support(&mut self) -> Vec<AgentEvent> {
        if self.supported {
            return Vec::new();
        }
        self.supported = true;
        let mut events = vec![event(ProviderObservation::Supported)];
        // Resume does not prove provider-native replay before this runtime.
        // Preserve the missing prefix instead of claiming an empty full history.
        if self.resumed {
            events.push(event(ProviderObservation::Gap {
                reason: ChildGap::Retention,
            }));
        }
        events
    }
    fn index(&self, id: &ProviderChildId) -> Option<usize> {
        self.entries
            .iter()
            .position(|child| &child.descriptor.id == id)
    }
    fn descriptor(&mut self, descriptor: ChildDescriptor) -> Result<AgentEvent, ChildGap> {
        let mut proposed = self
            .entries
            .iter()
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
        if let Some(row) = self
            .entries
            .iter_mut()
            .find(|row| row.descriptor.id == descriptor.id)
        {
            row.descriptor = descriptor.clone();
        } else {
            self.entries.push(Child {
                descriptor: descriptor.clone(),
                translator: Translator::default(),
                background: None,
            });
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
    fn activity(&mut self, id: &ProviderChildId, events: Vec<AgentEvent>) -> Vec<AgentEvent> {
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
    /// Only explicitly foreground tasks inherit an observed parent cancellation.
    /// Missing background metadata never fabricates a terminal task report.
    pub(super) fn canceled(&mut self) -> Vec<AgentEvent> {
        use vessel_proto::subagents::ChildStatus;
        let descriptors = self
            .entries
            .iter()
            .filter(|child| {
                child.background == Some(false)
                    && child.descriptor.status == Some(ChildStatus::Running)
            })
            .map(|child| {
                let mut descriptor = child.descriptor.clone();
                descriptor.status = Some(ChildStatus::Canceled);
                descriptor
            })
            .collect::<Vec<_>>();
        let mut out = Vec::new();
        for descriptor in descriptors {
            match self.descriptor(descriptor) {
                Ok(event) => out.push(event),
                Err(reason) => out.extend(self.gap(reason)),
            }
        }
        out
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
