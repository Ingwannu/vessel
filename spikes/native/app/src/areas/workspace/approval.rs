//! A repository's commands, shown before they may run (ADR 0027; the
//! page's `approval-dialog.tsx`, `command-list.tsx`, `script-list.tsx`):
//! exactly what will run, in order (setup, teardown, and the scripts and
//! services the digest covers, ADR 0031), what changed since the set
//! approved last, then Approve (the digest shown is the one sent), or start
//! without them. Drawn in place (in Settings under the project's commands;
//! in the worktree start's card): the kit's dialog is track C's.
//!
//! Every script of the set is drawn, in full: a set is never offered for
//! approval with any of it out of view.

use vessel_app_kit::theme::radius;
use vessel_app_kit::ui::{
    Context, Div, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, button};
use vessel_app_view::workspace::command_changes::{
    ApprovalRefusal, Change, CommandLine, Compared, ListLine, Listed, ScriptLine,
};
use vessel_app_view::workspace::words::{Labels, Local, WorkspaceWords};
use vessel_proto::scripts::ScriptEntryView;
use vessel_proto::workspace::ProjectCommand;

use super::widgets::{Tone, badge, banner};
use crate::shell::Shell;

/// What a press does.
pub(super) type Action = Box<dyn Fn(&mut Shell, &mut Window, &mut Context<'_, Shell>)>;

/// What a person can do from the panel.
pub(super) struct Actions {
    /// Approve and run.
    pub(super) approve: Action,
    /// Start without setup, where there is something to start.
    pub(super) skip: Option<Action>,
    /// Cancel.
    pub(super) cancel: Action,
}

/// What the panel shows.
pub(super) struct Panel<'a> {
    /// Keeps two panels' buttons apart.
    pub(super) id: &'a str,
    /// The set asked for.
    pub(super) asked: &'a Listed<'a>,
    /// What it is compared with.
    pub(super) compared: &'a Compared,
    /// Where setup runs ("in each new worktree of shop on laptop").
    pub(super) place: &'a str,
    /// Whether the approval is out.
    pub(super) busy: bool,
}

/// The reader's zone and today, for "approved today 14:03".
pub(super) fn local() -> Local {
    let now = chrono::Local::now();
    Local {
        offset: *now.offset(),
        today: now.date_naive(),
    }
}

/// The panel.
pub(super) fn render(
    shell: &Shell,
    panel: &Panel<'_>,
    actions: Actions,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let Panel {
        id,
        asked,
        compared,
        place,
        busy,
    } = *panel;
    let words = shell.words();
    let labels = words.labels();
    let (setup, teardown, scripts) = match compared {
        Compared::Changed {
            setup,
            teardown,
            scripts,
            ..
        } => (setup.clone(), teardown.clone(), scripts.clone()),
        Compared::None => (
            as_listed(asked.setup),
            as_listed(asked.teardown),
            asked
                .scripts
                .iter()
                .map(|script| ScriptLine::Line {
                    change: Change::Same,
                    script: script.clone(),
                })
                .collect(),
        ),
    };
    let mut card = div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .p(px(16.0))
        .rounded(px(radius::R4))
        .border_1()
        .border_color(tokens.neutral.border)
        .bg(tokens.neutral.surface1)
        .child(div().type_(Type::Title).child(labels.approval_title))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.approval_copy(place, !scripts.is_empty())),
        )
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.compared(compared, &local())),
        )
        .child(command_list(words, labels.setup, &setup, tokens))
        .child(command_list(words, labels.teardown, &teardown, tokens));
    if !scripts.is_empty() {
        card = card.child(script_list(words, &scripts, tokens));
    }
    let approval = asked.approval();
    if let Err(ApprovalRefusal::Terminals) = approval {
        card = card.child(banner(labels.approval_unseen, true, None, tokens));
    }
    let buttons = buttons(id, busy, approval, actions, labels, tokens, cx);
    card.child(banner(labels.script_warning, false, None, tokens))
        .child(
            div()
                .type_(Type::Mono)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.digest(asked.digest.as_str())),
        )
        .child(buttons)
}

/// Cancel, start without setup (when offered), approve and run.
fn buttons(
    id: &str,
    busy: bool,
    approval: Result<(), ApprovalRefusal>,
    actions: Actions,
    labels: &'static Labels,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let Actions {
        approve,
        skip,
        cancel,
    } = actions;
    let mut buttons = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .justify_end()
        .gap(px(8.0))
        .child(
            button(format!("{id}-cancel"), labels.cancel, false, tokens)
                .on_click(cx.listener(move |this, _event, window, cx| cancel(this, window, cx))),
        );
    if let Some(skip) = skip {
        buttons = buttons.child(
            button(
                format!("{id}-skip"),
                labels.start_without_setup,
                false,
                tokens,
            )
            .on_click(cx.listener(move |this, _event, window, cx| skip(this, window, cx))),
        );
    }
    if approval.is_err() {
        return buttons;
    }
    buttons = buttons.child(
        button(
            format!("{id}-approve"),
            labels.approve_and_run,
            true,
            tokens,
        )
        .opacity(if busy { 0.5 } else { 1.0 })
        .on_click(cx.listener(move |this, _event, window, cx| {
            if !busy {
                approve(this, window, cx);
            }
        })),
    );
    buttons
}

