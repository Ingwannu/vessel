//! Cursor phase must retain the painted rows; appearance and output still replace them.
use super::*;
use gpui::TestAppContext;
use vessel_app_kit::fonts::{FontStack, Typeface, active};

// These two fixtures share the process-wide code-font snapshot. Holding it
// across the read-only fixture keeps a concurrent font-change test from
// correctly invalidating rows during its cursor-only assertion.
static FONT_FIXTURE: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct RestoreFonts {
    context: TestAppContext,
    interface: FontStack,
    code: FontStack,
}
impl Drop for RestoreFonts {
    fn drop(&mut self) {
        self.context.update(|cx| {
            vessel_app_kit::set_font_stacks(cx, self.interface.clone(), self.code.clone());
        });
    }
}

#[gpui::test]
fn cursor_phase_reuses_rows_and_preserves_cursor_and_output(cx: &mut TestAppContext) {
    let _font_fixture = FONT_FIXTURE.lock().unwrap();
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
fn quiet_code_stack_change_invalidates_rows_with_unchanged_metrics(cx: &mut TestAppContext) {
    let _font_fixture = FONT_FIXTURE.lock().unwrap();
    crate::shell::language_tests::init(cx);
    let _restore = RestoreFonts {
        context: cx.clone(),
        interface: active(Typeface::Interface),
        code: active(Typeface::Code),
    };
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        cx.set_reduce_motion(true);
        shell.terminal_opened(
            "laptop".parse().unwrap(),
            SessionId::random(),
            TerminalSize::default(),
            cx,
        );
        shell.route = crate::routes::Route::Session;
        replay_all(
            shell.areas.terminal.model.as_mut().unwrap(),
            "quiet 한글".as_bytes(),
        );
        let area = &mut shell.areas.terminal;
        area.phase = Phase::Ready;
        area.fit = Some(GridSize::STANDARD);
        area.probe.set(Some(Bounds::new(
            gpui::point(px(0.0), px(0.0)),
            gpui::size(px(640.0), px(384.0)),
        )));
        area.grid.update(cx, |grid, _| {
            grid.cell = vessel_app_kit::terminal_grid::CellSize {
                width: px(8.0),
                height: px(16.0),
            };
        });
        shell.terminal_present(window, cx);
        let before = Rc::clone(&shell.areas.terminal.grid.read(cx).frame);
        let generation = shell
            .areas
            .terminal
            .model
            .as_ref()
            .unwrap()
            .grid()
            .generation();
        let cell = shell.areas.terminal.grid.read(cx).cell;
        let next = FontStack::from_names(
            ["DejaVu Sans Mono".into(), "Pretendard".into()],
            Typeface::Code,
        );
        assert_ne!(active(Typeface::Code), next);
        vessel_app_kit::set_font_stacks(cx, active(Typeface::Interface), next);
        shell.terminal_measured(cell, cx);
        shell.terminal_present(window, cx);
        let after = Rc::clone(&shell.areas.terminal.grid.read(cx).frame);
        assert_ne!(before.rows.as_ptr(), after.rows.as_ptr());
        assert_eq!(before.rows, after.rows);
        assert_eq!(before.font_size, after.font_size);
        assert_eq!(shell.areas.terminal.grid.read(cx).cell, cell);
        assert_eq!(
            shell
                .areas
                .terminal
                .model
                .as_ref()
                .unwrap()
                .grid()
                .generation(),
            generation
        );
        assert!(shell.areas.terminal.blinking.is_none());
        shell.terminal_present(window, cx);
        assert!(Rc::ptr_eq(
            &after,
            &shell.areas.terminal.grid.read(cx).frame
        ));
    });
}
