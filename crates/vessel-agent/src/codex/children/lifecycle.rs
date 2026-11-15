//! Native collaboration state establishes routes; completing a parent tool does not end a child.
use super::{Children, identity, wire};
use vessel_proto::subagents::{ChildGap, ChildStatus, ProviderChildId, ProviderText};
use vessel_proto::{AgentEvent, ContentBlock};
impl Children {
    pub(super) fn lifecycle(
        &mut self,
        root: &str,
        envelope: wire::ItemEnvelope,
        out: &mut Vec<AgentEvent>,
    ) {
        let parent = if envelope.thread_id == root {
            None
        } else {
            let Ok(id) = envelope.thread_id.parse::<ProviderChildId>() else {
                return;
            };
            if !self.entries.contains_key(&id) {
                return;
            }
            Some(id)
        };
        match envelope.item {
            wire::ChildItem::CollabAgentToolCall(call) => {
                self.collab(root, parent.as_ref(), &call, out);
            }
            wire::ChildItem::SubAgentActivity(activity) => {
                self.activity(root, parent, &activity, out);
            }
            wire::ChildItem::Other => {}
        }
    }
    fn collab(
        &mut self,
        root: &str,
        parent: Option<&ProviderChildId>,
        call: &wire::Collab,
        out: &mut Vec<AgentEvent>,
    ) {
        if call
            .sender_thread_id
            .as_deref()
            .is_some_and(|sender| sender != parent.map_or(root, ProviderChildId::as_str))
        {
            return;
        }
        if call.receiver_thread_ids.len() > vessel_proto::subagents::bounds::MAX_CHILDREN
            || call.agents_states.len() > vessel_proto::subagents::bounds::MAX_CHILDREN
        {
            out.extend(self.gap(ChildGap::IngestionBudget));
            return;
        }
        for raw in &call.receiver_thread_ids {
            let Ok(id) = raw.parse::<ProviderChildId>() else {
                out.extend(self.gap(ChildGap::IngestionBudget));
                continue;
            };
            if raw == root
                || (call.tool != wire::Tool::SpawnAgent && !self.entries.contains_key(&id))
            {
                continue;
            }
            let mut descriptor = self.entries.get(&id).map_or_else(
                || identity(id.clone(), parent.cloned()),
                |row| row.descriptor.clone(),
            );
            if descriptor.parent.as_ref() != parent && call.tool == wire::Tool::SpawnAgent {
                out.extend(self.gap(ChildGap::Nesting));
                continue;
            }
            if let Some(prompt) = &call.prompt {
                let Ok(text) = prompt.parse::<ProviderText>() else {
                    out.extend(self.gap(ChildGap::IngestionBudget));
                    continue;
                };
                descriptor.description = Some(text);
            }
            if let Some(state) = call.agents_states.get(raw) {
                descriptor.status = status(state.status).or(descriptor.status);
                if let Some(message) = &state.message {
                    let Ok(text) = message.parse::<ProviderText>() else {
                        out.extend(self.gap(ChildGap::IngestionBudget));
                        continue;
                    };
                    descriptor.subtitle = Some(text);
                }
            }
            match self.descriptor(descriptor) {
                Ok(event) => out.push(event),
                Err(reason) => {
                    out.extend(self.gap(reason));
                    continue;
                }
            }
            if matches!(call.tool, wire::Tool::SpawnAgent | wire::Tool::SendInput) {
                self.prompt(&id, &call.id, call.prompt.as_deref(), out);
            }
        }
    }
    fn activity(
        &mut self,
        root: &str,
        parent: Option<ProviderChildId>,
        activity: &wire::Activity,
        out: &mut Vec<AgentEvent>,
    ) {
        let Ok(id) = activity.agent_thread_id.parse::<ProviderChildId>() else {
            out.extend(self.gap(ChildGap::IngestionBudget));
            return;
        };
        if activity.agent_thread_id == root {
            return;
        }
        let Ok(title) = activity.agent_path.parse::<ProviderText>() else {
            out.extend(self.gap(ChildGap::IngestionBudget));
            return;
        };
        let mut descriptor = self.entries.get(&id).map_or_else(
            || identity(id.clone(), parent),
            |row| row.descriptor.clone(),
        );
        descriptor.title = Some(title);
        descriptor.status = match activity.kind {
            wire::ActivityKind::Started | wire::ActivityKind::Interacted => {
                Some(ChildStatus::Running)
            }
            wire::ActivityKind::Interrupted => Some(ChildStatus::Canceled),
            wire::ActivityKind::Unknown => descriptor.status,
        };
        // The activity id is not a new task identity; it is a reported marker only.
        if activity.id.parse::<ProviderChildId>().is_err() {
            out.extend(self.gap(ChildGap::IngestionBudget));
            return;
        }
        match self.descriptor(descriptor) {
            Ok(event) => out.push(event),
            Err(reason) => out.extend(self.gap(reason)),
        }
    }
    fn prompt(
        &mut self,
        id: &ProviderChildId,
        call: &str,
        prompt: Option<&str>,
        out: &mut Vec<AgentEvent>,
    ) {
        let Some(prompt) = prompt.filter(|text| !text.is_empty()) else {
            return;
        };
        let Some(child) = self.entries.get_mut(id) else {
            return;
        };
        if child.prompts.contains(call) {
            return;
        }
        if !self.budget.retain(call) {
            out.extend(self.gap(ChildGap::IngestionBudget));
            return;
        }
        let _new = child.prompts.insert(call.to_owned());
        out.extend(self.wrap(
            id,
            vec![AgentEvent::UserMessageChunk {
                content: ContentBlock::Text {
                    text: prompt.to_owned(),
                },
            }],
        ));
    }
}
fn status(status: wire::Status) -> Option<ChildStatus> {
    match status {
        wire::Status::PendingInit | wire::Status::Running => Some(ChildStatus::Running),
        wire::Status::Completed => Some(ChildStatus::Completed),
        wire::Status::Errored => Some(ChildStatus::Failed),
        wire::Status::Shutdown => Some(ChildStatus::Canceled),
        wire::Status::NotFound | wire::Status::Unknown => None,
    }
}
