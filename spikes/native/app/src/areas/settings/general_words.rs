//! General's retained projection: ready before the first frame, rebuilt only
//! when its language or typed storage facts change. No I/O or lazy paint path.

use vessel_app_kit::ui::{ClipboardItem, Context, SharedString};
use vessel_app_view::localization::Translator;
use vessel_app_view::prefs::Kept;

use crate::shell::Shell;

/// A complete prepared hint, including a localized formatting-error fallback.
pub(crate) struct PreparedGeneral {
    pub(super) hint: SharedString,
}

impl PreparedGeneral {
    pub(crate) fn new(words: &'static Translator, kept: &Kept) -> Self {
        Self {
            hint: hint(words, kept),
        }
    }

    fn refresh(&mut self, words: &'static Translator, kept: &Kept) {
        self.hint = hint(words, kept);
    }
}

fn hint(words: &Translator, kept: &Kept) -> SharedString {
    words.general().storage(kept).map_or_else(
        |_| words.general().storage_unavailable().to_owned().into(),
        Into::into,
    )
}

impl Shell {
    /// Code that replaces `prefs.kept` after startup must call this too. Today
    /// those facts are launch-stable; language dispatch is the production caller.
    pub(crate) fn refresh_general_words(&mut self) {
        self.areas
            .settings
            .general
            .refresh(self.words, &self.prefs.kept);
    }

    /// Copy the preferences path, then the copied toast in Paseo's words
    /// ("Copied Preferences path", its copiedLabel pattern; TRACKER 804).
    pub(super) fn copy_preferences_path(&mut self, path: &str, cx: &mut Context<'_, Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(path.to_owned()));
        self.toast_copied(self.words.general().copied_path_label(), cx);
    }
}

#[cfg(test)]
mod tests;
