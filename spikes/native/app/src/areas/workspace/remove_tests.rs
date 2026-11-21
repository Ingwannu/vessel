//! A worktree kept only by its live terminals (ADR 0121, ruling 2): the
//! panel counts them, says Paseo's line, and its usual buttons remove with
//! `end_terminals` only after the count is read again unchanged; a live
//! session or a node below 40 leaves it as before.

use chrono::Utc;
use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use tokio::sync::mpsc::Receiver;
use vessel_app_kit::ui::{IntoElement, Render, div};
use vessel_app_link::Command;
use vessel_app_link::terminals::Request as TerminalRequest;
use vessel_app_link::workspace::{Ask, WorktreeAsk};
use vessel_app_view::board::{Attention, CardDetail, Phrase};
use vessel_app_view::{Card, Machine, Reach};
use vessel_proto::registry::DiffStat;
use vessel_proto::terminals::{TerminalInfo, TerminalKind, TerminalList, TerminalPlace};
use vessel_proto::workspace::{
    IfLost, KeptReason, RemoveWorktree, RemoveWorktreeOutcome, SetupState, WorktreeInfo,
};
use vessel_proto::{Creator, ProjectId, ProtocolVersion, SessionId, TerminalId, WorktreeId};

use super::{Removing, closing_terminals};
use crate::areas::workspace::detail::Detail;
use crate::shell::{Shell, language_tests};

struct Probe(Entity<Shell>, Receiver<Command>);

impl Render for Probe {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<'_, Self>,
    ) -> impl IntoElement {
        div()
    }
}

fn worktree() -> WorktreeInfo {
    WorktreeInfo {
        id: WorktreeId::random(),
        project: ProjectId::random(),
        path: "/work/app-wt".into(),
        branch: None,
        created_branch: None,
        base: None,
        created_at: Utc::now(),
        created_by: Creator::Owner,
        created_for: None,
        sessions: Vec::new(),
        setup: SetupState::None,
        kept: None,
    }
}

fn terminal(folder: &str, live: bool) -> TerminalInfo {
    let mut info = TerminalInfo::new(
        TerminalId::random(),
        TerminalPlace::Session {
            session: SessionId::random(),
        },
        folder.into(),
        TerminalKind::Shell {
            program: "/bin/sh".into(),
        },
        Utc::now(),
    );
    if !live {
        info.status = vessel_proto::SessionStatus::Exited { code: Some(0) };
    }
    info
}

fn list(terminals: Vec<TerminalInfo>) -> TerminalList {
    TerminalList {
        terminals,
        revision: 0,
    }
}

/// A shell whose projects machine speaks `protocol`, showing `worktree`'s
/// removal kept for being in use, with `cards` listed.
fn open<'a>(
    cx: &'a mut TestAppContext,
    worktree: &WorktreeInfo,
    protocol: ProtocolVersion,
    cards: Vec<Card>,
) -> (Entity<Probe>, &'a mut VisualTestContext) {
    language_tests::init(cx);
    let worktree = worktree.clone();
    cx.add_window_view(move |window, cx| {
        let mut parts = None;
        let shell = cx.new(|cx| {
            let (mut shell, commands) = language_tests::with_commands(window, cx);
            shell.fleet.machines = vec![Machine {
                name: "laptop".to_owned(),
                reach: Reach::Online,
                sessions: cards,
                agents: Vec::new(),
                protocol: Some(protocol),
                version: None,
                node: None,
                connection_epoch: None,
            }];
            let mut detail = Detail::new(worktree.project);
            detail.removing = Some(Removing::new(worktree, vec![KeptReason::InUse], None, None));
            shell.areas.workspace.page.detail = Some(detail);
            parts = Some(commands);
            shell
        });
        Probe(shell, parts.unwrap())
    })
}

fn sent(probe: &Entity<Probe>, cx: &mut VisualTestContext) -> Vec<Command> {
    probe.update(cx, |probe, _cx| {
        let mut sent = Vec::new();
        while let Ok(command) = probe.1.try_recv() {
            sent.push(command);
        }
        sent
    })
}

fn removing(shell: &Shell) -> &Removing {
    shell
        .areas
        .workspace
        .page
        .detail
        .as_ref()
        .and_then(|detail| detail.removing.as_ref())
        .unwrap()
}

