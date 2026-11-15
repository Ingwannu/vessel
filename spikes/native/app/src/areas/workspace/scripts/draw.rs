//! Settings card rows use shared kit controls and wrap actions on phones.
use super::super::widgets::{divided, empty, rows, section};
use super::ports;
use crate::shell::Shell;
use vessel_app_kit::ui::{
    Context, Div, ElementId, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div,
    px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, badge, button_with_kind, theme::spacing,
};
use vessel_app_view::workspace::command_changes::Listed;
use vessel_app_view::workspace::git::Reading;
use vessel_app_view::workspace::scripts::{
    Action, Entry, RunState, latest_run, retained_runs, run_state, words,
};
use vessel_proto::ProjectId;
use vessel_proto::scripts::ScriptKind;
use vessel_proto::terminals::TerminalInfo;

pub(in crate::areas::workspace) fn render(
    shell: &Shell,
    machine: &str,
    project: ProjectId,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let scripts = &shell.areas.workspace.scripts;
    if !scripts
        .selection
        .as_ref()
        .is_some_and(|(selected, id)| selected.as_str() == machine && *id == project)
    {
        return div();
    }
    content(
        &Card {
            shell,
            model: &scripts.model,
            host: &scripts.host,
            surface: &vessel_app_view::workspace::scripts::Surface::Project,
        },
        tokens,
        cx,
    )
}

/// Borrow owner-local controls while sharing one renderer across settings/tabs.
pub(super) struct Card<'a> {
    pub(super) shell: &'a Shell,
    pub(super) model: &'a vessel_app_view::workspace::scripts::Model,
    pub(super) host: &'a vessel_app_kit::TextField,
    pub(super) surface: &'a vessel_app_view::workspace::scripts::Surface,
}

pub(super) fn content(card: &Card<'_>, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let shell = card.shell;
    let words = shell.words.scripts();
    let model = card.model;
    let busy = model.read.outstanding().is_some() || model.writing.is_some();
    let mut refresh = button_with_kind(
        "scripts-refresh",
        words.refresh(),
        ButtonKind::Secondary,
        if busy {
            ButtonState::Disabled
        } else {
            ButtonState::Enabled
        },
        tokens,
    )
    .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET));
    if !busy {
        let expected = model.target.clone();
        let surface = card.surface.clone();
        refresh = refresh.on_click(cx.listener(move |shell, _event, window, cx| {
            shell.refresh_scripts_for(&surface, &expected, window, cx)
        }));
    }
    let mut body = div().flex().flex_col().gap(px(spacing::S3));
    if model.target.is_none() {
        body = body.child(ports::notice(words.read_failed(), tokens));
    }
    if let Some(issue) = &model.issue {
        body = body.child(ports::notice(words::issue(shell.words, issue), tokens));
    }
    match &model.read {
        Reading::Empty | Reading::Loading { previous: None, .. } => {
            body = body.child(empty(words.loading(), tokens));
        }
        Reading::Failed(_) => body = body.child(ports::notice(words.read_failed(), tokens)),
        Reading::Ready(_)
        | Reading::Loading {
            previous: Some(_), ..
        } => {}
    }
    if let Some(snapshot) = model.shown() {
        body = body
            .child(configured(card, snapshot, tokens, cx))
            .child(retained(card, snapshot, tokens, cx));
        body = body.child(ports::scan(card, tokens));
    }
    body = body
        .child(div().type_(Type::Body).child(words.host()))
        .child(card.host.element(tokens, true, cx))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.host_hint()),
        );
    section(words.title(), Some(div().child(refresh)), body, tokens)
}

fn configured(
    card: &Card<'_>,
    snapshot: &vessel_app_view::workspace::scripts::Snapshot,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let shell = card.shell;
    let words = shell.words.scripts();
    let Some(listed) = Listed::of(&snapshot.scripts.commands) else {
        return match &snapshot.scripts.commands {
            vessel_proto::workspace::ProjectCommands::None => empty(words.empty(), tokens),
            vessel_proto::workspace::ProjectCommands::Invalid { .. } => {
                ports::notice(words.invalid_config(), tokens)
            }
            vessel_proto::workspace::ProjectCommands::Listed { .. } => {
                ports::notice(words.unsupported_config(), tokens)
            }
        };
    };
    let mut body = div().flex().flex_col().gap(px(spacing::S3));
    if !listed.approved {
        body = body.child(ports::notice(words.approval(), tokens));
    }
    if listed.scripts.is_empty() {
        return body.child(empty(words.empty(), tokens));
    }
    let mut table = rows(tokens);
    for (index, entry) in listed.scripts.iter().map(Entry::of).enumerate() {
        let run = latest_run(entry.name, &snapshot.scripts.runs);
        table = table.child(divided(
            script_row(card, entry, run, tokens, cx),
            index,
            tokens,
        ));
    }
    body.child(table)
}
fn retained(
    card: &Card<'_>,
    snapshot: &vessel_app_view::workspace::scripts::Snapshot,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let shell = card.shell;
    let runs = retained_runs(&snapshot.scripts);
    if runs.is_empty() {
        return div();
    }
    let mut table = rows(tokens);
    for (index, run) in runs.into_iter().enumerate() {
        if let Some(entry) = Entry::retained(run) {
            table = table.child(divided(
                script_row(card, entry, Some(run), tokens, cx),
                index,
                tokens,
            ));
        }
    }
    section(shell.words.scripts().retained(), None, table, tokens)
}

