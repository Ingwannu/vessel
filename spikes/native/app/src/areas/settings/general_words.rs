//! General's retained projection: ready before the first frame, rebuilt only
//! when its language or typed storage facts change. No I/O or lazy paint path.

use vessel_app_kit::ui::{ClipboardItem, Context, SharedString};
use vessel_app_view::localization::Translator;
use vessel_app_view::prefs::Kept;

use crate::shell::Shell;

/// A complete prepared hint, including a localized formatting-error fallback.
pub(crate) struct PreparedGeneral {
    pub(super) hint: SharedString,
    copied_notice: Option<&'static str>,
}

impl PreparedGeneral {
    pub(crate) fn new(words: &'static Translator, kept: &Kept) -> Self {
        Self {
            hint: hint(words, kept),
            copied_notice: None,
        }
    }

    fn refresh(&mut self, words: &'static Translator, kept: &Kept, notice: &mut Option<String>) {
        self.hint = hint(words, kept);
        if let Some(previous) = self.copied_notice {
            let copied = words.general().path_copied();
            if notice.as_deref() == Some(previous) {
                *notice = Some(copied.to_owned());
            }
            self.copied_notice = Some(copied);
        }
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
            .refresh(self.words, &self.prefs.kept, &mut self.notice);
    }

    pub(super) fn copy_preferences_path(&mut self, path: &str, cx: &mut Context<'_, Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(path.to_owned()));
        let copied = self.words.general().path_copied();
        self.areas.settings.general.copied_notice = Some(copied);
        self.notice = Some(copied.to_owned());
        cx.notify();
    }
}

#[cfg(test)]
mod tests;
