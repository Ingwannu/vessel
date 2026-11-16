//! Actual queued Search events, using only the shared framed Scenario facade.
//! Source preparation: awaits the lead's checked G4 fixture/product exposure.

use gpui::{TestAppContext, VisualTestContext};
use vessel_app_link::{
    Event,
    testing::{PeerNodes, PeerSlot, Scenario},
};
use vessel_app_view::{Machine, machine_book::Name};
use vessel_proto::search::{SearchPage, Snippet, SnippetPart};
use vessel_proto::{Call, Outcome, Reply, SessionId};

use super::*;
use crate::areas::conversation::ConversationArea;
use crate::shell::language_tests;

fn machine(context: &ConnectionContext) -> Machine {
    Machine {
        name: context.machine().as_str().to_owned(),
        reach: Reach::Online,
        sessions: Vec::new(),
        agents: Vec::new(),
        protocol: Some(vessel_proto::PROTOCOL_VERSION),
        version: None,
        node: Some(context.node()),
        connection_epoch: Some(context.epoch()),
    }
}

fn page() -> SearchPage {
    SearchPage {
        hits: vec![SearchHit {
            session: SessionId::random(),
            seq: EventSeq::new(1),
            at: chrono::DateTime::UNIX_EPOCH,
            role: MessageRole::Reply,
            snippet: Snippet {
                parts: vec![SnippetPart {
                    text: "needle".to_owned(),
                    matched: true,
                }],
                cut_before: false,
                cut_after: false,
            },
        }],
        next: None,
        reached: None,
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.executor().advance_clock(SETTLE);
    cx.run_until_parked();
}

fn answer(scene: &mut Scenario, shell: &Shell) -> Searched {
    // Ignore bounded unrelated actor notices without installing a newer Fleet.
    const MAX_EVENTS: usize = 32;
    for _ in 0..MAX_EVENTS {
        if let Event::Searched(answer) = scene.event(shell.fixture_link()).unwrap() {
            return answer;
        }
    }
    panic!("expected a genuine queued Search answer");
}

fn queued(scene: &mut Scenario, shell: &Shell) -> Searched {
    queued_at(scene, shell, PeerSlot::Primary)
}
fn queued_at(scene: &mut Scenario, shell: &Shell, peer: PeerSlot) -> Searched {
    assert!(matches!(
        scene.request_from(shell.fixture_link(), peer).unwrap(),
        Call::SearchMessages(_)
    ));
    scene
        .reply_to(
            shell.fixture_link(),
            peer,
            Outcome::Ok(Reply::SearchMessages(page())),
        )
        .unwrap();
    let answer = answer(scene, shell);
    assert!(
        answer.delivery.is_some(),
        "only the real actor issues the guard"
    );
    assert!(answer.result.is_ok());
    assert_eq!(answer.connection, scene.context_of(peer));
    answer
}

#[gpui::test]
fn actual_search_reply_survives_replacing_the_conversation_resident(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (link, mut scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let (shell, cx) = cx.add_window_view(move |window, cx| {
        let mut shell = Shell::with_test_link(window, cx, link);
        shell.fleet.machines = vec![machine(&context)];
        shell.message_search_query("needle", cx);
        shell.areas.conversation = ConversationArea::new(cx);
        shell
    });
    settle(cx);
    shell.update(cx, |shell, cx| {
        let answer = queued(&mut scene, shell);
        shell.message_searched(answer, cx);
        assert_eq!(shell.message_search_rows().len(), 1);
        let ticket = shell.areas.palette.messages_search.view.ticket();
        shell.areas.conversation = ConversationArea::new(cx);
        assert_eq!(shell.areas.palette.messages_search.view.ticket(), ticket);
        assert_eq!(shell.message_search_rows().len(), 1);
        assert_eq!(scene.transcript(shell.fixture_link()).unwrap().len(), 1);
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn a_real_queued_search_reply_cannot_install_before_the_retired_fleet_frame(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (link, mut scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let original = context.clone();
    let (shell, cx) = cx.add_window_view(move |window, cx| {
        let mut shell = Shell::with_test_link(window, cx, link);
        shell.fleet.machines = vec![machine(&context)];
        shell.message_search_query("needle", cx);
        shell
    });
    settle(cx);
    shell.update(cx, |shell, cx| {
        let held = queued(&mut scene, shell);
        let ticket = shell.areas.palette.messages_search.view.ticket();
        scene.retire(shell.fixture_link()).unwrap();
        // The displayed admission is deliberately unchanged: the weak lease
        // must close this gap before a later Fleet event reaches the Shell.
        assert_eq!(shell.fleet.machines[0].connection_context(), Some(original));
        shell.message_searched(held, cx);
        assert!(shell.message_search_rows().is_empty());
        assert_eq!(
            shell.areas.palette.messages_search.view.query(),
            Some("needle")
        );
        assert_eq!(shell.areas.palette.messages_search.view.ticket(), ticket);
        assert_eq!(scene.transcript(shell.fixture_link()).unwrap().len(), 1);
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn a_genuine_queued_old_query_is_invalid_before_the_replacement_debounce(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (link, mut scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let (shell, cx) = cx.add_window_view(move |window, cx| {
        let mut shell = Shell::with_test_link(window, cx, link);
        shell.fleet.machines = vec![machine(&context)];
        shell.message_search_query("needle", cx);
        shell
    });
    settle(cx);
    shell.update(cx, |shell, cx| {
        let held = queued(&mut scene, shell);
        shell.message_search_query("replacement", cx);
        let ticket = shell.areas.palette.messages_search.view.ticket();
        assert_ne!(held.ticket, ticket);
        shell.message_searched(held, cx);
        assert!(shell.message_search_rows().is_empty());
        assert_eq!(
            shell.areas.palette.messages_search.view.query(),
            Some("replacement")
        );
        assert_eq!(shell.areas.palette.messages_search.view.ticket(), ticket);
        assert_eq!(scene.transcript(shell.fixture_link()).unwrap().len(), 1);
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn retiring_the_original_peer_invalidates_a_painted_and_prepared_search_hit(
    cx: &mut TestAppContext,
) {
    use vessel_proto::{HarnessKind, SessionInfo, SessionMode, SessionSpec};

    language_tests::init(cx);
    let (link, mut scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let (shell, cx) = cx.add_window_view(move |window, cx| {
        let mut shell = Shell::with_test_link(window, cx, link);
        shell.fleet.machines = vec![machine(&context)];
        shell.message_search_query("needle", cx);
        shell
    });
    settle(cx);
    shell.update_in(cx, |shell, window, cx| {
        let answer = queued(&mut scene, shell);
        let found = &answer.result.as_ref().unwrap().hits[0];
        let hit = HitId {
            machine: answer.machine.clone(),
            session: found.session,
            seq: found.seq,
        };
        let info = SessionInfo::new(
            hit.session,
            SessionSpec::new(
                HarnessKind::ClaudeCode,
                SessionMode::Structured {
                    model: None,
                    resume: None,
                    options: Vec::new(),
                },
            ),
            chrono::Utc::now(),
        );
        shell.fleet.machines[0]
            .sessions
            .push(Card::from_session(&hit.machine, &info));
        shell.message_searched(answer, cx);
        let prepared = shell.message_search_destination(&hit).unwrap();
        scene.retire(shell.fixture_link()).unwrap();
        assert!(shell.message_search_destination(&hit).is_none());
        shell.open_search_hit(prepared, window, cx);
        assert!(shell.areas.session.open.is_none());
        assert_eq!(shell.route, crate::routes::Route::Draft);
        assert!(shell.notice.is_some());
        assert_eq!(scene.transcript(shell.fixture_link()).unwrap().len(), 1);
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn two_actual_same_node_sources_keep_only_the_still_active_peers_queued_results(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (link, mut scene) = Scenario::start_pair(
        &Name::parse("first").unwrap(),
        &Name::parse("second").unwrap(),
        PeerNodes::SameNode,
    )
    .unwrap();
    let primary = scene.context();
    let secondary = scene.context_of(PeerSlot::Secondary).unwrap();
    assert_eq!(primary.node(), secondary.node());
    assert_ne!(primary.epoch(), secondary.epoch());
    let other = secondary.clone();
    let (shell, cx) = cx.add_window_view(move |window, cx| {
        let mut shell = Shell::with_test_link(window, cx, link);
        shell.fleet.machines = vec![machine(&primary), machine(&secondary)];
        shell.message_search_query("needle", cx);
        shell
    });
    settle(cx);
    shell.update(cx, |shell, cx| {
        let old = queued_at(&mut scene, shell, PeerSlot::Primary);
        let current = queued_at(&mut scene, shell, PeerSlot::Secondary);
        let ticket = shell.areas.palette.messages_search.view.ticket();
        scene
            .retire_peer(shell.fixture_link(), PeerSlot::Primary)
            .unwrap();
        // Hold the retirement/Fleet notices: only the genuine original weak
        // guards can distinguish these same-node queued results at receive.
        assert_eq!(
            shell.fleet.machines[1].connection_context(),
            Some(other.clone())
        );
        shell.message_searched(old, cx);
        assert!(shell.message_search_rows().is_empty());
        shell.message_searched(current, cx);
        let rows = shell.message_search_rows();
        assert_eq!(rows.len(), 1);
        assert!(matches!(rows.first(), Some(Row::Hit(hit)) if hit.machine == "second"));
        assert_eq!(shell.areas.palette.messages_search.view.ticket(), ticket);
        assert_eq!(
            shell.areas.palette.messages_search.view.query(),
            Some("needle")
        );
        assert_eq!(
            scene
                .transcript_of(shell.fixture_link(), PeerSlot::Primary)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            scene
                .transcript_of(shell.fixture_link(), PeerSlot::Secondary)
                .unwrap()
                .len(),
            1
        );
        scene.close(shell.fixture_link()).unwrap();
    });
}
