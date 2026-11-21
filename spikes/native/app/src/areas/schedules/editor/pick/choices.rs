use super::{Field, Shell};
use vessel_app_kit::ComboRow;
use vessel_app_view::{
    Picks, SelectControl,
    board::{Agent, agent_name},
    connection::ConnectionContext,
    controls_of,
    schedules::{
        cadence::IntervalUnit,
        editor::{cadence, io::Form},
    },
    with_picks,
};
use vessel_proto::{
    GetHarnessOptions, ModelId, OptionChoice, SessionMode,
    workspace::{ProjectState, SessionPlace},
};

#[derive(Debug, Clone)]
pub(super) enum Change {
    None,
    Host(ConnectionContext),
    Project(SessionPlace),
    OpenProvider(GetHarnessOptions),
    SelectModel {
        provider: GetHarnessOptions,
        model: Option<ModelId>,
    },
    Option(OptionChoice),
    Cadence(cadence::Choice),
    Unit(IntervalUnit),
}

pub(super) struct Choice {
    pub(super) row: ComboRow,
    pub(super) change: Change,
}
impl Choice {
    fn new(label: impl Into<vessel_app_kit::ui::SharedString>, change: Change) -> Self {
        Self {
            row: ComboRow::new(label),
            change,
        }
    }
}

pub(super) fn empty(shell: &Shell, field: Field) -> &'static str {
    let words = shell.words.schedules();
    match field {
        Field::Project => words.form_empty_projects(),
        Field::Model => words.form_empty_models(),
        Field::Thinking => words.form_empty_thinking(),
        Field::Mode => words.form_empty_modes(),
        Field::Host | Field::Cadence | Field::Unit => words.form_empty_choices(),
    }
}

pub(super) fn items(shell: &Shell, form: &Form, field: Field) -> Vec<Choice> {
    match field {
        Field::Host => shell
            .fleet
            .machines
            .iter()
            .filter_map(|machine| {
                let context = machine.connection_context()?;
                let mut row = Choice::new(machine.name.clone(), Change::Host(context.clone()));
                row.row.checked = form.connection() == &context;
                Some(row)
            })
            .collect(),
        Field::Project => places(form),
        Field::Model => models(shell, form),
        Field::Thinking | Field::Mode => control_for(shell, form, field)
            .into_iter()
            .flat_map(|control| {
                control
                    .values
                    .iter()
                    .map(|value| {
                        let mut row = Choice::new(
                            value.name.clone(),
                            Change::Option(control.choose(value.id.clone())),
                        );
                        row.row.checked = value.id == control.current;
                        row
                    })
                    .collect::<Vec<_>>()
            })
            .collect(),
        Field::Cadence => cadence::Choice::ALL
            .into_iter()
            .map(|choice| {
                let mut row = Choice::new(choice.label(shell.words), Change::Cadence(choice));
                row.row.checked = choice == form.draft.cadence.choice();
                row
            })
            .collect(),
        Field::Unit => form
            .draft
            .cadence
            .units()
            .iter()
            .copied()
            .map(|unit| {
                let mut row = Choice::new(unit_label(shell, unit), Change::Unit(unit));
                row.row.checked = unit == form.draft.cadence.unit();
                row
            })
            .collect(),
    }
}

fn places(form: &Form) -> Vec<Choice> {
    let Some(Ok(facts)) = form.targets() else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for project in facts
        .projects
        .iter()
        .filter(|project| matches!(project.state, ProjectState::Ready))
    {
        let place = SessionPlace::Project {
            project: project.id,
            dir: None,
        };
        let mut row = Choice::new(project.name.to_string(), Change::Project(place));
        row.row.checked = matches!(form.draft.place(), Some(SessionPlace::Project { project: id, .. }) if *id == project.id);
        rows.push(row);
        for worktree in facts
            .worktrees
            .iter()
            .filter(|tree| tree.project == project.id)
        {
            let label = worktree
                .branch
                .as_ref()
                .map_or_else(|| worktree.path.display().to_string(), ToString::to_string);
            let mut row = Choice::new(
                label,
                Change::Project(SessionPlace::Worktree {
                    worktree: worktree.id,
                }),
            );
            row.row.detail = Some(project.name.to_string().into());
            row.row.checked = matches!(form.draft.place(), Some(SessionPlace::Worktree { worktree: id }) if *id == worktree.id);
            rows.push(row);
        }
    }
    rows
}

