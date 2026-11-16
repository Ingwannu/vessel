//! Regressions for asynchronous ownership and quiet retained frames.

use super::*;
use gpui::TestAppContext;
use vessel_app_kit::{Scheme, Theme, Tokens};
use vessel_app_link::{ActAnswer, ActKind, Acted};
use vessel_app_view::terminal::grid::Scroll;

fn setup(cx: &mut TestAppContext) -> (Entity<Shell>, &mut gpui::VisualTestContext) {
    crate::shell::language_tests::init(cx);
    cx.add_window_view(Shell::without_io)
}

fn open(shell: &mut Shell, session: SessionId, cx: &mut Context<'_, Shell>) {
    assert!(shell.terminal_opened(
        "laptop".parse().unwrap(),
        session,
        TerminalSize::default(),
        cx,
    ));
    shell.route = crate::routes::Route::Session;
    shell.areas.session.open = Some(crate::areas::session::Open {
        machine: "laptop".into(),
        session,
        title: "terminal".into(),
        agent: "fake".into(),
        structured: false,
        live: true,
        archived_at: None,
        binding: None,
    });
}

fn text(model: &TerminalModel) -> String {
    use vessel_app_view::terminal::grid::Glyph;
    model
        .snapshot(Scroll(0))
        .cells
        .iter()
        .filter_map(|cell| match cell.glyph() {
            Glyph::Scalar(ch) => Some(ch),
            Glyph::Empty | Glyph::Continuation => None,
        })
        .collect()
}

fn same_screen(actual: &TerminalModel, expected: &TerminalModel) {
    let a = actual.snapshot(Scroll(0));
    let b = expected.snapshot(Scroll(0));
    assert_eq!(a.size, b.size);
    assert_eq!(a.cells, b.cells);
    assert_eq!(a.cursor, b.cursor);
    assert_eq!(a.history, b.history);
}

#[gpui::test]
fn reopening_a_rejects_the_old_history_completion(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        let (a, b) = (SessionId::random(), SessionId::random());
        open(shell, a, cx);
        let opening = shell.areas.terminal.shown.clone().unwrap();
        let mut old = shell.areas.terminal.model.take().unwrap();
        replay_all(&mut old, b"old opening");
        open(shell, b, cx);
        open(shell, a, cx);
        shell.terminal_loaded(&opening, old, TerminalLog::default(), cx);
        assert_eq!(shell.areas.terminal.phase, Phase::Loading);
        assert!(!text(shell.areas.terminal.model.as_ref().unwrap()).contains("old opening"));
    });
}

#[gpui::test]
fn quiet_theme_change_replaces_cells_background_and_cursor(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        cx.set_reduce_motion(true);
        open(shell, SessionId::random(), cx);
        replay_all(shell.areas.terminal.model.as_mut().unwrap(), b"quiet");
        let area = &mut shell.areas.terminal;
        area.phase = Phase::Ready;
        area.fit = Some(GridSize::STANDARD);
        area.probe.set(Some(Bounds::new(
            gpui::point(Pixels::ZERO, Pixels::ZERO),
            gpui::size(px(640.0), px(384.0)),
        )));
        area.grid.update(cx, |grid, _| {
            grid.cell = vessel_app_kit::terminal_grid::CellSize {
                width: px(8.0),
                height: px(16.0),
            };
        });
        shell.terminal_draw(cx);
        let before = Rc::clone(&shell.areas.terminal.grid.read(cx).frame);
        cx.set_global(Theme {
            tokens: Tokens::of(Scheme::Light),
        });
        let cell = shell.areas.terminal.grid.read(cx).cell;
        shell.terminal_measured(cell, cx);
        let after = &shell.areas.terminal.grid.read(cx).frame;
        assert_ne!(before.background, after.background);
        assert_ne!(before.cursor, after.cursor);
        assert_ne!(before.rows, after.rows);
    });
}

