//! Settings › Overview for the chosen machine, as Paseo's `HostSettingsPage`
//! (`screens/settings/host-page.tsx`): the status and version badges
//! (`HostStatusBadges`), an alert while the machine does not answer
//! (`HostConnectionError`), "Appearance" with its name and rename
//! (`host-appearance-section.tsx`: `HostAppearanceSection`,
//! `HostRenameButton`, `AdaptiveRenameModal`), and "Danger zone" with
//! "Remove machine" (`RemoveHostSection`).
//!
//! [Decision Log]
//! - Purpose: Paseo's host page, over what Vessel already has: the book's
//!   rename and remove (`link::machines`, the CLI's own book code), the
//!   machine's reach and node version from the fleet, how the book reaches
//!   it.
//! - Choice: Paseo's sections, rows and dialogs; the rules of the rename
//!   and remove dialogs are `view::overview` (tested). The book's answer
//!   closes a dialog, or its refusal is said inside it.
//! - Not shown, nothing behind them yet: Paseo's colour and sidebar-badge
//!   rows (the app keeps no machine appearance), "Restart daemon" and
//!   "Update daemon" (no such call; the node-control track brings them).
//! - The name action uses the kit's shared `host_rename_button` unchanged.
//!   The editor remains here. Its text is focused but not selected on open;
//!   select-all waits on the kit field module's split.
//! - Difference: the badges are the kit's (the pills on `surface3`, where
//!   Paseo's are bare outlines).

use vessel_app_kit::alert::{self, Content};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::badge::{self, badge};
use vessel_app_kit::dialog::DialogState;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonState, TextField, Tokens, Type, TypeExt as _, dot, host_rename_button,
    outline_button_with_glyph, settings,
};
use vessel_app_link::Command;
use vessel_app_link::machines::{self, Answer};
use vessel_app_view::machine_book::{Edit, Name};
use vessel_app_view::machines::Transport;
use vessel_app_view::overview::{Ask, Editing, ask};
use vessel_app_view::{Machine, Reach};

use crate::shell::Shell;

mod dialogs;
pub(crate) use dialogs::dialogs;

/// The page's state.
pub(crate) struct State {
    rename: Option<Editing>,
    /// What is wrong with the typed name, after a press of Rename.
    problem: Option<String>,
    field: TextField,
    rename_dialog: DialogState,
    remove: Option<Editing>,
    remove_dialog: DialogState,
}

impl State {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            rename: None,
            problem: None,
            field: TextField::with_change(
                "",
                |shell: &mut Shell, text, _window, cx| shell.overview_rename(&text, cx),
                // Paseo clears the error as the name is typed again.
                |shell: &mut Shell, _text, cx| {
                    shell.areas.settings.overview.problem = None;
                    cx.notify();
                },
                window,
                cx,
            ),
            rename_dialog: DialogState::new(cx),
            remove: None,
            remove_dialog: DialogState::new(cx),
        }
    }
}

impl Shell {
    /// The book answered an edit: close a dialog whose edit succeeded, or
    /// keep its refusal. Whether the page said it.
    pub(crate) fn overview_answer(&mut self, answer: &Answer) -> bool {
        let Answer::Edited { edit, result } = answer else {
            return false;
        };
        let state = &mut self.areas.settings.overview;
        let mut said = false;
        for dialog in [&mut state.rename, &mut state.remove] {
            if let Some(open) = dialog.as_mut()
                && open.pending.as_ref() == Some(edit)
            {
                said = true;
                if open.answered(edit, result) {
                    *dialog = None;
                }
            }
        }
        said
    }

    fn overview_open_rename(
        &mut self,
        name: Name,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let words = self.t.lang.overview();
        let state = &mut self.areas.settings.overview;
        state.field.set_text(name.as_str(), window, cx);
        state
            .field
            .set_placeholder(words.rename_placeholder, window, cx);
        state.problem = None;
        state.rename = Some(Editing::open(name));
        state.field.focus(window, cx);
        cx.notify();
    }

    /// Rename was pressed (or Enter in the field).
    fn overview_rename(&mut self, typed: &str, cx: &mut Context<'_, Self>) {
        let others: Vec<String> = self
            .fleet
            .machines
            .iter()
            .map(|machine| machine.name.clone())
            .collect();
        let state = &mut self.areas.settings.overview;
        let Some(open) = state.rename.as_mut().filter(|open| open.pending.is_none()) else {
            return;
        };
        let others: Vec<&str> = others
            .iter()
            .map(String::as_str)
            .filter(|name| *name != open.machine.as_str())
            .collect();
        let send = match ask(&open.machine, typed, &others) {
            Ask::Unchanged => None,
            Ask::Problem(problem) => {
                state.problem = Some(self.t.lang.machines().name_problem(&problem));
                None
            }
            Ask::Send(edit) => {
                open.sent(edit.clone());
                Some(edit)
            }
        };
        if let Some(edit) = send {
            self.command(Command::Machines(machines::Ask::Edit(edit)));
        }
        cx.notify();
    }