#[gpui::test]
fn each_kept_reply_supplies_its_own_warning_count(cx: &mut TestAppContext) {
    let worktree = worktree();
    let (probe, cx) = open(cx, &worktree, vessel_proto::PROTOCOL_VERSION, Vec::new());
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let ask = RemoveWorktree {
        worktree: worktree.id,
        if_lost: IfLost::Keep,
        end_terminals: false,
    };
    let dirty = KeptReason::Uncommitted { files: 1 };
    shell.update_in(cx, |shell, _window, _cx| {
        for diff_stat in [
            Some(DiffStat {
                additions: 2,
                deletions: 1,
            }),
            None,
        ] {
            shell.worktree_removed(
                "laptop",
                &ask,
                Ok(RemoveWorktreeOutcome::Kept {
                    reasons: vec![dirty],
                    diff_stat,
                }),
            );
            assert_eq!(removing(shell).reasons, [dirty]);
            assert_eq!(removing(shell).diff_stat, diff_stat);
        }
    });
}

#[gpui::test]
fn terminals_alone_are_closed_with_the_removal_after_the_count_is_read_again(
    cx: &mut TestAppContext,
) {
    let worktree = worktree();
    let (probe, cx) = open(cx, &worktree, vessel_proto::PROTOCOL_VERSION, Vec::new());
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    shell.update_in(cx, |shell, _window, _cx| {
        shell.removal_count_terminals("laptop");
    });
    assert!(matches!(
        sent(&probe, cx).as_slice(),
        [Command::Terminals(ask)] if matches!(ask.request, TerminalRequest::ListAll)
    ));
    let two = list(vec![
        terminal("/work/app-wt", true),
        terminal("/work/app-wt/web", true),
        terminal("/work/app-wt", false),
        terminal("/work/other", true),
    ]);
    shell.update_in(cx, |shell, _window, cx| {
        shell.removal_terminals_listed("laptop", Some(&two), cx);
        assert_eq!(removing(shell).terminals, Some(2));
        assert_eq!(closing_terminals(shell, removing(shell)), Some(2));
        // Discard pressed: the count is read again before anything is
        // removed.
        shell.remove_with(IfLost::Discard, cx);
    });
    assert!(matches!(
        sent(&probe, cx).as_slice(),
        [Command::Terminals(ask)] if matches!(ask.request, TerminalRequest::ListAll)
    ));
    // It changed meanwhile: the press stops and the line says the new one.
    let three = list(vec![
        terminal("/work/app-wt", true),
        terminal("/work/app-wt", true),
        terminal("/work/app-wt", true),
    ]);
    shell.update_in(cx, |shell, _window, cx| {
        shell.removal_terminals_listed("laptop", Some(&three), cx);
        assert_eq!(removing(shell).terminals, Some(3));
        assert_eq!(removing(shell).busy, None);
    });
    assert!(sent(&probe, cx).is_empty());
    // The snapshot pressed, unchanged: removed with its terminals, as
    // that button asks.
    shell.update_in(cx, |shell, _window, cx| {
        shell.remove_with(IfLost::Snapshot, cx);
        shell.removal_terminals_listed("laptop", Some(&three), cx);
    });
    let asked = sent(&probe, cx);
    let removal = removal_in(&asked).unwrap_or_else(|| panic!("{asked:?}"));
    assert_eq!(removal.worktree, worktree.id);
    assert_eq!(removal.if_lost, IfLost::Snapshot);
    assert!(removal.end_terminals);
}

/// A live session on the laptop running in the worktree's `web` folder.
fn session_in_worktree() -> Card {
    Card {
        session: SessionId::random(),
        structured: true,
        live: true,
        title: "Fix the cart".to_owned(),
        machine: "laptop".to_owned(),
        agent: "Claude".to_owned(),
        lane: None,
        phrase: Phrase::Starting,
        attention: Attention::AtRest,
        at: Utc::now(),
        detail: CardDetail {
            cwd: Some("/work/app-wt/web".to_owned()),
            ..CardDetail::default()
        },
    }
}

/// The worktree removal among `asked`, if one was asked.
fn removal_in(asked: &[Command]) -> Option<RemoveWorktree> {
    asked.iter().find_map(|command| {
        if let Command::Workspace {
            ask: Ask::Worktree(WorktreeAsk::Remove(remove)),
            ..
        } = command
        {
            Some(remove.clone())
        } else {
            None
        }
    })
}

