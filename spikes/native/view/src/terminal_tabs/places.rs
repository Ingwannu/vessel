//! Where a workspace's shells are (ADR 0121's native consumer): the place
//! a new shell opens in, which listed shells a workspace's strip shows, and
//! the sessions whose own shells it shows as well. The page keeps the same
//! rules (`data/workspace/terminal-place.ts`, TRACKER 829), and the node
//! places an agent's shell by them (TRACKER 832).
//!
//! [Decision Log]
//! - Purpose: from protocol 40 a terminal belongs to its workspace folder,
//!   as Paseo keys one by its workspace (`use-workspace-terminals.ts`
//!   75-100). A shell opened beside the session in view died with that
//!   session's archive and showed in no other session's tabs.
//! - Constraint: a native layout is keyed by a session, not a workspace, so
//!   the workspace is the session's: its worktree, its project, or outside
//!   projects its folder. A session with no folder of its own keeps the
//!   session place: the app cannot name that folder (the page and the node
//!   do the same).
//! - Strip: the place's list as the node matched it, plus the live shells
//!   beside the workspace's unarchived sessions. Those are made by older
//!   nodes, by agents below 40, or before the upgrade, and the page reads
//!   them the same way (`readStrip`). A project's strip takes every
//!   project place of its project, whatever its `dir` (the coordinator's
//!   ruling on 832's review R2): an agent in a subfolder opens there. A
//!   folder's list is the node's, matched on its canonical path, which the
//!   app cannot work out.
//! - Older nodes: below 40 everything stays beside the session, as before.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use vessel_proto::terminals::{
    TerminalInfo, TerminalList, TerminalPlace, WORKSPACE_TERMINALS_PROTOCOL,
};
use vessel_proto::{ProjectId, ProtocolVersion, SessionId, WorktreeId};

use super::shows_as_tab;
use crate::board::Card;

/// Whether a node speaking `protocol` keeps terminals by workspace place.
#[must_use]
pub fn workspace_places(protocol: Option<ProtocolVersion>) -> bool {
    protocol
        .zip(WORKSPACE_TERMINALS_PROTOCOL)
        .is_some_and(|(speaks, since)| speaks >= since)
}

/// The place a new shell opens in for `card`'s session, on a node speaking
/// `protocol`.
#[must_use]
pub fn place_for(card: &Card, protocol: Option<ProtocolVersion>) -> TerminalPlace {
    let beside = TerminalPlace::Session {
        session: card.session,
    };
    if !workspace_places(protocol) {
        return beside;
    }
    match Workspace::of(card) {
        Workspace::Worktree(worktree) => TerminalPlace::Worktree { worktree },
        Workspace::Project(project) => TerminalPlace::Project { project, dir: None },
        Workspace::Folder(path) => TerminalPlace::Folder {
            path: PathBuf::from(path),
        },
        Workspace::Alone(_) => beside,
    }
}

/// Whether opening `card`'s session shows a terminal kept at `place` among
/// its tabs: its own shell, its worktree's, any of its project's, or its
/// folder's when it has neither (the page's `holds`,
/// `data/terminal-notices.ts`). A folder is compared as a path, part by
/// part. The node keeps a folder canonical (ADR 0121), which the app
/// cannot work out, so a folder reached through a link or a `..` is asked
/// of the node with [`folder_sessions`].
#[must_use]
pub fn holds(card: &Card, place: &TerminalPlace) -> bool {
    match place {
        TerminalPlace::Session { session } => card.session == *session,
        TerminalPlace::Worktree { worktree } => {
            matches!(Workspace::of(card), Workspace::Worktree(at) if at == *worktree)
        }
        TerminalPlace::Project { project, .. } => {
            matches!(Workspace::of(card), Workspace::Project(at) if at == *project)
        }
        TerminalPlace::Folder { path } => {
            matches!(Workspace::of(card), Workspace::Folder(at) if Path::new(at) == path)
        }
    }
}

/// `cards`' sessions whose workspace is a folder of their own, one per
/// folder in their order: the candidates the node is asked about when a
/// folder terminal matches none of them as a path.
#[must_use]
pub fn folder_sessions<'a>(cards: impl IntoIterator<Item = &'a Card>) -> Vec<(SessionId, PathBuf)> {
    let mut seen = BTreeSet::new();
    cards
        .into_iter()
        .filter_map(|card| match Workspace::of(card) {
            Workspace::Folder(path) => seen
                .insert(path)
                .then(|| (card.session, PathBuf::from(path))),
            Workspace::Worktree(_) | Workspace::Project(_) | Workspace::Alone(_) => None,
        })
        .collect()
}

/// Whether `info`, from the node's list for `asked`, belongs in that
/// workspace's strip: any project place of a project, any folder the node
/// matched for a folder, the very place otherwise.
#[must_use]
pub fn in_place(info: &TerminalInfo, asked: &TerminalPlace) -> bool {
    match asked {
        TerminalPlace::Project { project, .. } => matches!(
            &info.place,
            TerminalPlace::Project { project: at, .. } if at == project
        ),
        TerminalPlace::Folder { .. } => matches!(info.place, TerminalPlace::Folder { .. }),
        TerminalPlace::Session { .. } | TerminalPlace::Worktree { .. } => info.place == *asked,
    }
}

/// The sessions whose own shells `card`'s workspace strip shows as well:
/// `card`'s, and the unarchived sessions of `cards` (its machine's) in the
/// same workspace.
#[must_use]
pub fn workspace_sessions<'a>(
    card: &Card,
    cards: impl IntoIterator<Item = &'a Card>,
) -> BTreeSet<SessionId> {
    let shown = Workspace::of(card);
    let mut sessions: BTreeSet<SessionId> = cards
        .into_iter()
        .filter(|other| other.detail.marks.archived_at.is_none())
        .filter(|other| Workspace::of(other) == shown)
        .map(|other| other.session)
        .collect();
    let _added = sessions.insert(card.session);
    sessions
}

/// The live shells in `list` beside one of `sessions`.
#[must_use]
pub fn beside_sessions(list: &TerminalList, sessions: &BTreeSet<SessionId>) -> Vec<TerminalInfo> {
    list.terminals
        .iter()
        .filter(|info| {
            info.place
                .session()
                .is_some_and(|session| sessions.contains(&session))
                && shows_as_tab(info)
        })
        .cloned()
        .collect()
}

/// A session's workspace, as its card says.
#[derive(Debug, PartialEq, Eq)]
enum Workspace<'a> {
    Worktree(WorktreeId),
    Project(ProjectId),
    Folder(&'a str),
    /// No folder of its own: a workspace of one.
    Alone(SessionId),
}

impl<'a> Workspace<'a> {
    fn of(card: &'a Card) -> Self {
        let detail = &card.detail;
        if let Some(worktree) = detail.worktree {
            Self::Worktree(worktree)
        } else if let Some(runs_in) = detail.runs_in {
            Self::Project(runs_in.project())
        } else if let Some(cwd) = detail.cwd.as_deref() {
            Self::Folder(cwd)
        } else {
            Self::Alone(card.session)
        }
    }
}

#[cfg(test)]
#[path = "places_tests.rs"]
mod tests;
