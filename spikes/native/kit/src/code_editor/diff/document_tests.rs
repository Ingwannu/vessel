//! The real outer `VirtualList` follows keyboard selection on the same engine.
use super::*;
use crate::VirtualList;
use crate::code_editor::diff::tests::Plain;
use gpui::{Render, TestAppContext};
use std::num::NonZeroU32;
use std::sync::Arc;

struct Owner {
    editor: DiffDocumentEditor,
    files: VirtualList,
    reveals: usize,
}
impl Render for Owner {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let editor = self.editor.clone();
        // This fixture's list fills the root window. Snapshot its actual width
        // before List holds its RefCell while invoking this row renderer.
        let width = window.viewport_size().width;
        self.files.element(move |_, window, cx| {
            editor.element(&Tokens::of(crate::theme::Scheme::Dark), width, window, cx)
        })
    }
}
#[gpui::test]
fn keyboard_end_and_shift_selection_reveal_the_real_document_owner(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let source = (0..2500)
        .map(|row| format!("source {row}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (owner, cx) =
        cx.add_window_view(|window, cx| document_owner(&source, DiffWrapping::Scroll, window, cx));
    cx.update(|window, cx| {
        window.activate_window();
        owner.update(cx, |owner, cx| owner.editor.editor.focus(window, cx));
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let end = if cfg!(target_os = "macos") {
        "cmd-down"
    } else {
        "ctrl-end"
    };
    cx.simulate_keystrokes(end);
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    let viewport_height = cx.update(|window, _cx| window.viewport_size().height);
    owner.read_with(cx, |owner, cx| {
        assert!(owner.reveals > 0);
        let state = owner.editor.editor.editor.state.read(cx);
        assert_eq!(state.cursor_position().line, 2499);
        let cursor = state.cursor();
        let bounds = state
            .range_to_bounds(&(cursor..cursor))
            .expect("newly revealed caret row is shaped");
        assert!(bounds.top() >= px(0.));
        assert!(
            bounds.bottom() <= viewport_height,
            "the external owner reveals the end, not a nested card"
        );
    });
    cx.simulate_keystrokes("shift-up");
    cx.run_until_parked();
    owner.read_with(cx, |owner, cx| {
        let state = owner.editor.editor.editor.state.read(cx);
        assert!(!state.selected_text().to_string().is_empty());
        assert_eq!(state.value(), source);
        assert!(state.presentation().is_readonly());
    });
}

fn document_owner(
    source: &str,
    wrapping: DiffWrapping,
    window: &mut Window,
    cx: &mut Context<'_, Owner>,
) -> Owner {
    let input = DiffText::new(
        Arc::from(source),
        source
            .split('\n')
            .enumerate()
            .map(|(index, _)| super::super::DiffLine {
                number: NonZeroU32::new(u32::try_from(index.saturating_add(1)).unwrap()),
                tone: super::super::DiffTone::Context,
            })
            .collect::<Vec<_>>()
            .into(),
    )
    .unwrap();
    let files = VirtualList::new(false);
    files.reset(1);
    Owner {
        editor: DiffDocumentEditor::new(
            input,
            Plain,
            wrapping,
            |owner: &mut Owner, bounds, _window, cx| {
                owner.files.reveal_bounds(bounds, px(0.));
                owner.reveals = owner.reveals.saturating_add(1);
                cx.notify();
            },
            window,
            cx,
        ),
        files,
        reveals: 0,
    }
}

#[gpui::test]
fn native_wheel_packets_over_the_document_scroll_its_outer_owner(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let source = std::iter::repeat_n("wide source ".repeat(40), 2500)
        .collect::<Vec<_>>()
        .join("\n");
    for (wrapping, horizontal_drift) in [
        (DiffWrapping::Scroll, 0.),
        (DiffWrapping::Scroll, -0.5),
        (DiffWrapping::Wrap, 0.),
    ] {
        let (owner, cx) =
            cx.add_window_view(|window, cx| document_owner(&source, wrapping, window, cx));
        cx.run_until_parked();
        let before = owner.read_with(cx, |owner, _cx| owner.files.top());
        cx.simulate_event(gpui::ScrollEvent {
            position: point(px(100.), px(100.)),
            delta: gpui::ScrollDelta::Pixels(point(px(horizontal_drift), px(-80.))),
            touch_phase: gpui::TouchPhase::Moved,
            ..Default::default()
        });
        cx.run_until_parked();
        owner.read_with(cx, |owner, cx| {
            let state = owner.editor.editor.editor.state.read(cx);
            assert_ne!(
                owner.files.top(),
                before,
                "outer owner must scroll: mode={wrapping:?}, x={horizontal_drift}, inner={:?}, input={:?}",
                state.scroll_offset(),
                state.input_bounds(),
            );
            assert_eq!(
                state.scroll_offset().y,
                px(0.),
                "the full-height editor has no inner vertical card scroll"
            );
            assert_eq!(state.value(), source);
        });
    }
}

#[gpui::test]
fn horizontal_wheel_packets_keep_the_same_document_engine_scroll(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let source = std::iter::repeat_n("wide source ".repeat(40), 2500)
        .collect::<Vec<_>>()
        .join("\n");
    let (owner, cx) =
        cx.add_window_view(|window, cx| document_owner(&source, DiffWrapping::Scroll, window, cx));
    cx.run_until_parked();
    let before = owner.read_with(cx, |owner, _cx| owner.files.top());
    cx.simulate_event(gpui::ScrollEvent {
        position: point(px(100.), px(100.)),
        delta: gpui::ScrollDelta::Pixels(point(px(-80.), px(-0.5))),
        touch_phase: gpui::TouchPhase::Moved,
        ..Default::default()
    });
    cx.run_until_parked();
    owner.read_with(cx, |owner, cx| {
        assert_eq!(owner.files.top(), before);
        let state = owner.editor.editor.editor.state.read(cx);
        assert!(state.scroll_offset().x < px(0.));
        assert_eq!(state.scroll_offset().y, px(0.));
        assert_eq!(state.value(), source);
    });
}
