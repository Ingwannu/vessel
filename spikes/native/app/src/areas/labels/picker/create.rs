//! Create label as a page of the Labels menu (TRACKER 792; Paseo's
//! `WorkspaceLabelCreatePage`, `workspace-labels/picker.tsx`): beside the
//! row menu, a flyout at the Labels list's Create label row ([`beside`]);
//! on a phone, and for the picker with no menu around it, a sheet titled
//! Create label whose header leads back to the list ([`overlay`]). Both
//! draw the kit's form page: the name in a menu field, the swatches on the
//! rows' rail, Create, and a failure under it. The draft, its submit and
//! the host's answer are the picker's as before.

use std::rc::Rc;
use vessel_app_kit::dialog::{DialogContent, DialogSize, HeaderBack, dialog};
use vessel_app_kit::menu::{FormPage, ItemStatus, MenuItem, ROW_TEXT_INSET, form_flyout, form_rows};
use vessel_app_kit::theme::spacing::S2;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, KeyDownEvent,
    ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, select};
use vessel_app_link::labels::Command;
use vessel_app_view::labels::{Assignment, EditAccess, PickerPage};

use super::{Opening, PickerPlace, WorkspacePicker, availability};
use crate::shell::Shell;

/// The page's id among the row menu's pages (opened from the Labels page,
/// press only: a page you type into is not one the pointer opens).
pub(crate) const CREATE: &str = "labels-create";

/// The page's rows, while there is a draft.
fn rows(
    shell: &Shell,
    picker: &WorkspacePicker,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    let state = &shell.areas.labels;
    let (draft, pending) = match &picker.page {
        PickerPage::Listing => return None,
        PickerPage::Creating(draft) => (draft, false),
        PickerPage::Submitting { draft, .. } => (draft, true),
    };
    let opening = picker.opening;
    let connected = availability(state, picker).connected();
    let words = shell.words.labels();
    // Paseo's Create row: disabled while offline or the name is empty;
    // "Creating…" with the spinner while the host answers.
    let mut action = MenuItem::new(words.create_confirm().to_owned());
    action.disabled = !connected || draft.label().is_none();
    if pending {
        action.status = ItemStatus::Pending(words.creating().to_owned().into());
    }
    let editability = if !pending && connected {
        EditAccess::Editable
    } else {
        EditAccess::ReadOnly
    };
    let swatches = div()
        .px(px(ROW_TEXT_INSET))
        .py(px(S2))
        .child(super::super::colors::choices_for(
            shell,
            draft.color(),
            editability,
            &color_choice(opening),
            tokens,
            cx,
        ));
    let page = FormPage::new(&picker.name, action)
        .with_body(swatches)
        .with_hint(hint(shell, picker).map(Into::into));
    let act: Rc<dyn Fn(&mut Shell, &mut Window, &mut Context<'_, Shell>)> =
        Rc::new(move |shell, _window, cx| shell.labels_picker_submit(opening, cx));
    Some(
        form_rows(page, act, narrow, tokens, cx)
            .debug_selector(|| "workspace-label-create".to_owned())
            // Escape returns to the list, as the sheet's back and Paseo's
            // `goBack` do.
            .on_key_down(cx.listener(move |shell, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    shell.labels_picker_back(opening, window, cx);
                    cx.stop_propagation();
                }
            })),
    )
}

/// What shows under Create: a failure, else why nothing can change.
fn hint(shell: &Shell, picker: &WorkspacePicker) -> Option<&'static str> {
    let draft_error = match &picker.page {
        PickerPage::Creating(draft) | PickerPage::Submitting { draft, .. } => draft.error(),
        PickerPage::Listing => None,
    };
    if let Some(failure) = picker.error.or(draft_error) {
        return Some(super::super::draw::failure_text(failure, shell));
    }
    if picker.access == EditAccess::ReadOnly {
        return Some(shell.words.labels().read_only());
    }
    availability(&shell.areas.labels, picker)
        .writable()
        .err()
        .map(|failure| super::super::draw::failure_text(failure, shell))
}

/// The page beside the row menu: its flyout's panel, as wide as the list.
pub(super) fn beside(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let picker = shell.areas.labels.picker.as_ref()?;
    if !picker.visible || !matches!(picker.place, PickerPlace::Beside) {
        return None;
    }
    let rows = rows(shell, picker, tokens, false, cx)?;
    Some(form_flyout("workspace-label-create-page", rows, px(select::WIDTH), tokens).into_any_element())
}

