//! Settings › Terminals' dialogs: Paseo's `TerminalProfileEditModal`
//! (`screens/settings/terminal-profile-edit-modal.tsx`) for Add and Edit,
//! and its remove confirmation (`host-page.tsx` `handleRemove`).
//!
//! [Decision Log]
//! - Reference: Name, Command and Arguments, each labelled; an error under
//!   Name or Command after Save when it is empty, cleared as that field is
//!   typed in; Enter moves to the next field and saves from the last; while
//!   the save waits the fields are read-only, Cancel and the dismiss are
//!   held and Save reads "Saving..."; the node's answer closes the dialog,
//!   or its failure shows under the fields. 480 wide. Remove asks
//!   'Remove "{name}"?' with a destructive Remove; its failure is said over
//!   the list.
//! - Choice: the kit dialog and fields, the rules in
//!   `view::terminal_profiles` (tested): a draft is checked against the
//!   profile bounds before it is sent, so a field too long is said here and
//!   not refused by the node.
//! - Difference: a refusal is said in the app's words for its code, never
//!   the node's text (the Devices page's rule).

use vessel_app_kit::ui::{Context, Window};
use vessel_app_kit::{Editability, TextField};
use vessel_app_view::devices::MachineKey;
use vessel_app_view::terminal_profiles::{
    Draft, DraftErrors, Problem, Target, draft_of, new_id, with_draft,
};
use vessel_proto::ErrorCode;
use vessel_proto::terminal_profiles::Id;

use crate::shell::Shell;

/// The edit dialog's state.
pub(super) struct Edit {
    machine: MachineKey,
    target: Target,
    pub(super) name: TextField,
    command: TextField,
    args: TextField,
    pub(super) errors: DraftErrors,
    saving: bool,
    pub(super) refused: Option<Problem>,
}

impl Edit {
    /// Whether its save waits for the node.
    pub(super) const fn saving(&self) -> bool {
        self.saving
    }

    /// The machine whose list it changes.
    pub(super) const fn machine(&self) -> &MachineKey {
        &self.machine
    }

    /// The node did not keep the save: the fields are editable again and
    /// the reason shows under them.
    pub(super) fn refused(&mut self, problem: Problem, cx: &mut Context<'_, Shell>) {
        self.saving = false;
        self.refused = Some(problem);
        self.editable(Editability::Editable, cx);
    }

    fn editable(&self, editability: Editability, cx: &mut Context<'_, Shell>) {
        for field in [&self.name, &self.command, &self.args] {
            field.set_editability(editability, cx);
        }
    }
}

impl Shell {
    /// Add (the section's plus): an empty dialog with a fresh id.
    pub(super) fn terminal_profile_add(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let now = chrono::Utc::now().timestamp_millis();
        let Ok(id) = new_id(u64::try_from(now).unwrap_or_default(), random()) else {
            return;
        };
        self.terminal_profile_open(Target::Add(id), &Draft::default(), window, cx);
    }

    /// A row's Edit: its name, command and arguments in the fields.
    pub(super) fn terminal_profile_edit(
        &mut self,
        id: &Id,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(draft) = self
            .terminal_profiles_page_list()
            .and_then(|list| list.get(id))
            .map(draft_of)
        else {
            return;
        };
        self.terminal_profile_open(Target::Edit(id.clone()), &draft, window, cx);
    }

    fn terminal_profile_open(
        &mut self,
        target: Target,
        draft: &Draft,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(machine) = self.areas.terminal_profiles.page.clone() else {
            return;
        };
        // The catalog is the app's for its life: borrowed apart from `self`.
        let translator = self.words;
        let words = translator.terminal_profiles();
        let name = TextField::with_change(
            words.name_placeholder(),
            |shell: &mut Self, _text, window, cx| {
                shell.terminal_profile_next(Field::Command, window, cx);
            },
            |shell: &mut Self, _text, cx| {
                if let Some(edit) = shell.areas.terminal_profiles.edit.as_mut() {
                    edit.errors.name = false;
                }
                cx.notify();
            },
            window,
            cx,
        );
        let command = TextField::with_change(
            words.command_placeholder(),
            |shell: &mut Self, _text, window, cx| {
                shell.terminal_profile_next(Field::Args, window, cx);
            },
            |shell: &mut Self, _text, cx| {
                if let Some(edit) = shell.areas.terminal_profiles.edit.as_mut() {
                    edit.errors.command = false;
                }
                cx.notify();
            },
            window,
            cx,
        );
        let args = TextField::new(
            words.args_placeholder(),
            |shell: &mut Self, _text, window, cx| shell.terminal_profile_save(window, cx),
            window,
            cx,
        );
        name.set_text(&draft.name, window, cx);
        command.set_text(&draft.command, window, cx);
        args.set_text(&draft.args, window, cx);
        let state = &mut self.areas.terminal_profiles;
        state.edit_dialog.focus(window, cx);
        name.focus(window, cx);
        state.edit = Some(Edit {
            machine,
            target,
            name,
            command,
            args,
            errors: DraftErrors::default(),
            saving: false,
            refused: None,
        });
        cx.notify();
    }

