//! Source/revision release never doubles as command approval or execution.

use super::*;
use vessel_proto::forge::{ForgeKind, PullNumber, RepoRef};
use vessel_proto::registry::{BlockedSource, ReleaseWorkspaceAutomation, WorkspaceAutomation};

fn source(number: u64) -> BlockedSource {
    BlockedSource::ChangeRequest {
        number: PullNumber::new(number).unwrap(),
        head_repository: RepoRef {
            kind: ForgeKind::Github,
            host: "github.com".parse().unwrap(),
            path: "fork/project".parse().unwrap(),
        },
    }
}

#[test]
fn only_the_retained_source_and_revision_can_release_an_active_workspace() {
    let mut db = db();
    let mut first = record(WorkspaceId::random());
    let id = first.descriptor.id;
    first.descriptor.automation = WorkspaceAutomation::Blocked { source: source(1) };
    put(&mut db, first, None).unwrap();
    let expected = RegistryRevision::ZERO;
    let before = load(&db).unwrap();
    for request in [
        ReleaseWorkspaceAutomation {
            workspace: id,
            expected,
            source: source(2),
        },
        ReleaseWorkspaceAutomation {
            workspace: id,
            expected: expected.next().unwrap(),
            source: source(1),
        },
    ] {
        assert!(matches!(
            apply(&mut db, &RegistryWrite::Release(request)),
            Err(StoreError::RegistryConflict)
        ));
        assert_eq!(load(&db).unwrap(), before);
    }
    let request = ReleaseWorkspaceAutomation {
        workspace: id,
        expected,
        source: source(1),
    };
    apply(&mut db, &RegistryWrite::Release(request.clone())).unwrap();
    let released = workspace(&db, id).unwrap().unwrap();
    assert_eq!(released.descriptor.automation, WorkspaceAutomation::Allowed);
    assert_eq!(released.descriptor.revision, expected.next().unwrap());
    assert!(load(&db).unwrap().memberships.is_empty());
    assert!(matches!(
        apply(&mut db, &RegistryWrite::Release(request)),
        Err(StoreError::RegistryConflict)
    ));
}

#[test]
fn an_archive_claim_blocks_source_release_and_rolls_back() {
    let mut db = db();
    let mut first = record(WorkspaceId::random());
    let id = first.descriptor.id;
    first.descriptor.automation = WorkspaceAutomation::Blocked { source: source(1) };
    first.descriptor.lifecycle = WorkspaceLifecycle::Archiving {
        at: DateTime::UNIX_EPOCH,
    };
    put(&mut db, first, None).unwrap();
    let before = load(&db).unwrap();
    assert!(matches!(
        apply(
            &mut db,
            &RegistryWrite::Release(ReleaseWorkspaceAutomation {
                workspace: id,
                expected: RegistryRevision::ZERO,
                source: source(1)
            })
        ),
        Err(StoreError::RegistryInactive(_))
    ));
    assert_eq!(load(&db).unwrap(), before);
}
