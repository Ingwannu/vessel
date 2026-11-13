//! Renaming a session where its title stands (the page's
//! `features/session/session-title.tsx`, protocol 9): a click on the
//! title turns it into a field holding the title; Enter or leaving the
//! field saves, Escape keeps the old title. The title is checked with
//! the node's own rules first (`view::session::check_title`), so a
//! refusal is said in the bar at once; the node still decides, and its
//! answer (the session as it now stands) names the bar.
//!
//! Taking the edit out of the area is what ends it: Enter and Escape take
//! it, so the blur that follows when the field leaves the window finds
//! nothing to save a second time (the page's `ended` flag).

use vessel_app_kit::TextField;
use vessel_app_kit::ui::{Context, Subscription, Window};
use vessel_app_link::{Act, ActKind};
use vessel_app_view::session::check_title;

use crate::shell::Shell;

/// A title being edited.
pub(crate) struct Rename {
    /// The field, holding the title as it is typed.
    pub(crate) field: TextField,
    /// What the title was when the edit began.
    was: String,
    /// Saves when the field loses the keys; dropped with the edit.
    _left: Subscription,
}

impl std::fmt::Debug for Rename {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rename")
            .field("was", &self.was)
            .finish_non_exhaustive()
    }
}

impl Shell {
    /// Turn the title (`shown`) into a field.
    pub(super) fn start_rename(
        &mut self,
        shown: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let words = self.t.lang.conversation();
        let field = TextField::new(words.title_field(), Self::rename_entered, window, cx);
        field.set_text(shown, window, cx);
        field.focus(window, cx);
        let left = cx.on_blur(&field.focus_handle(cx), window, |shell, _window, cx| {
            shell.rename_left(cx);
        });
        // A new edit starts clean: the last refusal was about another.
        self.areas.conversation.refused = None;
        self.areas.conversation.rename = Some(Rename {
            field,
            was: shown.to_owned(),
            _left: left,
        });
        cx.notify();
    }

    /// Escape: the title stays.
    pub(super) fn cancel_rename(&mut self, cx: &mut Context<'_, Self>) {
        if self.areas.conversation.rename.take().is_some() {
            cx.notify();
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "the field's submit callback hands the text over by value"
    )]
    fn rename_entered(&mut self, text: String, _window: &mut Window, cx: &mut Context<'_, Self>) {
        self.save_rename(&text, cx);
    }

    fn rename_left(&mut self, cx: &mut Context<'_, Self>) {
        let Some(text) = self
            .areas
            .conversation
            .rename
            .as_ref()
            .map(|rename| rename.field.text(cx))
        else {
            return;
        };
        self.save_rename(&text, cx);
    }

    /// Save `text` as the title: said in the bar when it cannot be one,
    /// nothing asked when it is the title already.
    fn save_rename(&mut self, text: &str, cx: &mut Context<'_, Self>) {
        let Some(rename) = self.areas.conversation.rename.take() else {
            return;
        };
        match check_title(text) {
            Err(why) => {
                let words = self.t.lang.conversation();
                self.areas.conversation.refused = Some((ActKind::Rename, words.title_refused(why)));
            }
            Ok(title) if title.as_str() == rename.was => {}
            Ok(title) => {
                self.areas.conversation.refused = None;
                self.act(Act::Rename(title));
            }
        }
        cx.notify();
    }
}