    /// Enter in Name or Command: the next field takes the keys.
    fn terminal_profile_next(
        &mut self,
        to: Field,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(edit) = self.areas.terminal_profiles.edit.as_ref() {
            match to {
                Field::Command => edit.command.focus(window, cx),
                Field::Args => edit.args.focus(window, cx),
            }
        }
    }

    /// Save: what is wrong is said under its field; otherwise the whole
    /// list goes to the node and the dialog waits.
    pub(super) fn terminal_profile_save(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(list) = self.terminal_profiles_page_list().cloned() else {
            return;
        };
        let state = &mut self.areas.terminal_profiles;
        let Some(edit) = state.edit.as_mut().filter(|edit| !edit.saving) else {
            return;
        };
        edit.refused = None;
        let draft = Draft {
            name: edit.name.text(cx),
            command: edit.command.text(cx),
            args: edit.args.text(cx),
        };
        match with_draft(&list, &edit.target, &draft) {
            Err(errors) => edit.errors = errors,
            Ok(next) => {
                edit.errors = DraftErrors::default();
                let machine = edit.machine.clone();
                if self.terminal_profiles_save(machine, next)
                    && let Some(edit) = self.areas.terminal_profiles.edit.as_mut()
                {
                    edit.saving = true;
                    edit.editable(Editability::ReadOnly, cx);
                    // The dialog holds the keys while it waits, so Escape
                    // still reaches it (and is held).
                    self.areas.terminal_profiles.edit_dialog.focus(window, cx);
                }
            }
        }
        cx.notify();
    }

    /// Cancel, Escape or the scrim: held while the save waits.
    fn terminal_profile_cancel(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.terminal_profiles;
        if state.edit.as_ref().is_some_and(|edit| !edit.saving) {
            state.edit = None;
            state.edit_dialog.dismissed(window, cx);
            cx.notify();
        }
    }

    /// A row's Remove: the confirmation.
    pub(super) fn terminal_profile_remove_ask(
        &mut self,
        id: Id,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.terminal_profiles;
        state.removing = Some(id);
        state.remove_dialog.focus(window, cx);
        cx.notify();
    }

    fn terminal_profile_remove_keep(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.terminal_profiles;
        state.removing = None;
        state.remove_dialog.dismissed(window, cx);
        cx.notify();
    }

    /// Remove confirmed: it closes, and the list without the row is sent.
    fn terminal_profile_remove(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.terminal_profiles;
        let gone = state.removing.take();
        state.remove_dialog.dismissed(window, cx);
        let machine = state.page.clone();
        let next = gone
            .zip(self.terminal_profiles_page_list())
            .map(|(id, list)| vessel_app_view::terminal_profiles::removed(list, &id));
        if let (Some(machine), Some(next)) = (machine, next) {
            let _sent = self.terminal_profiles_save(machine, next);
        }
        cx.notify();
    }
}

#[derive(Clone, Copy)]
enum Field {
    Command,
    Args,
}

/// A random part for a new id (Paseo's `Math.random()`): a fresh random
/// identity's hash, so no randomness crate is added for it.
fn random() -> u64 {
    use std::hash::{DefaultHasher, Hash as _, Hasher as _};
    let mut hasher = DefaultHasher::new();
    vessel_proto::SessionId::random().hash(&mut hasher);
    hasher.finish()
}

/// A refused save, in the app's words.
pub(super) fn refusal(problem: Problem, shell: &Shell) -> &'static str {
    let words = shell.words.terminal_profiles();
    match problem {
        Problem::Refused(ErrorCode::Forbidden | ErrorCode::Unauthorized) => words.forbidden(),
        Problem::TooOld => words.too_old(),
        Problem::Offline | Problem::Refused(_) => words.refused(),
    }
}

mod draw;
pub(super) use draw::dialogs;
