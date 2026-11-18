//! The workspace picker on the existing combobox and adaptive dialog,
//! opened from a session row's menu (Paseo's sidebar Labels page).
//!
//! [Decision Log]
//! - Purpose: assign and create labels for one workspace without inferring
//!   its owner.
//! - Constraint: the workspace is the registry's (a session's membership),
//!   never a directory; the admission is the one the row's host answered.
//! - Choice: the row menu supplies the `WorkspaceTarget`, the registry's
//!   ordered names and the host's access; the followed registry keeps them
//!   current while the picker is open (`follow.rs`).
//! - Tradeoff: a session with no workspace membership (an older one) has no
//!   Labels item.
//! - Place (TRACKER 791): the row menu's Labels page shows the list beside
//!   the menu (`PickerPlace::Beside`, drawn in its flyout, keys with the
//!   menu until a press or Enter opens the page) or, on a phone, as the
//!   canonical picker's sheet with the page's way back (`Page`); the
//!   canonical entry draws it on its own (`Popover`).

mod create;
mod draw;
#[cfg(test)]
mod tests;

use vessel_app_kit::field::FieldId;
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
    /// Where its list is drawn.
    place: PickerPlace,
}

/// Where the workspace picker's list is drawn (TRACKER 791).
#[derive(Clone)]
pub(crate) enum PickerPlace {
    /// Its own popover at a point (a phone's sheet).
    Popover,
    /// The row menu's Labels page beside the menu, drawn in its flyout
    /// ([`super::picker_beside`]).
    Beside,
    /// The row menu's Labels page on a phone: the sheet that replaces the
    /// menu's, its title row leading back with `back`.
    Page(vessel_app_kit::picker::Back<Shell>),
}

/// Where a row menu's Labels picker is, for the menu to follow it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowPicker {
    /// None open beside or as a page of a menu.
    None,
    /// Its list is up.
    Open,
    /// Its Create dialog is up, the list closed behind it.
    Dialog,
    /// Dismissed by the person (Escape, the scrim): its menu closes too.
    Dismissed,
}

impl Shell {
    /// The sole intended entry for canonical workspace row/header owners;
    /// false when it cannot open now (the area's error says why).
    pub(crate) fn labels_picker_open(
        &mut self,
        target: WorkspaceTarget,
        assigned: Vec<Name>,
        access: EditAccess,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        self.labels_picker_start(target, assigned, access, PickerPlace::Popover, window, cx)
    }

    /// The picker at `place`: as a menu's page it is the canonical picker
    /// (its list holds the keys) given the page's way back; beside its
    /// menu the list takes presses and leaves the keys with the menu.
    pub(crate) fn labels_picker_open_in(
        &mut self,
        target: WorkspaceTarget,
        assigned: Vec<Name>,
        access: EditAccess,
        place: PickerPlace,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if matches!(place, PickerPlace::Beside) {
            return self.labels_picker_start(target, assigned, access, place, window, cx);
        }
        let opened = self.labels_picker_open(target, assigned, access, window, cx);
        if let Some(picker) = self.areas.labels.picker.as_mut().filter(|_| opened) {
            picker.place = place;
        }
        opened
    }

    fn labels_picker_start(
        &mut self,
        target: WorkspaceTarget,
        assigned: Vec<Name>,
        access: EditAccess,
        place: PickerPlace,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let state = &mut self.areas.labels;
        // Hiding the manager cannot discard an accepted operation.
        if state.manager.state().busy() {
            state.error = Some(Failure::Pending);
            return false;
        }
        let opening = match state.tickets.issue() {
            Ok(ticket) => Opening::new(ticket),
            Err(failure) => {
                state.error = Some(failure);
                return false;
            }
        };
        state.opening = None;
        let search = Some(self.words.labels().search());
        if matches!(place, PickerPlace::Beside) {
            state.picker_combo.open_beside(search, window, cx);
        } else {
            state.picker_combo.open(search, window, cx);
        }
        let name = name_field("", window, cx);
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
            place,
        });
        cx.notify();
        true
    }

    /// Close a row menu's Labels picker (its page closed).
    pub(crate) fn labels_row_picker_close(&mut self) {
        let state = &mut self.areas.labels;
        if state
            .picker
            .as_ref()
            .is_some_and(|picker| !matches!(picker.place, PickerPlace::Popover))
        {
            state.picker = None;
            state.picker_combo.close();
        }
    }

    /// Move the keys into a row menu's Labels list (its page opened by a
    /// press or Enter): its search holds them, the menu's return on close.
    pub(crate) fn labels_row_picker_keys(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.labels.picker_combo.take_keys(true, window, cx);
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
        // The committed names go to the rows' projection; the picker needs
        // only how it ended.
        let settled = result
            .as_ref()
            .map(|_committed| ())
            .map_err(|failure| *failure);
        let accepted = if creating {
            picker.page.completed(event.ticket, settled)
        } else {
            picker.model.finish(name, event.ticket, settled)
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

fn field_submitted(
    shell: &mut Shell,
    source: FieldId,
    _text: String,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if let Some(picker) = &shell.areas.labels.picker
        && picker.name.id() == source
        && picker.name.focus_handle(cx).is_focused(window)
    {
        let opening = picker.opening;
        shell.labels_picker_submit(opening, cx);
    }
}

fn field_changed(shell: &mut Shell, source: FieldId, text: &str, cx: &mut Context<'_, Shell>) {
    let state = &mut shell.areas.labels;
    let Some(picker) = &state.picker else {
        return;
    };
    if picker.name.id() != source || !picker.visible || !availability(state, picker).connected() {
        return;
    }
    if let Some(picker) = &mut state.picker
        && let Some(draft) = picker.page.draft_mut()
    {
        draft.set_text(text);
        cx.notify();
    }
}

fn name_field(placeholder: &str, window: &mut Window, cx: &mut Context<'_, Shell>) -> TextField {
    TextField::with_source(
        placeholder,
        field_submitted,
        |shell, source, text, cx| field_changed(shell, source, &text, cx),
        window,
        cx,
    )
}

fn next_opening(state: &mut super::Labels) -> Option<Opening> {
    match state.tickets.issue() {
        Ok(ticket) => Some(Opening::new(ticket)),
        Err(failure) => {
            if let Some(picker) = &mut state.picker {
                picker.error = Some(failure);
            }
            None
        }
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

/// The list inside the row menu's Labels flyout (TRACKER 791).
pub(super) fn beside(
    shell: &Shell,
    tokens: &vessel_app_kit::Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    draw::beside(shell, tokens, window, cx)
}

impl super::Labels {
    /// The open picker's target, for the followed registry to refresh.
    pub(super) fn picker_target(&self) -> Option<WorkspaceTarget> {
        self.picker.as_ref().map(|picker| picker.target.clone())
    }

    /// Where a row menu's Labels picker is: none, its list up, its Create
    /// dialog up, or dismissed (hidden, or its list closed on its own).
    pub(crate) fn row_picker(&self) -> RowPicker {
        let Some(picker) = &self.picker else {
            return RowPicker::None;
        };
        if matches!(picker.place, PickerPlace::Popover) {
            return RowPicker::None;
        }
        let listing = matches!(picker.page, PickerPage::Listing);
        if !picker.visible || (listing && !self.picker_combo.is_open()) {
            RowPicker::Dismissed
        } else if listing {
            RowPicker::Open
        } else {
            RowPicker::Dialog
        }
    }
}
