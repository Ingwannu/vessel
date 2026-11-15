//! The same key/action boundary drives pointer and keyboard row presentation.
use super::*;
use crate::workspace::git::{RequestKeys, StashScope, Success, Unavailable, Written};

#[test]
fn pending_and_success_belong_to_the_original_action_until_its_own_timer_expires() {
    let mut keys = RequestKeys::default();
    let key = keys.issue().unwrap();
    let fetch = Action::Quick(GitAction::Fetch);
    let other = Action::Quick(GitAction::Push);
    let mut writing = Writing::default();
    assert!(writing.begin_action(key, WriteIntent::Quick(GitAction::Fetch)));
    assert!(matches!(
        menu_feedback(fetch, &writing),
        MenuFeedback::Pending(WriteIntent::Quick(GitAction::Fetch))
    ));
    assert!(matches!(menu_feedback(other, &writing), MenuFeedback::Idle));
    let _finished = writing
        .finish(key, Ok(Written::Said(Success::Fetched)))
        .unwrap();
    assert!(
        matches!(menu_feedback(fetch, &writing), MenuFeedback::Success(receipt) if receipt.key() == key)
    );
    assert!(matches!(menu_feedback(other, &writing), MenuFeedback::Idle));
    assert!(!writing.expire_success(keys.issue().unwrap()));
    assert!(matches!(
        menu_feedback(fetch, &writing),
        MenuFeedback::Success(_)
    ));
    assert!(writing.expire_success(key));
    assert!(matches!(menu_feedback(fetch, &writing), MenuFeedback::Idle));
}

#[test]
fn direct_writes_and_unavailable_explanations_stay_open_while_forms_replace_the_menu() {
    let direct = [
        Action::Quick(GitAction::Push),
        Action::Quick(GitAction::Pull),
        Action::Quick(GitAction::Fetch),
        Action::SaveStash(StashScope::Tracked),
        Action::SaveStash(StashScope::WithUntracked),
    ];
    let forms = [
        Action::Quick(GitAction::Commit),
        Action::SwitchBranch,
        Action::CreateBranch,
        Action::RenameBranch,
        Action::Stashes,
    ];
    for action in direct {
        assert_eq!(
            menu_close(ActionRow {
                action,
                unavailable: None,
                starts_group: false
            }),
            MenuClose::KeepOpen
        );
    }
    for action in forms {
        assert_eq!(
            menu_close(ActionRow {
                action,
                unavailable: None,
                starts_group: false
            }),
            MenuClose::OpenSurface
        );
        assert_eq!(
            menu_close(ActionRow {
                action,
                unavailable: Some(Unavailable::Reading),
                starts_group: false
            }),
            MenuClose::KeepOpen
        );
    }
}

#[test]
fn a_new_write_removes_the_previous_success_without_transferring_its_feedback() {
    let mut keys = RequestKeys::default();
    let mut writing = Writing::default();
    let fetch = keys.issue().unwrap();
    assert!(writing.begin_action(fetch, WriteIntent::Quick(GitAction::Fetch)));
    let _finished = writing
        .finish(fetch, Ok(Written::Said(Success::Fetched)))
        .unwrap();
    let push = keys.issue().unwrap();
    assert!(writing.begin_action(push, WriteIntent::Quick(GitAction::Push)));
    assert!(matches!(
        menu_feedback(Action::Quick(GitAction::Fetch), &writing),
        MenuFeedback::Idle
    ));
    assert!(matches!(
        menu_feedback(Action::Quick(GitAction::Push), &writing),
        MenuFeedback::Pending(_)
    ));
    assert!(!writing.expire_success(fetch));
    assert!(writing.busy());
}
