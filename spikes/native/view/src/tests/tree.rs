//! `web/src/data/tree.test.ts`, "the session tree".

use vessel_proto::SessionStatus;

use super::fixture::{agent_of, fleet, machine, session, short};
use crate::Fleet;
use crate::board::Origin;

const RUNNING: SessionStatus = SessionStatus::Running;

/// A person's session on laptop whose agent started one on gpu-box, which
/// started one on build-server (DESIGN.md §2.4), and an unrelated one.
fn state(listed: &[&str]) -> Fleet {
    let exited = SessionStatus::Exited { code: Some(0) };
    let all = [
        machine(
            "laptop",
            true,
            &[
                session("3f2a", "2026-11-07T10:00:00Z", RUNNING),
                session("9aa0", "2026-11-07T11:00:00Z", RUNNING),
            ],
        ),
        machine(
            "gpu-box",
            true,
            &[agent_of(
                session("a91c", "2026-11-07T10:01:00Z", RUNNING),
                "3f2a",
            )],
        ),
        machine(
            "build-server",
            true,
            &[agent_of(
                session("c07e", "2026-11-07T10:02:00Z", exited),
                "a91c",
            )],
        ),
    ];
    fleet(
        all.into_iter()
            .filter(|machine| listed.contains(&machine.name.as_str()))
            .collect(),
    )
}

#[test]
fn nests_agent_started_sessions_under_their_creator_across_nodes() {
    let fleet = state(&["laptop", "gpu-box", "build-server"]);
    let rows = fleet.tree(None);
    let seen: Vec<(String, usize, &str)> = rows
        .iter()
        .map(|row| (short(row.card, 4), row.depth, row.card.machine.as_str()))
        .collect();
    assert_eq!(
        seen,
        [
            ("9aa0".to_owned(), 0, "laptop"),
            ("3f2a".to_owned(), 0, "laptop"),
            ("a91c".to_owned(), 1, "gpu-box"),
            ("c07e".to_owned(), 2, "build-server"),
        ]
    );
    assert_eq!(
        rows[2].parent.map(|parent| short(parent, 4)).as_deref(),
        Some("3f2a")
    );
    // A child's parent may run on another node; the row says which.
    let parents: Vec<Option<&str>> = rows
        .iter()
        .map(|row| row.parent.map(|parent| parent.machine.as_str()))
        .collect();
    assert_eq!(parents, [None, None, Some("laptop"), Some("gpu-box")]);
}

#[test]
fn makes_a_session_whose_creator_is_not_listed_a_root() {
    let fleet = state(&["gpu-box", "build-server"]);
    let seen: Vec<(String, usize)> = fleet
        .tree(None)
        .iter()
        .map(|row| (short(row.card, 4), row.depth))
        .collect();
    assert_eq!(seen, [("a91c".to_owned(), 0), ("c07e".to_owned(), 1)]);
}

#[test]
fn says_which_session_started_one_and_on_which_machine() {
    let fleet = state(&["laptop", "gpu-box", "build-server"]);
    let rows = fleet.tree(None);
    let origin = |at: usize| match rows[at].origin() {
        Origin::Agent {
            id,
            parent,
            elsewhere,
        } => Some((id, parent.is_some(), elsewhere)),
        Origin::Owner | Origin::Device(_) => None,
    };
    assert!(origin(0).is_none());
    assert_eq!(origin(2), Some(("3f2a".to_owned(), true, Some("laptop"))));
    assert_eq!(origin(3), Some(("a91c".to_owned(), true, Some("gpu-box"))));
    // Listed alone, a child still names its creator, without a link.
    let alone = state(&["gpu-box"]);
    let rows = alone.tree(None);
    assert!(matches!(
        rows[0].origin(),
        Origin::Agent { id, parent: None, elsewhere: None } if id == "3f2a"
    ));
}
