//! Actual owner entry/completion calls retain the limit while an executor owns the model.
use super::replay::replay_all;
use super::*;
use gpui::TestAppContext;

#[gpui::test]
fn scrollback_change_during_output_handover_applies_on_return_and_new_open(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        shell.terminal_opened(
            "laptop".parse().unwrap(),
            SessionId::random(),
            TerminalSize::default(),
            cx,
        );
        shell.areas.terminal.phase = Phase::Ready;
        let opening = shell.areas.terminal.shown.clone().unwrap();
        shell.areas.terminal.fit = Some(GridSize::new(12, 2).unwrap());
        let mut model = shell.areas.terminal.model.take().unwrap();
        model.resize(shell.areas.terminal.fit.unwrap());
        replay_all(&mut model, b"0\r\n1\r\n2\r\n3\r\n4\r\n5");
        assert_eq!(model.grid().history_len(), 4);
        shell.terminal_scrollback_changed(ScrollbackLines::new(0).unwrap(), cx);
        assert!(shell.areas.terminal.model.is_none());
        shell.terminal_applied(&opening, model, cx);
        assert_eq!(
            shell
                .areas
                .terminal
                .model
                .as_ref()
                .unwrap()
                .grid()
                .history_len(),
            0
        );
        shell.terminal_closed();
        shell.terminal_opened(
            "laptop".parse().unwrap(),
            SessionId::random(),
            TerminalSize::default(),
            cx,
        );
        let model = shell.areas.terminal.model.as_mut().unwrap();
        replay_all(model, b"0\r\n1\r\n2\r\n3\r\n4\r\n5");
        assert_eq!(model.grid().history_len(), 0);
    });
}

#[gpui::test]
fn scrollback_change_during_history_read_applies_before_loaded_model_is_drawn(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        shell.terminal_opened(
            "laptop".parse().unwrap(),
            SessionId::random(),
            TerminalSize::default(),
            cx,
        );
        let opening = shell.areas.terminal.shown.clone().unwrap();
        let mut model = shell.areas.terminal.model.take().unwrap();
        model.resize(GridSize::new(12, 2).unwrap());
        replay_all(&mut model, b"0\r\n1\r\n2\r\n3\r\n4\r\n5");
        shell.areas.terminal.fit = Some(GridSize::new(12, 2).unwrap());
        shell.terminal_scrollback_changed(ScrollbackLines::new(1).unwrap(), cx);
        assert_eq!(shell.areas.terminal.phase, Phase::Loading);
        assert!(shell.areas.terminal.model.is_none());
        shell.terminal_loaded(&opening, model, TerminalLog::default(), cx);
        assert_eq!(
            shell
                .areas
                .terminal
                .model
                .as_ref()
                .unwrap()
                .grid()
                .history_len(),
            1
        );
    });
}

#[gpui::test]
fn lowering_then_raising_while_model_is_away_never_revives_trimmed_history(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        shell.terminal_opened(
            "laptop".parse().unwrap(),
            SessionId::random(),
            TerminalSize::default(),
            cx,
        );
        shell.areas.terminal.phase = Phase::Ready;
        let opening = shell.areas.terminal.shown.clone().unwrap();
        shell.areas.terminal.fit = Some(GridSize::new(12, 2).unwrap());
        let mut model = shell.areas.terminal.model.take().unwrap();
        model.resize(GridSize::new(12, 2).unwrap());
        replay_all(&mut model, b"0\r\n1\r\n2\r\n3\r\n4\r\n5");
        assert_eq!(model.grid().history_len(), 4);
        shell.terminal_scrollback_changed(ScrollbackLines::new(0).unwrap(), cx);
        shell.terminal_scrollback_changed(ScrollbackLines::new(8).unwrap(), cx);
        shell.terminal_applied(&opening, model, cx);
        let model = shell.areas.terminal.model.as_mut().unwrap();
        assert_eq!(model.grid().history_len(), 0);
        replay_all(model, b"\r\n6\r\n7");
        assert_eq!(model.grid().history_len(), 2);
    });
}

#[gpui::test]
fn new_opening_drops_the_old_pending_trim_and_rejects_its_completion(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        shell.terminal_opened(
            "laptop".parse().unwrap(),
            SessionId::random(),
            TerminalSize::default(),
            cx,
        );
        let old_opening = shell.areas.terminal.shown.clone().unwrap();
        let old_model = shell.areas.terminal.model.take().unwrap();
        shell.terminal_scrollback_changed(ScrollbackLines::new(0).unwrap(), cx);
        shell.terminal_scrollback_changed(ScrollbackLines::new(8).unwrap(), cx);
        shell.terminal_closed();
        shell.terminal_opened(
            "laptop".parse().unwrap(),
            SessionId::random(),
            TerminalSize::default(),
            cx,
        );
        let current = shell.areas.terminal.shown.clone().unwrap();
        let mut model = shell.areas.terminal.model.take().unwrap();
        model.resize(GridSize::new(12, 2).unwrap());
        shell.areas.terminal.fit = Some(GridSize::new(12, 2).unwrap());
        replay_all(&mut model, b"0\r\n1\r\n2\r\n3\r\n4\r\n5");
        shell.terminal_loaded(&current, model, TerminalLog::default(), cx);
        shell.terminal_applied(&old_opening, old_model, cx);
        assert_eq!(
            shell
                .areas
                .terminal
                .model
                .as_ref()
                .unwrap()
                .grid()
                .history_len(),
            4
        );
    });
}
