//! Cursor phase must retain the painted rows; appearance and output still replace them.
use super::*;
use gpui::TestAppContext;

#[gpui::test]
fn cursor_phase_reuses_rows_and_preserves_cursor_and_output(cx: &mut TestAppContext) {
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
            b"painted rows",
        );
        shell.areas.terminal.phase = Phase::Ready;
        shell.terminal_draw(cx);
        let on = Rc::clone(&shell.areas.terminal.grid.read(cx).frame);
        assert!(on.cursor.is_some());
        assert!(!on.rows.is_empty());
        shell.areas.terminal.blink = Blink::Off;
        shell.terminal_draw(cx);
        let off = Rc::clone(&shell.areas.terminal.grid.read(cx).frame);
        assert_eq!(off.cursor, None);
        assert_eq!(on.rows.as_ptr(), off.rows.as_ptr());
        shell.areas.terminal.blink = Blink::On;
        shell.terminal_draw(cx);
        let restored = Rc::clone(&shell.areas.terminal.grid.read(cx).frame);
        assert_eq!(restored.cursor, on.cursor);
        assert_eq!(on.rows.as_ptr(), restored.rows.as_ptr());
        replay_all(shell.areas.terminal.model.as_mut().unwrap(), b" changed");
        shell.terminal_draw(cx);
        let changed = &shell.areas.terminal.grid.read(cx).frame;
        assert_ne!(on.rows.as_ptr(), changed.rows.as_ptr());
        assert_ne!(on.rows, changed.rows);
    });
}

#[gpui::test]
fn scrollback_change_while_replay_owns_model_applies_on_return_and_new_open(
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
