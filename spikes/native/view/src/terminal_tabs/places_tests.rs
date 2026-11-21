//! The place rules of a workspace's shells (ADR 0121's native consumer).

use std::path::PathBuf;

use chrono::Utc;
use vessel_proto::terminals::{
    TerminalInfo, TerminalKind, TerminalList, TerminalPlace, WORKSPACE_TERMINALS_PROTOCOL,
};
use vessel_proto::workspace::RelPath;
use vessel_proto::{ProjectId, ProtocolVersion, SessionId, TerminalId, WorktreeId};

use super::{
    beside_sessions, folder_sessions, holds, in_place, place_for, workspace_places,
    workspace_sessions,
};
use crate::board::{Attention, Card, CardDetail, Phrase, RunsIn};

fn card(detail: CardDetail) -> Card {
    Card {
        session: SessionId::random(),
        structured: true,
        live: true,
        title: "work".to_owned(),
        machine: "laptop".to_owned(),
        agent: "Claude".to_owned(),
        lane: None,
        phrase: Phrase::Starting,
        attention: Attention::AtRest,
        at: Utc::now(),
        detail,
    }
}

fn shell(place: TerminalPlace) -> TerminalInfo {
    TerminalInfo::new(
        TerminalId::random(),
        place,
        "/work".into(),
        TerminalKind::Shell {
            program: "/bin/sh".into(),
        },
        Utc::now(),
    )
}

fn at_40() -> Option<ProtocolVersion> {
    WORKSPACE_TERMINALS_PROTOCOL
}

fn below_40() -> Option<ProtocolVersion> {
    WORKSPACE_TERMINALS_PROTOCOL.map(|since| ProtocolVersion::new(since.get().saturating_sub(1)))
}

#[test]
fn a_new_shell_opens_in_the_sessions_workspace_from_40() {
    let worktree = WorktreeId::random();
    let project = ProjectId::random();
    let in_worktree = card(CardDetail {
        worktree: Some(worktree),
        runs_in: Some(RunsIn::Worktree(project)),
        cwd: Some("/work/app-wt".to_owned()),
        ..CardDetail::default()
    });
    assert_eq!(
        place_for(&in_worktree, at_40()),
        TerminalPlace::Worktree { worktree }
    );
    let in_project = card(CardDetail {
        project: Some(project),
        runs_in: Some(RunsIn::Checkout(project)),
        cwd: Some("/work/app/web".to_owned()),
        ..CardDetail::default()
    });
    assert_eq!(
        place_for(&in_project, at_40()),
        TerminalPlace::Project { project, dir: None }
    );
    let in_folder = card(CardDetail {
        cwd: Some("/work/notes".to_owned()),
        ..CardDetail::default()
    });
    assert_eq!(
        place_for(&in_folder, at_40()),
        TerminalPlace::Folder {
            path: PathBuf::from("/work/notes")
        }
    );
    // No folder of its own: beside the session, as the page and the node.
    let folderless = card(CardDetail::default());
    assert_eq!(
        place_for(&folderless, at_40()),
        TerminalPlace::Session {
            session: folderless.session
        }
    );
}

#[test]
fn below_40_and_unknown_a_shell_stays_beside_the_session() {
    let worktree = card(CardDetail {
        worktree: Some(WorktreeId::random()),
        ..CardDetail::default()
    });
    let beside = TerminalPlace::Session {
        session: worktree.session,
    };
    assert!(!workspace_places(below_40()));
    assert!(!workspace_places(None));
    assert_eq!(place_for(&worktree, below_40()), beside);
    assert_eq!(place_for(&worktree, None), beside);
}

/// The coordinator's ruling on 832's review R2: a project's strip takes
/// every project place of its project, an agent's subfolder shell too.
#[test]
fn a_projects_strip_takes_every_dir_of_its_project_and_no_other_place() {
    let project = ProjectId::random();
    let asked = TerminalPlace::Project { project, dir: None };
    let root = shell(asked.clone());
    let sub = shell(TerminalPlace::Project {
        project,
        dir: Some("web".parse::<RelPath>().unwrap()),
    });
    let other = shell(TerminalPlace::Project {
        project: ProjectId::random(),
        dir: None,
    });
    let worktree = shell(TerminalPlace::Worktree {
        worktree: WorktreeId::random(),
    });
    assert!(in_place(&root, &asked));
    assert!(in_place(&sub, &asked));
    assert!(!in_place(&other, &asked));
    assert!(!in_place(&worktree, &asked));
    let list = TerminalList {
        terminals: vec![root.clone(), sub.clone(), other, worktree],
        revision: 0,
    };
    let mut expected = vec![root.id, sub.id];
    expected.sort_unstable();
    assert_eq!(super::super::standalone(&list, &asked), expected);
}

