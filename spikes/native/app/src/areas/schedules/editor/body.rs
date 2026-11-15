//! Source Name/Prompt/target/cadence/Max runs field order.
use super::draw::{Selection, framed, select, shared};
use super::{AnyElement, Context, Picker, Shell, Tokens, kit};
use kit::{
    Type, TypeExt as _,
    theme::spacing,
    ui::{
        IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
    },
};
use std::sync::Arc;
use vessel_app_view::schedules::editor::{
    cadence::Choice, fields::Field, presentation::OperationProblem,
};
pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let area = &shell.areas.schedules;
    let form = match &area.data.editor_cache {
        Ok(Some(form)) => form,
        Ok(None) => return div().into_any_element(),
        Err(notice) => return super::super::draw::row_notice(notice, tokens).into_any_element(),
    };
    let Some(inputs) = &area.ui.editor.inputs else {
        return div().into_any_element();
    };
    let mut body = div().flex().flex_col().gap(px(spacing::S6));
    if let Some(notice) = &form.local {
        body = body.child(super::super::draw::row_notice(notice, tokens));
    }
    if let Some(line) = &form.read_only {
        body = body.child(super::super::draw::row_notice(
            &vessel_app_view::schedules::presentation::Notice {
                label: Arc::clone(line),
                variant: vessel_app_view::alert::Variant::Info,
            },
            tokens,
        ));
    }
    if let Some(target) = &form.target {
        body = body.child(framed(
            shell.words.schedules().form_target_label().into(),
            kit::select::readonly_value(
                shared(target),
                kit::ControlSizing::from_compact(touch),
                tokens,
            )
            .into_any_element(),
            None,
            tokens,
        ));
    }
    for field in form.text.iter().filter(|field| field.field != Field::Limit) {
        let engine = match field.field {
            Field::Name => &inputs.name,
            Field::Prompt => &inputs.prompt,
            Field::Limit => &inputs.limit,
        };
        body = body.child(framed(
            shared(&field.label),
            engine.element(tokens, touch, cx),
            field.error.as_ref().map(shared),
            tokens,
        ));
    }
    if !form.text.is_empty() {
        let model = &area.data.editor;
        if model.draft().is_some_and(|draft| draft.opened().is_some())
            || shell.fleet.machines.len() > 1
            || model.connection().is_none()
        {
            let value = model.connection().map_or_else(
                || {
                    kit::select::FormValue::Placeholder(
                        shell.words.schedules().form_host_placeholder().into(),
                    )
                },
                |connection| {
                    kit::select::FormValue::Selected(
                        connection.machine().as_str().to_owned().into(),
                    )
                },
            );
            body = body.child(select(
                shell,
                Selection::new(Picker::Host, value)
                    .unavailable(model.draft().is_some_and(|draft| draft.opened().is_some())),
                tokens,
                touch,
                cx,
            ));
        }
        if model.connection().is_some() {
            let project = model.project_id().and_then(|id| {
                model
                    .targets()?
                    .projects
                    .iter()
                    .find(|project| project.id == id)
            });
            let value = project.map_or_else(
                || {
                    kit::select::FormValue::Placeholder(
                        shell.words.schedules().form_project_placeholder().into(),
                    )
                },
                |project| kit::select::FormValue::Selected(project.name.as_str().to_owned().into()),
            );
            let leading = project.map(|project| {
                kit::picker::initial_tile(kit::picker::initial_of(project.name.as_str()), tokens)
                    .into_any_element()
            });
            body = body.child(select(
                shell,
                Selection::new(Picker::Project, value).leading(leading),
                tokens,
                touch,
                cx,
            ));
            if model.project_id().is_some()
                || model.draft().is_some_and(|draft| draft.opened().is_some())
            {
                let controls = form.controls.as_ref();
                let model_name = controls
                    .and_then(|controls| controls.model.as_ref())
                    .map(|control| control.current_name().to_owned());
                let value = model_name.map_or_else(
                    || {
                        kit::select::FormValue::Placeholder(
                            shell.words.schedules().form_model_placeholder().into(),
                        )
                    },
                    |name| kit::select::FormValue::Selected(name.into()),
                );
                let agent = model
                    .draft()
                    .and_then(|draft| draft.target())
                    .map(|target| &target.spec().harness);
                let leading = agent.map(|agent| {
                    crate::agent_mark::mark(
                        Some(agent),
                        kit::agent_mark::Size::Sixteen,
                        kit::agent_mark::Space::Row,
                        tokens,
                    )
                    .into_any_element()
                });
                body = body.child(select(
                    shell,
                    Selection::new(Picker::Model, value).leading(leading),
                    tokens,
                    touch,
                    cx,
                ));
                if let Some(controls) = controls {
                    for (picker, control) in [
                        (Picker::Thinking, controls.effort.as_ref()),
                        (Picker::Mode, controls.permissions.as_ref()),
                    ] {
                        if let Some(control) = control {
                            body = body.child(select(
                                shell,
                                Selection::new(
                                    picker,
                                    kit::select::FormValue::Selected(
                                        control.current_name().to_owned().into(),
                                    ),
                                ),
                                tokens,
                                touch,
                                cx,
                            ));
                        }
                    }
                }
                if let Some(target) = model.draft().and_then(|draft| draft.target()) {
                    let on = target.finish()
                        == vessel_app_view::schedules::editor::definition::Finish::Archive;
                    let visit = form.visit;
                    let toggle = kit::settings::switch(
                        "schedule-archive",
                        on,
                        !form.blocked,
                        touch,
                        0,
                        tokens,
                    );
                    let toggle = if form.blocked {
                        toggle
                    } else {
                        toggle.on_click(cx.listener(move |shell, _, _, cx| {
                            let model = &mut shell.areas.schedules.data.editor;
                            let finish = if on {
                                vessel_app_view::schedules::editor::definition::Finish::Keep
                            } else {
                                vessel_app_view::schedules::editor::definition::Finish::Archive
                            };
                            if model.finish(visit, finish) {
                                let _changed = shell.schedules_changed(cx);
                                cx.notify();
                            }
                        }))
                    };
                    body = body.child(framed(
                        shell.words.schedules().form_archive().into(),
                        toggle.into_any_element(),
                        None,
                        tokens,
                    ));
                }
            }
        }
    }
    let model = &area.data.editor;
    let Some(draft) = model.draft() else {
        return body.into_any_element();
    };
    let cadence_label = match draft.cadence.choice() {
        Choice::Preset(preset) => preset.label(shell.words),
        Choice::Interval => shell.words.schedules().form_interval(),
        Choice::Custom => shell.words.schedules().form_custom(),
    };
    let mut cadence = div().flex().flex_col().gap(px(spacing::S3)).child(select(
        shell,
        Selection::new(
            Picker::Cadence,
            kit::select::FormValue::Selected(cadence_label.into()),
        ),
        tokens,
        touch,
        cx,
    ));
    if let Some(unit) = draft.cadence.unit() {
        cadence = cadence.child(
            div()
                .flex()
                .gap(px(spacing::S3))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(inputs.rule.element(tokens, touch, cx)),
                )
                .child(select(
                    shell,
                    Selection::new(
                        Picker::Unit,
                        kit::select::FormValue::Selected(shell.t.lang.interval_unit(unit).into()),
                    ),
                    tokens,
                    touch,
                    cx,
                )),
        );
    } else {
        cadence = cadence.child(inputs.rule.element(tokens, touch, cx));
    }
    cadence = cadence.child(select(
        shell,
        Selection::new(
            Picker::Zone,
            kit::select::FormValue::Selected(draft.cadence.zone().as_str().to_owned().into()),
        ),
        tokens,
        touch,
        cx,
    ));
    let preview_error = form.problems.iter().find_map(|problem| match problem {
        OperationProblem::Preview(notice) => Some(shared(&notice.label)),
        OperationProblem::Targets(_) | OperationProblem::Options(_) | OperationProblem::Save(_) => {
            None
        }
    });
    let error = form.cadence_error.as_ref().map(shared).or(preview_error);
    let cadence_frame = framed(
        shell.words.schedules().form_cadence().into(),
        cadence.into_any_element(),
        error,
        tokens,
    );
    body = body.child(cadence_frame);
    if let Some(description) = &form.cadence {
        body = body.child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .whitespace_normal()
                .child(shared(description)),
        );
    }
    if !form.viewer_known && !form.next_runs.is_empty() {
        body = body.child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .whitespace_normal()
                .child(shell.words.schedules().form_viewer_display_utc()),
        );
    }
    if !form.next_runs.is_empty() {
        let mut runs = div()
            .flex()
            .flex_col()
            .gap(px(spacing::S1))
            .type_(Type::Meta)
            .text_color(tokens.neutral.foreground_muted)
            .child(shell.words.schedules().form_next_runs());
        for at in &form.next_runs {
            runs = runs.child(div().whitespace_normal().child(shared(at)));
        }
        body = body.child(runs);
    }
    if let Some(field) = form.text.iter().find(|field| field.field == Field::Limit) {
        body = body.child(framed(
            shared(&field.label),
            inputs.limit.element(tokens, touch, cx),
            field.error.as_ref().map(shared),
            tokens,
        ));
    }
    for problem in &form.problems {
        let notice = match problem {
            OperationProblem::Targets(notice)
            | OperationProblem::Options(notice)
            | OperationProblem::Save(notice) => Some(notice),
            OperationProblem::Preview(_) => None,
        };
        if let Some(notice) = notice {
            body = body.child(super::super::draw::row_notice(notice, tokens));
        }
    }
    body.into_any_element()
}
