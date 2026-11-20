//! An activity card's menu: Paseo's `ActivityKebab` (`pull-request-panel/
//! pane.tsx`, pinned 0.15.4) on a comment or a review, and the thread
//! header's own (Open alone): what each offers, in Paseo's order, and the
//! forge's name its "Open on" row says.
//!
//! [Decision Log]
//! - Purpose: Copy and Open on GitHub, which a card's own press does not
//!   give (a press folds a card with words), as Paseo's menu does.
//! - Choice: the rule is here, pure and tested; the app draws the kit's
//!   menu from it. Add to chat while the session has a composer and the
//!   item something to add (`attach::can_add`, Paseo's
//!   `canAddPullRequestActivityToChat`), Copy while it has words, Open
//!   on the forge.
//! - Difference: Paseo always lists Open; an item the node sends without
//!   a page has no Open row here (the row would do nothing), and a menu
//!   with no row draws no kebab.

use vessel_proto::forge::{ForgeKind, RepoRef, TimelineItem};

use super::attach::can_add;

/// One row of a card's menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardAct {
    /// Add the item to the session's composer.
    AddToChat,
    /// Copy its words.
    Copy,
    /// Open its page on the forge.
    Open,
}

/// What a comment's or a review's menu offers; `attach`: the session has
/// a composer to add to (Paseo's `attachEnabled`).
#[must_use]
pub fn item_acts(item: &TimelineItem, attach: bool) -> Vec<CardAct> {
    let (body, url) = match item {
        TimelineItem::Review { body, url, .. } | TimelineItem::Comment { body, url, .. } => {
            (body, url)
        }
    };
    let mut acts = Vec::new();
    if attach && can_add(item) {
        acts.push(CardAct::AddToChat);
    }
    if !body.as_str().trim().is_empty() {
        acts.push(CardAct::Copy);
    }
    if url.is_some() {
        acts.push(CardAct::Open);
    }
    acts
}

/// What a thread header's menu offers: Open, on its first comment's page.
#[must_use]
pub fn thread_acts(first: Option<&TimelineItem>) -> Vec<CardAct> {
    let page = first.is_some_and(|item| match item {
        TimelineItem::Review { url, .. } | TimelineItem::Comment { url, .. } => url.is_some(),
    });
    if page { vec![CardAct::Open] } else { Vec::new() }
}

/// The forge's name for "Open on …" (Paseo's `brandLabel`): GitHub, or
/// the host for a kind this app does not know.
#[must_use]
pub fn brand(repo: &RepoRef) -> String {
    match repo.kind {
        ForgeKind::Github => "GitHub".to_owned(),
        ForgeKind::Unknown => repo.host.as_str().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone as _, Utc};
    use vessel_proto::forge::{PullBody, ReviewState};

    use super::*;

    fn review(state: ReviewState, body: &str, url: bool) -> TimelineItem {
        TimelineItem::Review {
            id: "r1".parse().unwrap(),
            author: Some("reviewer".parse().unwrap()),
            body: PullBody::lossy(body),
            created_at: Utc.with_ymd_and_hms(2026, 11, 19, 10, 0, 0).unwrap(),
            url: url.then(|| "https://github.com/acme/app/pull/1#r1".parse().unwrap()),
            state,
        }
    }

    fn comment(body: &str, url: bool) -> TimelineItem {
        TimelineItem::Comment {
            id: "c1".parse().unwrap(),
            author: Some("author".parse().unwrap()),
            body: PullBody::lossy(body),
            created_at: Utc.with_ymd_and_hms(2026, 11, 19, 10, 0, 0).unwrap(),
            url: url.then(|| "https://github.com/acme/app/pull/1#c1".parse().unwrap()),
            review: None,
            thread: None,
        }
    }

    #[test]
    fn a_comment_with_words_offers_all_three_in_paseos_order() {
        assert_eq!(
            item_acts(&comment("Looks fine", true), true),
            [CardAct::AddToChat, CardAct::Copy, CardAct::Open]
        );
    }

    #[test]
    fn without_a_composer_there_is_nothing_to_add() {
        assert_eq!(
            item_acts(&comment("Looks fine", true), false),
            [CardAct::Copy, CardAct::Open]
        );
    }

    #[test]
    fn a_change_request_without_words_adds_and_opens_but_copies_nothing() {
        assert_eq!(
            item_acts(&review(ReviewState::ChangesRequested, " ", true), true),
            [CardAct::AddToChat, CardAct::Open]
        );
        assert_eq!(
            item_acts(&review(ReviewState::Approved, "", true), true),
            [CardAct::Open]
        );
    }

    #[test]
    fn an_item_without_a_page_has_no_open_row() {
        assert_eq!(
            item_acts(&comment("Looks fine", false), false),
            [CardAct::Copy]
        );
        assert!(item_acts(&review(ReviewState::Approved, "", false), true).is_empty());
    }

    #[test]
    fn a_threads_menu_opens_its_first_comment() {
        assert_eq!(thread_acts(Some(&comment("x", true))), [CardAct::Open]);
        assert!(thread_acts(Some(&comment("x", false))).is_empty());
        assert!(thread_acts(None).is_empty());
    }

    #[test]
    fn the_brand_is_github_or_the_host() {
        let repo = |kind| RepoRef {
            kind,
            host: "git.example.com".parse().unwrap(),
            path: "acme/app".parse().unwrap(),
        };
        assert_eq!(brand(&repo(ForgeKind::Github)), "GitHub");
        assert_eq!(brand(&repo(ForgeKind::Unknown)), "git.example.com");
    }
}
