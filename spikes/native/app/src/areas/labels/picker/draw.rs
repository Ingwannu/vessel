//! Existing multi-select combobox; choosing a row never closes the surface.

use std::rc::Rc;
use vessel_app_kit::ui::{AnyElement, Context, Window};
use vessel_app_kit::{
    ComboHandlers, ComboHeader, ComboRow, Leading, Side, Tokens, combobox, combobox_inline, select,
};
use vessel_app_view::labels::{Assignment, Catalog, Label, PickerPage};

use super::{Opening, PickerPlace, availability};
use crate::shell::Shell;

enum Choice {
    Toggle(Label, Assignment),
    Create,
}

pub(super) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let picker = shell.areas.labels.picker.as_ref()?;
    if !picker.visible {
        return None;
    }
    match picker.page {
        PickerPage::Creating(_) | PickerPage::Submitting { .. } => {
            return super::create::overlay(shell, tokens, window, cx);
        }
        PickerPage::Listing => {}
    }
    // Beside its row menu the menu's flyout draws the list ([`beside`]).
    if matches!(picker.place, PickerPlace::Beside) {
        return None;
    }
    let (header, rows, handlers) = list(shell, picker, tokens, cx);
    combobox(
        "workspace-label-picker",
        &shell.areas.labels.picker_combo,
        &header,
        &rows,
        &handlers,
        select::WIDTH,
        tokens,
        window,
        cx,
    )
}

/// The list inside the row menu's Labels flyout (TRACKER 791).
pub(super) fn beside(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let picker = shell.areas.labels.picker.as_ref()?;
    if !picker.visible
        || !matches!(picker.place, PickerPlace::Beside)
        || !matches!(picker.page, PickerPage::Listing)
    {
        return None;
    }
    let (header, rows, handlers) = list(shell, picker, tokens, cx);
    combobox_inline(
        "workspace-label-picker",
        &shell.areas.labels.picker_combo,
        &header,
        &rows,
        &handlers,
        select::WIDTH,
        tokens,
        window,
        cx,
    )
}

/// The picker's list: its title and search, the host's labels checked as
/// assigned, and "Create label"; a phone's page leads back to its menu.
fn list(
    shell: &Shell,
    picker: &super::WorkspacePicker,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> (ComboHeader, Vec<ComboRow>, ComboHandlers<Shell>) {
    let state = &shell.areas.labels;
    let fallback = Catalog::default();
    // Node lookup here resolves cached display colors only, never a connection.
    let catalog = state
        .hosts
        .iter()
        .find(|host| host.key == picker.target.connection().node())
        .map_or(&fallback, |host| &host.catalog);
    let online = availability(state, picker).connected();
    let query = state.picker_combo.query(cx);
    let entries = picker.model.rows(catalog, &picker.assigned, &query);
    let mut choices = Vec::new();
    let mut rows = Vec::new();
    for entry in entries {
        let assignment = match entry.assignment {
            Assignment::Assigned => Assignment::Unassigned,
            Assignment::Unassigned => Assignment::Assigned,
        };
        rows.push(ComboRow {
            leading: Some(Leading::Dot(super::super::colors::foreground(
                entry.label.color,
                tokens,
            ))),
            checked: entry.assignment == Assignment::Assigned,
            disabled: !online || entry.pending.is_some(),
            trailing: entry.pending.map(|_| shell.words.labels().pending().into()),
            ..ComboRow::new(entry.label.name.as_str().to_owned())
        });
        choices.push(Choice::Toggle(entry.label, assignment));
    }
    rows.push(ComboRow {
        disabled: !online,
        opens: true,
        ..ComboRow::new(shell.words.labels().create())
    });
    choices.push(Choice::Create);
    let opening = picker.opening;
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.labels.picker_combo,
        chose: Rc::new(move |shell: &mut Shell, index, window, cx| {
            let Some(choice) = choices.get(index) else {
                return;
            };
            match choice {
                Choice::Toggle(label, assignment) => {
                    shell.labels_picker_toggle(opening, label, *assignment, cx);
                }
                Choice::Create => shell.labels_picker_create(opening, window, cx),
            }
        }),
        back: match &picker.place {
            PickerPlace::Page(back) => Some(Rc::clone(back)),
            PickerPlace::Popover | PickerPlace::Beside => None,
        },
    };
    let words = shell.words.labels();
    let header = ComboHeader {
        title: Some(words.title().into()),
        back: matches!(picker.place, PickerPlace::Page(_)),
        search: true,
        empty: words.no_matches().into(),
        note: note(shell, picker).map(Into::into),
        side: Side::Below,
    };
    (header, rows, handlers)
}

fn note(shell: &Shell, picker: &super::WorkspacePicker) -> Option<&'static str> {
    if let Some(failure) = picker.error.or_else(|| picker.model.error()) {
        return Some(super::super::draw::failure_text(failure, shell));
    }
    if picker.access == vessel_app_view::labels::EditAccess::ReadOnly {
        return Some(shell.words.labels().read_only());
    }
    availability(&shell.areas.labels, picker)
        .writable()
        .err()
        .map(|failure| super::super::draw::failure_text(failure, shell))
}

impl Shell {
    pub(super) fn labels_picker_create(
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
            || !state.picker_combo.is_open()
            || !availability(state, picker).connected()
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
                state.picker_combo.close();
                picker.name = super::name_field(self.words.labels().name(), window, cx);
                picker.applied_editability = None;
                picker.dialog.focus(window, cx);
                picker.name.focus(window, cx);
            }
            Err(failure) => picker.error = Some(failure),
        }
        self.labels_picker_settle(cx);
        cx.notify();
    }
}
