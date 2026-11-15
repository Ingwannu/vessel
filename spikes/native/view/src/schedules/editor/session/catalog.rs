//! Browsing an agent catalog is a read, not a change to the saved target.
//! Keep one selected catalog while the existing bounded options slot browses
//! another provider. Only an offered model row commits that provider and pick.
use super::super::definition::NewTarget;
use super::change::{agent_spec, controls_for};
use super::{Model, State, Visit, io};
use vessel_proto::{
    GetHarnessOptions, HarnessOptions, HarnessRef, OptionCategory, OptionChoice, SessionSpec,
};
#[derive(Debug, Clone)]
pub(super) struct Catalog {
    pub(super) request: io::Request,
    pub(super) options: HarnessOptions,
}
impl Catalog {
    pub(super) fn matches(&self, spec: &SessionSpec) -> bool {
        matches!(self.request.operation(), io::Operation::Options(input)
            if input.harness == spec.harness && input.profile == spec.profile)
    }
}
/// Current catalog feedback; retrying retains its prior failure, not a fake empty list.
#[derive(Debug, Clone, Copy)]
pub enum CatalogStatus<'a> {
    /// No provider catalog has been requested.
    Unopened,
    /// Initial read in progress, with no prior failure.
    Loading,
    /// Failed current read, not a missing-model result.
    Failed(&'a super::Failure),
    /// Explicit retry in progress; preserve the source error and disabled retry.
    Retrying(&'a super::Failure),
    /// The exact requested catalog is ready.
    Ready,
}
impl Model {
    /// View-only feedback from the same options mailbox and retained last failure.
    #[must_use]
    pub fn catalog_status(&self) -> CatalogStatus<'_> {
        let State::Active(active) = &self.state else {
            return CatalogStatus::Unopened;
        };
        if active.options.pending() {
            active
                .catalog_failure
                .as_ref()
                .map_or(CatalogStatus::Loading, CatalogStatus::Retrying)
        } else if let Some(failure) = active.options.failure() {
            CatalogStatus::Failed(failure)
        } else if active.options.value().is_some() {
            CatalogStatus::Ready
        } else {
            CatalogStatus::Unopened
        }
    }
    /// The selected target's catalog receipt, even while another agent is browsed.
    #[must_use]
    pub fn selected_options_request(&self) -> Option<&io::Request> {
        let State::Active(active) = &self.state else {
            return None;
        };
        let catalog = active.selected_catalog.as_ref()?;
        catalog
            .matches(active.draft.target()?.spec())
            .then_some(&catalog.request)
    }
    /// Current browsed catalog receipt; callers retain it with the offered model.
    #[must_use]
    pub fn catalog_request(&self) -> Option<&io::Request> {
        let State::Active(active) = &self.state else {
            return None;
        };
        active.options.request()
    }
    /// The browsed provider comes from the exact options request, not the draft.
    #[must_use]
    pub fn catalog_harness(&self) -> Option<&HarnessRef> {
        let io::Operation::Options(input) = self.catalog_request()?.operation() else {
            return None;
        };
        Some(&input.harness)
    }
    /// Read one installed provider without replacing the selected spec/profile.
    pub fn browse_agent(&mut self, visit: Visit, harness: HarnessRef) -> Option<io::Request> {
        if self.visit() != Some(visit) || self.blocked() || self.draft()?.fields().is_none() {
            return None;
        }
        if !self.targets()?.harnesses.iter().any(|entry| {
            entry.kind == harness
                && matches!(entry.structured, vessel_proto::StructuredSupport::Available)
        }) {
            return None;
        }
        let spec = agent_spec(self.draft()?, &harness, self.project_id());
        let input = GetHarnessOptions {
            harness,
            profile: spec.profile,
        };
        let operation = io::Operation::Options(input);
        if self
            .catalog_request()
            .is_some_and(|request| request.operation() == &operation)
            && self
                .mailbox(&operation)
                .is_some_and(|slot| slot.pending() || slot.value().is_some())
        {
            return None;
        }
        let request = io::Request::on(self.connection()?.clone(), self.ticket()?, operation);
        if let State::Active(active) = &mut self.state {
            active.catalog_failure = None;
            active.options.begin(request.clone());
        }
        Some(request)
    }
    /// Dynamic browsing controls use the browsed provider, with no draft mutation.
    #[must_use]
    pub fn catalog_controls(&self, lang: crate::Lang) -> Option<crate::Controls> {
        let State::Active(active) = &self.state else {
            return None;
        };
        let Some(io::Value::Options(options)) = active.options.value() else {
            return None;
        };
        let spec = agent_spec(self.draft()?, self.catalog_harness()?, self.project_id());
        controls_for(&spec, options, lang)
    }
    /// Commit only a model actually offered by this visit's exact catalog receipt.
    /// A provider drill-down, failed lookup or retained old row cannot change it.
    pub fn choose_model(
        &mut self,
        visit: Visit,
        request: &io::Request,
        choice: OptionChoice,
    ) -> bool {
        if self.visit() != Some(visit) || self.blocked() || self.catalog_request() != Some(request)
        {
            return false;
        }
        let State::Active(active) = &self.state else {
            return false;
        };
        let Some(io::Value::Options(options)) = active.options.value() else {
            return false;
        };
        let valid = options.options.iter().any(|entry| entry.id == choice.id
            && entry.category == OptionCategory::Model
            && matches!((&entry.kind, &choice.value),
                (vessel_proto::OptionKind::Select { values, .. }, vessel_proto::ChoiceValue::Select { value })
                    if values.iter().any(|offered| offered.id == *value)));
        if !valid {
            return false;
        }
        let catalog = Catalog {
            request: request.clone(),
            options: options.clone(),
        };
        let Some(harness) = self.catalog_harness() else {
            return false;
        };
        let Some(draft) = self.draft() else {
            return false;
        };
        let finish = draft
            .target()
            .map_or_else(Default::default, NewTarget::finish);
        let Ok(target) = NewTarget::new(agent_spec(draft, harness, self.project_id()), finish)
        else {
            return false;
        };
        if !self.target(visit, target) {
            return false;
        }
        if let State::Active(active) = &mut self.state {
            active.selected_catalog = Some(catalog);
        }
        self.option(visit, choice)
    }
    /// Source model-browser-view.ts79–112: sole provider, otherwise explicit
    /// selected provider/model. This form has no browser profile rows.
    #[must_use]
    pub fn initial_browser_harness(&self) -> Option<&HarnessRef> {
        let targets = self.targets()?;
        let mut providers = targets
            .harnesses
            .iter()
            .filter(|entry| matches!(entry.structured, vessel_proto::StructuredSupport::Available));
        let first = providers.next()?;
        if providers.next().is_none() {
            return Some(&first.kind);
        }
        let target = self.draft()?.target()?;
        let explicit = match &target.spec().mode {
            vessel_proto::SessionMode::Structured { model, options, .. } => {
                model
                    .as_ref()
                    .is_some_and(|value| !value.as_str().is_empty())
                    || self.options().is_some_and(|catalog| {
                        options.iter().any(|choice| {
                            catalog.options.iter().any(|entry| {
                                entry.id == choice.id && entry.category == OptionCategory::Model
                                    && matches!(&choice.value, vessel_proto::ChoiceValue::Select { value } if !value.as_str().is_empty())
                            })
                        })
                    })
            }
            vessel_proto::SessionMode::Terminal { .. } => false,
        };
        explicit
            .then(|| {
                targets.harnesses.iter().find(|entry| {
                    entry.kind == target.spec().harness
                        && matches!(entry.structured, vessel_proto::StructuredSupport::Available)
                })
            })
            .flatten()
            .map(|entry| &entry.kind)
    }
}
