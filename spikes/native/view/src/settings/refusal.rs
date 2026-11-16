//! Project typed preference refusals to whole messages, using the model's bounds.
use crate::localization::{FormatError, Number, SettingsNotices};
use crate::prefs::{MAX_ACTIONS, MAX_MACHINES, Refused};

/// A refusal explanation could not be prepared; never display this diagnostic.
#[derive(Debug, thiserror::Error)]
pub enum RefusalWordsError {
    /// The message could not be formatted in this language or English.
    #[error(transparent)]
    Format(#[from] FormatError),
    /// The model's bound is outside the formatter's exact count range.
    #[error("preference bound exceeds the exact translation count range: {0}")]
    Limit(#[from] std::num::TryFromIntError),
}

impl SettingsNotices<'_> {
    /// Prepare the complete explanation from a closed refusal and its own bound.
    ///
    /// # Errors
    /// Returns a formatting failure or an out-of-range model bound.
    pub fn refusal(self, reason: Refused) -> Result<String, RefusalWordsError> {
        match reason {
            Refused::Actions => Ok(self.actions(Number::count(u32::try_from(MAX_ACTIONS)?))?),
            Refused::Machines => Ok(self.machines(Number::count(u32::try_from(MAX_MACHINES)?))?),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::localization::{Locale, Number, Translator};
    use crate::prefs::Refused;

    #[test]
    fn both_refusal_kinds_have_whole_english_and_korean_explanations() {
        let en = Translator::get(Locale::from_tag("en").unwrap())
            .unwrap()
            .settings_notices();
        let ko = Translator::get(Locale::from_tag("ko").unwrap())
            .unwrap()
            .settings_notices();
        assert_eq!(
            en.refusal(Refused::Actions).unwrap(),
            "That change was not kept. Keys can be kept for at most \u{2068}128\u{2069} actions."
        );
        assert_eq!(
            ko.refusal(Refused::Actions).unwrap(),
            "그 변경은 보관되지 않았습니다. 단축키는 최대 \u{2068}128\u{2069}개 동작에만 보관할 수 있습니다."
        );
        assert_eq!(
            en.refusal(Refused::Machines).unwrap(),
            "That change was not kept. Defaults can be kept for at most \u{2068}32\u{2069} machines."
        );
        assert_eq!(
            ko.refusal(Refused::Machines).unwrap(),
            "그 변경은 보관되지 않았습니다. 기본값은 최대 \u{2068}32\u{2069}대의 머신에 대해서만 보관할 수 있습니다."
        );
    }

    #[test]
    fn english_plural_rules_choose_whole_singular_notices() {
        let words = Translator::get(Locale::from_tag("en").unwrap())
            .unwrap()
            .settings_notices();
        assert_eq!(
            words.actions(Number::count(1)).unwrap(),
            "That change was not kept. Keys can be kept for at most \u{2068}1\u{2069} action."
        );
        assert_eq!(
            words.machines(Number::count(1)).unwrap(),
            "That change was not kept. Defaults can be kept for at most \u{2068}1\u{2069} machine."
        );
    }
}
