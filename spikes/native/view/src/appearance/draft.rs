//! A local form draft: typing does not mutate saved preferences. The caller
//! renders preview overrides from valid draft data and commits on blur/submit.

/// A local draft's complete state, including its saved fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Draft<T, P = T> {
    /// No uncommitted edit.
    Saved(T),
    /// Typed text with a usable preview.
    Preview { saved: T, text: DraftText, value: P },
    /// Empty/invalid input; the preview uses the saved setting.
    Invalid { saved: T, text: DraftText },
}

/// Raw text is allowed only at this editing boundary, not as a preference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftText(String);

/// What committing the draft does to the saved preference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Commit<T> {
    /// A validated value differs from the saved one.
    Change(T),
    /// Nothing changes; replace invalid or stale draft text with this value.
    Restore(T),
}

impl<T: Clone + Eq, P> Draft<T, P> {
    /// Edit from the retained saved value, not from an earlier preview.
    #[must_use]
    pub fn edit(&self, text: String, parse: fn(&str) -> Option<P>) -> Self {
        let saved = self.saved().clone();
        let value = parse(&text);
        let text = DraftText(text);
        match value {
            Some(value) => Self::Preview { saved, text, value },
            None => Self::Invalid { saved, text },
        }
    }

    /// The actual saved value, independent of local typing.
    #[must_use]
    pub fn saved(&self) -> &T {
        match self {
            Self::Saved(saved) | Self::Preview { saved, .. } | Self::Invalid { saved, .. } => saved,
        }
    }

    /// A valid local preview; invalid text never replaces global settings.
    #[must_use]
    pub fn preview(&self) -> Option<&P> {
        match self {
            Self::Preview { value, .. } => Some(value),
            Self::Saved(_) | Self::Invalid { .. } => None,
        }
    }

    /// Commit on blur/submit; an external saved change replaces this draft.
    #[must_use]
    pub fn commit_with(self, resolve: fn(P) -> Option<T>) -> Commit<T> {
        match self {
            Self::Preview { saved, value, .. } => match resolve(value) {
                Some(next) if next != saved => Commit::Change(next),
                Some(_) | None => Commit::Restore(saved),
            },
            Self::Invalid { saved, .. } | Self::Saved(saved) => Commit::Restore(saved),
        }
    }
}

impl DraftText {
    /// The local text as typed; never serialized as a preference.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_invalid_valid_and_repeated_values_keeps_one_saved_fallback() {
        let saved = Draft::Saved(14_u8);
        let invalid = saved.edit("".into(), |text| text.parse().ok());
        assert_eq!(invalid.preview(), None);
        let valid = invalid.edit("16".into(), |text| text.parse().ok());
        assert_eq!(*valid.saved(), 14);
        assert_eq!(valid.preview(), Some(&16));
        assert_eq!(valid.commit_with(Some), Commit::Change(16));
        assert_eq!(invalid.commit_with(Some), Commit::Restore(14));
        assert_eq!(
            saved
                .edit("14".into(), |text| text.parse().ok())
                .commit_with(Some),
            Commit::Restore(14)
        );
    }
}
