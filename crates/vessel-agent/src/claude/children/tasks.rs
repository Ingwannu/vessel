//! Reported lifecycle and aliases outlive turns; a Task tool return is not completion.
use super::super::wire::{
    TaskKind, TaskNotification, TaskProgress, TaskStarted, TaskStatus, TaskUpdated,
};
use super::{Children, identity};
use vessel_proto::subagents::{
    ChildDescriptor, ChildGap, ChildStatus, ProviderChildId, ProviderText,
};
use vessel_proto::{AgentEvent, ContentBlock};

impl Children {
    pub(super) fn started(&mut self, task: &TaskStarted) -> Vec<AgentEvent> {
        self.saw_tasks = true;
        let mut out = self.support();
        if !task.is_child() {
            return out;
        }
        let Some(tool) = task
            .tool_use_id
            .as_deref()
            .and_then(|id| id.parse::<ProviderChildId>().ok())
        else {
            out.extend(self.gap(ChildGap::IngestionBudget));
            return out;
        };
        let Ok(task_id) = task.task_id.parse::<ProviderChildId>() else {
            out.extend(self.gap(ChildGap::IngestionBudget));
            return out;
        };
        // The native task id survives re-announcement with a different tool id.
        // Tool ids remain aliases; legacy sidechain-only CLIs keep their real tool id.
        let canonical = self
            .tasks
            .get(&task_id)
            .cloned()
            .unwrap_or_else(|| task_id.clone());
        let mut budget = self.budget;
        if (!self.tasks.contains_key(&task_id)
            && !budget.pair(task_id.as_str(), canonical.as_str()))
            || (!self.aliases.contains_key(&tool)
                && !budget.pair(tool.as_str(), canonical.as_str()))
        {
            out.extend(self.gap(ChildGap::IngestionBudget));
            return out;
        }
        if self.aliases.get(&tool).is_some_and(|id| id != &canonical) {
            out.extend(self.gap(ChildGap::Nesting));
            return out;
        }
        let descriptor = match self.started_descriptor(&canonical, &tool, task) {
            Ok(descriptor) => descriptor,
            Err(reason) => {
                out.extend(self.gap(reason));
                return out;
            }
        };
        match self.descriptor(descriptor) {
            Ok(observed) => out.push(observed),
            Err(reason) => {
                out.extend(self.gap(reason));
                return out;
            }
        }
        self.budget = budget;
        let _previous = self.tasks.insert(task_id, canonical.clone());
        let _previous = self.aliases.insert(tool, canonical.clone());
        if let Some(child) = self
            .entries
            .iter_mut()
            .find(|row| row.descriptor.id == canonical)
            && task.is_backgrounded.is_some()
        {
            child.background = task.is_backgrounded;
        }
        let prompt = if task.task_type == Some(TaskKind::LocalWorkflow) {
            task.description.as_deref()
        } else {
            task.prompt.as_deref()
        };
        if let Some(prompt) = prompt.filter(|text| !text.trim().is_empty()) {
            out.extend(self.activity(
                &canonical,
                vec![AgentEvent::UserMessageChunk {
                    content: ContentBlock::Text {
                        text: prompt.to_owned(),
                    },
                }],
            ));
        }
        out
    }
    fn started_descriptor(
        &self,
        canonical: &ProviderChildId,
        tool: &ProviderChildId,
        task: &TaskStarted,
    ) -> Result<ChildDescriptor, ChildGap> {
        let mut descriptor = self
            .entries
            .iter()
            .find(|row| &row.descriptor.id == canonical)
            .map_or_else(
                || identity(canonical.clone(), self.owners.get(tool).cloned()),
                |child| child.descriptor.clone(),
            );
        if let Some(kind) = task.subagent_type.as_deref() {
            descriptor.title = Some(
                kind.parse::<ProviderText>()
                    .map_err(|_invalid| ChildGap::IngestionBudget)?,
            );
        }
        if let Some(description) = task.description.as_deref() {
            descriptor.description = Some(
                description
                    .parse::<ProviderText>()
                    .map_err(|_invalid| ChildGap::IngestionBudget)?,
            );
        }
        descriptor.status = Some(ChildStatus::Running);
        Ok(descriptor)
    }
    pub(super) fn updated(&mut self, task: &TaskUpdated) -> Vec<AgentEvent> {
        if let Some(patch) = &task.patch {
            if let Some(id) = self.resolve_task(&task.task_id)
                && let Some(child) = self.entries.iter_mut().find(|row| row.descriptor.id == id)
                && patch.is_backgrounded.is_some()
            {
                child.background = patch.is_backgrounded;
            }
            self.status(&task.task_id, patch.status)
        } else {
            Vec::new()
        }
    }
    pub(super) fn notified(&mut self, task: &TaskNotification) -> Vec<AgentEvent> {
        let mut out = self.status(&task.task_id, task.status);
        if let Some(tokens) = task.usage.as_ref().and_then(|usage| usage.total_tokens) {
            out.extend(self.usage(&task.task_id, tokens));
        }
        out
    }
    pub(super) fn progress(&mut self, task: &TaskProgress) -> Vec<AgentEvent> {
        task.usage
            .as_ref()
            .and_then(|usage| usage.total_tokens)
            .map_or_else(Vec::new, |tokens| self.usage(&task.task_id, tokens))
    }
    fn resolve_task(&self, task: &str) -> Option<ProviderChildId> {
        let key = task.parse::<ProviderChildId>().ok()?;
        self.tasks.get(&key).cloned()
    }
    fn status(&mut self, task: &str, status: Option<TaskStatus>) -> Vec<AgentEvent> {
        let Some(status) = status.and_then(reported) else {
            return Vec::new();
        };
        let Some(id) = self.resolve_task(task) else {
            return Vec::new();
        };
        let Some(child) = self.entries.iter().find(|row| row.descriptor.id == id) else {
            return Vec::new();
        };
        if child.descriptor.status == Some(status) {
            return Vec::new();
        }
        let mut descriptor = child.descriptor.clone();
        descriptor.status = Some(status);
        match self.descriptor(descriptor) {
            Ok(event) => vec![event],
            Err(reason) => self.gap(reason),
        }
    }
    fn usage(&mut self, task: &str, tokens: u64) -> Vec<AgentEvent> {
        let Some(id) = self.resolve_task(task) else {
            return Vec::new();
        };
        let Some(child) = self.entries.iter().find(|row| row.descriptor.id == id) else {
            return Vec::new();
        };
        let Ok(subtitle) = format!("{tokens} tokens").parse::<ProviderText>() else {
            return self.gap(ChildGap::IngestionBudget);
        };
        if child.descriptor.subtitle.as_ref() == Some(&subtitle) {
            return Vec::new();
        }
        let mut descriptor = child.descriptor.clone();
        descriptor.subtitle = Some(subtitle);
        match self.descriptor(descriptor) {
            Ok(event) => vec![event],
            Err(reason) => self.gap(reason),
        }
    }
}
fn reported(status: TaskStatus) -> Option<ChildStatus> {
    match status {
        TaskStatus::Pending | TaskStatus::Running | TaskStatus::Paused => {
            Some(ChildStatus::Running)
        }
        TaskStatus::Completed => Some(ChildStatus::Completed),
        TaskStatus::Failed => Some(ChildStatus::Failed),
        TaskStatus::Killed | TaskStatus::Stopped => Some(ChildStatus::Canceled),
        TaskStatus::Unknown => None,
    }
}
