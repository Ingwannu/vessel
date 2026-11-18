//! A workspace layout's rename (TRACKER 800): sent with the original node's
//! authority, the dialog waits for its own ticket; success names the card
//! and closes it, a refusal keeps the typed title with the reason.
use super::*;
use crate::areas::conversation::RenameRoute;
use crate::shell::language_tests;
use gpui::{AppContext as _, Render, TestAppContext, VisualTestContext};
use vessel_app_kit::ui::{Entity, IntoElement, ParentElement as _, Window, div};
use vessel_app_link::workspace::layout::{Answer, Ask};
use vessel_app_view::board::CardDetail;
use vessel_app_view::layout::RenameTicket;
use vessel_app_view::{Attention, Card, Machine, Phrase, Reach};
use vessel_proto::{SessionId, WorkspaceId};

struct Probe(Entity<Shell>);

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = vessel_app_kit::tokens(cx);
        self.0.update(cx, |shell, cx| {
            div().children(crate::areas::conversation::rename_dialog(
                shell, &tokens, window, cx,
            ))
        })
    }
}

fn card(session: SessionId) -> Card {
    Card {
        session,
        structured: true,
        live: true,
        title: "Fix the cart".to_owned(),
        machine: "laptop".to_owned(),
        agent: "Claude Code".to_owned(),
        lane: None,
        phrase: Phrase::default(),
        attention: Attention::AtRest,
        at: chrono::DateTime::UNIX_EPOCH,
        detail: CardDetail::default(),
    }
}

/// The dialog open on a workspace layout's route, its title typed and
/// Enter pressed; the request it sent.
fn sent(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext, Ask, SessionId) {
    language_tests::init(cx);
    let node = NodeId::random();
    let session = SessionId::random();
    let (probe, cx) =
        cx.add_window_view(|window, cx| Probe(cx.new(|cx| Shell::without_io(window, cx))));
    cx.update(|window, _| window.activate_window());
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| {
            shell.fleet.machines = vec![Machine {
                name: "laptop".to_owned(),
                reach: Reach::Online,
                sessions: vec![card(session)],
                agents: Vec::new(),
                protocol: Some(vessel_proto::PROTOCOL_VERSION),
                version: None,
                node: Some(node),
                connection_epoch: None,
            }];
            let route = LayoutRoute {
                workspace: LayoutKey::Workspace {
                    node,
                    workspace: WorkspaceId::random(),
                },
                machine: HostAlias::parse("laptop").unwrap(),
                node,
                target: RenameTarget::Agent(session),
                phase: RenamePhase::Editing,
            };
            shell.start_rename("Fix the cart", RenameRoute::Layout(route), window, cx);
            if let Some(rename) = shell.areas.conversation.rename.as_ref() {
                rename.field.set_text("Fix the basket", window, cx);
            }
        });
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    let ask = probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, _| {
            let route = shell.areas.conversation.rename_layout_route().unwrap();
            let RenamePhase::Sending(submission) = route.phase.clone() else {
                panic!("the rename waits for its answer");
            };
            Ask {
                node: route.node,
                machine: route.machine.clone(),
                submission,
            }
        })
    });
    (probe, cx, ask, session)
}

fn renamed(probe: &Entity<Probe>, cx: &mut VisualTestContext, answer: Answer) {
    probe.update(cx, |probe, cx| {
        probe
            .0
            .update(cx, |shell, cx| shell.layout_renamed(answer, cx));
    });
}

fn open(probe: &Entity<Probe>, cx: &mut VisualTestContext) -> bool {
    probe.read_with(cx, |probe, cx| {
        probe.0.read(cx).areas.conversation.rename.is_some()
    })
}

#[gpui::test]
fn a_workspace_rename_takes_only_its_ticket_and_names_the_card(cx: &mut TestAppContext) {
    let (probe, cx, ask, session) = sent(cx);
    assert_eq!(ask.submission.title.as_str(), "Fix the basket");
    // An earlier form's answer is not this one's.
    let mut stale = ask.clone();
    stale.submission.ticket = RenameTicket::ZERO;
    renamed(
        &probe,
        cx,
        Answer {
            ask: stale,
            result: Ok("Elsewhere".parse().unwrap()),
        },
    );
    assert!(open(&probe, cx), "still waiting for its own answer");
    renamed(
        &probe,
        cx,
        Answer {
            ask,
            result: Ok("Fix the basket".parse().unwrap()),
        },
    );
    assert!(!open(&probe, cx));
    probe.read_with(cx, |probe, cx| {
        let card = probe.0.read(cx).fleet.machines[0]
            .sessions
            .iter()
            .find(|card| card.session == session)
            .map(|card| card.title.clone());
        assert_eq!(card.as_deref(), Some("Fix the basket"));
    });
}

#[gpui::test]
fn a_workspace_refusal_keeps_the_typed_title_with_the_reason(cx: &mut TestAppContext) {
    let (probe, cx, ask, _session) = sent(cx);
    renamed(
        &probe,
        cx,
        Answer {
            ask,
            result: Err(RenameProblem::Offline),
        },
    );
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        let rename = shell.areas.conversation.rename.as_ref().unwrap();
        assert_eq!(rename.field.text(cx), "Fix the basket");
        assert_eq!(rename.error(), Some(shell.words.layout().rename_offline()));
        assert!(!rename.pending());
    });
}
