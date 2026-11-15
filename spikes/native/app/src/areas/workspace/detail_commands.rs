//! Exact command-set inspection. No confirmation resolves a later selection.
use super::approval::{self, Actions, Panel};
use super::detail::Detail;
use super::widgets::{Tone, badge, banner, empty, row, rows, title};
use crate::shell::Shell;
use vessel_app_kit::ui::{
    Context, Div, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, button_with_kind};
use vessel_app_view::workspace::command_changes::{Listed, compare_approved};
use vessel_app_view::workspace::git::Reading;
use vessel_proto::workspace::{ProjectCommands, ProjectInfo};

pub(super) fn render(
    shell: &Shell,
    machine: &str,
    project: &ProjectInfo,
    detail: &Detail,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words();
    let labels = words.labels();
    let model = &shell.areas.workspace.command_access;
    let owner = model.target.as_ref().filter(|target| {
        target.project == project.id && target.context.machine().as_str() == machine
    });
    let current = owner.is_some_and(|target| shell.command_owner_is_current(target));
    let refresh_machine = machine.to_owned();
    let id = project.id;
    let refresh = button_with_kind(
        "commands-refresh",
        shell.words.scripts().refresh(),
        ButtonKind::Secondary,
        if current && (model.reading.outstanding().is_some() || model.writing.is_some()) {
            ButtonState::Disabled
        } else {
            ButtonState::Enabled
        },
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, window, cx| {
        shell.refresh_command_card(&refresh_machine, id, window, cx)
    }));
    let mut column = div().flex().flex_col().gap(px(10.0)).child(refresh);
    if model.exhausted {
        return column.child(empty(shell.words.scripts().exhausted(), tokens));
    }
    if !current {
        return column.child(banner(
            shell.words.approval().connection_changed(),
            true,
            None,
            tokens,
        ));
    }
    if let Some(problem) = &model.failure {
        column = column.child(banner(words.problem(problem), true, None, tokens));
    }
    let commands = match &model.reading {
        Reading::Failed(problem) => return column.child(empty(words.problem(problem), tokens)),
        Reading::Empty if model.failure.is_some() => return column,
        Reading::Empty | Reading::Loading { previous: None, .. } => {
            return column.child(empty(labels.reading_file, tokens));
        }
        Reading::Ready(commands)
        | Reading::Loading {
            previous: Some(commands),
            ..
        } => commands,
    };
    let commands = match commands {
        ProjectCommands::None => return column.child(empty(labels.no_commands, tokens)),
        ProjectCommands::Invalid { message } => {
            return column.child(empty(words.invalid_commands(message), tokens));
        }
        ProjectCommands::Listed { .. } => commands,
    };
    let Some(listed) = Listed::of(commands) else {
        return column;
    };
    let facts: Vec<String> = listed
        .setup
        .iter()
        .chain(listed.teardown)
        .map(ToString::to_string)
        .chain(
            listed
                .scripts
                .iter()
                .map(|script| script.name.as_str().to_owned()),
        )
        .take(4)
        .collect();
    let ticket = model
        .ticket()
        .filter(|ticket| shell.command_ticket_is_current(ticket));
    let enabled = ticket.is_some();
    let element_key = ticket
        .as_ref()
        .map_or_else(|| "inert".to_owned(), |ticket| ticket.element_key());
    let control = if listed.approved {
        let ticket = ticket.clone();
        let button = button_with_kind(
            format!("commands-withdraw-{element_key}"),
            labels.withdraw,
            ButtonKind::Secondary,
            if enabled {
                ButtonState::Enabled
            } else {
                ButtonState::Disabled
            },
            tokens,
        )
        .on_click(cx.listener(move |shell, _event, window, cx| {
            if let Some(ticket) = &ticket {
                shell.approve_project_commands(ticket, false, window, cx);
            }
        }));
        div()
            .child(badge(labels.approved, Tone::Ok, tokens))
            .child(button)
    } else {
        let ticket = ticket.clone();
        let button = button_with_kind(
            format!("commands-review-{element_key}"),
            labels.review,
            ButtonKind::Secondary,
            if enabled {
                ButtonState::Enabled
            } else {
                ButtonState::Disabled
            },
            tokens,
        )
        .on_click(cx.listener(move |shell, _event, window, cx| {
            if let Some(ticket) = &ticket {
                shell.review_project_commands(ticket, window, cx);
            }
        }));
        div()
            .child(badge(labels.not_approved, Tone::Warn, tokens))
            .child(button)
    };
    column = column.child(rows(tokens).child(row(
        title(
            words.commands_title(
                listed.setup.len(),
                listed.teardown.len(),
                listed.scripts.len(),
            ),
            tokens,
        ),
        &facts,
        Some(control),
        tokens,
    )));
    if detail.reviewing && !listed.approved {
        let place = words.approval_place(project.name.as_str(), machine, true);
        let compared = compare_approved(&listed, None);
        let panel_id = format!("settings-approval-{element_key}");
        let approve = ticket.clone();
        let cancel = owner.cloned();
        column = column.child(approval::render(
            shell,
            &Panel {
                id: &panel_id,
                asked: &listed,
                compared: &compared,
                place: &place,
                busy: model.writing.is_some()
                    || model.reading.outstanding().is_some()
                    || ticket.is_none(),
            },
            Actions {
                approve: Box::new(move |shell, window, cx| {
                    if let Some(ticket) = &approve {
                        shell.approve_project_commands(ticket, true, window, cx);
                    }
                }),
                skip: None,
                cancel: Box::new(move |shell, _window, cx| {
                    if let Some(target) = &cancel {
                        shell.cancel_command_review(target, cx);
                    }
                }),
            },
            tokens,
            cx,
        ));
    }
    column
}
