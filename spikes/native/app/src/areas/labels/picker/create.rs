//! Create-and-assign shares the manager's one swatch renderer and kit dialog.

use std::rc::Rc;
use vessel_app_kit::dialog::{DialogContent, DialogSize, dialog};
use vessel_app_kit::theme::spacing::S3;
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, button_with_kind};
use vessel_app_link::labels::Command;
use vessel_app_view::labels::{Assignment, EditAccess, PickerPage};

use super::{Opening, availability};
use crate::shell::Shell;

pub(super) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.labels;
    let picker = state.picker.as_ref()?;
    let (draft, pending) = match &picker.page {
        PickerPage::Listing => return None,
        PickerPage::Creating(draft) => (draft, false),
        PickerPage::Submitting { draft, .. } => (draft, true),
    };
    let opening = picker.opening;
    let editable = !pending && availability(state, picker).connected();
    let can_submit = editable && draft.label().is_some();
    let on_choose = color_choice(opening);
    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(S3))
        .child(picker.name.element(tokens, false, cx))
        .child(super::super::colors::choices_for(
            shell,
            draft.color(),
            if editable {
                EditAccess::Editable
            } else {
                EditAccess::ReadOnly
            },
            &on_choose,
            tokens,
            cx,
        ));
    if let Some(failure) = picker.error.or_else(|| draft.error()) {
        body = body.child(super::super::draw::failure_text(failure, shell));
    }
    if picker.access == EditAccess::ReadOnly {
        body = body.child(shell.words.labels().read_only());
    } else if let Err(failure) = availability(state, picker).writable() {
        body = body.child(super::super::draw::failure_text(failure, shell));
    }
    let footer = vec![
        button_with_kind(
            "workspace-label-create-back",
            shell.words.labels().back(),
            ButtonKind::Secondary,
            enabled(!pending),
            tokens,
        )
        .when(!pending, |button| {
            button.on_click(cx.listener(move |shell, _, window, cx| {
                shell.labels_picker_back(opening, window, cx);
            }))
        })
        .into_any_element(),
        button_with_kind(
            "workspace-label-create-submit",
            if pending {
                shell.words.labels().pending()
            } else {
                shell.words.labels().create()
            },
            ButtonKind::Primary,
            enabled(can_submit),
            tokens,
        )
        .when(can_submit, |button| {
            button.on_click(
                cx.listener(move |shell, _, _window, cx| shell.labels_picker_submit(opening, cx)),
            )
        })
        .into_any_element(),
    ];
    Some(dialog(
        &picker.dialog,
        DialogContent {
            title: shell.words.labels().create().into(),
            body: body.into_any_element(),
            footer,
            size: DialogSize::Normal,
        },
        tokens,
        window,
        vessel_app_kit::breakpoint::phone(window),
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
        if picker.opening != opening || !availability(state, picker).connected() {
            return;
        }
        if let Some(picker) = &mut state.picker
            && let Some(draft) = picker.page.draft_mut()
            && draft.color() != color
        {
            draft.set_color(color);
            state.color_turn.advance();
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
        if let Some(picker) = &mut state.picker
            && picker.opening == opening
            && !matches!(picker.page, PickerPage::Submitting { .. })
        {
            picker.page = PickerPage::Listing;
            picker.error = None;
            picker.dialog.dismissed(window, cx);
            state
                .picker_combo
                .open(Some(self.words.labels().search()), window, cx);
        }
        cx.notify();
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

const fn enabled(yes: bool) -> ButtonState {
    if yes {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    }
}
