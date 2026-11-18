//! Renaming a session in a dialog, as Paseo's `AdaptiveRenameModal`
//! (`components/rename-modal.tsx`, opened by `use-workspace-tab-rename.tsx`):
//! "Rename agent", the title in a field, prefilled and selected, then
//! Cancel and Rename side by side. A card on a wide window, a sheet from
//! the bottom on a phone (the kit dialog).
//!
//! [Decision Log]
//! - Purpose: TRACKER 789 (coordinator's return, 2026-11-18). The rename
//!   was a field where the title stands (the page's `session-title.tsx`);
//!   in the compact header that field overlapped the line under the title,
//!   and Paseo renames through its modal at every width.
//! - Choice: one way in (`start_rename`) for both widths: a press on the
//!   bar's title, and the compact switcher's row menu Rename. The title is
//!   checked with the node's own rules first (`view::session::check_title`):
//!   a title it cannot be keeps the dialog open with the reason under the
//!   field, as Paseo's `validate` error does. Rename is disabled while the
//!   text is the title already or cannot be one (Paseo's `submitDisabled`).
//! - Values (Paseo's): the body's parts 12 apart and 8 above its foot; the
//!   two buttons flex 1 each, 8 apart, secondary Cancel, primary Rename.
//! - Cost: Paseo waits for the save ("Saving…") and closes on success. The
//!   rename goes to the node as before and the dialog closes when it is
//!   sent; a node's refusal shows where it did, under the session's title.
//!   Leaving the field no longer saves (a dialog has Cancel).

use vessel_app_kit::theme::spacing::{S2, S3};
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, ControlSizing, DialogContent, DialogSize, TextField, Tokens, dialog,
};
use vessel_app_link::Act;
use vessel_app_view::session::check_title;

use crate::shell::Shell;

/// A title being edited.
pub(crate) struct Rename {
    /// The field, holding the title as it is typed.
    pub(crate) field: TextField,
    /// What the title was when the edit began.
    was: String,
    /// Why the typed text cannot be a title, once Rename was asked.
    invalid: Option<String>,
}

impl std::fmt::Debug for Rename {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rename")
            .field("was", &self.was)
            .finish_non_exhaustive()
    }
}

impl Shell {
    /// Open the rename dialog for the title as shown (`shown`), its text
    /// selected.
    pub(super) fn start_rename(
        &mut self,
        shown: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let words = self.t.lang.conversation();
        let field = TextField::with_change(
            words.title_field(),
            Self::rename_entered,
            |shell: &mut Self, _text, cx| {
                // Typing clears the last reason and updates Rename.
                if let Some(rename) = shell.areas.conversation.rename.as_mut() {
                    rename.invalid = None;
                }
                cx.notify();
            },
            window,
            cx,
        );
        field.set_text(shown, window, cx);
        self.areas.conversation.rename_dialog.focus(window, cx);
        field.focus(window, cx);
        field.select_all(window, cx);
        // A new edit starts clean: the last refusal was about another.
        self.areas.conversation.refused = None;
        self.areas.conversation.rename = Some(Rename {
            field,
            was: shown.to_owned(),
            invalid: None,
        });
        cx.notify();
    }

    /// Cancel, Escape or the scrim: the title stays.
    pub(super) fn cancel_rename(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.areas.conversation.rename.take().is_some() {
            self.areas.conversation.rename_dialog.dismissed(window, cx);
            cx.notify();
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "the field's submit callback hands the text over by value"
    )]
    fn rename_entered(&mut self, text: String, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.save_rename(&text, window, cx);
    }

    fn rename_pressed(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(text) = self
            .areas
            .conversation
            .rename
            .as_ref()
            .map(|rename| rename.field.text(cx))
        else {
            return;
        };
        self.save_rename(&text, window, cx);
    }

    /// Save `text` as the title: the dialog stays with the reason when it
    /// cannot be one, and nothing is asked when it is the title already.
    fn save_rename(&mut self, text: &str, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(rename) = self.areas.conversation.rename.as_mut() else {
            return;
        };
        match check_title(text) {
            Err(why) => {
                let words = self.t.lang.conversation();
                rename.invalid = Some(words.title_refused(why));
            }
            Ok(title) if title.as_str() == rename.was => {}
            Ok(title) => {
                self.areas.conversation.rename = None;
                self.areas.conversation.rename_dialog.dismissed(window, cx);
                self.areas.conversation.refused = None;
                self.act(Act::Rename(title));
            }
        }
        cx.notify();
    }
}

/// Whether Rename may be pressed: the text is a title, and another one.
fn savable(rename: &Rename, text: &str) -> bool {
    check_title(text).is_ok_and(|title| title.as_str() != rename.was)
}

/// The dialog, while a rename is open.
pub(super) fn rename_dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let rename = shell.areas.conversation.rename.as_ref()?;
    let words = shell.words.layout();
    let narrow = vessel_app_kit::breakpoint::phone(window);
    let sizing = ControlSizing::from_compact(narrow);
    let text = rename.field.text(cx);
    let cancel = vessel_app_kit::button_with_kind_at(
        "session-rename-cancel",
        words.cancel().to_owned(),
        ButtonKind::Secondary,
        ButtonState::Enabled,
        sizing,
        tokens,
    )
    .flex_1()
    .on_click(cx.listener(|shell, _event, window, cx| shell.cancel_rename(window, cx)));
    let save_state = if savable(rename, &text) {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    };
    let save = vessel_app_kit::button_with_kind_at(
        "session-rename-save",
        words.rename_save().to_owned(),
        ButtonKind::Primary,
        save_state,
        sizing,
        tokens,
    )
    .flex_1();
    let save = if save_state == ButtonState::Enabled {
        save.on_click(cx.listener(|shell, _event, window, cx| shell.rename_pressed(window, cx)))
    } else {
        save
    };
    let body = div()
        .flex()
        .flex_col()
        .gap(px(S3))
        .pb(px(S2))
        .child(rename.field.element(tokens, narrow, cx))
        .children(rename.invalid.clone().map(|why| {
            div()
                .text_color(tokens.neutral.destructive)
                .child(why)
        }))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(S2))
                .child(cancel)
                .child(save),
        );
    Some(dialog(
        &shell.areas.conversation.rename_dialog,
        DialogContent {
            title: words.rename_agent().to_owned().into(),
            body: body.into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        Shell::cancel_rename,
        cx,
    ))
}

#[cfg(test)]
mod tests;
