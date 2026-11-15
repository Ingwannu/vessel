//! The actual body receives wheel and touch-pan events, without a node.
use super::*;
use gpui::{
    AppContext as _, Render, ScrollDelta, ScrollEvent, ScrollSource, TestAppContext, point,
};
use std::fmt::Write as _;
use vessel_app_kit::ui::{IntoElement, ParentElement as _, Styled as _, Window, div};

struct Probe(Entity<Shell>);
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.0.update(cx, |shell, cx| {
            div()
                .size_full()
                .flex()
                .child(body(shell, &vessel_app_kit::tokens(cx), window, cx))
        })
    }
}

fn words(shell: &Shell, cx: &gpui::App) -> String {
    shell
        .areas
        .terminal
        .grid
        .read(cx)
        .frame
        .rows
        .iter()
        .flat_map(|row| row.runs.iter().map(|run| run.text.as_ref()))
        .collect()
}

#[gpui::test]
fn terminal_body_scrolls_back_and_returns_to_live_for_pointer_and_touch(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (probe, cx) =
        cx.add_window_view(|window, cx| Probe(cx.new(|cx| Shell::without_io(window, cx))));
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, cx| {
            cx.set_reduce_motion(true);
            shell.terminal_opened(
                "laptop".parse().unwrap(),
                SessionId::random(),
                TerminalSize::default(),
                cx,
            );
            let bytes = (0..300).fold(String::new(), |mut s, n| {
                write!(s, "row-{n:03}\r\n").unwrap();
                s
            });
            replay_all(
                shell.areas.terminal.model.as_mut().unwrap(),
                bytes.as_bytes(),
            );
            shell.areas.terminal.phase = Phase::Ready;
            shell.terminal_draw(cx);
            cx.notify();
        });
    });
    cx.run_until_parked();
    for source in [ScrollSource::Pointer, ScrollSource::Touch] {
        cx.simulate_event(ScrollEvent {
            position: point(px(100.0), px(100.0)),
            delta: ScrollDelta::Lines(point(0.0, 10000.0)),
            source,
            ..Default::default()
        });
        cx.run_until_parked();
        probe.read_with(cx, |probe, cx| {
            probe.0.read_with(cx, |shell, cx| {
                assert!(words(shell, cx).contains("row-000"));
            });
        });
        cx.simulate_event(ScrollEvent {
            position: point(px(100.0), px(100.0)),
            delta: ScrollDelta::Lines(point(0.0, -10000.0)),
            source,
            ..Default::default()
        });
        cx.run_until_parked();
        probe.read_with(cx, |probe, cx| {
            probe.0.read_with(cx, |shell, cx| {
                assert!(!words(shell, cx).contains("row-000"));
                assert!(words(shell, cx).contains("row-299"));
            });
        });
    }
}

#[gpui::test]
fn replay_keeps_pointer_and_touch_intent_in_both_directions(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    for source in [ScrollSource::Pointer, ScrollSource::Touch] {
        shell.update(cx, |shell, cx| {
            shell.terminal_opened(
                "laptop".parse().unwrap(),
                SessionId::random(),
                TerminalSize::default(),
                cx,
            );
            let bytes = (0..300).fold(String::new(), |mut s, n| {
                writeln!(s, "row-{n:03}\r").unwrap();
                s
            });
            replay_all(
                shell.areas.terminal.model.as_mut().unwrap(),
                bytes.as_bytes(),
            );
            shell.areas.terminal.phase = Phase::Ready;
            shell.terminal_draw(cx);
            shell
                .areas
                .terminal
                .pending
                .extend_from_slice(b"row-300\r\n");
            shell.terminal_apply(cx);
            assert!(shell.areas.terminal.model.is_none());
            let cell = vessel_app_kit::terminal_grid::CellSize {
                width: px(8.0),
                height: px(16.0),
            };
            shell.terminal_scroll(
                &ScrollEvent {
                    delta: ScrollDelta::Lines(point(0.0, 10000.0)),
                    source,
                    ..Default::default()
                },
                cell,
                cx,
            );
        });
        cx.run_until_parked();
        shell.read_with(cx, |shell, cx| {
            assert!(words(shell, cx).contains("row-000"));
        });
        shell.update(cx, |shell, cx| {
            shell
                .areas
                .terminal
                .pending
                .extend_from_slice(b"row-301\r\n");
            shell.terminal_apply(cx);
            assert!(shell.areas.terminal.model.is_none());
            let cell = vessel_app_kit::terminal_grid::CellSize {
                width: px(8.0),
                height: px(16.0),
            };
            shell.terminal_scroll(
                &ScrollEvent {
                    delta: ScrollDelta::Lines(point(0.0, -10000.0)),
                    source,
                    ..Default::default()
                },
                cell,
                cx,
            );
        });
        cx.run_until_parked();
        shell.read_with(cx, |shell, cx| {
            assert!(!words(shell, cx).contains("row-000"));
            assert!(words(shell, cx).contains("row-301"));
        });
    }
}

#[gpui::test]
fn replacing_an_opening_discards_its_inflight_scroll_intent(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        shell.terminal_opened(
            "laptop".parse().unwrap(),
            SessionId::random(),
            TerminalSize::default(),
            cx,
        );
        replay_all(
            shell.areas.terminal.model.as_mut().unwrap(),
            b"old\r\n".repeat(300).as_slice(),
        );
        shell.areas.terminal.phase = Phase::Ready;
        shell.terminal_draw(cx);
        shell
            .areas
            .terminal
            .pending
            .extend_from_slice(b"old completion\r\n");
        shell.terminal_apply(cx);
        assert!(shell.areas.terminal.model.is_none());
        let cell = vessel_app_kit::terminal_grid::CellSize {
            width: px(8.0),
            height: px(16.0),
        };
        shell.terminal_scroll(
            &ScrollEvent {
                delta: ScrollDelta::Lines(point(0.0, 10000.0)),
                source: ScrollSource::Touch,
                ..Default::default()
            },
            cell,
            cx,
        );
        shell.terminal_opened(
            "laptop".parse().unwrap(),
            SessionId::random(),
            TerminalSize::default(),
            cx,
        );
        replay_all(shell.areas.terminal.model.as_mut().unwrap(), b"new opening");
        shell.areas.terminal.phase = Phase::Ready;
        shell.terminal_draw(cx);
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, cx| {
        assert!(words(shell, cx).contains("new opening"));
        assert!(!words(shell, cx).contains("old"));
        let area = &shell.areas.terminal;
        assert_eq!(
            area.viewport.offset(area.model.as_ref().unwrap().grid()),
            vessel_app_view::terminal::grid::Scroll(0)
        );
    });
}
