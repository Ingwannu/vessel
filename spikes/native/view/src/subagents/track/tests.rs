use super::*;
use crate::subagents::Visibility;
use vessel_proto::subagents::{ChildDescriptors, DetachReceipt};
use vessel_proto::{Creator, HarnessKind, SessionId, SessionMode, SessionSpec};

fn target(node: NodeId) -> Target {
    Target::new(
        crate::connection::ConnectionEpochs::default()
            .admit(MachineKey::from_book("fixture"), node)
            .unwrap(),
        SessionId::random(),
    )
}
fn managed(parent: SessionRef, node: NodeId, time: i64) -> ManagedChild {
    let target = target(node);
    let info = SessionInfo::new(
        target.address.session,
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![],
            },
        ),
        chrono::DateTime::from_timestamp(time, 0).unwrap(),
    )
    .with_creator(Creator::Agent {
        node: parent.node,
        session: parent.session,
    });
    ManagedChild {
        target,
        info: Box::new(info),
        finished: Some(true),
        visibility: Visibility::Active,
    }
}
fn provider(id: &str, status: Option<ChildStatus>) -> ChildDescriptor {
    ChildDescriptor {
        id: id.parse().unwrap(),
        parent: None,
        title: None,
        description: None,
        subtitle: None,
        status,
        created_at: None,
        updated_at: None,
    }
}
fn listed(track: &mut Track, rows: Vec<ChildDescriptor>) {
    let read = track.refresh(&mut Tickets::default()).unwrap();
    assert!(track.listed(
        &read,
        Ok(ProviderChildren {
            children: ChildDescriptors::try_from(rows).unwrap(),
            history: ChildHistory::Complete
        })
    ));
}

#[test]
fn managed_rows_use_current_parent_full_identity_and_oldest_first_order() {
    let parent = target(NodeId::random());
    let node = NodeId::random();
    let host = Host {
        machine: MachineKey::from_book("fixture"),
        node,
        context: crate::connection::ConnectionEpochs::default()
            .admit(MachineKey::from_book("fixture"), node)
            .unwrap(),
    };
    let mut detached = managed(parent.address, node, 1);
    detached.info.detach_receipt = Some(DetachReceipt::new(
        parent.address,
        chrono::DateTime::UNIX_EPOCH,
    ));
    let mut archived = managed(parent.address, node, 2);
    archived.info.marks.archived_at = Some(chrono::DateTime::UNIX_EPOCH);
    let mut pending = managed(parent.address, node, 3);
    pending.visibility = Visibility::PendingArchive;
    let old = managed(parent.address, node, 4);
    let newer = managed(parent.address, node, 8);
    let mut wrong_parent = parent.address;
    wrong_parent.node = NodeId::random();
    let other = managed(wrong_parent, node, 0);
    let expected = vec![old.target.address, newer.target.address];
    let mut track = Track::new(parent);
    track
        .managed_host(&host, vec![newer, detached, pending, archived, other, old])
        .unwrap();
    assert_eq!(
        track
            .managed()
            .iter()
            .map(|row| row.target.address)
            .collect::<Vec<_>>(),
        expected
    );
    assert!(track.detach_choice(expected[0]).is_some());
    assert!(track.detach_choice(wrong_parent).is_none());
}
#[test]
fn provider_hide_never_archives_managed_children_or_invents_unknown_completion() {
    let mut track = Track::new(target(NodeId::random()));
    listed(
        &mut track,
        vec![
            provider("unknown", None),
            provider("running", Some(ChildStatus::Running)),
            provider("done", Some(ChildStatus::Completed)),
            provider("failed", Some(ChildStatus::Failed)),
            provider("canceled", Some(ChildStatus::Canceled)),
        ],
    );
    assert_eq!(
        track.counts(),
        TrackCounts {
            total: 5,
            running: 1,
            finished: 3
        }
    );
    track.hide_finished_providers();
    assert_eq!(
        track.counts(),
        TrackCounts {
            total: 2,
            running: 1,
            finished: 0
        }
    );
    assert_eq!(
        track
            .providers(None)
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        vec!["unknown", "running"]
    );
    assert!(track.managed().is_empty());
    listed(
        &mut track,
        vec![
            provider("done", Some(ChildStatus::Completed)),
            provider("running", Some(ChildStatus::Running)),
        ],
    );
    assert_eq!(track.counts().total, 1);
}
#[test]
fn stale_reads_failures_and_parent_changes_cannot_replace_good_rows() {
    let mut track = Track::new(target(NodeId::random()));
    listed(&mut track, vec![provider("one", None)]);
    let mut tickets = Tickets::default();
    let old = track.refresh(&mut tickets).unwrap();
    let current = track.refresh(&mut tickets).unwrap();
    assert!(!track.listed(&old, Err(TrackProblem::Offline)));
    assert!(track.listed(&current, Err(TrackProblem::Timeout)));
    assert_eq!(track.counts().total, 1);
    assert_eq!(track.load(), TrackLoad::Failed(TrackProblem::Timeout));
    let mut another = Track::new(target(NodeId::random()));
    let _next = another.refresh(&mut tickets).unwrap();
    assert!(!another.listed(&old, Err(TrackProblem::Offline)));
}
#[test]
fn provider_nesting_selects_only_direct_children_and_keeps_unknown_parents_explicit() {
    let mut track = Track::new(target(NodeId::random()));
    let root = provider("root", None);
    let mut nested = provider("nested", None);
    nested.parent = Some(root.id.clone());
    let mut orphan = provider("orphan", None);
    orphan.parent = Some("missing".parse().unwrap());
    listed(&mut track, vec![root.clone(), nested, orphan]);
    assert_eq!(track.providers(None).count(), 2);
    assert_eq!(
        track.providers(Some(&root.id)).next().unwrap().id.as_str(),
        "nested"
    );
}

#[test]
fn nested_navigation_uses_the_same_observed_path_and_missing_parent_stays_visible() {
    let mut track = Track::new(target(NodeId::random()));
    let root = provider("root", Some(ChildStatus::Running));
    let mut nested = provider("nested", None);
    nested.parent = Some(root.id.clone());
    let mut orphan = provider("orphan", None);
    orphan.parent = Some("missing".parse().unwrap());
    listed(&mut track, vec![root, nested, orphan]);
    assert_eq!(
        track
            .providers(None)
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        vec!["root", "orphan"]
    );
    let id = "nested".parse().unwrap();
    assert_eq!(
        track
            .provider_path(&id)
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        vec!["root", "nested"]
    );
    assert_eq!(track.providers(Some(&"root".parse().unwrap())).count(), 1);
}

#[test]
fn mixed_fanout_retains_separate_failed_and_working_counts_without_unknown_status_invention() {
    let mut track = Track::new(target(NodeId::random()));
    listed(
        &mut track,
        vec![
            provider("failed", Some(ChildStatus::Failed)),
            provider("running", Some(ChildStatus::Running)),
            provider("unknown", None),
            provider("completed", Some(ChildStatus::Completed)),
        ],
    );
    assert_eq!(
        track.pill_counts(),
        vec![
            PillCount {
                kind: PillKind::Failed,
                count: 1
            },
            PillCount {
                kind: PillKind::Working,
                count: 1
            }
        ]
    );
    let _hidden = track.hide_provider(&"failed".parse().unwrap());
    assert_eq!(
        track.pill_counts(),
        vec![PillCount {
            kind: PillKind::Working,
            count: 1
        }]
    );
    let _hidden = track.hide_provider(&"running".parse().unwrap());
    assert!(track.pill_counts().is_empty());
    assert_eq!(track.counts().total, 2);
}