fn script_row(
    card: &Card<'_>,
    entry: Entry<'_>,
    run: Option<&TerminalInfo>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let shell = card.shell;
    let words = shell.words.scripts();
    let Entry {
        name,
        kind,
        command,
        dir,
        port,
        observed_dir,
    } = entry;
    let model = card.model;
    let state = run_state(run);
    let actions = row_actions(card, name, run, tokens, cx);
    let pending = model
        .writing
        .as_ref()
        .is_some_and(|(_, action)| match action {
            Action::Run(script) => script == name,
            Action::Restart(id) | Action::Stop(id) => run.is_some_and(|run| run.id == *id),
        });
    let mut head = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(spacing::S2))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .child(name.to_string()),
        )
        .child(badge::badge(
            words::status(shell.words, state),
            tone(state),
            badge::Size::ExtraSmall,
            None,
            tokens,
        ));
    if pending {
        head = head.child(div().type_(Type::Meta).child(words.pending()));
    }
    if let RunState::Exited(Some(code)) = state {
        head = head.child(div().type_(Type::Meta).child(code.to_string()));
    }
    let kind = match kind {
        ScriptKind::Script => words.script(),
        ScriptKind::Service => words.service(),
    };
    let kind = port.map_or_else(
        || kind.to_owned(),
        |port| format!("{kind} · {} {port}", words.declared_port()),
    );
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .p(px(spacing::S4))
        .child(head)
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(kind),
        )
        .child(div().type_(Type::Mono).child(command.to_string()))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(observed_dir.map_or_else(
                    || dir.map_or_else(|| words.root().to_owned(), ToString::to_string),
                    |folder| folder.display().to_string(),
                )),
        )
        .children(if observed_dir.is_none() {
            actual_run(shell, run, tokens)
        } else {
            None
        })
        .children(run.filter(|run| run.status.is_live()).map(|_run| {
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.restart_uses_run().to_owned())
        }))
        .child(actions)
        .child(ports::for_run(card, run, tokens, cx))
}
fn actual_run(shell: &Shell, run: Option<&TerminalInfo>, tokens: &Tokens) -> Option<Div> {
    let Entry {
        name: _,
        kind,
        command,
        dir: _,
        port: _,
        observed_dir,
    } = Entry::retained(run?)?;
    let words = shell.words.scripts();
    let kind = match kind {
        ScriptKind::Script => words.script(),
        ScriptKind::Service => words.service(),
    };
    Some(
        div()
            .flex()
            .flex_col()
            .gap(px(spacing::S1_5))
            .child(
                div()
                    .type_(Type::Meta)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(words.observed_run().to_owned()),
            )
            .child(div().type_(Type::Meta).child(kind.to_owned()))
            .child(div().type_(Type::Mono).child(command.to_string()))
            .children(
                observed_dir
                    .map(|folder| div().type_(Type::Meta).child(folder.display().to_string())),
            ),
    )
}

fn row_actions(
    card: &Card<'_>,
    name: &vessel_proto::scripts::ScriptName,
    run: Option<&TerminalInfo>,
    tokens: &Tokens,
    cx: &Context<'_, Shell>,
) -> Div {
    let shell = card.shell;
    let words = shell.words.scripts();
    let mut actions = div().flex().flex_wrap().gap(px(spacing::S1_5));
    if let Some(run) = run
        && let Some(ticket) = card.model.action_ticket()
        && matches!(
            ticket.target.surface,
            vessel_app_view::workspace::scripts::Surface::Panel { .. }
        )
    {
        let terminal = run.id;
        actions = actions.child(
            button_with_kind(
                ElementId::Name(format!("script-terminal-{terminal}").into()),
                words.open_terminal(),
                ButtonKind::Secondary,
                ButtonState::Enabled,
                tokens,
            )
            .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
            .on_click(cx.listener(move |shell, _event, window, cx| {
                shell.script_panel_reveal(&ticket, terminal, window, cx);
            })),
        );
    }
    if let Some(run) = run.filter(|run| run.status.is_live()) {
        actions = actions
            .child(action(
                card,
                Action::Restart(run.id),
                words.restart(),
                format!("script-restart-{name}"),
                tokens,
                cx,
            ))
            .child(action(
                card,
                Action::Stop(run.id),
                words.stop(),
                format!("script-stop-{name}"),
                tokens,
                cx,
            ));
    } else {
        actions = actions.child(action(
            card,
            Action::Run(name.clone()),
            words.run(),
            format!("script-run-{name}"),
            tokens,
            cx,
        ));
    }
    actions
}

fn action(
    card: &Card<'_>,
    action: Action,
    label: &str,
    id: String,
    tokens: &Tokens,
    cx: &Context<'_, Shell>,
) -> Div {
    let shell = card.shell;
    let ticket = card.model.action_ticket();
    let enabled = ticket
        .as_ref()
        .is_some_and(|ticket| shell.script_card_is_current(&ticket.target))
        && card.model.permits(&action);
    let control = button_with_kind(
        ElementId::Name(id.into()),
        label.to_owned(),
        ButtonKind::Secondary,
        if enabled {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        tokens,
    )
    .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET));
    div().child(if enabled {
        control.on_click(cx.listener(move |shell, _event, window, cx| {
            if let Some(ticket) = &ticket {
                shell.script_action_for(ticket, action.clone(), window, cx);
            }
        }))
    } else {
        control
    })
}
fn tone(state: RunState) -> badge::Variant {
    match state {
        RunState::Running => badge::Variant::Muted,
        RunState::Starting | RunState::Backoff => badge::Variant::Warning,
        RunState::Failed => badge::Variant::Error,
        RunState::Idle | RunState::Exited(_) | RunState::Stopped | RunState::Unknown => {
            badge::Variant::Muted
        }
    }
}
