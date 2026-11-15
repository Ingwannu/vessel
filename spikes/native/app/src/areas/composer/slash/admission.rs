//! Read-only admission for the sole shortcut interceptor, using the live menu.
//! No renderer cache, chosen-row mutation, focus change or inferred route fallback.
use super::{Over, Shell};
use vessel_app_kit::field::composer_hooks::ComposerKey;
use vessel_app_kit::ui::App;
use vessel_app_view::composer::drafts::DraftKey;

impl Shell {
    /// A menu key must reach its existing composer hook before a global action.
    /// This is local ownership only, never permission to send/interrupt a session.
    /// The dispatcher still requires its canonical active composer context.
    pub(crate) fn composer_command_menu_owns_key(
        &self,
        target: DraftKey,
        key: ComposerKey,
        cx: &App,
    ) -> bool {
        if self.composer_shell_field(target).is_none() {
            return false;
        }
        let over = match target {
            DraftKey::New => Over::Draft,
            DraftKey::Session { .. } => Over::Session,
        };
        self.slash_menu(over, cx)
            .is_some_and(|menu| menu.takes_key(key))
    }
}

#[cfg(test)]
mod tests;
