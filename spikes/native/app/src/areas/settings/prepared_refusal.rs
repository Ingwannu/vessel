//! Retain a preference refusal's cause so language changes cannot leave an
//! English Display diagnostic inside a Korean or pseudo Settings page.
use vessel_app_kit::ui::SharedString;
use vessel_app_view::localization::Translator;
use vessel_app_view::prefs::Refused;

use crate::shell::Shell;

pub(crate) struct PreparedRefusal {
    reason: Refused,
    pub(super) words: SharedString,
}

impl PreparedRefusal {
    pub(crate) fn new(catalog: &Translator, reason: Refused) -> Self {
        Self {
            reason,
            words: catalog.settings_notices().refusal(reason).map_or_else(
                |_| {
                    catalog
                        .settings_notices()
                        .change_not_kept()
                        .to_owned()
                        .into()
                },
                Into::into,
            ),
        }
    }
}

impl Shell {
    pub(crate) fn refresh_preference_refusal(&mut self) {
        if let Some(previous) = &self.areas.settings.refused {
            self.areas.settings.refused = Some(PreparedRefusal::new(self.words, previous.reason));
        }
    }
}

#[cfg(test)]
mod tests;
