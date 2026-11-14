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
//! - Stand-ins until the shared controls land (G2): the pencil is a plain
//!   glyph button here, not `host_rename_button`; Remove has no Trash2
//!   leading glyph (the kit's button has no leading content yet). The
//!   field is focused when the dialog opens but its text is not selected
//!   (the kit's field has no select-all yet).
//! - Difference: the badges are the kit's (the pills on `surface3`, where
//!   Paseo's are bare outlines).

use vessel_app_kit::alert::{self, Content};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::badge::{self, badge};
use vessel_app_kit::dialog::{DialogContent, DialogSize, DialogState, dialog};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, TextField, Tokens, Type, TypeExt as _, button_with_kind, dot, glyph,
    outline_button, settings,
};
use vessel_app_link::Command;
use vessel_app_link::machines::{self, Answer};
use vessel_app_view::machine_book::{Edit, Failure, Name};
use vessel_app_view::machines::Transport;
use vessel_app_view::overview::{Ask, Editing, ask};
use vessel_app_view::settings::Section;
use vessel_app_view::{Machine, Reach};

use crate::routes::Route;
use crate::shell::Shell;

/// The page's state.
pub(crate) struct State {
    /// How the book reaches each machine, read when the page is shown.
    book: Option<Vec<machines::Machine>>,
    /// The machine the book was last read for.
    read_for: Option<String>,
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
            book: None,
            read_for: None,
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
    /// Before drawing: read the book when the page is shown for a machine
    /// it was not read for; forget it when the page is left.
    pub(crate) fn settle_overview(&mut self, window: &Window) {
        let shown = self.route == Route::Settings
            && self.areas.settings.section == Section::Overview
            && (!vessel_app_kit::breakpoint::phone(window) || self.areas.settings.pushed);
        let chosen = super::choices::chosen_machine(self)
            .filter(|_| shown)
            .map(|machine| machine.name.clone());
        let state = &mut self.areas.settings.overview;
        if chosen.is_none() {
            state.book = None;
            state.read_for = None;
            return;
        }
        if state.read_for != chosen {
            state.read_for = chosen;
            self.command(Command::Machines(machines::Ask::Read));
        }
    }

