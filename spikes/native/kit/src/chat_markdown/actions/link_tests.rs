//! Link marks preserve partial source selection and rendered find offsets.

use std::sync::{Arc, Mutex};

use gpui::{
    AppContext as _, Context, Entity, IntoElement, Modifiers, MouseButton, ParentElement as _,
    Render, Styled as _, TestAppContext, Window, div, point, px,
};
use gpui_base::text::{TextView, TextViewState};

#[gpui::test]
fn direct_engine_keeps_the_component_installed_theme(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap();
        let defaults = gpui_base::TextViewDefaults::global(cx);
        let installed = defaults.style().unwrap();
        let tokens = crate::Tokens::of(crate::Scheme::Dark);
        let actual =
            crate::chat_markdown::presentation(crate::chat_markdown::Look::of(&tokens), cx);
        assert_eq!(actual.foreground(), installed.foreground());
        assert_eq!(actual.link(), installed.link());
        assert_eq!(actual.selection(), installed.selection());
        assert_eq!(actual.inline_code(), installed.inline_code());
        assert_eq!(actual.paragraph_gap(), installed.paragraph_gap());
        assert_eq!(actual.is_dark(), installed.is_dark());
        assert_eq!(actual.code_block().overflow.x, Some(gpui::Overflow::Scroll));
    });
}

use super::{Actions, CopyStyle, InlineLinkKind, RecognizedLink, Source, finish};

const PATH: &str = "src/lib.rs:4";

fn recognize(text: &str, _: InlineLinkKind) -> Vec<RecognizedLink> {
    text.find(PATH)
        .and_then(|start| RecognizedLink::new(text, start..start.saturating_add(PATH.len()), PATH))
        .into_iter()
        .collect()
}

struct Probe {
    state: Entity<TextViewState>,
    clicks: Arc<Mutex<Vec<(String, Source)>>>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let clicks = Arc::clone(&self.clicks);
        let actions = Actions::new(move |href, _, _| {
            clicks
                .lock()
                .unwrap()
                .push((href.as_str().to_owned(), href.source()))
        })
        .with_inline_links(recognize);
        div()
            .w(px(130.0))
            .child(gpui_base::TextSelectionLayer)
            .child(finish(
                TextView::new(&self.state),
                self.state.clone(),
                Some(actions),
                CopyStyle::MarkdownCodeAware,
            ))
    }
}

#[gpui::test]
fn inferred_token_keeps_partial_selection_and_find_source_offsets(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    for source in [
        "prefix words src/lib.rs:4 suffix words beyond more",
        "prefix words `src/lib.rs:4` suffix words beyond more",
        "**prefix words src/lib.rs:4 suffix words beyond more**",
        "**prefix words `src/lib.rs:4` suffix words beyond more**",
    ] {
        let clicks = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&clicks);
        let (probe, window) = cx.add_window_view(move |_, cx| Probe {
            state: cx.new(|cx| TextViewState::markdown(source, cx)),
            clicks,
        });
        window.run_until_parked();
        window.update(|window, cx| {
            let _paint = window.draw(cx);
        });
        let state = probe.read_with(window, |probe, _| probe.state.clone());
        let bounds = state.read_with(window, |state, _| state.bounds());
        let start = bounds.origin + point(px(12.0), px(4.0));
        let end = bounds.bottom_right() - point(px(4.0), px(4.0));
        window.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        window.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
        window.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        window.update(|window, cx| {
            let _paint = window.draw(cx);
        });
        state.read_with(window, |state, _| {
            let rendered = state.rendered_text();
            assert_eq!(rendered.source(), source);
            let path_start = source.find(PATH).unwrap();
            let found = rendered
                .range_for_source(path_start..path_start.saturating_add(PATH.len()))
                .unwrap();
            assert_eq!(&rendered.as_str()[found], PATH, "find offsets did not move");
            let selected = state
                .selected_source_range()
                .expect("drag selection maps to source");
            assert!(
                selected.start > 0 || selected.end < source.len(),
                "this must be a partial selection"
            );
            assert!(state.selected_text().contains(PATH));
            assert_eq!(
                state.selected_text().matches(PATH).count(),
                1,
                "overlapping code/emphasis/link marks must not copy bytes twice"
            );
            if !source.starts_with("**") {
                assert_eq!(
                    state.selected_text().trim_end_matches('\n'),
                    &source[selected]
                );
            }
            assert!(
                !state.selected_text().contains("](src/lib.rs:4)"),
                "inferred links must not fabricate copied markdown"
            );
        });
        assert!(
            captured.lock().unwrap().is_empty(),
            "dragging across a link does not activate it"
        );
    }
}