    fn overview_remove(&mut self, cx: &mut Context<'_, Self>) {
        let Some(open) = self
            .areas
            .settings
            .overview
            .remove
            .as_mut()
            .filter(|open| open.pending.is_none())
        else {
            return;
        };
        let edit = Edit::Forget(open.machine.clone());
        open.sent(edit.clone());
        self.command(Command::Machines(machines::Ask::Edit(edit)));
        cx.notify();
    }
}

/// The Overview page.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let Some(machine) = super::choices::chosen_machine(shell) else {
        return div().into_any_element();
    };
    let words = shell.t.lang.overview();
    div()
        .flex()
        .flex_col()
        .child(badges(machine, shell, tokens))
        .children(unreachable(machine, shell, tokens))
        .child(settings::section(
            words.appearance,
            settings::card(tokens).child(name_row(machine, shell, tokens, cx)),
            tokens,
        ))
        .child(settings::section(
            words.danger_zone,
            settings::card(tokens).child(remove_row(machine, shell, tokens, compact, cx)),
            tokens,
        ))
        .into_any_element()
}

/// Paseo's `HostConnectionError`, on the Overview and Connections pages:
/// while the machine does not answer, an alert in the app's words (Paseo
/// shows the connection's last error there).
pub(super) fn unreachable(machine: &Machine, shell: &Shell, tokens: &Tokens) -> Option<Div> {
    matches!(machine.reach, Reach::Unreachable(_)).then(|| {
        div().mb(px(spacing::S6)).child(alert::alert(
            Content {
                title: Some(shell.t.lang.overview().unreachable.into()),
                description: Some(shell.t.unreachable_hint.into_any_element()),
                action: None,
            },
            alert::Variant::Error,
            alert::Size::Small,
            tokens,
        ))
    })
}

/// Paseo's `HostStatusBadges`: the status with its dot, how the book
/// reaches the machine, its node's version.
fn badges(machine: &Machine, shell: &Shell, tokens: &Tokens) -> Div {
    let words = shell.t.lang.overview();
    let status = tokens.status();
    let (label, variant, signal) = match machine.reach {
        Reach::Online => (words.online, badge::Variant::Success, status.success),
        // Paseo's tone for an offline host.
        Reach::Unreachable(_) => (words.offline, badge::Variant::Warning, status.warning),
    };
    let transport = shell
        .areas
        .settings
        .book
        .entry(&machine.name)
        .map(|entry| reach_words(shell, entry.transport));
    let version = machine
        .version
        .as_ref()
        .and_then(vessel_app_view::NodeVersion::label);
    let pill = |text: String| {
        badge(
            text,
            badge::Variant::Muted,
            badge::Size::Small,
            None,
            tokens,
        )
    };
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(spacing::S1))
        .mb(px(spacing::S6))
        .child(badge(
            label,
            variant,
            badge::Size::Small,
            Some(dot(signal.foreground).into_any_element()),
            tokens,
        ))
        .children(transport.map(|text| pill(text.to_owned())))
        .children(version.map(pill))
}

fn reach_words(shell: &Shell, transport: Transport) -> &'static str {
    shell.t.lang.machines().reach(transport)
}

/// "Name": the machine's name, muted, and the pencil that renames it (a
/// name the book cannot take has no pencil).
fn name_row(machine: &Machine, shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.overview();
    let neutral = tokens.neutral;
    let pencil = Name::parse(&machine.name).ok().map(|name| {
        host_rename_button("settings-overview-rename", ButtonState::Enabled, tokens).on_click(
            cx.listener(move |shell, _event, window, cx| {
                shell.overview_open_rename(name.clone(), window, cx);
            }),
        )
    });
    let value = div()
        .flex()
        .flex_row()
        .items_center()
        .justify_end()
        .gap(px(spacing::S2))
        .min_w_0()
        .flex_1()
        .child(
            div()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .type_(Type::Body)
                .text_color(neutral.foreground_muted)
                .child(machine.name.clone()),
        )
        .children(pencil);
    let title = div()
        .flex_none()
        .whitespace_nowrap()
        .type_(Type::Body)
        .text_color(neutral.foreground)
        .child(words.name);
    // This reference row stays beside its value on both layouts.
    settings::row_with_content(
        title.into_any_element(),
        Some(value.into_any_element()),
        true,
        false,
        tokens,
    )
}

/// "Remove machine" and its button, which asks first.
fn remove_row(
    machine: &Machine,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.t.lang.overview();
    let name = Name::parse(&machine.name).ok();
    // Paseo's outline Remove with its Trash2 before the words, both in
    // the destructive colour.
    let button = outline_button_with_glyph(
        "settings-overview-remove",
        Glyph::Trash2,
        words.remove,
        tokens.neutral.destructive,
        name.is_some(),
        tokens,
    )
    .when_some(name, |button, name| {
        button.on_click(cx.listener(move |shell, _event, window, cx| {
            let state = &mut shell.areas.settings.overview;
            state.remove = Some(Editing::open(name.clone()));
            state.remove_dialog.focus(window, cx);
            cx.notify();
        }))
    });
    settings::row_at(
        words.remove_title,
        Some(words.remove_hint.into()),
        None,
        Some(button.into_any_element()),
        true,
        compact,
        tokens,
    )
}
