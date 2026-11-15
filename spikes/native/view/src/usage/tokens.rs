//! Reported token fields retain absence and exact integer counts.
use crate::{grouped, localization::Translator};
use vessel_proto::{TokenCount, TurnUsage};

/// Independent fields are never added together by the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenField {
    /// The agent's reported total, not a reconstructed sum.
    Total,
    /// Reported input.
    Input,
    /// Reported output.
    Output,
    /// Optional reasoning.
    Thought,
    /// Optional cache reads.
    CachedRead,
    /// Optional cache writes.
    CachedWrite,
}
impl TokenField {
    /// Stable order in the settings report.
    pub const ALL: [Self; 6] = [
        Self::Total,
        Self::Input,
        Self::Output,
        Self::Thought,
        Self::CachedRead,
        Self::CachedWrite,
    ];

    /// Whole translated labels are shared by all surfaces.
    #[must_use]
    pub fn label(self, words: &'static Translator) -> &'static str {
        let text = words.usage();
        match self {
            Self::Total => text.total_tokens(),
            Self::Input => text.input_tokens(),
            Self::Output => text.output_tokens(),
            Self::Thought => text.thought_tokens(),
            Self::CachedRead => text.cached_read(),
            Self::CachedWrite => text.cached_write(),
        }
    }
    /// Missing optional fields stay absent even when other fields were reported.
    #[must_use]
    pub fn count(self, usage: Option<&TurnUsage>) -> Option<TokenCount> {
        let usage = usage?;
        match self {
            Self::Total => Some(usage.total),
            Self::Input => Some(usage.input),
            Self::Output => Some(usage.output),
            Self::Thought => usage.thought,
            Self::CachedRead => usage.cached_read,
            Self::CachedWrite => usage.cached_write,
        }
    }
}

/// Full integer precision is cheap in Rust and avoids the page's unsafe-number gap.
#[must_use]
pub fn token_count(value: Option<TokenCount>, words: &'static Translator) -> String {
    value.map_or_else(
        || words.usage().none().to_owned(),
        |count| grouped(count.get()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::{ENGLISH, Locale};

    #[test]
    fn optional_absence_is_not_zero_and_totals_are_not_reconstructed() {
        let usage = TurnUsage {
            total: TokenCount::new(123),
            input: TokenCount::new(80),
            output: TokenCount::new(10),
            thought: None,
            cached_read: Some(TokenCount::new(0)),
            cached_write: Some(TokenCount::new(u64::MAX)),
        };
        let words = Translator::get(ENGLISH).unwrap();
        assert_eq!(
            token_count(TokenField::Total.count(Some(&usage)), words),
            "123"
        );
        assert_eq!(
            token_count(TokenField::Thought.count(Some(&usage)), words),
            "Not reported"
        );
        assert_eq!(
            token_count(TokenField::CachedRead.count(Some(&usage)), words),
            "0"
        );
        assert_eq!(
            token_count(TokenField::CachedWrite.count(Some(&usage)), words),
            "18,446,744,073,709,551,615"
        );
        assert!(TokenField::Total.count(None).is_none());
    }

    #[test]
    fn missing_values_and_field_labels_are_localized_in_both_catalogs() {
        let en = Translator::get(ENGLISH).unwrap();
        let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
        assert_eq!(TokenField::Thought.label(en), "Reasoning tokens");
        assert_eq!(TokenField::Thought.label(ko), "추론 토큰");
        assert_eq!(token_count(None, ko), "보고되지 않음");
    }
}