#[gpui::test]
fn a_live_session_in_the_worktree_keeps_the_panel_as_before(cx: &mut TestAppContext) {
    let worktree = worktree();
    let in_it = session_in_worktree();
    let (probe, cx) = open(cx, &worktree, vessel_proto::PROTOCOL_VERSION, vec![in_it]);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    shell.update_in(cx, |shell, _window, cx| {
        shell.removal_terminals_listed(
            "laptop",
            Some(&list(vec![terminal("/work/app-wt", true)])),
            cx,
        );
        assert_eq!(removing(shell).terminals, Some(1));
        assert_eq!(
            closing_terminals(shell, removing(shell)),
            None,
            "a live session keeps it"
        );
        // A press asks as before: no count, no terminals ended.
        shell.remove_with(IfLost::Discard, cx);
    });
    let asked = sent(&probe, cx);
    assert!(
        matches!(
            asked.as_slice(),
            [Command::Workspace {
                ask: Ask::Worktree(WorktreeAsk::Remove(remove)),
                ..
            }] if remove.if_lost == IfLost::Discard && !remove.end_terminals
        ),
        "{asked:?}"
    );
}

#[gpui::test]
fn below_40_nothing_is_counted(cx: &mut TestAppContext) {
    let worktree = worktree();
    let older = vessel_proto::terminals::WORKSPACE_TERMINALS_PROTOCOL
        .map(|since| ProtocolVersion::new(since.get().saturating_sub(1)))
        .unwrap();
    let (probe, cx) = open(cx, &worktree, older, Vec::new());
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    shell.update_in(cx, |shell, _window, _cx| {
        shell.removal_count_terminals("laptop");
    });
    assert!(sent(&probe, cx).is_empty(), "nothing is counted below 40");
}

/// An app review note on 0121n: the terminals ended between the count and
/// the press, so nothing keeps the worktree, and the press finishes with
/// the plain removal.
#[gpui::test]
fn a_recount_of_none_finishes_the_press_with_the_plain_removal(cx: &mut TestAppContext) {
    let worktree = worktree();
    let (probe, cx) = open(cx, &worktree, vessel_proto::PROTOCOL_VERSION, Vec::new());
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    shell.update_in(cx, |shell, _window, cx| {
        let one = list(vec![terminal("/work/app-wt", true)]);
        shell.removal_terminals_listed("laptop", Some(&one), cx);
        shell.remove_with(IfLost::Discard, cx);
    });
    drop(sent(&probe, cx));
    shell.update_in(cx, |shell, _window, cx| {
        let ended = list(vec![terminal("/work/app-wt", false)]);
        shell.removal_terminals_listed("laptop", Some(&ended), cx);
        assert_eq!(removing(shell).terminals, Some(0));
        assert_eq!(removing(shell).busy, Some(IfLost::Discard));
    });
    let asked = sent(&probe, cx);
    let removal = removal_in(&asked).unwrap_or_else(|| panic!("{asked:?}"));
    assert_eq!(removal.worktree, worktree.id);
    assert_eq!(removal.if_lost, IfLost::Discard);
    assert!(!removal.end_terminals, "nothing is left to end");
}

/// The terminals ended, but a session now runs in the worktree: the node
/// would keep it, so the press stops.
#[gpui::test]
fn a_recount_of_none_with_a_session_in_it_stops_the_press(cx: &mut TestAppContext) {
    let worktree = worktree();
    let (probe, cx) = open(cx, &worktree, vessel_proto::PROTOCOL_VERSION, Vec::new());
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    shell.update_in(cx, |shell, _window, cx| {
        let one = list(vec![terminal("/work/app-wt", true)]);
        shell.removal_terminals_listed("laptop", Some(&one), cx);
        shell.remove_with(IfLost::Snapshot, cx);
        if let Some(laptop) = shell.fleet.machines.first_mut() {
            laptop.sessions.push(session_in_worktree());
        }
    });
    drop(sent(&probe, cx));
    shell.update_in(cx, |shell, _window, cx| {
        shell.removal_terminals_listed("laptop", Some(&list(Vec::new())), cx);
        assert_eq!(removing(shell).busy, None);
    });
    assert!(removal_in(&sent(&probe, cx)).is_none());
}
