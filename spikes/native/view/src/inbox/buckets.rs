//! Paseo's attention ordering: requests, failure, working, review, done.
//! The existing sidebar selection/filter is reused; log marks are only the
//! explicit compatibility fallback when a node has no inbox rollup contract.
use crate::board::Phrase;
use crate::prefs::SidebarView;
use crate::{Card, Fleet};
use vessel_proto::{DisplayStatus, inbox_attention::SessionAttention};

pub use crate::attention::Bucket;

impl Bucket {
    /// A current request wins over failure; work wins over finished attention.
    #[must_use]
    pub fn of(card: &Card, attention: Option<SessionAttention>) -> Self {
        let failed = attention.is_some_and(|state| state.failed.get() > 0)
            || card.detail.failure.is_some()
            || matches!(card.phrase, Phrase::Failed);
        if attention.is_some_and(|state| state.waiting.get() > 0)
            || matches!(card.phrase, Phrase::Node(DisplayStatus::Blocked))
            || (attention.is_none() && card.needs_you() && !failed)
        {
            Self::NeedsInput
        } else if failed {
            Self::Failed
        } else if card.is_working() {
            Self::Working
        } else if attention.map_or(card.detail.marks.unread, |state| state.unread.get() > 0) {
            Self::Review
        } else {
            Self::Done
        }
    }
}
/// One display group, borrowing the fleet's cards rather than cloning them.
#[derive(Debug)]
pub struct Group<'a> {
    /// Priority and localized label/colour selection.
    pub bucket: Bucket,
    /// The bounded visible prefix.
    pub sessions: Vec<&'a Card>,
    /// Remaining matches, not dropped from the fleet.
    pub more: usize,
}
/// Filter once and derive each status once; sorting does not repeatedly scan
/// the attention store. Names break equal-time ties deterministically.
#[must_use]
pub fn groups<'a>(
    fleet: &'a Fleet,
    view: SidebarView,
    limit: usize,
    attention: impl Fn(&Card) -> Option<SessionAttention>,
) -> Vec<Group<'a>> {
    let mut cards: Vec<_> = crate::sidebar::listed(
        fleet.machines.iter().flat_map(|machine| &machine.sessions),
        view,
    )
    .into_iter()
    .map(|card| (Bucket::of(card, attention(card)), card))
    .collect();
    cards.sort_by(|(one, a), (two, b)| {
        one.cmp(two)
            .then(b.detail.active_at.cmp(&a.detail.active_at))
            .then(a.machine.cmp(&b.machine))
            .then(a.title.cmp(&b.title))
            .then(a.session.cmp(&b.session))
    });
    Bucket::ALL
        .into_iter()
        .filter_map(|bucket| {
            let rows: Vec<_> = cards
                .iter()
                .filter(|(state, _)| *state == bucket)
                .map(|(_, card)| *card)
                .collect();
            (!rows.is_empty()).then(|| Group {
                bucket,
                more: rows.len().saturating_sub(limit),
                sessions: rows.into_iter().take(limit).collect(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Machine;
    use crate::board::Attention;
    use vessel_proto::inbox::InboxCount;
    use vessel_proto::{HarnessKind, SessionId, SessionInfo, SessionMode, SessionSpec};
    fn card() -> Card {
        Card::from_session(
            "node",
            &SessionInfo::new(
                SessionId::random(),
                SessionSpec::new(
                    HarnessKind::Codex,
                    SessionMode::Structured {
                        model: None,
                        resume: None,
                        options: Vec::new(),
                    },
                ),
                chrono::DateTime::UNIX_EPOCH,
            ),
        )
    }
    fn state(card: &Card, unread: u64, waiting: u64, failed: u64) -> SessionAttention {
        SessionAttention {
            session: card.session,
            unread: InboxCount::new(unread),
            waiting: InboxCount::new(waiting),
            failed: InboxCount::new(failed),
        }
    }
    #[test]
    fn requests_win_over_failure_and_read_failure_is_not_finished_or_done() {
        let mut card = card();
        card.attention = Attention::AtRest;
        assert_eq!(
            Bucket::of(&card, Some(state(&card, 0, 0, 1))),
            Bucket::Failed
        );
        assert_eq!(
            Bucket::of(&card, Some(state(&card, 0, 1, 1))),
            Bucket::NeedsInput
        );
        card.attention = Attention::Working;
        assert_eq!(
            Bucket::of(&card, Some(state(&card, 1, 0, 0))),
            Bucket::Working
        );
        card.attention = Attention::AtRest;
        assert_eq!(
            Bucket::of(&card, Some(state(&card, 1, 0, 0))),
            Bucket::Review
        );
        assert_eq!(Bucket::of(&card, Some(state(&card, 0, 0, 0))), Bucket::Done);
    }
    #[test]
    fn every_priority_has_one_group_and_group_limits_do_not_drop_the_more_count() {
        let mut cards = Vec::new();
        for bucket in Bucket::ALL {
            let mut card = card();
            card.title = format!("{bucket:?}");
            card.attention = if bucket == Bucket::Working {
                Attention::Working
            } else {
                Attention::AtRest
            };
            cards.push(card);
        }
        let mut extra = card();
        extra.title = "NeedsInput".to_owned();
        extra.attention = Attention::AtRest;
        cards.push(extra);
        let mut machine = Machine::unreachable("node".to_owned(), "fixture");
        machine.sessions = cards;
        let fleet = Fleet {
            machines: vec![machine],
            read_at: None,
        };
        let groups = groups(
            &fleet,
            SidebarView {
                ended: true,
                ..SidebarView::default()
            },
            1,
            |card| {
                Some(match card.title.as_str() {
                    "NeedsInput" => state(card, 0, 1, 0),
                    "Failed" => state(card, 0, 0, 1),
                    "Review" => state(card, 1, 0, 0),
                    _ => state(card, 0, 0, 0),
                })
            },
        );
        assert_eq!(
            groups.iter().map(|group| group.bucket).collect::<Vec<_>>(),
            Bucket::ALL
        );
        assert_eq!(groups[0].sessions.len(), 1);
        assert_eq!(groups[0].more, 1);
    }
}
