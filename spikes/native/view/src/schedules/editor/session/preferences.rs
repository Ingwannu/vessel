//! Reuse the canonical native preferences; this model owns no preference store.
use super::change::picks_for;
use super::{Model, Visit};
use vessel_proto::{
    ChoiceValue, HarnessRef, OptionCategory, OptionChoice, OptionKind, SessionMode,
};
const REMEMBERED_CATEGORIES: [OptionCategory; 3] = [
    OptionCategory::Model,
    OptionCategory::ThoughtLevel,
    OptionCategory::Mode,
];
fn named_option(
    options: &[vessel_proto::SessionOption],
    category: OptionCategory,
) -> Option<&vessel_proto::SessionOption> {
    options.iter().find(|option| {
        option.category == category && matches!(&option.kind, OptionKind::Select { .. })
    })
}
impl Model {
    /// Prefer the remembered available provider for a fresh project, else the first available one.
    /// Never replace an existing selection or an opened definition.
    #[must_use]
    pub fn initial_agent(&self, preferred: Option<&HarnessRef>) -> Option<&HarnessRef> {
        let draft = self.draft()?;
        if draft.opened().is_some() || draft.target().is_some() {
            return None;
        }
        let targets = self.targets()?;
        let available = |harness: &vessel_proto::Harness| {
            matches!(
                harness.structured,
                vessel_proto::StructuredSupport::Available
            )
        };
        preferred
            .and_then(|preferred| {
                targets
                    .harnesses
                    .iter()
                    .find(|harness| available(harness) && harness.kind == *preferred)
            })
            .or_else(|| targets.harnesses.iter().find(|harness| available(harness)))
            .map(|harness| &harness.kind)
    }
    /// Typed current model/thinking/mode values, including an explicitly offered default.
    /// Unavailable values and unrelated spec options never become remembered choices.
    #[must_use]
    pub fn remembered_choices(&self) -> Option<Vec<OptionChoice>> {
        let spec = self.draft()?.target()?.spec();
        let options = self.options()?;
        let picks = picks_for(spec, options)?;
        let offered = crate::with_picks(&options.options, &picks);
        let named = REMEMBERED_CATEGORIES.map(|category| named_option(&offered, category));
        Some(
            offered
                .iter()
                .filter_map(|option| {
                    if !named.iter().flatten().any(|entry| entry.id == option.id) {
                        return None;
                    }
                    let OptionKind::Select { current, values } = &option.kind else {
                        return None;
                    };
                    values
                        .iter()
                        .any(|value| value.id == *current)
                        .then_some(OptionChoice {
                            id: option.id.clone(),
                            value: ChoiceValue::Select {
                                value: current.clone(),
                            },
                        })
                })
                .collect(),
        )
    }
    /// Seed only a fresh unedited selection from currently offered preferences.
    /// Existing edits, explicit choices, old visits and blocked forms stay unchanged.
    pub fn restore_choices(&mut self, visit: Visit, choices: &[OptionChoice]) -> bool {
        if self.visit() != Some(visit) || self.blocked() {
            return false;
        }
        let Some(draft) = self.draft() else {
            return false;
        };
        if draft.opened().is_some() {
            return false;
        }
        let Some(target) = draft.target() else {
            return false;
        };
        if !matches!(&target.spec().mode, SessionMode::Structured { model: None, options, .. } if options.is_empty())
        {
            return false;
        }
        let mut changed = false;
        for choice in choices {
            let relevant = self.options().is_some_and(|options| {
                REMEMBERED_CATEGORIES.iter().any(|category| {
                    named_option(&options.options, *category)
                        .is_some_and(|named| named.id == choice.id)
                })
            });
            if relevant {
                changed = self.option(visit, choice.clone()) || changed;
            }
        }
        changed
    }
}