/// A folder's list is the node's, matched on the canonical path the app
/// cannot work out: any folder place it lists is the workspace's.
#[test]
fn a_folders_strip_takes_what_the_node_matched() {
    let asked = TerminalPlace::Folder {
        path: PathBuf::from("/work/./notes"),
    };
    let listed = shell(TerminalPlace::Folder {
        path: PathBuf::from("/work/notes"),
    });
    assert!(in_place(&listed, &asked));
    let beside = shell(TerminalPlace::Session {
        session: SessionId::random(),
    });
    assert!(!in_place(&beside, &asked));
}

#[test]
fn the_strip_reads_the_shells_beside_the_workspaces_unarchived_sessions() {
    let worktree = WorktreeId::random();
    let in_worktree = |archived: bool| {
        let mut detail = CardDetail {
            worktree: Some(worktree),
            ..CardDetail::default()
        };
        if archived {
            detail.marks.archived_at = Some(Utc::now());
        }
        card(detail)
    };
    let shown = in_worktree(false);
    let neighbour = in_worktree(false);
    let archived = in_worktree(true);
    let elsewhere = card(CardDetail {
        worktree: Some(WorktreeId::random()),
        ..CardDetail::default()
    });
    let cards = [&shown, &neighbour, &archived, &elsewhere];
    let sessions = workspace_sessions(&shown, cards);
    assert_eq!(
        sessions,
        [shown.session, neighbour.session].into_iter().collect()
    );
    // A session with no folder is a workspace of one.
    let folderless = card(CardDetail::default());
    let alone = card(CardDetail::default());
    assert_eq!(
        workspace_sessions(&folderless, [&folderless, &alone]),
        [folderless.session].into_iter().collect()
    );

    let beside_neighbour = shell(TerminalPlace::Session {
        session: neighbour.session,
    });
    let mut ended = shell(TerminalPlace::Session {
        session: shown.session,
    });
    ended.status = vessel_proto::SessionStatus::Exited { code: Some(0) };
    let beside_elsewhere = shell(TerminalPlace::Session {
        session: elsewhere.session,
    });
    let list = TerminalList {
        terminals: vec![beside_neighbour.clone(), ended, beside_elsewhere],
        revision: 0,
    };
    let read = beside_sessions(&list, &sessions);
    assert_eq!(
        read.iter().map(|info| info.id).collect::<Vec<_>>(),
        vec![beside_neighbour.id]
    );
}

/// A notice's click finds the session that holds its terminal as the page
/// does: its own shell, its worktree, any folder of its project, its folder
/// as a path. A folder that matches no session's is asked of the node,
/// once per folder.
#[test]
fn a_session_holds_its_own_shells_its_workspaces_and_its_folder_as_a_path() {
    let (worktree, project) = (WorktreeId::random(), ProjectId::random());
    let in_worktree = card(CardDetail {
        worktree: Some(worktree),
        runs_in: Some(RunsIn::Worktree(project)),
        ..CardDetail::default()
    });
    let in_project = card(CardDetail {
        runs_in: Some(RunsIn::Checkout(project)),
        ..CardDetail::default()
    });
    let in_folder = card(CardDetail {
        cwd: Some("/work/notes/".to_owned()),
        ..CardDetail::default()
    });
    let beside = TerminalPlace::Session {
        session: in_worktree.session,
    };
    assert!(holds(&in_worktree, &beside));
    assert!(!holds(&in_project, &beside));
    assert!(holds(&in_worktree, &TerminalPlace::Worktree { worktree }));
    let below = TerminalPlace::Project {
        project,
        dir: Some("crates".parse::<RelPath>().unwrap()),
    };
    assert!(holds(&in_project, &below), "any folder of its project");
    assert!(
        !holds(&in_worktree, &below),
        "a worktree is a workspace of its own"
    );
    let folder = |path: &str| TerminalPlace::Folder {
        path: PathBuf::from(path),
    };
    assert!(holds(&in_folder, &folder("/work/./notes")));
    assert!(!holds(&in_folder, &folder("/real/notes")));
    let again = card(CardDetail {
        cwd: Some("/work/notes/".to_owned()),
        ..CardDetail::default()
    });
    assert_eq!(
        folder_sessions([&in_worktree, &in_folder, &in_project, &again]),
        vec![(in_folder.session, PathBuf::from("/work/notes/"))]
    );
}
