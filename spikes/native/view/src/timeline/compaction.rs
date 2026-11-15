//! Compaction labels and metadata merging retain the provider's explicit meaning.

use std::borrow::Cow;
use vessel_proto::TokenCount;
use vessel_proto::timeline::{Compaction, CompactionStatus, CompactionTrigger};

const TOKENS_PER_THOUSAND: u64 = 1000;
const ROUNDING_HALF: u64 = 500;

/// A rounded display quantity, distinct from the original token observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionThousands(u64);

impl CompactionThousands {
    fn of(tokens: TokenCount) -> Self {
        let whole = tokens.get().div_euclid(TOKENS_PER_THOUSAND);
        let round_up = u64::from(tokens.get().rem_euclid(TOKENS_PER_THOUSAND) >= ROUNDING_HALF);
        Self(whole.saturating_add(round_up))
    }

    /// Exact integer thousands; no float conversion loses large observations.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Loading and explicit triggers take priority over an optional token count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionCaption {
    /// Context maintenance is still running.
    Loading,
    /// The provider explicitly reported automatic maintenance.
    Auto,
    /// A person explicitly requested it.
    Manual,
    /// Only the pre-maintenance size was reported.
    Tokens(CompactionThousands),
    /// Completion without trigger or positive token metadata.
    Completed,
}

impl CompactionCaption {
    /// Paseo `message-compaction-label.ts`: status, trigger, positive tokens, fallback.
    #[must_use]
    pub fn of(item: &Compaction) -> Self {
        if item.status == CompactionStatus::Loading {
            return Self::Loading;
        }
        match item.trigger {
            Some(CompactionTrigger::Auto) => Self::Auto,
            Some(CompactionTrigger::Manual) => Self::Manual,
            None => item
                .pre_tokens
                .filter(|tokens| tokens.get() > 0)
                .map_or(Self::Completed, |tokens| {
                    Self::Tokens(CompactionThousands::of(tokens))
                }),
        }
    }
}

/// Completing the earliest outstanding marker fills only supplied metadata.
#[must_use]
pub fn merge_compaction(previous: &Compaction, incoming: &Compaction) -> Compaction {
    Compaction {
        status: incoming.status,
        trigger: incoming.trigger.or(previous.trigger),
        pre_tokens: incoming.pre_tokens.or(previous.pre_tokens),
    }
}

impl<'a> crate::localization::TimelineItems<'a> {
    /// Plain captions borrow cached values; the numeric case formats a whole message.
    #[must_use]
    pub fn compaction(self, item: &Compaction) -> Cow<'a, str> {
        match CompactionCaption::of(item) {
            CompactionCaption::Loading => Cow::Borrowed(self.compacting()),
            CompactionCaption::Auto => Cow::Borrowed(self.compacted_auto()),
            CompactionCaption::Manual => Cow::Borrowed(self.compacted_manual()),
            CompactionCaption::Completed => Cow::Borrowed(self.compacted()),
            CompactionCaption::Tokens(tokens) => Cow::Owned(
                self.compacted_tokens(&tokens.get().to_string())
                    .unwrap_or_else(|_| self.compacted().to_owned()),
            ),
        }
    }
}

#[cfg(test)]
mod tests;
