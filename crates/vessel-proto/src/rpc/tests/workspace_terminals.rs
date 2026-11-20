//! ADR 0121: a workspace place (`worktree`, `folder`) and `worktree.remove`'s
//! `end_terminals` need `WORKSPACE_TERMINALS_PROTOCOL`; an older peer's
//! lists leave such terminals out, and the older places stay as they were.
use std::path::PathBuf;

use super::*;
use crate::terminals::{
    CreateTerminal, ListTerminals, TerminalInfo, TerminalKind, TerminalList, TerminalPlace,
    WORKSPACE_TERMINALS_PROTOCOL,
};
use crate::workspace::{IfLost, RemoveWorktree};
use crate::{SessionId, TerminalId, WorktreeId};

fn folder() -> TerminalPlace {
    TerminalPlace::Folder {
        path: PathBuf::from("/work/site"),
    }
}

fn shell(place: TerminalPlace) -> TerminalInfo {
    TerminalInfo::new(
        TerminalId::random(),
        place,
        PathBuf::from("/work/site"),
        TerminalKind::Shell {
            program: PathBuf::from("/bin/sh"),
        },
        chrono::Utc::now(),
    )
}

#[test]
fn a_workspace_place_and_ending_a_worktrees_terminals_need_their_protocol() {
    // The newest protocol until the next one lands.
    assert_eq!(WORKSPACE_TERMINALS_PROTOCOL, Some(crate::PROTOCOL_VERSION));
    let create = |place| {
        CreateTerminal {
            place,
            size: None,
            title: None,
        }
        .into_call()
    };
    let beside = TerminalPlace::Session {
        session: SessionId::random(),
    };
    assert_eq!(create(beside).since(), Some(ProtocolVersion::new(11)));
    assert_eq!(create(folder()).since(), WORKSPACE_TERMINALS_PROTOCOL);
    let worktree = TerminalPlace::Worktree {
        worktree: WorktreeId::random(),
    };
    let listed = ListTerminals {
        place: Some(worktree),
        changed_from: None,
    };
    assert_eq!(listed.into_call().since(), WORKSPACE_TERMINALS_PROTOCOL);
    let remove = |end_terminals| {
        RemoveWorktree {
            worktree: WorktreeId::random(),
            if_lost: IfLost::Keep,
            end_terminals,
        }
        .into_call()
    };
    assert!(remove(false).since() < WORKSPACE_TERMINALS_PROTOCOL);
    assert_eq!(remove(true).since(), WORKSPACE_TERMINALS_PROTOCOL);
}

#[test]
fn an_older_peers_list_leaves_out_the_workspace_places_only() {
    let since = WORKSPACE_TERMINALS_PROTOCOL.unwrap();
    let older = ProtocolVersion::new(since.get().saturating_sub(1));
    let beside = shell(TerminalPlace::Session {
        session: SessionId::random(),
    });
    let list = TerminalList {
        terminals: vec![shell(folder()), beside.clone()],
        revision: 3,
    };
    let reply = Reply::ListTerminals(list.clone());
    let Some(Reply::ListTerminals(seen)) = reply.for_protocol(older) else {
        panic!("an older peer's list is rewritten");
    };
    assert_eq!(seen.terminals, vec![beside]);
    assert_eq!(seen.revision, 3);
    assert_eq!(reply.for_protocol(since), None, "a current peer reads it as it is");
    assert!(folder().readable_by(since));
    assert!(!folder().readable_by(older));
}
