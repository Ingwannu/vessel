//! Paseo's pull request action (`git/policy.ts`): "View PR" with one read,
//! "Create PR" off the base branch, and why not when it would be refused.

use chrono::{TimeZone as _, Utc};
use vessel_proto::forge::{ForgeAccess, GhState, PullState, PullSummary, SessionPullRequest};
use vessel_proto::workspace::{Distance, GitStatus};

use super::{PullEntry, Unavailable, pull_entry};
use crate::workspace::git::fixtures::status;
use crate::workspace::pull::PullPhase;
use crate::workspace::pull::fixtures::summary;

fn session(access: ForgeAccess, summary: Option<PullSummary>) -> SessionPullRequest {
    SessionPullRequest {
        access,
        summary,
        feedback: false,
    }
}

fn ready() -> ForgeAccess {
    ForgeAccess::Ready { login: None }
}

fn on_worktree(ahead: u32) -> GitStatus {
    let mut facts = status(vec![]);
    facts.base = Some(Distance {
        name: "main".parse().unwrap(),
        ahead,
        behind: 0,
    });
    facts
}

#[test]
fn a_session_without_a_watched_pull_request_offers_none() {
    assert_eq!(pull_entry(None, &status(vec![])), PullEntry::Hidden);
}

#[test]
fn one_read_is_viewed_whatever_its_state_and_even_when_access_failed_since() {
    let host = "github.com".parse().unwrap();
    let held = session(
        ForgeAccess::SignedOut {
            host,
            gh: GhState::SignedOut,
        },
        Some(summary(PullState::Open, true)),
    );
    let PullEntry::View { number, phase, .. } = pull_entry(Some(&held), &on_worktree(2)) else {
        panic!("viewed");
    };
    assert_eq!(number.get(), 42);
    assert_eq!(phase, PullPhase::Draft);
    let merged = session(ready(), Some(summary(PullState::Merged, false)));
    assert!(matches!(
        pull_entry(Some(&merged), &on_worktree(0)),
        PullEntry::View {
            phase: PullPhase::Merged,
            ..
        }
    ));
}

#[test]
fn ready_with_new_commits_or_an_unknown_base_creates() {
    let pull = session(ready(), None);
    assert_eq!(pull_entry(Some(&pull), &on_worktree(3)), PullEntry::Create);
    // A project's own folder has no worktree base: the node answers
    // `no_commits` itself if there is nothing new.
    assert_eq!(pull_entry(Some(&pull), &status(vec![])), PullEntry::Create);
}

#[test]
fn nothing_new_on_the_branch_is_paseo_s_no_commits() {
    let pull = session(ready(), None);
    assert_eq!(
        pull_entry(Some(&pull), &on_worktree(0)),
        PullEntry::Unavailable(Unavailable::NoCommits)
    );
}

#[test]
fn the_base_branch_and_a_session_outside_a_project_offer_none() {
    for access in [ForgeAccess::OnBase, ForgeAccess::NotPlaced] {
        let pull = session(access, None);
        assert_eq!(pull_entry(Some(&pull), &on_worktree(1)), PullEntry::Hidden);
    }
}

#[test]
fn a_detached_head_says_so_before_anything_the_forge_says() {
    let mut detached = on_worktree(1);
    detached.branch = None;
    let pull = session(ready(), None);
    assert_eq!(
        pull_entry(Some(&pull), &detached),
        PullEntry::Unavailable(Unavailable::NoBranch)
    );
}

#[test]
fn each_forge_refusal_keeps_its_host_and_time() {
    let host: vessel_proto::forge::ForgeHost = "github.com".parse().unwrap();
    let at = Utc.with_ymd_and_hms(2026, 11, 14, 10, 0, 0).unwrap();
    let cases = [
        (ForgeAccess::NoRemote, Unavailable::NoRemote),
        (
            ForgeAccess::NotSupported { host: host.clone() },
            Unavailable::NotSupported(host.clone()),
        ),
        (
            ForgeAccess::SignedOut {
                host: host.clone(),
                gh: GhState::Missing,
            },
            Unavailable::SignedOut(host.clone()),
        ),
        (
            ForgeAccess::RateLimited {
                host: host.clone(),
                until: at,
            },
            Unavailable::RateLimited(host.clone(), at),
        ),
        (
            ForgeAccess::Unreachable {
                host: host.clone(),
                retry_at: at,
            },
            Unavailable::Unreachable(host, at),
        ),
        (ForgeAccess::Limit, Unavailable::Limit),
        (ForgeAccess::Ended, Unavailable::Ended),
        (ForgeAccess::Unknown, Unavailable::Unknown),
    ];
    for (access, why) in cases {
        let pull = session(access, None);
        assert_eq!(
            pull_entry(Some(&pull), &on_worktree(1)),
            PullEntry::Unavailable(why)
        );
    }
}
