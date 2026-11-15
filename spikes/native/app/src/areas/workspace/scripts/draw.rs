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
    let words = shell.words.scripts();
    let scripts = &shell.areas.workspace.scripts;
    let model = &scripts.model;
    if !model
        .target
        .as_ref()
        .is_some_and(|target| target.machine.as_str() == machine && target.project == project)
    {
        return div();
    }
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
        refresh =
            refresh.on_click(cx.listener(|shell, _event, _window, cx| shell.refresh_scripts(cx)));
    }
    let mut body = div().flex().flex_col().gap(px(spacing::S3));
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
            .child(configured(shell, snapshot, tokens, cx))
            .child(retained(shell, snapshot, tokens, cx));
        body = body.child(ports::scan(shell, tokens));
    }
    body = body
        .child(div().type_(Type::Body).child(words.host()))
        .child(scripts.host.element(tokens, true, cx))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.host_hint()),
        );
    section(words.title(), Some(div().child(refresh)), body, tokens)
}

fn configured(
    shell: &Shell,
    snapshot: &vessel_app_view::workspace::scripts::Snapshot,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
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
    let mut card = rows(tokens);
    for (index, entry) in listed.scripts.iter().map(Entry::of).enumerate() {
        let run = latest_run(entry.name, &snapshot.scripts.runs);
        card = card.child(divided(
            script_row(shell, entry, run, tokens, cx),
            index,
            tokens,
        ));
    }
    body.child(card)
}
fn retained(
    shell: &Shell,
    snapshot: &vessel_app_view::workspace::scripts::Snapshot,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let runs = retained_runs(&snapshot.scripts);
    if runs.is_empty() {
        return div();
    }
    let mut card = rows(tokens);
    for (index, run) in runs.into_iter().enumerate() {
        if let Some(entry) = Entry::retained(run) {
            card = card.child(divided(
                script_row(shell, entry, Some(run), tokens, cx),
                index,
                tokens,
            ));
        }
    }
    section(shell.words.scripts().retained(), None, card, tokens)
}

fn script_row(
    shell: &Shell,
    entry: Entry<'_>,
    run: Option<&TerminalInfo>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.scripts();
    let Entry {
        name,
        kind,
        command,
        dir,
        port,
    } = entry;
    let model = &shell.areas.workspace.scripts.model;
    let state = run_state(run);
    let actions = row_actions(shell, name, run, tokens, cx);
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
                .child(dir.map_or_else(|| words.root().to_owned(), ToString::to_string)),
        )
        .child(actions)
        .child(ports::for_run(shell, run, tokens, cx))
}
fn row_actions(
    shell: &Shell,
    name: &vessel_proto::scripts::ScriptName,
    run: Option<&TerminalInfo>,
    tokens: &Tokens,
    cx: &Context<'_, Shell>,
) -> Div {
    let words = shell.words.scripts();
    let mut actions = div().flex().flex_wrap().gap(px(spacing::S1_5));
    if let Some(run) = run.filter(|run| run.status.is_live()) {
        actions = actions
            .child(action(
                shell,
                Action::Restart(run.id),
                words.restart(),
                format!("script-restart-{name}"),
                tokens,
                cx,
            ))
            .child(action(
                shell,
                Action::Stop(run.id),
                words.stop(),
                format!("script-stop-{name}"),
                tokens,
                cx,
            ));
    } else {
        actions = actions.child(action(
            shell,
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
    shell: &Shell,
    action: Action,
    label: &str,
    id: String,
    tokens: &Tokens,
    cx: &Context<'_, Shell>,
) -> Div {
    let enabled = shell.areas.workspace.scripts.model.permits(&action);
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
        control.on_click(
            cx.listener(move |shell, _event, _window, cx| shell.script_action(action.clone(), cx)),
        )
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
