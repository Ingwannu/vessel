//! Staged workspace picker using the existing combobox and adaptive dialog.
//!
//! [Decision Log]
//! - Purpose: prepare the whole workflow without inferring a workspace owner.
//! - Constraint: the reviewed native registry row/header caller is not present.
//! - Choice: literal cfg(test) registration until that real caller can supply
//!   WorkspaceTarget, ordered names and active/read-only policy. No fake route.
//! - Tradeoff: tests compile/render this adapter; it is not a shipped entry yet.

mod create;
mod draw;
#[cfg(test)]
mod tests;

use vessel_app_kit::ui::{AnyElement, Context, Window};
use vessel_app_kit::{DialogState, TextField};
use vessel_app_link::labels::{Answer, Command, Event};
use vessel_app_view::labels::{
    Assignment, EditAccess, Failure, HostState, Label, Name, Picker, PickerPage, WorkspaceTarget,
};

use super::surface::Opening;
use crate::shell::Shell;

pub(super) struct WorkspacePicker {
    visible: bool,
    return_to_list: std::cell::Cell<bool>,
    opening: Opening,
    target: WorkspaceTarget,
    assigned: Vec<Name>,
    access: EditAccess,
    model: Picker,
    page: PickerPage,
    name: TextField,
    dialog: DialogState,
    error: Option<Failure>,
    applied_editability: Option<EditAccess>,
}

impl Shell {
    /// The sole intended entry for canonical workspace row/header owners.
    pub(crate) fn labels_picker_open(
        &mut self,
        target: WorkspaceTarget,
        assigned: Vec<Name>,
        access: EditAccess,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.labels;
        // Hiding the manager cannot discard an accepted operation.
        if state.manager.state().busy() {
            state.error = Some(Failure::Pending);
            return;
        }
        let opening = match state.tickets.issue() {
            Ok(ticket) => Opening::new(ticket),
            Err(failure) => {
                state.error = Some(failure);
                return;
            }
        };
        state.opening = None;
        state
            .picker_combo
            .open(Some(self.words.labels().search()), window, cx);
        let name = TextField::with_change(
            "",
            |shell: &mut Shell, _text, window, cx| {
                if let Some(picker) = &shell.areas.labels.picker
                    && picker.name.focus_handle(cx).is_focused(window)
                {
                    let opening = picker.opening;
                    shell.labels_picker_submit(opening, cx);
                }
            },
            |shell, text, cx| {
                if let Some(picker) = &mut shell.areas.labels.picker
                    && let Some(draft) = picker.page.draft_mut()
                {
                    draft.set_text(&text);
                    cx.notify();
                }
            },
            window,
            cx,
        );
        state.picker = Some(WorkspacePicker {
            visible: true,
            return_to_list: std::cell::Cell::new(false),
            opening,
            target,
            assigned,
            access,
            model: Picker::default(),
            page: PickerPage::Listing,
            name,
            dialog: DialogState::new(cx),
            error: None,
            applied_editability: None,
        });
        cx.notify();
    }

    /// Registry projection is authoritative; unary assignment replies only settle.
    pub(crate) fn labels_picker_sync(
        &mut self,
        target: &WorkspaceTarget,
        assigned: Vec<Name>,
        access: EditAccess,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(picker) = &mut self.areas.labels.picker
            && &picker.target == target
        {
            picker.assigned = assigned;
            picker.access = access;
            cx.notify();
        }
        self.labels_picker_settle(cx);
    }

    pub(super) fn labels_picker_settle(&mut self, cx: &mut Context<'_, Self>) {
        let editable = self.areas.labels.picker.as_ref().is_some_and(|picker| {
            availability(&self.areas.labels, picker).connected()
                && matches!(picker.page, PickerPage::Creating(_))
        });
        let policy = if editable {
            EditAccess::Editable
        } else {
            EditAccess::ReadOnly
        };
        if let Some(picker) = &mut self.areas.labels.picker
            && picker.applied_editability != Some(policy)
        {
            picker.name.set_editability(
                match policy {
                    EditAccess::Editable => vessel_app_kit::Editability::Editable,
                    EditAccess::ReadOnly => vessel_app_kit::Editability::ReadOnly,
                },
                cx,
            );
            picker.applied_editability = Some(policy);
        }
    }

    fn labels_picker_toggle(
        &mut self,
        opening: Opening,
        label: &Label,
        assignment: Assignment,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.labels;
        let Some(picker) = &state.picker else {
            return;
        };
        if picker.opening != opening
            || !state.picker_combo.is_open()
            || !matches!(picker.page, PickerPage::Listing)
        {
            return;
        }
        let availability = availability(state, picker);
        let context = picker.target.connection().clone();
        let workspace = picker.target.workspace();
        let ticket = match state.tickets.issue() {
            Ok(ticket) => ticket,
            Err(failure) => {
                state.error = Some(failure);
                return;
            }
        };
        let Some(picker) = state.picker.as_mut() else {
            return;
        };
        picker.error = None;
        match picker.model.begin(availability, &label.name, ticket) {
            Ok(()) => self.command(vessel_app_link::Command::Labels(
                vessel_app_link::labels::Request {
                    context,
                    ticket,
                    ask: Command::Assign {
                        workspace,
                        label: label.clone(),
                        assignment,
                    },
                },
            )),
            Err(Failure::Pending) => {}
            Err(failure) => picker.error = Some(failure),
        }
        cx.notify();
    }

    pub(super) fn labels_picker_answer(
        &mut self,
        event: &Event,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let Some(picker) = &mut self.areas.labels.picker else {
            return false;
        };
        let Answer::Assigned {
            workspace,
            name,
            result,
        } = &event.answer
        else {
            return false;
        };
        if event.context != *picker.target.connection() || *workspace != picker.target.workspace() {
            return false;
        }
        let creating =
            matches!(picker.page, PickerPage::Submitting {ticket, ..} if ticket == event.ticket);
        let accepted = if creating {
            picker.page.completed(event.ticket, *result)
        } else {
            picker.model.finish(name, event.ticket, *result)
        };
        if accepted && creating && result.is_ok() {
            picker.return_to_list.set(true);
        }
        if accepted {
            self.labels_picker_settle(cx);
            cx.notify();
        }
        accepted
    }
}

fn availability(state: &super::Labels, picker: &WorkspacePicker) -> HostState {
    if picker.access == EditAccess::ReadOnly {
        return HostState::Offline;
    }
    state
        .hosts
        .iter()
        .find(|host| host.connection.as_ref() == Some(picker.target.connection()))
        .map_or(HostState::Offline, |host| host.state)
}

pub(super) fn overlay(
    shell: &Shell,
    tokens: &vessel_app_kit::Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if let Some(picker) = &shell.areas.labels.picker
        && picker.visible
        && picker.return_to_list.replace(false)
    {
        let opening = picker.opening;
        cx.defer_in(window, move |shell, window, cx| {
            shell.labels_picker_back(opening, window, cx);
        });
    }
    draw::overlay(shell, tokens, window, cx)
}