fn models(shell: &Shell, form: &Form) -> Vec<Choice> {
    let words = shell.words.schedules();
    if let Some(provider) = &shell.areas.schedules.editor.browsing {
        return match form.options(provider) {
            Some(Ok(report)) => {
                let agent = agent_name(&provider.harness);
                let controls = controls_of(
                    &report.options,
                    &agent,
                    vessel_app_view::Lang::from_tag(shell.words.locale().tag()),
                );
                let mut rows = vec![Choice::new(
                    words.form_default_model(),
                    Change::SelectModel {
                        provider: provider.clone(),
                        model: None,
                    },
                )];
                if let Some(model) = controls.model {
                    rows.extend(model.values.into_iter().map(|value| {
                        let model = ModelId::new(value.id.as_str());
                        Choice::new(
                            value.name,
                            Change::SelectModel {
                                provider: provider.clone(),
                                model: Some(model),
                            },
                        )
                    }));
                }
                rows
            }
            Some(Err(error)) => {
                let mut row = Choice::new(
                    words.form_model_retry(),
                    Change::OpenProvider(provider.clone()),
                );
                row.row.detail =
                    Some(vessel_app_view::schedules::locale::failure(error, shell.words).into());
                vec![row]
            }
            None => {
                let mut row = Choice::new(words.form_model_loading(), Change::None);
                row.row.disabled = true;
                vec![row]
            }
        };
    }
    let Some(Ok(facts)) = form.targets() else {
        return Vec::new();
    };
    Agent::known(&facts.harnesses, &facts.agents)
        .into_iter()
        .map(|agent| {
            let profile = form
                .draft
                .spec()
                .filter(|spec| spec.harness == agent.harness)
                .and_then(|spec| spec.profile.clone());
            let available = agent.available();
            let mut row = Choice::new(
                agent.name,
                Change::OpenProvider(GetHarnessOptions {
                    harness: agent.harness,
                    profile,
                }),
            );
            row.row.opens = available;
            row.row.disabled = !row.row.opens;
            row.row.detail = agent.unavailable.map(Into::into);
            row
        })
        .collect()
}

pub(in crate::areas::schedules::editor) fn control_for(
    shell: &Shell,
    form: &Form,
    field: Field,
) -> Option<SelectControl> {
    let spec = form.draft.spec()?;
    let SessionMode::Structured { options, .. } = &spec.mode else {
        return None;
    };
    let Some(Ok(report)) = form.options(&GetHarnessOptions {
        harness: spec.harness.clone(),
        profile: spec.profile.clone(),
    }) else {
        return None;
    };
    let options = with_picks(&report.options, &Picks::from_choices(options.clone()));
    let catalog = controls_of(
        &options,
        &agent_name(&spec.harness),
        vessel_app_view::Lang::from_tag(shell.words.locale().tag()),
    );
    match field {
        Field::Thinking => catalog.effort,
        Field::Mode => catalog.permissions,
        Field::Host | Field::Project | Field::Model | Field::Cadence | Field::Unit => None,
    }
}

pub(in crate::areas::schedules::editor) fn model_label(shell: &Shell, form: &Form) -> String {
    let Some(spec) = form.draft.spec() else {
        return shell.words.schedules().form_model_placeholder().to_owned();
    };
    let provider = agent_name(&spec.harness);
    let SessionMode::Structured {
        model: Some(model), ..
    } = &spec.mode
    else {
        return provider;
    };
    let request = GetHarnessOptions {
        harness: spec.harness.clone(),
        profile: spec.profile.clone(),
    };
    let label = form
        .options(&request)
        .and_then(|reply| reply.as_ref().ok())
        .and_then(|received| {
            controls_of(
                &received.options,
                &provider,
                vessel_app_view::Lang::from_tag(shell.words.locale().tag()),
            )
            .model
        })
        .and_then(|group| {
            group
                .values
                .into_iter()
                .find(|value| value.id.as_str() == model.as_str())
        })
        .map_or_else(|| model.as_str().to_owned(), |choice| choice.name);
    format!("{provider} · {label}")
}

pub(in crate::areas::schedules::editor) fn project_label(shell: &Shell, form: &Form) -> String {
    let place = form.draft.place();
    if let (Some(place), Some(Ok(facts))) = (place, form.targets()) {
        match place {
            SessionPlace::Project { project, .. } => {
                if let Some(project) = facts.projects.iter().find(|row| row.id == *project) {
                    return project.name.to_string();
                }
            }
            SessionPlace::Worktree { worktree } => {
                if let Some(tree) = facts.worktrees.iter().find(|row| row.id == *worktree) {
                    return tree
                        .branch
                        .as_ref()
                        .map_or_else(|| tree.path.display().to_string(), ToString::to_string);
                }
            }
        }
    }
    if let Some(cwd) = form.draft.spec().and_then(|spec| spec.cwd.as_ref()) {
        return cwd.display().to_string();
    }
    shell
        .words
        .schedules()
        .form_project_placeholder()
        .to_owned()
}

pub(in crate::areas::schedules::editor) fn unit_label(
    shell: &Shell,
    unit: IntervalUnit,
) -> &'static str {
    let words = shell.words.schedules();
    match unit {
        IntervalUnit::Seconds => words.form_second(),
        IntervalUnit::Minutes => words.form_minute(),
        IntervalUnit::Hours => words.form_hour(),
        IntervalUnit::Days => words.form_day(),
    }
}