#[gpui::test]
fn explicit_links_keep_authored_provenance_and_fenced_code_is_not_recognized(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_base::init);
    for (source, authored) in [
        ("[src/lib.rs:4](https://example.com)", true),
        ("```text\nsrc/lib.rs:4\n```", false),
    ] {
        let clicks = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&clicks);
        let (_, window) = cx.add_window_view(move |_, cx| Probe {
            state: cx.new(|cx| TextViewState::markdown(source, cx)),
            clicks,
        });
        window.run_until_parked();
        window.simulate_click(point(px(8.0), px(8.0)), Modifiers::default());
        let seen = captured.lock().unwrap();
        if authored {
            assert_eq!(
                &*seen,
                &[("https://example.com".to_owned(), Source::Authored)]
            );
        } else {
            assert!(seen.is_empty());
        }
    }
}

#[gpui::test]
fn code_copy_uses_parsed_inline_and_fenced_regions(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    for (source, expected) in [
        ("`echo 한글`", Some("echo 한글")),
        ("```sh\n  echo 한글\n```", Some("  echo 한글")),
        ("````sh\necho '```'\n````", Some("echo '```'")),
        ("text `echo hi`", None),
        ("```sh\necho one\n```\n\n```sh\necho two\n```", None),
    ] {
        let (probe, window) = cx.add_window_view(move |_, cx| Probe {
            state: cx.new(|cx| TextViewState::markdown(source, cx)),
            clicks: Arc::default(),
        });
        window.run_until_parked();
        window.update(|window, cx| {
            let _paint = window.draw(cx);
        });
        let state = probe.read_with(window, |probe, _| probe.state.clone());
        window.update(|_, cx| state.update(cx, |state, cx| state.select_all(cx)));
        state.read_with(window, |state, _| {
            assert_eq!(
                state.selected_code_text(),
                None,
                "whole-view selection retains Markdown boundaries"
            );
        });
        window.update(|window, cx| {
            let focus = state.read(cx).focus_handle().clone();
            focus.focus(window, cx);
            let _paint = window.draw(cx);
            focus.dispatch_action(&gpui_base::input::Copy, window, cx);
            assert_eq!(
                cx.read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref(),
                Some(source.trim())
            );
        });
        if let Some(expected) = expected {
            // Stay within the rendered code, including every character. Message
            // Select All above instead crosses the code's semantic boundary.
            let bounds = state.read_with(window, |state, _| state.bounds());
            let start = bounds.origin
                + if source.starts_with('`') && !source.starts_with("```") {
                    point(px(1.0), px(8.0))
                } else {
                    point(px(17.0), px(40.0))
                };
            let end = point(bounds.right() - px(2.0), start.y);
            window.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
            window.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
            window.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
            window.update(|window, cx| {
                let _paint = window.draw(cx);
                assert_eq!(
                    state.read(cx).selected_code_text().as_deref(),
                    Some(expected)
                );
                let focus = window.focused(cx).expect("code drag focuses TextView");
                focus.dispatch_action(&gpui_base::input::Copy, window, cx);
                assert_eq!(
                    cx.read_from_clipboard()
                        .and_then(|item| item.text())
                        .as_deref(),
                    Some(expected)
                );
            });
        }
    }
}
