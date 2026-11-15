//! Captured palette/menu callbacks share one live intention boundary.
use super::*;
use vessel_app_view::workspace::git::{ActionRow, RequestKeys, Unavailable};
use vessel_app_view::workspace::git_quick::GitAction;
use vessel_proto::ProjectId;
use vessel_proto::forge::MergeMethod;
use vessel_proto::workspace::WorkspaceRoot;

fn repository() -> Target {
    Target {
        machine: "machine-a".to_owned(),
        root: WorkspaceRoot::Project {
            project: ProjectId::random(),
        },
    }
}

fn push(unavailable: Option<Unavailable>) -> MenuEntry {
    MenuEntry::Git(ActionRow {
        action: Action::Quick(GitAction::Push),
        unavailable,
        starts_group: false,
    })
}

fn projection(target: GitCommandTarget, entries: Vec<MenuEntry>) -> GitCommands {
    GitCommands {
        target,
        primary: None,
        entries,
    }
}

#[test]
fn leaving_and_returning_to_the_same_repository_does_not_revive_old_callbacks() {
    let mut keys = RequestKeys::default();
    let repository = repository();
    let old = GitCommandTarget {
        repository: repository.clone(),
        generation: keys.issue().unwrap(),
        session: None,
        pull: PullEntry::Hidden,
    };
    let _other = keys.issue().unwrap();
    let live = projection(
        GitCommandTarget {
            repository,
            generation: keys.issue().unwrap(),
            session: None,
            pull: PullEntry::Hidden,
        },
        vec![push(None)],
    );
    assert_eq!(current_entry(&old, &live, &push(None)), None);
    assert_eq!(
        current_entry(&live.target, &live, &push(None)),
        live.entries.first()
    );
}

#[test]
fn another_machine_root_or_session_cannot_use_the_captured_authority() {
    let mut keys = RequestKeys::default();
    let captured = GitCommandTarget {
        repository: repository(),
        generation: keys.issue().unwrap(),
        session: Some(SessionId::random()),
        pull: PullEntry::Hidden,
    };
    let mut changed = captured.clone();
    changed.repository.machine = "machine-b".to_owned();
    let mut other_root = captured.clone();
    other_root.repository.root = repository().root;
    let mut other_session = captured.clone();
    other_session.session = Some(SessionId::random());
    let mut other_pull = captured.clone();
    other_pull.pull = PullEntry::Create;
    for target in [changed, other_root, other_session, other_pull] {
        assert_eq!(
            current_entry(
                &captured,
                &projection(target, vec![push(None)]),
                &push(None)
            ),
            None
        );
    }
}

#[test]
fn the_live_availability_replaces_the_captured_availability() {
    let mut keys = RequestKeys::default();
    let target = GitCommandTarget {
        repository: repository(),
        generation: keys.issue().unwrap(),
        session: None,
        pull: PullEntry::Hidden,
    };
    let live = projection(target.clone(), vec![push(Some(Unavailable::Writing))]);
    assert_eq!(
        current_entry(&target, &live, &push(None)),
        Some(&push(Some(Unavailable::Writing)))
    );
    assert_eq!(
        current_entry(&target, &live, &MenuEntry::Merge(MergeMethod::Squash)),
        None
    );
}

#[test]
fn a_removed_or_different_merge_method_is_not_substituted() {
    let mut keys = RequestKeys::default();
    let target = GitCommandTarget {
        repository: repository(),
        generation: keys.issue().unwrap(),
        session: None,
        pull: PullEntry::Hidden,
    };
    let live = projection(target.clone(), vec![MenuEntry::Merge(MergeMethod::Rebase)]);
    assert_eq!(
        current_entry(&target, &live, &MenuEntry::Merge(MergeMethod::Squash)),
        None
    );
    assert_eq!(
        current_entry(&target, &live, &MenuEntry::Merge(MergeMethod::Rebase)),
        live.entries.first()
    );
}