#[gpui::test]
fn queued_small_records_do_no_inline_parsing(cx: &mut TestAppContext) {
    const QUEUED_RECORDS: usize = 8;
    let (shell, cx) = setup(cx);
    let line = b"small\r\n";
    let budget = vessel_app_view::terminal::parse::FEED_BUDGET;
    let payload = line.repeat(budget.checked_div(line.len().saturating_mul(4)).unwrap());
    let expected_bytes = payload.repeat(QUEUED_RECORDS);
    assert!(payload.len() < budget);
    assert!(expected_bytes.len() > budget);
    shell.update(cx, |shell, cx| {
        let a = SessionId::random();
        open(shell, a, cx);
        shell.areas.terminal.phase = Phase::Ready;
        for index in 0..QUEUED_RECORDS {
            let seq = u64::try_from(index).unwrap();
            shell.terminal_records(&a, &[record(seq, &payload)], cx);
        }
        // One executor owns the model; the UI has parsed none of the burst.
        assert!(shell.areas.terminal.model.is_none());
        assert!(!shell.areas.terminal.pending.is_empty());
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        let model = shell.areas.terminal.model.as_ref().unwrap();
        let mut expected = TerminalModel::new(model.grid().size(), ScrollbackLines::default());
        replay_all(&mut expected, &expected_bytes);
        same_screen(model, &expected);
        assert_eq!(
            shell.areas.terminal.log.next(),
            vessel_proto::EventSeq::new(u64::try_from(QUEUED_RECORDS).unwrap())
        );
        assert!(shell.areas.terminal.pending.is_empty());
    });
}

#[gpui::test]
fn holding_motion_stops_an_existing_cursor_timer(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        cx.set_reduce_motion(false);
        open(shell, SessionId::random(), cx);
        shell.areas.terminal.phase = Phase::Ready;
        shell.areas.terminal.playback = Playback::Blinking;
        shell.terminal_blink(cx);
        assert!(shell.areas.terminal.blinking.is_some());
        cx.set_reduce_motion(true);
    });
    cx.executor().advance_clock(BLINK);
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(shell.areas.terminal.blinking.is_none());
        assert_eq!(shell.areas.terminal.blink, Blink::On);
    });
}

#[gpui::test]
fn unread_history_is_not_repeated_as_a_conversation_failure(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        let a = SessionId::random();
        let shown = crate::areas::session::Open {
            machine: "laptop".into(),
            session: a,
            title: "terminal".into(),
            agent: "fake".into(),
            structured: false,
            live: true,
            archived_at: None,
            binding: None,
        };
        shell.conversation_opened(&shown, cx);
        open(shell, a, cx);
        shell.acted(
            ActAnswer {
                machine: "laptop".into(),
                session: a,
                terminal: shell.areas.terminal.shown.clone(),
                kind: ActKind::Log,
                result: Err(vessel_app_link::ActFailed {
                    code: None,
                    message: "history unavailable".into(),
                }),
            },
            cx,
        );
        assert_eq!(shell.areas.terminal.phase, Phase::Unread);
        assert_eq!(shell.areas.conversation.lost, None);
        assert_eq!(shell.areas.conversation.refused, None);
    });
}

fn record(seq: u64, bytes: &[u8]) -> EventRecord {
    EventRecord {
        seq: vessel_proto::EventSeq::new(seq),
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::Terminal(vessel_proto::TerminalOutput {
            data: vessel_proto::TerminalBytes::new(bytes.to_vec().into()),
        }),
    }
}

#[derive(Clone, Copy)]
enum CompletionOrder {
    BeforeNewRead,
    AfterNewRead,
}

