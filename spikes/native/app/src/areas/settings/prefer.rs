//! Changing one preference from Settings and applying what changes at
//! once (split from `mod.rs` for the file budget, docs/QUALITY.md M3).

use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_view::prefs::PreferenceChange;

use super::{apply_composer, apply_text_sizes, apply_theme};
use crate::shell::Shell;

impl Shell {
    /// Change one preference as the page's rules allow, keep it, and show
    /// what changes at once.
    pub(crate) fn prefer(
        &mut self,
        change: &PreferenceChange,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        // Prepare before persisting: a catalog failure leaves the displayed
        // choice and the saved choice in agreement, and never occurs in paint.
        let language = if let PreferenceChange::Language(preference) = change {
            match self.language.prepare(*preference) {
                Ok(words) => Some(words),
                Err(error) => {
                    tracing::error!(%error, "the embedded translations could not change");
                    return;
                }
            }
        } else {
            None
        };
        match self.prefs.prefer(change) {
            Ok(()) => {
                self.areas.settings.refused = None;
                self.command(Command::SavePrefs(Box::new(self.prefs.clone())));
                match change {
                    PreferenceChange::Language(_) => {
                        if let Some(words) = language {
                            self.apply_language(words, window, cx);
                        }
                    }
                    PreferenceChange::Theme(_) => apply_theme(self, window, cx),
                    PreferenceChange::Send(_) => apply_composer(self, window, cx),
                    PreferenceChange::InterfaceSize(_) | PreferenceChange::CodeSize(_) => {
                        apply_text_sizes(self, cx);
                    }
                    PreferenceChange::ContentWidth(width) => {
                        crate::areas::session::set_content_width(*width);
                        let list = &self.areas.conversation.list;
                        list.changed(0..list.rows());
                    }
                    PreferenceChange::Conversation(display) => {
                        self.conversation_display(*display);
                    }
                    PreferenceChange::Animations(animations) => {
                        crate::motion::animations(animations.on(), cx);
                        window.refresh();
                    }
                    // Read where they are drawn, on the next frame.
                    PreferenceChange::SidebarPanel(_)
                    | PreferenceChange::InspectorPanel(_)
                    | PreferenceChange::Delivery(_)
                    | PreferenceChange::Alerts(_)
                    | PreferenceChange::TitleCount(_)
                    | PreferenceChange::SidebarGroup(_)
                    | PreferenceChange::SidebarSort(_)
                    | PreferenceChange::SidebarEnded(_)
                    | PreferenceChange::Shortcuts { .. }
                    | PreferenceChange::ResetShortcuts
                    | PreferenceChange::DefaultAgent { .. }
                    | PreferenceChange::AgentDefaults { .. }
                    | PreferenceChange::ForgetDefaults { .. } => {}
                }
            }
            Err(refused) => {
                self.areas.settings.refused =
                    Some(super::PreparedRefusal::new(self.words, refused));
            }
        }
        cx.notify();
    }
}
