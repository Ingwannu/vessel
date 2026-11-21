//! Use the composer's remembered choices only for a fresh form. An existing
//! definition stays byte-for-byte the source for every untouched option.
use super::Shell;
use vessel_app_link::Command;
use vessel_app_view::{
    Picks,
    schedules::editor::io::{Ask, Form},
    with_picks,
};
use vessel_proto::{
    ChoiceValue, GetHarnessOptions, ModelId, OptionCategory, OptionChoice, OptionKind, SessionMode,
    StructuredSupport,
};

impl Shell {
    pub(super) fn schedule_initial_provider(&mut self) {
        let editor = &mut self.areas.schedules.editor;
        let Some(form) = &mut editor.model else {
            return;
        };
        if form.draft.opened().is_some() || form.draft.spec().is_some() {
            return;
        }
        let Some(Ok(targets)) = form.targets() else {
            return;
        };
        let available = || {
            targets
                .harnesses
                .iter()
                .filter(|harness| matches!(harness.structured, StructuredSupport::Available))
        };
        let harness = available()
            .find(|harness| self.prefs.agent.as_ref() == Some(&harness.kind))
            .or_else(|| available().next())
            .map(|harness| harness.kind.clone());
        if let Some(harness) = harness {
            form.draft.select_model(harness.clone(), None);
            editor.restore_prefs = true;
            self.schedule_form_issue(Ask::Options(GetHarnessOptions {
                harness,
                profile: None,
            }));
        }
    }
}

pub(super) fn restore(
    form: &mut Form,
    request: &GetHarnessOptions,
    prefs: &vessel_app_link::Prefs,
) {
    let Some(choices) = prefs.choices(form.connection().machine().as_str(), &request.harness)
    else {
        return;
    };
    let Some(Ok(options)) = form.options(request) else {
        return;
    };
    let accepted = choices
        .iter()
        .filter_map(|choice| {
            let ChoiceValue::Select { value } = &choice.value else {
                return None;
            };
            options.options.iter().find_map(|option| {
                if option.id != choice.id || !remembered(option.category) {
                    return None;
                }
                let OptionKind::Select { values, .. } = &option.kind else {
                    return None;
                };
                values
                    .iter()
                    .any(|item| &item.id == value)
                    .then_some((option.category, choice.clone()))
            })
        })
        .collect::<Vec<_>>();
    if !form
        .draft
        .spec()
        .is_some_and(|spec| spec.harness == request.harness)
    {
        return;
    }
    for (category, choice) in accepted {
        match (category, &choice.value) {
            (OptionCategory::Model, ChoiceValue::Select { value }) => {
                form.draft
                    .select_model(request.harness.clone(), Some(ModelId::new(value.as_str())));
            }
            _ => form.draft.set_option(choice),
        }
    }
}

pub(super) fn keep(shell: &mut Shell) {
    let Some(form) = &shell.areas.schedules.editor.model else {
        return;
    };
    let Some(spec) = form.draft.spec() else {
        return;
    };
    let SessionMode::Structured { model, options, .. } = &spec.mode else {
        return;
    };
    let ask = GetHarnessOptions {
        harness: spec.harness.clone(),
        profile: spec.profile.clone(),
    };
    let Some(Ok(report)) = form.options(&ask) else {
        return;
    };
    let kept = with_picks(&report.options, &Picks::from_choices(options.clone()));
    let choices = kept
        .into_iter()
        .filter_map(|option| {
            if !remembered(option.category) {
                return None;
            }
            let OptionKind::Select { current, values } = option.kind else {
                return None;
            };
            let value = if option.category == OptionCategory::Model {
                model
                    .as_ref()
                    .and_then(|model| {
                        values
                            .iter()
                            .find(|value| value.id.as_str() == model.as_str())
                            .map(|value| value.id.clone())
                    })
                    .unwrap_or(current)
            } else {
                current
            };
            Some(OptionChoice {
                id: option.id,
                value: ChoiceValue::Select { value },
            })
        })
        .collect::<Vec<_>>();
    shell.prefs.agent = Some(spec.harness.clone());
    shell.prefs.keep_choices(
        form.connection().machine().as_str(),
        &spec.harness,
        &choices,
    );
    shell.command(Command::SavePrefs(Box::new(shell.prefs.clone())));
}

const fn remembered(category: OptionCategory) -> bool {
    matches!(
        category,
        OptionCategory::Model | OptionCategory::Mode | OptionCategory::ThoughtLevel
    )
}
