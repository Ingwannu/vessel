//! Input transitions are checked against the original form visit before parse.
use super::*;
use crate::schedules::editor::{cadence, definition::NewTarget};
use vessel_proto::workspace::SessionPlace;
use vessel_proto::{HarnessRef, SessionMode, SessionSpec};

/// Only the new-chat fields the reference allows; recurring prompts expose none.
#[derive(Debug)]
pub enum FieldChange {
    /// Optional display name; blank clears.
    Name(String),
    /// Prompt text, parsed once at this edge.
    Prompt(String),
    /// Optional whole count; invalid nonblank remains invalid.
    Limit(String),
}
impl Model {
    /// Apply only this visit, before any save/uncertainty blocks further edits.
    pub fn field(&mut self, expected: Visit, change: FieldChange) -> bool {
        if self.visit() != Some(expected) || self.blocked() {
            return false;
        }
        let Some(fields) = self.draft_mut().and_then(Draft::fields_mut) else {
            return false;
        };
        match change {
            FieldChange::Name(text) => fields.set_name(text),
            FieldChange::Prompt(text) => fields.set_prompt(text),
            FieldChange::Limit(text) => fields.set_limit(text),
        }
        true
    }
    /// Every cadence/zone edit retires its old projection before changing input.
    pub fn cadence(&mut self, expected: Visit, edit: impl FnOnce(&mut cadence::Draft)) -> bool {
        if self.visit() != Some(expected) || self.blocked() {
            return false;
        }
        if let State::Active(active) = &mut self.state {
            active.preview.invalidate();
        }
        let Some(draft) = self.draft_mut() else {
            return false;
        };
        edit(&mut draft.cadence);
        true
    }
    /// Target changes preserve the full spec supplied by the existing form,
    /// rather than reconstructing it from a few visible selectors.
    pub(super) fn target(&mut self, expected: Visit, target: NewTarget) -> bool {
        if self.visit() != Some(expected) || self.blocked() {
            return false;
        }
        let Some(current) = self.draft_mut().and_then(Draft::target_mut) else {
            return false;
        };
        *current = Some(target);
        true
    }
    /// Use a node-listed project root; no viewer-side path join or cwd inference.
    pub fn project(&mut self, expected: Visit, project: vessel_proto::ProjectId) -> bool {
        if self.visit() != Some(expected) || self.blocked() {
            return false;
        }
        if self.draft().is_none_or(|draft| draft.fields().is_none()) {
            return false;
        }
        let Some(targets) = self.targets() else {
            return false;
        };
        if !targets.projects.iter().any(|listed| {
            listed.id == project
                && matches!(listed.state, vessel_proto::workspace::ProjectState::Ready)
        }) {
            return false;
        }
        let target = self.draft().and_then(Draft::target).cloned();
        if let State::Active(active) = &mut self.state {
            active.project = Some(project);
        }
        if let Some(target) = target {
            let mut spec = target.spec().clone();
            spec.cwd = None;
            spec.place = Some(Box::new(SessionPlace::Project { project, dir: None }));
            let Ok(target) = NewTarget::new(spec, target.finish()) else {
                return false;
            };
            return self.target(expected, target);
        }
        true
    }
    /// A chosen installed/available agent can offer its dynamic options. No
    /// guessed options/models are inserted for a missing installation.
    pub fn agent(&mut self, expected: Visit, harness: HarnessRef) -> Option<io::Request> {
        if self.visit() != Some(expected) || self.blocked() {
            return None;
        }
        let targets = self.targets()?;
        if !targets.harnesses.iter().any(|installed| {
            installed.kind == harness
                && matches!(
                    installed.structured,
                    vessel_proto::StructuredSupport::Available
                )
        }) {
            return None;
        }
        let draft = self.draft()?;
        if draft.opened().is_some_and(|opened| {
            matches!(
                opened.definition.target,
                vessel_proto::automation::ScheduleTarget::Session { .. }
                    | vessel_proto::automation::ScheduleTarget::Unknown
            )
        }) {
            return None;
        }
        let finish = draft.target().map_or(
            super::super::definition::Finish::default(),
            NewTarget::finish,
        );
        let spec = agent_spec(draft, &harness, self.project_id());
        let target = NewTarget::new(spec.clone(), finish).ok()?;
        if !self.target(expected, target) {
            return None;
        }
        let connection = self.connection()?.clone();
        let request = io::Request::on(
            connection,
            self.ticket()?,
            io::Operation::Options(vessel_proto::GetHarnessOptions {
                harness,
                profile: spec.profile,
            }),
        );
        if let State::Active(active) = &mut self.state {
            active.options.begin(request.clone());
        }
        Some(request)
    }
}