/// The page as a sheet (a phone, or the picker with no menu around it):
/// titled Create label, its header leading back to the list.
pub(super) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let picker = shell.areas.labels.picker.as_ref()?;
    if matches!(picker.place, PickerPlace::Beside) {
        return None;
    }
    let narrow = vessel_app_kit::breakpoint::phone(window);
    let rows = rows(shell, picker, tokens, narrow, cx)?;
    let opening = picker.opening;
    let content = DialogContent {
        title: shell.words.labels().create().into(),
        body: rows.into_any_element(),
        footer: Vec::new(),
        size: DialogSize::Normal,
    }
    .with_header_back(
        HeaderBack::new(cx.listener(move |shell, _event, window, cx| {
            shell.labels_picker_back(opening, window, cx);
        }))
        .with_label(shell.words.labels().back()),
    );
    Some(dialog(
        &picker.dialog,
        content,
        tokens,
        window,
        narrow,
        move |shell, window, cx| shell.labels_picker_close(opening, window, cx),
        cx,
    ))
}

fn color_choice(opening: Opening) -> super::super::colors::ColorChoice {
    Rc::new(move |shell, color, cx| {
        let state = &mut shell.areas.labels;
        let Some(picker) = &state.picker else {
            return;
        };
        if picker.opening != opening || !picker.visible || !availability(state, picker).connected()
        {
            return;
        }
        if let Some(picker) = &mut state.picker
            && let Some(draft) = picker.page.draft_mut()
            && draft.color() != color
        {
            draft.set_color(color);
            cx.notify();
        }
    })
}

impl Shell {
    pub(super) fn labels_picker_submit(&mut self, opening: Opening, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.labels;
        let Some(picker) = &state.picker else {
            return;
        };
        if picker.opening != opening
            || !picker.visible
            || !matches!(picker.page, PickerPage::Creating(_))
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
        let Some(picker) = &mut state.picker else {
            return;
        };
        match picker.page.submit(availability, ticket) {
            Ok(label) => {
                picker.error = None;
                self.command(vessel_app_link::Command::Labels(
                    vessel_app_link::labels::Request {
                        context,
                        ticket,
                        ask: Command::Assign {
                            workspace,
                            label,
                            assignment: Assignment::Assigned,
                        },
                    },
                ));
            }
            Err(failure) => picker.error = Some(failure),
        }
        self.labels_picker_settle(cx);
        cx.notify();
    }

    pub(super) fn labels_picker_back(
        &mut self,
        opening: Opening,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.labels;
        let Some(picker) = &state.picker else {
            return;
        };
        if picker.opening != opening
            || !picker.visible
            || matches!(picker.page, PickerPage::Submitting { .. })
        {
            return;
        }
        let Some(next) = super::next_opening(state) else {
            cx.notify();
            return;
        };
        if let Some(picker) = &mut state.picker {
            picker.opening = next;
            picker.page = PickerPage::Listing;
            picker.error = None;
            picker.dialog.dismissed(window, cx);
            // Beside the row menu the list stayed open under its page: the
            // keys go back to it; a sheet's list opens again.
            if state.picker_combo.is_open() {
                state.picker_combo.take_keys(true, window, cx);
            } else {
                state
                    .picker_combo
                    .open(Some(self.words.labels().search()), window, cx);
            }
        }
        cx.notify();
    }

    /// The row menu opened the Create label page beside the list (a press
    /// or Enter on its row): a fresh draft whose name holds the keys; the
    /// list stays open beside it.
    pub(crate) fn labels_row_picker_create(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.labels;
        let Some(picker) = &state.picker else {
            return;
        };
        if !picker.visible
            || !matches!(picker.place, PickerPlace::Beside)
            || !matches!(picker.page, PickerPage::Listing)
        {
            return;
        }
        let Some(next) = super::next_opening(state) else {
            cx.notify();
            return;
        };
        let Some(picker) = &mut state.picker else {
            return;
        };
        match picker.page.create() {
            Ok(()) => {
                picker.opening = next;
                picker.error = None;
                picker.name = super::name_field(self.words.labels().name(), window, cx);
                picker.applied_editability = None;
                picker.name.focus(window, cx);
            }
            Err(failure) => picker.error = Some(failure),
        }
        self.labels_picker_settle(cx);
        cx.notify();
    }

    /// The row menu's Create label page closed while its draft was open
    /// (a press outside it, the menu's Escape): back to the list.
    pub(crate) fn labels_row_picker_create_closed(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(picker) = &self.areas.labels.picker
            && matches!(picker.page, PickerPage::Creating(_))
        {
            let opening = picker.opening;
            self.labels_picker_back(opening, window, cx);
        }
    }

    pub(super) fn labels_picker_close(
        &mut self,
        opening: Opening,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(picker) = &mut self.areas.labels.picker
            && picker.opening == opening
        {
            picker.visible = false;
            picker.dialog.dismissed(window, cx);
        }
        cx.notify();
    }
}
