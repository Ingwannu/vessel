//! Hold an actual submitted command while invoking the real Refresh handler.
use crate::shell::{Shell, language_tests};
use tokio::sync::mpsc::{Receiver, error::TryRecvError};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{
    Command, Link,
    diagnostics::{Ask, Message},
};
use vessel_app_view::devices::Tickets;
use vessel_app_view::diagnostics::collection::{Phase, Problem, Progress};
use vessel_app_view::{Machine, Reach};
use vessel_proto::NodeId;

const COMMAND_ROOM: usize = 1;

#[gpui::test]
fn refresh_waits_for_the_held_read_and_does_not_spend_a_ticket(cx: &mut gpui::TestAppContext) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        exercise_refresh,
        |shell| &shell.areas.session.composer,
        |shell| {
            assert_eq!(
                shell.areas.settings.diagnostics.collection.phase(),
                &Phase::Hidden
            );
        },
    );
}

fn exercise_refresh(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let (mut commands, held) = hold_first_read(shell, window, cx);
    let mut expected = Tickets::default();
    assert_eq!(held.ticket, expected.next_ticket().unwrap());
    let first = held.ticket;
    assert!(!shell.areas.settings.diagnostics.collection.can_start());
    shell.refresh_diagnostic(window, cx);
    let state = &shell.areas.settings.diagnostics;
    assert_eq!(state.collection.ticket(), Some(first));
    assert_eq!(
        state.collection.rows().first().unwrap().progress,
        Progress::Running
    );
    assert!(matches!(commands.try_recv(), Err(TryRecvError::Empty)));
    shell.diagnostic_answer(Message::Answered(held.refused(Problem::TimedOut)));
    assert!(shell.areas.settings.diagnostics.collection.can_start());
    shell.refresh_diagnostic(window, cx);
    let Command::Diagnostics(next) = commands.try_recv().unwrap() else {
        panic!("refresh must dispatch the next diagnostic read");
    };
    assert_eq!(next.ticket, expected.next_ticket().unwrap());
    assert_ne!(next.ticket, first);
    shell.areas.settings.diagnostics.collection.close();
    shell.refresh_diagnostic(window, cx);
    assert_eq!(
        shell.areas.settings.diagnostics.collection.phase(),
        &Phase::Hidden
    );
    assert!(matches!(commands.try_recv(), Err(TryRecvError::Empty)));
}

fn hold_first_read(
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> (Receiver<Command>, Ask) {
    let (link, mut commands) = Link::detached(COMMAND_ROOM).unwrap();
    language_tests::attach_link(shell, link);
    let mut machine = Machine::unreachable("held-host".to_owned(), "unused fixture error");
    machine.reach = Reach::Online;
    let context = vessel_app_view::connection::ConnectionEpochs::default()
        .admit(
            vessel_app_view::connection::MachineKey::from_book("held-host"),
            NodeId::random(),
        )
        .unwrap();
    machine.node = Some(context.node());
    machine.connection_epoch = Some(context.epoch());
    shell.fleet.machines.push(machine);
    shell.run_diagnostic(None, window, cx);
    let Command::Diagnostics(held) = commands.try_recv().unwrap() else {
        panic!("the initial run must dispatch a diagnostic read");
    };
    (commands, held)
}
