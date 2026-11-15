use super::*;
use crate::composer::drafts::DraftKey;
use crate::composer::slash::Step;
use crate::machine_book::Name;
use std::time::Duration;
use vessel_proto::{
    ProjectId,
    workspace::{RelPath, WorkspaceRoot},
};
fn scope() -> Scope {
    Scope {
        draft: DraftKey::New,
        machine: Name::parse("laptop").unwrap(),
        root: WorkspaceRoot::Project {
            project: "00000000-0000-4000-8000-000000000001"
                .parse::<ProjectId>()
                .unwrap(),
        },
    }
}
fn row(path: &str, kind: SuggestionKind) -> Suggestion {
    Suggestion {
        path: path.parse::<RelPath>().unwrap(),
        kind,
    }
}
#[test]
fn token_detection_preserves_emails_unicode_and_invalid_character_rules() {
    for text in [
        "person@example.invalid",
        "name+@file",
        "name_@file",
        "@has space",
        "@bad\"name",
        "@bad'name",
        "@<tag>",
    ] {
        assert!(active(text, text.len()).unwrap().is_none(), "{text}");
    }
    for text in ["@", "read @src/main.rs", "한@파일.rs", "(@folder)"] {
        assert!(active(text, text.len()).unwrap().is_some(), "{text}");
    }
    let text = "한 @src/가.rs tail";
    let end = text.find(" tail").unwrap();
    let mention = active(text, end).unwrap().unwrap();
    assert_eq!(text.get(mention.range()).unwrap(), "@src/가.rs");
    assert_eq!(
        quoted(&"my file.rs".parse::<RelPath>().unwrap()),
        "\"my file.rs\""
    );
    assert!(
        active("@한", 2).unwrap().is_some(),
        "caret inside Unicode is brought down safely"
    );
}
#[test]
fn stale_debounce_reply_and_escape_cannot_reopen_or_replace_a_new_query() {
    let mut menu = Menu::default();
    let old = menu
        .observe(Some(scope()), "@a", 2, Duration::ZERO)
        .unwrap()
        .unwrap();
    let now = menu
        .observe(Some(scope()), "@ab", 3, Duration::ZERO)
        .unwrap()
        .unwrap();
    assert!(menu.begin(old.ticket).is_none());
    assert!(!menu.arrived(
        old.ticket,
        Ok(vec![row("old", SuggestionKind::File)]),
        Duration::ZERO
    ));
    assert!(menu.begin(now.ticket).is_some());
    assert!(menu.arrived(
        now.ticket,
        Ok(vec![
            row("best", SuggestionKind::File),
            row("folder", SuggestionKind::Directory)
        ]),
        Duration::ZERO
    ));
    assert_eq!(
        menu.row(menu.selected().unwrap()).unwrap().path.as_str(),
        "best"
    );
    assert_eq!(menu.step(Step::Down), Some(0));
    menu.dismiss();
    assert!(
        menu.observe(Some(scope()), "@ab", 3, Duration::ZERO)
            .unwrap()
            .is_none()
    );
    assert_eq!(menu.phase(), &Phase::Hidden);
    assert!(
        menu.observe(Some(scope()), "@abc", 4, Duration::ZERO)
            .unwrap()
            .is_some()
    );
}
#[test]
fn results_are_cached_only_within_the_same_scope_and_ttl() {
    let mut menu = Menu::default();
    let query = menu
        .observe(Some(scope()), "@", 1, Duration::ZERO)
        .unwrap()
        .unwrap();
    assert!(menu.begin(query.ticket).is_some());
    assert!(menu.arrived(
        query.ticket,
        Ok(vec![row("src", SuggestionKind::Directory)]),
        Duration::ZERO
    ));
    assert!(menu.observe(None, "", 0, Duration::ZERO).unwrap().is_none());
    assert!(
        menu.observe(Some(scope()), "@", 1, Duration::from_secs(1))
            .unwrap()
            .is_none()
    );
    assert!(matches!(menu.phase(), Phase::Rows { .. }));
    assert!(menu.observe(None, "", 0, CACHE_TTL).unwrap().is_none());
    assert!(
        menu.observe(Some(scope()), "@", 1, CACHE_TTL)
            .unwrap()
            .is_some()
    );
}
#[test]
fn empty_failure_and_oversized_replies_remain_distinct() {
    for result in [
        Ok(Vec::new()),
        Err(Failure::Offline),
        Ok(vec![
            row("x", SuggestionKind::File);
            RESULT_LIMIT.saturating_add(1)
        ]),
    ] {
        let mut menu = Menu::default();
        let query = menu
            .observe(Some(scope()), "@", 1, Duration::ZERO)
            .unwrap()
            .unwrap();
        assert!(menu.begin(query.ticket).is_some());
        let expected_empty = result.as_ref().is_ok_and(Vec::is_empty);
        assert!(menu.arrived(query.ticket, result, Duration::ZERO));
        assert_eq!(matches!(menu.phase(), Phase::Empty(_)), expected_empty);
        assert!(menu.rows().is_empty());
    }
}

#[test]
fn previous_candidates_never_cross_workspace_scope() {
    let mut menu = Menu::default();
    let first = menu
        .observe(Some(scope()), "@src", 4, Duration::ZERO)
        .unwrap()
        .unwrap();
    assert!(menu.begin(first.ticket).is_some());
    assert!(menu.arrived(
        first.ticket,
        Ok(vec![row("private-a", SuggestionKind::File)]),
        Duration::ZERO
    ));
    let mut other = scope();
    other.root = WorkspaceRoot::Project {
        project: ProjectId::random(),
    };
    let next = menu
        .observe(Some(other), "@src", 4, Duration::ZERO)
        .unwrap()
        .unwrap();
    assert!(
        menu.rows().is_empty(),
        "keep-previous-data must retain its workspace authority"
    );
    assert_eq!(menu.selected(), None);
    assert!(menu.begin(next.ticket).is_some());
    assert!(!menu.arrived(
        first.ticket,
        Ok(vec![row("late-a", SuggestionKind::File)]),
        Duration::ZERO
    ));
}

#[test]
fn invalid_query_outside_a_registered_scope_remains_hidden() {
    let text = format!("@{}", "x".repeat(4096));
    let mut menu = Menu::default();
    assert!(
        menu.observe(Some(scope()), &text, text.len(), Duration::ZERO)
            .is_err()
    );
    assert!(matches!(menu.phase(), Phase::Invalid(_)));
    assert!(
        menu.observe(None, &text, text.len(), Duration::ZERO)
            .unwrap()
            .is_none()
    );
    assert_eq!(menu.phase(), &Phase::Hidden);
}

#[test]
fn a_recreated_presentation_cannot_accept_a_late_ticket_from_its_previous_incarnation() {
    let mut first=Menu::default();
    let old=first.observe(Some(scope()),"@src",4,Duration::ZERO).unwrap().unwrap();
    let clock=first.ticket_clock();
    let mut replacement=Menu::default();
    replacement.inherit_ticket_clock(clock);
    let current=replacement.observe(Some(scope()),"@src",4,Duration::ZERO).unwrap().unwrap();
    assert_ne!(old.ticket,current.ticket);
    assert!(replacement.begin(current.ticket).is_some());
    assert!(!replacement.arrived(old.ticket,Ok(vec![row("src/stale.rs",SuggestionKind::File)]),Duration::ZERO));
    assert!(replacement.arrived(current.ticket,Ok(vec![row("src/current.rs",SuggestionKind::File)]),Duration::ZERO));
    assert_eq!(replacement.rows(),[row("src/current.rs",SuggestionKind::File)]);
}