impl Model {
    /// The project's typed id, available before choosing an agent/model.
    #[must_use]
    pub fn project_id(&self) -> Option<vessel_proto::ProjectId> {
        match &self.state {
            State::Active(active) => active.project,
            State::Closed | State::ChoosingHost { .. } => None,
        }
    }
    /// Render the existing option-control rules with the form's explicit picks.
    #[must_use]
    pub fn controls(&self, lang: crate::Lang) -> Option<crate::Controls> {
        let target = self.draft()?.target()?;
        let options = self.options()?;
        let mut picks = match &target.spec().mode {
            SessionMode::Structured { options, .. } => crate::Picks::from_choices(options.clone()),
            SessionMode::Terminal { .. } => return None,
        };
        if let SessionMode::Structured {
            model: Some(model), ..
        } = &target.spec().mode
        {
            for option in &options.options {
                if option.category != vessel_proto::OptionCategory::Model {
                    continue;
                }
                if let vessel_proto::OptionKind::Select { values, .. } = &option.kind
                    && let Some(value) = values
                        .iter()
                        .find(|value| value.id.as_str() == model.as_str())
                {
                    picks.pick(vessel_proto::OptionChoice {
                        id: option.id.clone(),
                        value: vessel_proto::ChoiceValue::Select {
                            value: value.id.clone(),
                        },
                    });
                }
            }
        }
        let offered = crate::with_picks(&options.options, &picks);
        let mut controls =
            crate::controls_of(&offered, &crate::agent_name(&target.spec().harness), lang);
        if let SessionMode::Structured {
            model: Some(model), ..
        } = &target.spec().mode
            && let Some(control) = &mut controls.model
            && !control
                .values
                .iter()
                .any(|value| value.id.as_str() == model.as_str())
        {
            // Keep an unavailable opened selection visible as unavailable;
            // the node's current default is not what this stored spec asks for.
            control.current = vessel_proto::OptionValueId::new(model.as_str());
        }
        Some(controls)
    }
    /// Apply only a currently offered typed option, preserving every unrelated
    /// stored spec field and choice. No raw option/model id is reparsed.
    pub fn option(&mut self, expected: Visit, choice: vessel_proto::OptionChoice) -> bool {
        if self.visit() != Some(expected) || self.blocked() {
            return false;
        }
        let Some(options) = self.options() else {
            return false;
        };
        let Some(option) = options.options.iter().find(|option| option.id == choice.id) else {
            return false;
        };
        let valid = match (&option.kind, &choice.value) {
            (
                vessel_proto::OptionKind::Select { values, .. },
                vessel_proto::ChoiceValue::Select { value },
            ) => values.iter().any(|offered| offered.id == *value),
            (vessel_proto::OptionKind::Toggle { .. }, vessel_proto::ChoiceValue::Toggle { .. }) => {
                true
            }
            (vessel_proto::OptionKind::Select { .. }, vessel_proto::ChoiceValue::Toggle { .. })
            | (vessel_proto::OptionKind::Toggle { .. }, vessel_proto::ChoiceValue::Select { .. })
            | (
                vessel_proto::OptionKind::Unknown,
                vessel_proto::ChoiceValue::Select { .. } | vessel_proto::ChoiceValue::Toggle { .. },
            ) => false,
        };
        if !valid {
            return false;
        }
        let model_choice = option.category == vessel_proto::OptionCategory::Model;
        let Some(target) = self.draft().and_then(Draft::target).cloned() else {
            return false;
        };
        let mut spec = target.spec().clone();
        let SessionMode::Structured { model, options, .. } = &mut spec.mode else {
            return false;
        };
        if model_choice {
            *model = None;
        }
        options.retain(|old| old.id != choice.id);
        options.push(choice);
        let Ok(target) = NewTarget::new(spec, target.finish()) else {
            return false;
        };
        self.target(expected, target)
    }
}

fn agent_spec(
    draft: &Draft,
    harness: &HarnessRef,
    project: Option<vessel_proto::ProjectId>,
) -> SessionSpec {
    let mut spec = draft.target().map_or_else(
        || SessionSpec::new(harness.clone(), structured()),
        |target| target.spec().clone(),
    );
    if spec.harness != *harness {
        spec.harness = harness.clone();
        // Only an explicit agent change owns this reset: another agent cannot
        // receive the old model/config/profile ids. Other metadata survives.
        spec.profile = None;
        spec.mode = structured();
    }
    if spec.place.is_none()
        && let Some(project) = project
    {
        // Existing worktree/subfolder remains unless a project was explicitly
        // selected. A fresh draft gets no invented node current directory.
        spec.cwd = None;
        spec.place = Some(Box::new(SessionPlace::Project { project, dir: None }));
    }
    spec
}
fn structured() -> SessionMode {
    SessionMode::Structured {
        model: None,
        resume: None,
        options: Vec::new(),
    }
}
