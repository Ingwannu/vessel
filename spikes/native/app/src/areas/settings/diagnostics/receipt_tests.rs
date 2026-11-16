//! Real actor receipts are revalidated after queueing and before clipboard export.
use crate::shell::{Shell, language_tests};
use gpui::{AppContext as _, TestAppContext};
use vessel_app_kit::ui::{ClipboardItem, Context, Window};
use vessel_app_link::{
    Event,
    diagnostics::{Answer, Message},
    testing::Scenario,
};
use vessel_app_view::{
    Machine, Reach,
    connection::ConnectionContext,
    diagnostics::collection::{Phase, Problem, Progress},
    machine_book::Name,
};
use vessel_proto::{Call, NodeInfo, NodeState, Outcome, ProtocolRange, Reply};

mod report;

const MAX_EVENTS: usize = 32;
const BEFORE: &str = "clipboard marker before rejected diagnostic copy";
const CANARY: &str = "synthetic-private-host-name";

fn prepare(
    shell: &mut Shell,
    context: &ConnectionContext,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let mut machine = Machine::unreachable(context.machine().as_str().into(), CANARY);
    machine.reach = Reach::Online;
    machine.node = Some(context.node());
    machine.connection_epoch = Some(context.epoch());
    shell.fleet.machines = vec![machine];
    shell.run_diagnostic(None, window, cx);
}
fn node(context: &ConnectionContext) -> NodeInfo {
    NodeInfo {
        id: context.node(),
        name: CANARY.into(),
        hostname: CANARY.into(),
        os: CANARY.into(),
        arch: CANARY.into(),
        version: CANARY.into(),
        protocol: ProtocolRange::CURRENT,
        started_at: chrono::DateTime::UNIX_EPOCH,
        state: NodeState::Serving,
        public_key: None,
        timezone: None,
    }
}
fn settled(scene: &mut Scenario, shell: &Shell, context: &ConnectionContext) -> Answer {
    for _ in 0..2 {
        let outcome = match scene.request(shell.fixture_link()).unwrap() {
            Call::NodeInfo(_) => Outcome::Ok(Reply::NodeInfo(node(context))),
            Call::Diagnostics(_) => {
                Outcome::Ok(Reply::Diagnostics(report::partial(context.node())))
            }
            other => panic!("unexpected diagnostic request: {other:?}"),
        };
        scene.reply(shell.fixture_link(), outcome).unwrap();
    }
    for _ in 0..MAX_EVENTS {
        if let Event::Diagnostics(Message::Answered(answer)) =
            scene.event(shell.fixture_link()).unwrap()
        {
            return answer;
        }
    }
    panic!("bounded actual diagnostic response");
}
#[gpui::test]
fn queued_receipt_cannot_install_after_retirement_before_the_new_fleet(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (link, mut scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        prepare(shell, &context, window, cx);
        let held = settled(&mut scene, shell, &context);
        scene.retire(shell.fixture_link()).unwrap();
        assert_eq!(
            shell.fleet.machines.first().unwrap().connection_context(),
            Some(context.clone())
        );
        shell.diagnostic_answer(Message::Answered(held));
        let run = &shell.areas.settings.diagnostics.collection;
        assert_eq!(run.phase(), &Phase::Ready);
        assert_eq!(
            run.rows().first().unwrap().progress,
            Progress::Failed(Problem::ConnectionChanged)
        );
        assert!(!run.report().unwrap().as_str().contains("Protocol:"));
        assert!(!run.report().unwrap().as_str().contains(CANARY));
        scene.close(shell.fixture_link()).unwrap();
    });
}
#[derive(Clone, Copy)]
enum End {
    Retirement,
    ClosedTransport,
}
#[gpui::test]
fn copied_snapshot_rechecks_original_weak_receipt_and_duplicate_answers_stay_bounded(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    for end in [End::Retirement, End::ClosedTransport] {
        copy_case(cx, end);
    }
}
fn copy_case(cx: &mut TestAppContext, end: End) {
    let (link, mut scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let context = scene.context();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        prepare(shell, &context, window, cx);
        let answer = settled(&mut scene, shell, &context);
        let duplicate = answer.clone();
        shell.diagnostic_answer(Message::Answered(answer));
        shell.diagnostic_answer(Message::Answered(duplicate));
        assert_eq!(shell.areas.settings.diagnostics.receipts.len(), 1);
        let report = shell
            .areas
            .settings
            .diagnostics
            .collection
            .report()
            .unwrap()
            .clone();
        assert!(report.as_str().contains("Protocol:"));
        assert!(report.as_str().contains("Daemon self process"));
        assert!(report.as_str().contains("\"pid\":42"));
        assert!(matches!(
            shell
                .areas
                .settings
                .diagnostics
                .collection
                .rows()
                .first()
                .unwrap()
                .progress,
            Progress::Partial(_)
        ));
        assert!(!report.as_str().contains(CANARY));
        cx.write_to_clipboard(ClipboardItem::new_string(BEFORE.into()));
        match end {
            End::Retirement => scene.retire(shell.fixture_link()).unwrap(),
            End::ClosedTransport => scene.close_peer(shell.fixture_link()).unwrap(),
        }
        shell.copy_diagnostic(&report, cx);
        assert!(
            cx.read_from_clipboard()
                .is_some_and(|item| item.text().is_some_and(|text| text == BEFORE))
        );
        assert!(shell.areas.settings.diagnostics.receipts.is_empty());
        assert!(shell.notice.is_none());
        scene.close(shell.fixture_link()).unwrap();
    });
}