/// Each command as it is when nothing is known to compare it with.
fn as_listed(commands: &[ProjectCommand]) -> Vec<CommandLine> {
    commands
        .iter()
        .map(|command| {
            CommandLine::Line(ListLine {
                change: Change::Same,
                command: command.clone(),
            })
        })
        .collect()
}

/// The gutter's mark, as a diff draws it.
const fn mark(change: Option<Change>) -> &'static str {
    match change {
        Some(Change::Added) => "+",
        Some(Change::Removed) => "\u{2212}",
        None => "~",
        Some(Change::Same) => "",
    }
}

/// The badge after a line (`None` for a changed one), none for a line as
/// approved.
fn change_badge(
    words: &dyn WorkspaceWords,
    change: Option<Change>,
    tokens: &Tokens,
) -> Option<Div> {
    let labels = words.labels();
    match change {
        Some(Change::Same) => None,
        Some(Change::Added) => Some(badge(labels.badge_new, Tone::Warn, tokens)),
        None => Some(badge(labels.badge_changed, Tone::Warn, tokens)),
        Some(Change::Removed) => Some(badge(labels.badge_removed, Tone::Plain, tokens)),
    }
}

fn line(words: &dyn WorkspaceWords, change: Option<Change>, text: Div, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(8.0))
        .py(px(4.0))
        .child(
            div()
                .w(px(12.0))
                .flex_none()
                .type_(Type::Mono)
                .text_color(tokens.neutral.foreground_muted)
                .child(mark(change)),
        )
        .child(text.flex_1().min_w(px(0.0)))
        .children(change_badge(words, change, tokens))
}

fn code(text: impl Into<vessel_app_kit::ui::SharedString>, struck: bool, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Mono)
        .text_color(if struck {
            tokens.neutral.foreground_muted
        } else {
            tokens.neutral.foreground
        })
        .when(struck, vessel_app_kit::ui::Styled::line_through)
        .child(text.into())
}

fn command_list(
    words: &dyn WorkspaceWords,
    title: &'static str,
    lines: &[CommandLine],
    tokens: &Tokens,
) -> Div {
    let mut list = div().flex().flex_col().child(
        div()
            .type_(Type::Kicker)
            .text_color(tokens.neutral.foreground_muted)
            .child(title),
    );
    if lines.is_empty() {
        return list.child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.labels().none),
        );
    }
    for entry in lines {
        list = list.child(match entry {
            CommandLine::Line(entry) => line(
                words,
                Some(entry.change),
                code(
                    entry.command.to_string(),
                    entry.change == Change::Removed,
                    tokens,
                ),
                tokens,
            ),
            CommandLine::Changed { command, was } => line(
                words,
                None,
                div()
                    .flex()
                    .flex_col()
                    .child(code(command.to_string(), false, tokens))
                    .child(
                        div()
                            .type_(Type::Meta)
                            .text_color(tokens.neutral.foreground_muted)
                            .child(words.was(was.as_str())),
                    ),
                tokens,
            ),
        });
    }
    list
}

fn script_list(words: &dyn WorkspaceWords, lines: &[ScriptLine], tokens: &Tokens) -> Div {
    let mut list = div().flex().flex_col().child(
        div()
            .type_(Type::Kicker)
            .text_color(tokens.neutral.foreground_muted)
            .child(words.labels().scripts_and_services),
    );
    for entry in lines {
        // A new approved field must break this display until it is shown or refused.
        let ScriptEntryView {
            name,
            kind,
            command,
            dir,
            port,
        } = entry.script().view();
        let (change, removed) = match entry {
            ScriptLine::Line { change, .. } => (Some(*change), *change == Change::Removed),
            ScriptLine::Changed { .. } => (None, false),
        };
        let place = dir.map_or_else(String::new, |dir| format!(" \u{b7} {dir}/"));
        let declared = port.map_or_else(String::new, |port| {
            format!(" \u{b7} {}", words.script_port(Some(port)))
        });
        let mut text = div()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(6.0))
                    .child(
                        div()
                            .type_(Type::Label)
                            .text_color(tokens.neutral.foreground)
                            .child(name.as_str().to_owned()),
                    )
                    .child(
                        div()
                            .type_(Type::Meta)
                            .text_color(tokens.neutral.foreground_muted)
                            .child(format!("{}{place}{declared}", words.script_kind(kind))),
                    ),
            )
            .child(code(command.to_string(), removed, tokens));
        if let ScriptLine::Changed { script, was } = entry {
            text = text.child(
                div()
                    .type_(Type::Meta)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(format!(
                        "{} \u{b7} {}",
                        words.script_change(was, script),
                        words.was(was.command.as_str())
                    )),
            );
        }
        list = list.child(line(words, change, text, tokens));
    }
    list
}

use vessel_app_kit::ui::FluentBuilder as _;
