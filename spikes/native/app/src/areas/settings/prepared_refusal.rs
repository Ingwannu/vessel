//! Retain a Settings refusal's cause so language changes cannot leave an
//! English Display diagnostic inside a Korean or pseudo Settings page.
use vessel_app_kit::ui::{Context, SharedString};
use vessel_app_view::localization::Translator;
use vessel_app_view::prefs::Refused;
use vessel_app_view::sidebar::navigation::TooMany;

use crate::shell::Shell;

// Sidebar editing can refuse before a PreferenceChange exists. Retain that
// typed cause beside model refusals so the same language refresh re-prepares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reason {
    Preference(Refused),
    SidebarNavigation(TooMany),
}

pub(crate) struct PreparedRefusal {
    reason: Reason,
    pub(super) words: SharedString,
}

impl PreparedRefusal {
    pub(crate) fn new(catalog: &Translator, reason: Refused) -> Self {
        Self::prepare(catalog, Reason::Preference(reason))
    }

    fn prepare(catalog: &Translator, reason: Reason) -> Self {
        let words = match reason {
            Reason::Preference(refused) => catalog.settings_notices().refusal(refused).map_or_else(
                |_| {
                    catalog
                        .settings_notices()
                        .change_not_kept()
                        .to_owned()
                        .into()
                },
                Into::into,
            ),
            Reason::SidebarNavigation(_) => catalog.sidebar().navigation_limit().to_owned().into(),
        };
        Self { reason, words }
    }
}

impl Shell {
    /// Keep pre-change sidebar capacity failures in the same prepared store;
    /// no order, visibility or preference writer runs for a refused edit.
    pub(crate) fn sidebar_navigation_refused(
        &mut self,
        reason: TooMany,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.settings.refused = Some(PreparedRefusal::prepare(
            self.words,
            Reason::SidebarNavigation(reason),
        ));
        cx.notify();
    }

    pub(crate) fn refresh_preference_refusal(&mut self) {
        if let Some(previous) = &self.areas.settings.refused {
            self.areas.settings.refused =
                Some(PreparedRefusal::prepare(self.words, previous.reason));
        }
    }
}

#[cfg(test)]
mod tests;
