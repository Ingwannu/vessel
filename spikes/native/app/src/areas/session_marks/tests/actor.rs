//! Real actor/guard queue ingress into the page handler; no fabricated capability.
use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{Command, Event, Reconnect, Scenario};
use vessel_app_view::connection::ConnectionContext;
use vessel_proto::{Call, Outcome};

fn rendered(context: &ConnectionContext) -> (Machine, SessionInfo) {
    let info = SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![],
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    );
    let name = context.machine().as_str();
    (
        Machine {
            name: name.to_owned(),
            node: Some(context.node()),
            connection_epoch: Some(context.epoch()),
            reach: Reach::Online,
            sessions: vec![Card::from_session(name, &info)],
            agents: vec![],
            protocol: Some(vessel_proto::PROTOCOL_VERSION),
            version: None,
        },
        info,
    )
}
const EVENT_LIMIT: usize = 8;
fn marked(scene: &mut Scenario, link: &vessel_app_link::Link) -> Answer {
    for _ in 0..EVENT_LIMIT {
        if let Event::SessionMarked(answer) = scene.event(link).unwrap() {
            return answer;
        }
    }
    panic!("mark reply missing from bounded actual actor event sequence")
}
fn protect_other_session(
    shell: &mut Shell,
    context: &ConnectionContext,
    text: &str,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> SessionId {
    let (_, info) = rendered(context);
    let session = info.id;
    let card = Card::from_session(context.machine().as_str(), &info);
    shell.areas.session.open = Some(crate::areas::session::Open::of(&card));
    shell
        .areas
        .conversation
        .open(context.machine().as_str(), session);
    shell.areas.conversation.info = Some(Box::new(info));
    shell.route = Route::Session;
    shell.areas.session.composer.set_text(text, window, cx);
    session
}

#[gpui::test]
fn real_queued_ack_retains_input_and_worker_hold_refuses_new_ingress(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, mut scene) =
        Scenario::start(&vessel_app_view::machine_book::Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        shell.update_in(cx, |shell, window, cx| {
        let (machine, mut info) = rendered(&context);
        let target = CapturedSessionTarget::capture(&machine, &machine.sessions[0]).unwrap();
        let protected = protect_other_session(shell, &context, "keep original 한글", window, cx);
        shell.fleet.machines = vec![machine];
        shell.mark_session_target(target.clone(), Mark::Pin, cx);
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::MarkSession(request) if request.session == info.id && request.mark == Mark::Pin));
        info.marks.pinned = true;
        scene.reply(shell.fixture_link(), Outcome::Ok(vessel_proto::Reply::MarkSession(info))).unwrap();
        let answer = marked(&mut scene, shell.fixture_link());
        let hold = shell.fixture_link().drain_session_marks().unwrap();
        // A second local request may reuse a page ticket, but has another mark.
        // Holding the real hub rejects ingress without issuing a second RPC.
        let other = vessel_app_view::session_marks::Requests::default().begin(target, Mark::Unpin).unwrap();
        shell.fixture_link().submit(Command::SessionMark(other)).unwrap();
        let rejected = marked(&mut scene, shell.fixture_link());
        shell.session_mark_answer(rejected, cx);
        assert!(shell.areas.session_marks.is_pending());
        shell.session_mark_answer(answer, cx);
        assert!(!shell.areas.session_marks.is_pending());
        assert_eq!(shell.areas.session.composer.text(cx), "keep original 한글");
        assert_eq!(shell.route, Route::Session);
        assert_eq!(shell.areas.session.open.as_ref().unwrap().session, protected);
        assert_eq!(shell.areas.conversation.info.as_ref().unwrap().id, protected);
        assert!(!shell.fleet.machines[0].sessions[0].detail.marks.pinned, "ACK never installs a unary snapshot into Fleet");
        assert_eq!(scene.transcript(shell.fixture_link()).unwrap().len(), 1);
        drop(hold);
    })
    }));
    shell.update_in(cx, |shell, _, _| {
        close_workers(shell, &mut scene, outcome.is_err());
    });
    if let Err(panic) = outcome {
        resume_unwind(panic);
    }
}

