//! A local name and a remote-tracking ref can have the same printable text.
use super::*;
use crate::workspace::git::fixtures;
use vessel_proto::workspace::WorktreeBase;

fn branch(remote: Option<&str>) -> Branch {
    Branch {
        name: "origin/topic".parse().unwrap(),
        current: remote.is_none(),
        default: false,
        remote: remote.map(|name| name.parse().unwrap()),
        head: "1111111111111111111111111111111111111111".parse().unwrap(),
        time: "2026-11-13T00:00:00Z".parse().unwrap(),
        upstream: None,
        worktree: None,
    }
}

#[test]
fn only_the_local_identity_is_checked_and_noop_when_local_and_remote_print_the_same_name() {
    let local = branch(None);
    let remote = branch(Some("origin"));
    let malformed = branch(Some("upstream"));
    let mut status = fixtures::status(vec![]);
    status.branch = Some("origin/topic".parse().unwrap());
    let root = WorkspaceRoot::Project {
        project: "6f1c2a3e-0000-4000-8000-000000000001".parse().unwrap(),
    };
    let list = Reading::Ready(BranchList {
        branches: vec![local.clone(), remote.clone(), malformed.clone()],
        more: false,
    });
    assert_eq!(
        branch_choices(
            &list,
            &Reading::Ready(status.clone()),
            &Writing::default(),
            ""
        ),
        BranchChoices::Rows {
            rows: vec![
                BranchChoice {
                    branch: local.clone(),
                    selected: true,
                    enabled: true
                },
                BranchChoice {
                    branch: remote.clone(),
                    selected: false,
                    enabled: true
                },
                BranchChoice {
                    branch: malformed.clone(),
                    selected: false,
                    enabled: true
                },
            ],
            more: false,
        }
    );
    assert_eq!(
        branch_selection(root, &local, status.branch.as_ref()),
        Ok(BranchSelection::Current)
    );
    assert_eq!(
        branch_selection(root, &remote, status.branch.as_ref()),
        Ok(BranchSelection::Switch(Switch {
            root,
            branch: "topic".parse().unwrap(),
            create: Some(WorktreeBase::Ref {
                name: "origin/topic".parse().unwrap()
            }),
        }))
    );
    assert_eq!(
        branch_selection(root, &malformed, status.branch.as_ref()),
        Err(SwitchProblem::RemoteMismatch {
            reference: "origin/topic".parse().unwrap(),
            remote: "upstream".parse().unwrap(),
        })
    );
}
