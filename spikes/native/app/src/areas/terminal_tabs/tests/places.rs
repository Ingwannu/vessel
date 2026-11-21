//! ADR 0121's native consumer in a terminal tab strip: from protocol 40 a
//! new shell opens in the session's workspace, the strip lists that place
//! (a project's every dir) and the shells beside the workspace's sessions,
//! and below 40 everything stays beside the session.

use chrono::Utc;
use gpui::TestAppContext;
use vessel_app_link::terminals::{Reply, Request};
use vessel_app_view::Card;
use vessel_app_view::board::{Attention, CardDetail, Phrase, RunsIn};
use vessel_proto::terminals::{TerminalInfo, TerminalKind, TerminalList, TerminalPlace};
use vessel_proto::workspace::RelPath;
use vessel_proto::{ProjectId, ProtocolVersion, SessionId, TerminalId, WorktreeId};

use super::{answer, open, sent};
use crate::shell::Shell;

fn card(session: SessionId, detail: CardDetail) -> Card {
    Card {
        session,
        structured: true,
        live: true,
        title: "Fix the cart".to_owned(),
        machine: "laptop".to_owned(),
        agent: "Claude".to_owned(),
        lane: None,
        phrase: Phrase::Starting,
        attention: Attention::AtRest,
        at: Utc::now(),
        detail,
    }
}

fn shell_at(place: TerminalPlace) -> TerminalInfo {
    TerminalInfo::new(
        TerminalId::random(),
        place,
        "/work/shop".into(),
        TerminalKind::Shell {
            program: "/bin/sh".into(),
        },
        Utc::now(),
    )
}

/// The laptop lists `cards` and speaks `protocol`.
fn fleet(shell: &mut Shell, cards: Vec<Card>, protocol: ProtocolVersion) {
    let laptop = shell.fleet.machines.first_mut().unwrap();
    laptop.sessions = cards;
    laptop.protocol = Some(protocol);
}

/// The places of the plain lists among `asked`.
fn listed_places(asked: Vec<Request>) -> Vec<TerminalPlace> {
    asked
        .into_iter()
        .filter_map(|request| {
            if let Request::List {
                place,
                changed_from: None,
            } = request
            {
                Some(place)
            } else {
                None
            }
        })
        .collect()
}

fn listed(shell: &Shell) -> Vec<TerminalId> {
    shell
        .areas
        .terminal_tabs
        .listed
        .as_ref()
        .map(|(_, ids)| ids.clone())
        .unwrap_or_default()
}

#[gpui::test]
fn a_worktree_sessions_shell_opens_there_and_its_strip_takes_the_shells_beside_it(
    cx: &mut TestAppContext,
) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let worktree = WorktreeId::random();
    let neighbour = SessionId::random();
    let in_worktree = |session| {
        card(
            session,
            CardDetail {
                worktree: Some(worktree),
                runs_in: Some(RunsIn::Worktree(ProjectId::random())),
                ..CardDetail::default()
            },
        )
    };
    shell.update_in(cx, |shell, window, cx| {
        fleet(
            shell,
            vec![in_worktree(session), in_worktree(neighbour)],
            vessel_proto::PROTOCOL_VERSION,
        );
        shell.terminal_tab_new(None, window, cx);
    });
    let place = TerminalPlace::Worktree { worktree };
    let created = sent(&probe, cx);
    assert!(
        matches!(created.as_slice(), [Request::Create { place: at }] if *at == place),
        "{created:?}"
    );

    // The poll asks the place's list and the shells beside both sessions.
    shell.update_in(cx, |shell, _window, _cx| shell.terminal_tabs_poll());
    let lists = listed_places(sent(&probe, cx));
    assert!(lists.contains(&place), "{lists:?}");
    for beside in [session, neighbour] {
        assert!(
            lists.contains(&TerminalPlace::Session { session: beside }),
            "{lists:?}"
        );
    }

    let in_place = shell_at(place.clone());
    let beside_neighbour = shell_at(TerminalPlace::Session { session: neighbour });
    shell.update_in(cx, |shell, _window, cx| {
        let list = |terminals| TerminalList {
            terminals,
            revision: 1,
        };
        answer(
            shell,
            Reply::Listed {
                place: place.clone(),
                result: Ok(list(vec![in_place.clone()])),
                waits: true,
            },
            cx,
        );
        answer(
            shell,
            Reply::Listed {
                place: TerminalPlace::Session { session: neighbour },
                result: Ok(list(vec![beside_neighbour.clone()])),
                waits: false,
            },
            cx,
        );
        answer(
            shell,
            Reply::Listed {
                place: TerminalPlace::Session { session },
                result: Ok(list(Vec::new())),
                waits: false,
            },
            cx,
        );
        let mut expected = vec![in_place.id, beside_neighbour.id];
        expected.sort_unstable();
        assert_eq!(listed(shell), expected);
    });
}

/// The coordinator's ruling on 832's review R2: a project workspace's strip
/// takes every project place of its project, so an agent's shell in a
/// subfolder shows there.
#[gpui::test]
fn a_project_strip_shows_an_agents_subfolder_shell(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let project = ProjectId::random();
    let place = TerminalPlace::Project { project, dir: None };
    let in_sub = shell_at(TerminalPlace::Project {
        project,
        dir: Some("web".parse::<RelPath>().unwrap()),
    });
    let other = shell_at(TerminalPlace::Project {
        project: ProjectId::random(),
        dir: None,
    });
    shell.update_in(cx, |shell, _window, cx| {
        fleet(
            shell,
            vec![card(
                session,
                CardDetail {
                    project: Some(project),
                    runs_in: Some(RunsIn::Checkout(project)),
                    cwd: Some("/work/shop".to_owned()),
                    ..CardDetail::default()
                },
            )],
            vessel_proto::PROTOCOL_VERSION,
        );
        shell.terminal_tabs_poll();
        answer(
            shell,
            Reply::Listed {
                place: place.clone(),
                result: Ok(TerminalList {
                    terminals: vec![in_sub.clone(), other],
                    revision: 1,
                }),
                waits: true,
            },
            cx,
        );
        assert_eq!(listed(shell), vec![in_sub.id]);
    });
}

#[gpui::test]
fn below_40_a_shell_stays_beside_the_session_and_nothing_else_is_asked(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let older = vessel_proto::terminals::WORKSPACE_TERMINALS_PROTOCOL
        .map(|since| ProtocolVersion::new(since.get().saturating_sub(1)))
        .unwrap();
    shell.update_in(cx, |shell, window, cx| {
        fleet(
            shell,
            vec![card(
                session,
                CardDetail {
                    worktree: Some(WorktreeId::random()),
                    ..CardDetail::default()
                },
            )],
            older,
        );
        shell.terminal_tab_new(None, window, cx);
        shell.terminal_tabs_poll();
    });
    let beside = TerminalPlace::Session { session };
    let asked = sent(&probe, cx);
    assert!(
        matches!(
            asked.as_slice(),
            [
                Request::Create { place: created },
                Request::List { place: listed, changed_from: None },
            ] if *created == beside && *listed == beside
        ),
        "{asked:?}"
    );
}