#[gpui::test]
fn real_retirement_after_queue_cannot_turn_the_ack_into_success(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, mut scene) =
        Scenario::start(&vessel_app_view::machine_book::Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        shell.update_in(cx, |shell, window, cx| {
            let (machine, mut info) = rendered(&context);
            let target = CapturedSessionTarget::capture(&machine, &machine.sessions[0]).unwrap();
            shell.fleet.machines = vec![machine];
            shell.route = Route::Session;
            shell
                .areas
                .session
                .composer
                .set_text("retain queued input", window, cx);
            shell.mark_session_target(target, Mark::Pin, cx);
            assert!(matches!(
                scene.request(shell.fixture_link()).unwrap(),
                Call::MarkSession(_)
            ));
            info.marks.pinned = true;
            scene
                .reply(
                    shell.fixture_link(),
                    Outcome::Ok(vessel_proto::Reply::MarkSession(info)),
                )
                .unwrap();
            let queued = marked(&mut scene, shell.fixture_link());
            scene.retire(shell.fixture_link()).unwrap();
            shell.session_mark_answer(queued, cx);
            assert!(!shell.areas.session_marks.is_pending());
            assert_eq!(
                shell.notice.as_deref(),
                Some(shell.words.session_marks().uncertain())
            );
            assert!(!shell.fleet.machines[0].sessions[0].detail.marks.pinned);
            assert_eq!(shell.areas.session.composer.text(cx), "retain queued input");
            assert_eq!(shell.route, Route::Session);
        })
    }));
    shell.update_in(cx, |shell, _, _| {
        close_workers(shell, &mut scene, outcome.is_err());
    });
    if let Err(panic) = outcome {
        resume_unwind(panic);
    }
}

#[gpui::test]
fn real_alias_and_same_node_reconnections_cannot_retarget_a_rendered_intent(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    for replacement in [Reconnect::SameNode, Reconnect::DifferentNode] {
        let (link, mut scene) =
            Scenario::start(&vessel_app_view::machine_book::Name::parse("fixture").unwrap())
                .unwrap();
        let context = scene.context();
        let (shell, cx) =
            cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            shell.update_in(cx, |shell, _, cx| {
                let (mut machine, _) = rendered(&context);
                let original =
                    CapturedSessionTarget::capture(&machine, &machine.sessions[0]).unwrap();
                let next = scene.reconnect(shell.fixture_link(), replacement).unwrap();
                assert_ne!(context, next);
                machine.node = Some(next.node());
                machine.connection_epoch = Some(next.epoch());
                shell.fleet.machines = vec![machine];
                shell.mark_session_target(original, Mark::Archive, cx);
                let refused = marked(&mut scene, shell.fixture_link());
                shell.session_mark_answer(refused, cx);
                assert!(!shell.areas.session_marks.is_pending());
                assert_eq!(
                    shell.notice.as_deref(),
                    Some(shell.words.session_marks().changed())
                );
                assert!(scene.transcript(shell.fixture_link()).unwrap().is_empty());
                assert!(
                    shell.fleet.machines[0].sessions[0]
                        .detail
                        .marks
                        .archived_at
                        .is_none()
                );
            })
        }));
        shell.update_in(cx, |shell, _, _| {
            close_workers(shell, &mut scene, outcome.is_err());
        });
        if let Err(panic) = outcome {
            resume_unwind(panic);
        }
    }
}

#[gpui::test]
fn observation_timeout_preserves_real_work_and_expired_hub_is_not_idle(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, mut scene) =
        Scenario::start(&vessel_app_view::machine_book::Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        shell.update_in(cx, |shell, window, cx| {
            let (machine, mut info) = rendered(&context);
            let target = CapturedSessionTarget::capture(&machine, &machine.sessions[0]).unwrap();
            let protected = protect_other_session(
                shell,
                &context,
                "retain while actual call waits",
                window,
                cx,
            );
            shell.fleet.machines = vec![machine];
            shell.mark_session_target(target, Mark::Pin, cx);
            assert!(matches!(
                scene.request(shell.fixture_link()).unwrap(),
                Call::MarkSession(_)
            ));
            assert!(matches!(
                shell.fixture_link().drain_session_marks(),
                Err(vessel_app_link::session_marks::DrainProblem::TimedOut)
            ));
            assert!(shell.areas.session_marks.is_pending());
            assert_eq!(
                shell.areas.session.composer.text(cx),
                "retain while actual call waits"
            );
            assert_eq!(
                shell.areas.session.open.as_ref().unwrap().session,
                protected
            );
            info.marks.pinned = true;
            scene
                .reply(
                    shell.fixture_link(),
                    Outcome::Ok(vessel_proto::Reply::MarkSession(info)),
                )
                .unwrap();
            shell.session_mark_answer(marked(&mut scene, shell.fixture_link()), cx);
            assert!(!shell.areas.session_marks.is_pending());
            assert_eq!(scene.transcript(shell.fixture_link()).unwrap().len(), 1);
        });
    }));
    shell.update_in(cx, |shell, _, _| {
        close_workers(shell, &mut scene, outcome.is_err());
        assert!(matches!(
            shell.fixture_link().drain_session_marks(),
            Err(vessel_app_link::session_marks::DrainProblem::Unavailable)
        ));
    });
    if let Err(panic) = outcome {
        resume_unwind(panic);
    }
}

fn close_workers(shell: &Shell, scene: &mut Scenario, failed: bool) {
    if failed {
        let _stopped = scene.close_peer(shell.fixture_link());
    }
    let drained = shell.fixture_link().drain_session_marks();
    let closed = scene.close(shell.fixture_link());
    assert!(
        drained.is_ok(),
        "original mark workers did not drain: {drained:?}"
    );
    closed.unwrap();
}
