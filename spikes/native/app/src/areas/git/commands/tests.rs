//! Pure logical intention checks; no acquisition/connection authority is inferred.
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

fn projection(target: LogicalTarget, entries: Vec<MenuEntry>) -> Projection {
    Projection { target, entries }
}
struct Projection {
    target: LogicalTarget,
    entries: Vec<MenuEntry>,
}
fn current_entry<'a>(
    captured: &LogicalTarget,
    live: &'a Projection,
    entry: &MenuEntry,
) -> Option<&'a MenuEntry> {
    logical_entry(captured, &live.target, &live.entries, entry)
}

#[test]
fn leaving_and_returning_to_the_same_repository_does_not_revive_old_callbacks() {
    let mut keys = RequestKeys::default();
    let repository = repository();
    let old = LogicalTarget {
        repository: repository.clone(),
        generation: keys.issue().unwrap(),
        session: None,
        pull: PullEntry::Hidden,
    };
    let _other = keys.issue().unwrap();
    let live = projection(
        LogicalTarget {
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
fn another_machine_root_or_session_cannot_use_the_captured_intention() {
    let mut keys = RequestKeys::default();
    let captured = LogicalTarget {
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
    let target = LogicalTarget {
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
    let target = LogicalTarget {
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

/// Real C5 acquisition supplies origin; only menu availability is synthetic.
pub(in crate::areas) fn source_projection(
    shell: &mut Shell,
    cx: &mut Context<'_, Shell>,
) -> GitCommands {
    use vessel_app_view::workspace::git::Reading;
    use vessel_proto::workspace::GitStatus;
    assert!(super::super::start(shell, cx).is_ok());
    let source = shell.shown_changes_context().unwrap();
    shell.areas.git.target = Some(Target {
        machine: source.connection().machine().as_str().to_owned(),
        root: source.root(),
    });
    // No status acquisition or mutation-authority proof is claimed by this input.
    shell.areas.git.status = Reading::Ready(GitStatus {
        branch: Some("topic".parse().unwrap()),
        head: None,
        upstream: None,
        base: None,
        operation: None,
        files: Vec::new(),
        more: false,
    });
    shell.areas.git.commands_generation = Some(shell.areas.git.keys.issue().unwrap());
    shell.git_command_entries().unwrap()
}