#[gpui::test]
fn old_read_and_output_completions_cannot_replace_a_reopened_model_or_log_cursor(
    cx: &mut TestAppContext,
) {
    let (shell, cx) = setup(cx);
    for order in [
        CompletionOrder::BeforeNewRead,
        CompletionOrder::AfterNewRead,
    ] {
        shell.update(cx, |shell, cx| {
            let (a, b) = (SessionId::random(), SessionId::random());
            open(shell, a, cx);
            let old = shell.areas.terminal.shown.clone().unwrap();
            let mut previous = shell.areas.terminal.model.take().unwrap();
            replay_all(&mut previous, b"old output\r\n");
            let mut old_history =
                TerminalModel::new(previous.grid().size(), ScrollbackLines::default());
            let mut old_log = TerminalLog::default();
            for bytes in old_log.apply(&[record(99, b"old history\r\n")]) {
                replay_all(&mut old_history, bytes);
            }
            open(shell, b, cx);
            open(shell, a, cx);
            let current = shell.areas.terminal.shown.clone().unwrap();
            let history = vec![record(0, b"new history\r\n")];
            let mut model = shell.areas.terminal.model.take().unwrap();
            let mut log = TerminalLog::default();
            for bytes in log.apply(&history) {
                replay_all(&mut model, bytes);
            }
            shell.terminal_records(&a, &[record(0, b"new history\r\n"), record(1, b"live")], cx);
            let stale_read = ActAnswer {
                machine: "laptop".into(),
                session: a,
                terminal: Some(old.clone()),
                kind: ActKind::Log,
                result: Ok(Acted::Log(vec![record(0, b"old history")])),
            };
            match order {
                CompletionOrder::BeforeNewRead => {
                    assert!(shell.terminal_acted(&stale_read, cx));
                    shell.terminal_loaded(&old, old_history, old_log, cx);
                    shell.terminal_applied(&old, previous, cx);
                    shell.terminal_loaded(&current, model, log, cx);
                }
                CompletionOrder::AfterNewRead => {
                    shell.terminal_loaded(&current, model, log, cx);
                    assert!(shell.terminal_acted(&stale_read, cx));
                    shell.terminal_loaded(&old, old_history, old_log, cx);
                    shell.terminal_applied(&old, previous, cx);
                }
            }
            // A differently named machine cannot use even the current token.
            let wrong_machine = ActAnswer {
                machine: "other".into(),
                session: a,
                terminal: Some(current),
                kind: ActKind::Log,
                result: Err(vessel_app_link::ActFailed {
                    code: None,
                    message: "wrong owner".into(),
                }),
            };
            assert!(shell.terminal_acted(&wrong_machine, cx));
        });
        cx.run_until_parked();
        shell.read_with(cx, |shell, _| {
            let area = &shell.areas.terminal;
            assert_eq!(area.phase, Phase::Ready);
            assert_eq!(area.log.next(), vessel_proto::EventSeq::new(2));
            let model = area.model.as_ref().unwrap();
            let mut expected = TerminalModel::new(model.grid().size(), ScrollbackLines::default());
            replay_all(&mut expected, b"new history\r\nlive");
            same_screen(model, &expected);
        });
    }
}

#[gpui::test]
fn motion_resumes_on_release_and_stops_on_blur_navigation_and_close(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    shell.update_in(cx, |_, window, _| window.activate_window());
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        cx.set_reduce_motion(true);
        open(shell, SessionId::random(), cx);
        shell.areas.terminal.phase = Phase::Ready;
        shell.focus_terminal(window, cx);
        assert!(shell.areas.terminal.blinking.is_none());
        cx.set_reduce_motion(false);
        shell.terminal_present(window, cx);
        assert!(shell.areas.terminal.blinking.is_some());
        shell.areas.session.composer.focus(window, cx);
        shell.terminal_present(window, cx);
        assert!(shell.areas.terminal.blinking.is_none());
        shell.focus_terminal(window, cx);
        assert!(shell.areas.terminal.blinking.is_some());
        shell.route = crate::routes::Route::Board;
        shell.terminal_present(window, cx);
        assert!(shell.areas.terminal.blinking.is_none());
        shell.route = crate::routes::Route::Session;
        shell.terminal_present(window, cx);
        assert!(shell.areas.terminal.blinking.is_some());
        cx.set_reduce_motion(true);
        shell.terminal_present(window, cx);
        let frame = Rc::clone(&shell.areas.terminal.grid.read(cx).frame);
        assert_eq!(shell.areas.terminal.blink, Blink::On);
        assert!(shell.areas.terminal.blinking.is_none());
        shell.terminal_present(window, cx);
        assert!(Rc::ptr_eq(
            &frame,
            &shell.areas.terminal.grid.read(cx).frame
        ));
    });
    cx.run_until_parked();
    let frame = shell.read_with(cx, |shell, cx| {
        Rc::clone(&shell.areas.terminal.grid.read(cx).frame)
    });
    cx.executor().advance_clock(BLINK.saturating_mul(3));
    cx.run_until_parked();
    shell.update(cx, |shell, cx| {
        assert!(Rc::ptr_eq(
            &frame,
            &shell.areas.terminal.grid.read(cx).frame
        ));
        assert!(shell.areas.terminal.blinking.is_none());
        shell.terminal_closed();
        assert!(shell.areas.terminal.shown.is_none());
        assert!(shell.areas.terminal.blinking.is_none());
    });
}