    /// The book answered: keep a read for the page; close a dialog whose
    /// edit succeeded, or keep its refusal. Whether the page said it.
    pub(crate) fn overview_answer(&mut self, answer: &Answer) -> bool {
        let state = &mut self.areas.settings.overview;
        match answer {
            Answer::Read(Ok(book)) => {
                state.book = Some(book.clone());
                false
            }
            Answer::Read(Err(_)) => false,
            Answer::Edited { edit, result } => {
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
                // The name changed under the page: read the book again.
                if result.is_ok() {
                    state.read_for = None;
                }
                said
            }
        }
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
    let unreachable = matches!(machine.reach, Reach::Unreachable(_));
    div()
        .flex()
        .flex_col()
        .child(badges(machine, shell, tokens))
        .when(unreachable, |page| {
            page.child(div().mb(px(spacing::S6)).child(alert::alert(
                Content {
                    title: Some(words.unreachable.into()),
                    description: Some(shell.t.unreachable_hint.into_any_element()),
                    action: None,
                },
                alert::Variant::Error,
                alert::Size::Small,
                tokens,
            )))
        })
        .child(settings::section(
            words.appearance,
            settings::card(tokens).child(name_row(machine, shell, tokens, compact, cx)),
            tokens,
        ))
        .child(settings::section(
            words.danger_zone,
            settings::card(tokens).child(remove_row(machine, shell, tokens, compact, cx)),
            tokens,
        ))
        .into_any_element()
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
        .overview
        .book
        .as_ref()
        .and_then(|book| {
            book.iter()
                .find(|entry| entry.name.as_str() == machine.name)
        })
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
fn name_row(
    machine: &Machine,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.t.lang.overview();
    let neutral = tokens.neutral;
    let pencil = Name::parse(&machine.name).ok().map(|name| {
        div()
            .id("settings-overview-rename")
            .p(px(4.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(move |style| style.bg(neutral.surface2))
            .child(glyph(Glyph::Pencil, 14.0, neutral.foreground_muted))
            .on_click(cx.listener(move |shell, _event, window, cx| {
                shell.overview_open_rename(name.clone(), window, cx);
            }))
    });
    let value = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .min_w_0()
        .flex_shrink(1.0)
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
    settings::row_at(
        words.name,
        None,
        None,
        Some(value.into_any_element()),
        true,
        compact,
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
    let button = outline_button(
        "settings-overview-remove",
        words.remove,
        name.is_some(),
        tokens,
    )
    .text_color(tokens.neutral.destructive)
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

/// The rename and remove dialogs, when open.
pub(crate) fn dialogs(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> impl Iterator<Item = AnyElement> {
    let rename = rename_dialog(shell, tokens, window, cx);
    let remove = remove_dialog(shell, tokens, window, cx);
    rename.into_iter().chain(remove)
}

fn rename_dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.overview;
    let open = state.rename.as_ref()?;
    let words = shell.t.lang.overview();
    let compact = crate::areas::draft::compact(window);
    let typed = state.field.text(cx);
    let pending = open.pending.is_some();
    // Paseo's submit is off while the name is unchanged or being saved.
    let can_submit = !pending && typed.trim() != open.machine.as_str();
    let error = state
        .problem
        .clone()
        .or_else(|| refusal(open.refused.as_ref(), shell));
    let body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(state.field.element(tokens, compact, cx))
        .children(error.map(|error| {
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.destructive)
                .child(error)
        }));
    let submit = button_with_kind(
        "settings-overview-rename-submit",
        words.rename_submit,
        ButtonKind::Primary,
        enabled(can_submit),
        tokens,
    )
    .when(can_submit, |button| {
        button.on_click(cx.listener(|shell, _event, _window, cx| {
            let typed = shell.areas.settings.overview.field.text(cx);
            shell.overview_rename(&typed, cx);
        }))
    });
    Some(dialog(
        &state.rename_dialog,
        DialogContent {
            title: words.rename_title.into(),
            body: body.into_any_element(),
            footer: vec![
                cancel(
                    "settings-overview-rename-cancel",
                    pending,
                    shell,
                    tokens,
                    cx,
                    |state| &mut state.rename,
                ),
                submit.into_any_element(),
            ],
            size: DialogSize::Normal,
        },
        tokens,
        window,
        compact,
        |shell: &mut Shell, _window, _cx| close(&mut shell.areas.settings.overview.rename),
        cx,
    ))
}

fn remove_dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.overview;
    let open = state.remove.as_ref()?;
    let words = shell.t.lang.overview();
    let pending = open.pending.is_some();
    let body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.remove_confirm(open.machine.as_str())),
        )
        .children(refusal(open.refused.as_ref(), shell).map(|error| {
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.destructive)
                .child(error)
        }));
    let confirm = button_with_kind(
        "settings-overview-remove-confirm",
        words.remove,
        ButtonKind::Destructive,
        enabled(!pending),
        tokens,
    )
    .when(!pending, |button| {
        button.on_click(cx.listener(|shell, _event, _window, cx| shell.overview_remove(cx)))
    });
    Some(dialog(
        &state.remove_dialog,
        DialogContent {
            title: words.remove_title.into(),
            body: body.into_any_element(),
            footer: vec![
                cancel(
                    "settings-overview-remove-cancel",
                    pending,
                    shell,
                    tokens,
                    cx,
                    |state| &mut state.remove,
                ),
                confirm.into_any_element(),
            ],
            size: DialogSize::Normal,
        },
        tokens,
        window,
        crate::areas::draft::compact(window),
        |shell: &mut Shell, _window, _cx| close(&mut shell.areas.settings.overview.remove),
        cx,
    ))
}

/// A dialog's Cancel: closes it, unless its edit is out (Paseo waits).
fn cancel(
    id: &'static str,
    pending: bool,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
    dialog: fn(&mut State) -> &mut Option<Editing>,
) -> AnyElement {
    button_with_kind(
        id,
        shell.t.lang.overview().cancel,
        ButtonKind::Secondary,
        enabled(!pending),
        tokens,
    )
    .when(!pending, |button| {
        button.on_click(cx.listener(move |shell, _event, _window, cx| {
            close(dialog(&mut shell.areas.settings.overview));
            cx.notify();
        }))
    })
    .into_any_element()
}

/// Close a dialog unless its edit is out.
fn close(dialog: &mut Option<Editing>) {
    if dialog.as_ref().is_some_and(|open| open.pending.is_none()) {
        *dialog = None;
    }
}

/// The book's refusal, in words.
fn refusal(failure: Option<&Failure>, shell: &Shell) -> Option<String> {
    failure.map(|failure| shell.t.lang.machine_book_failure(failure))
}

const fn enabled(on: bool) -> ButtonState {
    if on {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    }
}
