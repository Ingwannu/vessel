//! Pure transitions for the host card and selection/confirmation dialogs.
use super::{Action, Answer, Load, Output, Picker, Problem, Scope, Task};
use crate::devices::Ticket;
use vessel_proto::mcp::{Injection, McpSettings};
use vessel_proto::skills::{
    SaveSkillsSelection, SkillNames, SkillOperation, SkillSelection, SkillsSnapshot, SkillsState,
};

/// Human confirmation freezes what this dialog describes; the node rechecks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confirmation {
    /// Whole skill folders, including user additions.
    Uninstall(SkillNames),
    /// Non-destructive maintenance operations, add before update.
    Update(Vec<SkillOperation>),
    /// Fresh deletion names returned by the node for this exact draft.
    Removals {
        /// The draft whose fresh plan was returned.
        selection: SkillSelection,
        /// Exact names requiring explicit consent.
        names: SkillNames,
    },
}
/// One fixed host visit. A new visit creates a new page and discards old dialogs.
#[derive(Debug)]
pub struct Page {
    scope: Scope,
    /// Last confirmed tools value and outstanding task.
    pub tools: Load<McpSettings>,
    /// Last confirmed skill snapshot and outstanding task.
    pub skills: Load<SkillsSnapshot>,
    /// Local draft seeded on open.
    pub picker: Option<Picker>,
    /// Explicit confirmation, never an automatic mutation.
    pub confirmation: Option<Confirmation>,
}
impl Page {
    /// Enter without carrying an earlier visit's pending results or drafts.
    #[must_use]
    pub const fn new(scope: Scope) -> Self {
        Self {
            scope,
            tools: Load::Empty,
            skills: Load::Empty,
            picker: None,
            confirmation: None,
        }
    }
    /// Correlation used by the owner task API.
    #[must_use]
    pub const fn scope(&self) -> &Scope {
        &self.scope
    }
    fn task(&self, action: Action, ticket: Ticket) -> Task {
        Task {
            scope: self.scope.clone(),
            ticket,
            action,
        }
    }
    /// Focus refresh reads tools; it never writes a default.
    #[must_use]
    pub fn read_tools(&mut self, ticket: Ticket) -> Option<Task> {
        if self.tools.pending() {
            return None;
        }
        let task = self.task(Action::ReadTools, ticket);
        self.tools.begin(task.clone());
        Some(task)
    }
    /// Focus refresh preserves an already-open picker draft.
    #[must_use]
    pub fn read_skills(&mut self, ticket: Ticket) -> Option<Task> {
        if self.skills.pending() {
            return None;
        }
        let task = self.task(Action::ReadSkills, ticket);
        self.skills.begin(task.clone());
        Some(task)
    }
    /// Change only the injection switch after its current value was read.
    #[must_use]
    pub fn toggle_tools(&mut self, ticket: Ticket) -> Option<Task> {
        if self.tools.writes_blocked() {
            return None;
        }
        let current = self.tools.value()?;
        let next = match current.inject {
            Injection::Off => Injection::On,
            Injection::On => Injection::Off,
        };
        let task = self.task(Action::Inject(next), ticket);
        self.tools.begin(task.clone());
        Some(task)
    }
    /// An installed switch opens confirmation; an uninstalled switch installs.
    #[must_use]
    pub fn toggle_skills(&mut self, ticket: Ticket) -> Option<Task> {
        if self.skills.writes_blocked() || self.picker.is_some() {
            return None;
        }
        let status = self.skills.value()?;
        if status.state != SkillsState::NotInstalled {
            self.confirmation = Some(Confirmation::Uninstall(status.installed.clone()));
            return None;
        }
        if !has_selection(status) {
            return None;
        }
        self.begin_skills(Action::Reconcile, ticket)
    }
    /// Offer only add/update operations, never unselected deletions.
    pub fn ask_update(&mut self) {
        if self.skills.writes_blocked() || self.picker.is_some() {
            return;
        }
        if let Some(status) = self.skills.value() {
            let ops = maintenance(status);
            if status.state == SkillsState::Drift && !ops.is_empty() {
                self.confirmation = Some(Confirmation::Update(ops));
            }
        }
    }
    /// Open once from confirmed state; subsequent status reads do not reseed it.
    pub fn choose(&mut self) {
        if self.skills.pending() || self.picker.is_some() {
            return;
        }
        if let Some(status) = self.skills.value() {
            self.picker = Some(Picker::open(
                status.selection.clone(),
                status.available.clone(),
            ));
        }
    }
    /// Dismiss without writing, except while its save still lacks an answer.
    pub fn close_picker(&mut self) {
        if !self.skills.pending() {
            self.picker = None;
            self.confirmation = None;
        }
    }
    /// Cancel a confirmation without losing the underlying selection draft.
    pub fn cancel_confirmation(&mut self) {
        self.confirmation = None;
    }
    /// First save never preconfirms deletions from a stale UI status.
    #[must_use]
    pub fn save_picker(&mut self, ticket: Ticket) -> Option<Task> {
        if self.skills.writes_blocked() || self.confirmation.is_some() {
            return None;
        }
        let picker = self.picker.as_mut()?;
        picker.confirmed_once = false;
        let request = SaveSkillsSelection {
            selection: picker.draft.clone(),
            confirmed_removals: SkillNames::default(),
        };
        self.begin_skills(Action::Save(request), ticket)
    }
    /// Confirm only the action actually shown, on this fixed visit's target.
    #[must_use]
    pub fn confirm(&mut self, ticket: Ticket) -> Option<Task> {
        if self.skills.writes_blocked() {
            return None;
        }
        let action = match self.confirmation.take()? {
            Confirmation::Uninstall(_) => Action::Uninstall,
            Confirmation::Update(_) => Action::Reconcile,
            Confirmation::Removals { selection, names } => {
                self.picker.as_mut()?.confirmed_once = true;
                Action::Save(SaveSkillsSelection {
                    selection,
                    confirmed_removals: names,
                })
            }
        };
        self.begin_skills(action, ticket)
    }
    fn begin_skills(&mut self, action: Action, ticket: Ticket) -> Option<Task> {
        if self.skills.pending() {
            return None;
        }
        let task = self.task(action, ticket);
        self.skills.begin(task.clone());
        Some(task)
    }
    /// Ignore other targets, visits, consumers and superseded attempts.
    #[must_use]
    pub fn answered(&mut self, answer: Answer) -> bool {
        if answer.task.scope != self.scope {
            return false;
        }
        match &answer.task.action {
            Action::ReadTools | Action::Inject(_) => {
                if !self.tools.accepts(&answer.task) {
                    return false;
                }
                self.tools.finish(match answer.result {
                    Ok(Output::Tools(settings)) => Ok(settings),
                    Ok(Output::Skills(_) | Output::Saved(_)) => Err(Problem::InvalidReply),
                    Err(problem) => Err(problem),
                });
            }
            Action::ReadSkills | Action::Reconcile | Action::Uninstall => {
                if !self.skills.accepts(&answer.task) {
                    return false;
                }
                self.skills.finish(match answer.result {
                    Ok(Output::Skills(snapshot)) => Ok(snapshot),
                    Ok(Output::Tools(_) | Output::Saved(_)) => Err(Problem::InvalidReply),
                    Err(problem) => Err(problem),
                });
            }
            Action::Save(_) => {
                if !self.skills.accepts(&answer.task) {
                    return false;
                }
                self.saved(answer.result);
            }
        }
        true
    }
    fn saved(&mut self, result: Result<Output, Problem>) {
        match result {
            Ok(Output::Saved(saved)) => {
                self.skills.finish(Ok(saved.snapshot));
                if let Some(names) = saved.confirmation_required {
                    if let Some(picker) = &self.picker {
                        if picker.confirmed_once {
                            self.skills.finish(Err(Problem::Changed));
                        } else {
                            self.confirmation = Some(Confirmation::Removals {
                                selection: picker.draft.clone(),
                                names,
                            });
                        }
                    }
                } else {
                    self.picker = None;
                    self.confirmation = None;
                }
            }
            Ok(Output::Skills(_) | Output::Tools(_)) => {
                self.skills.finish(Err(Problem::InvalidReply));
            }
            Err(problem) => self.skills.finish(Err(problem)),
        }
    }
}
/// Empty custom selection disables installation, but never disables the picker.
#[must_use]
pub fn has_selection(status: &SkillsSnapshot) -> bool {
    match &status.selection {
        SkillSelection::All => !status.available.is_empty(),
        SkillSelection::Custom { skills } => skills
            .as_slice()
            .iter()
            .any(|name| status.available.contains(name)),
    }
}
/// Maintenance excludes deletions and has deterministic presentation order.
#[must_use]
pub fn maintenance(status: &SkillsSnapshot) -> Vec<SkillOperation> {
    let mut ops: Vec<_> = status
        .ops
        .iter()
        .filter(|op| !matches!(op, SkillOperation::Delete { .. }))
        .cloned()
        .collect();
    ops.sort_by(|left, right| {
        let kind = |op: &SkillOperation| match op {
            SkillOperation::Add { .. } => 0,
            SkillOperation::Update { .. } => 1,
            SkillOperation::Delete { .. } => 2,
        };
        kind(left)
            .cmp(&kind(right))
            .then_with(|| left.skill().cmp(right.skill()))
    });
    ops
}
