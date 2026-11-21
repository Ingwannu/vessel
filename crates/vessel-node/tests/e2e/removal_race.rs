//! A terminal starting beside a session in a worktree while the worktree
//! is removed (the window track C left open, closed in the join of wave
//! 2's first half by ADR 0030's "starting" count), raced over the wire.
//!
//! Unix only, as the terminals' tests are: the shell is `/bin/sh`.

use vessel_node::Shell;
use vessel_proto::ErrorCode;
use vessel_proto::terminals::{CloseTerminal, CreateTerminal, TerminalPlace};
use vessel_proto::workspace::{
    AddProject, IfLost, KeptReason, RemoveWorktree, RemoveWorktreeOutcome, SessionPlace,
};

use crate::agents::code;
use crate::forge::info_until;
use crate::support::{Fixture, within};
use crate::workspace::{in_place, lookup, new_worktree, repository};

/// A terminal opened beside a session in a worktree while the worktree is
/// removed: the removal sees it and keeps the worktree, or it is refused
/// before anything runs; never both a terminal and a removal (which would
/// have run teardown under it). Raced six times; either may win a round
/// (the removal usually does: opening a shell takes longer). Without the
/// hold a start takes on the worktree, the first round fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_terminal_and_its_worktree_s_removal_never_both_win() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| {
                config.harness_lookup = lookup(&fixture);
                config.terminals.shell = Some(Shell::plain("/bin/sh"));
            })
            .await;
        let owner = fixture.connect(&node).await;
        let folder = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(folder.path()).unwrap().join("app");
        std::fs::create_dir(&root).unwrap();
        repository(&root);
        let project = owner
            .call(AddProject {
                path: root,
                name: None,
            })
            .await
            .unwrap();
        let (mut kept, mut removed) = (0_u32, 0_u32);
        for _round in 0..6 {
            let worktree = new_worktree(&owner, project.id).await;
            // An ended session still has its folder, and a terminal may be
            // opened beside it; it no longer holds the worktree itself.
            let session = owner
                .create_session(in_place(
                    &fixture,
                    SessionPlace::Worktree {
                        worktree: worktree.id,
                    },
                ))
                .await
                .unwrap()
                .id;
            owner.kill(session).await.unwrap();
            let _ended = info_until(&owner, session, |info| !info.status.is_live()).await;
            let (opened, removal) = tokio::join!(
                owner.call(CreateTerminal {
                    place: TerminalPlace::Session { session },
                    size: None,
                    title: None,
                    profile: None,
                }),
                owner.call(RemoveWorktree {
                    worktree: worktree.id,
                    if_lost: IfLost::Discard,
                    end_terminals: false,
                })
            );
            match (opened, removal.unwrap()) {
                (Ok(shell), RemoveWorktreeOutcome::Kept { reasons }) => {
                    assert_eq!(reasons, [KeptReason::InUse]);
                    assert!(shell.folder.starts_with(&worktree.path));
                    kept = kept.saturating_add(1);
                    let _closed = owner
                        .call(CloseTerminal { terminal: shell.id })
                        .await
                        .unwrap();
                    let removed = owner
                        .call(RemoveWorktree {
                            worktree: worktree.id,
                            if_lost: IfLost::Discard,
                            end_terminals: false,
                        })
                        .await
                        .unwrap();
                    assert!(matches!(removed, RemoveWorktreeOutcome::Removed { .. }));
                }
                (Err(refused), RemoveWorktreeOutcome::Removed { .. }) => {
                    let refused = code(Err::<(), _>(refused));
                    assert!(
                        matches!(refused, ErrorCode::Conflict),
                        "refused as {refused:?}"
                    );
                    removed = removed.saturating_add(1);
                }
                (opened, outcome) => {
                    panic!("a terminal {opened:?} and a removal {outcome:?} at once")
                }
            }
            let _deleted = owner.delete_session(session).await.unwrap();
        }
        tracing::info!(kept, removed, "the race's outcomes");
        assert_eq!(kept.saturating_add(removed), 6);
        node.stop().await.unwrap();
    })
    .await;
}
