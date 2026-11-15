//! One hook per real editor Change; focus flags and current blur text stay authoritative.
use super::*;
use crate::{Scheme, TextField, Tokens};
use gpui::prelude::*;
use gpui::{Render, TestAppContext, div};

#[derive(Debug, PartialEq, Eq)]
enum Seen {
    Focus,
    Edit(String),
    Submit(String),
    Blur(String),
    Legacy(String),
}
struct Probe {
    field: TextField,
    legacy: TextField,
    other: TextField,
    events: Vec<Seen>,
    source_edits: usize,
    legacy_source_edits: usize,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        div()
            .flex()
            .flex_col()
            .child(self.field.element(&tokens, false, cx))
            .child(self.legacy.element(&tokens, false, cx))
            .child(self.other.element(&tokens, false, cx))
    }
}
#[gpui::test]
fn windowed_hooks_follow_the_single_editor_stream_and_legacy_has_no_blur_hook(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_base::init);
    let (probe, cx) = cx.add_window_view(|window, cx| Probe {
        field: TextField::with_events(
            "",
            |owner: &mut Probe, text, _, _| owner.events.push(Seen::Submit(text)),
            FieldEvents::<Probe> {
                on_edit: Some(|owner, text, _, cx| {
                    assert_eq!(text, owner.field.text(cx));
                    owner.events.push(Seen::Edit(text));
                }),
                on_focus: Some(|owner, _, _| {
                    assert!(owner.field.focused.get());
                    owner.events.push(Seen::Focus);
                }),
                on_blur: Some(|owner, text, _, cx| {
                    assert!(!owner.field.focused.get());
                    assert_eq!(text, owner.field.text(cx));
                    owner.events.push(Seen::Blur(text));
                }),
            },
            window,
            cx,
        ),
        legacy: TextField::with_change(
            "",
            |_: &mut Probe, _, _, _| {},
            |owner, text, _| owner.events.push(Seen::Legacy(text)),
            window,
            cx,
        ),
        other: TextField::new("", |_: &mut Probe, _, _, _| {}, window, cx),
        events: Vec::new(),
        source_edits: 0,
        legacy_source_edits: 0,
    });
    // Test-only observers count the engine primitives independently. Input
    // simulation inserts each Unicode scalar, and native Enter fallback can
    // emit a same-value Change; neither means the kit duplicated a callback.
    probe.update_in(cx, |owner, window, cx| {
        let crate::field::Kind::Line(state) = &owner.field.kind else {
            panic!("windowed line engine");
        };
        cx.subscribe_in(state, window, |owner, _, event: &InputEvent, _, _| {
            if matches!(event, InputEvent::Change) {
                owner.source_edits = owner.source_edits.saturating_add(1);
            }
        })
        .detach();
        let crate::field::Kind::Line(state) = &owner.legacy.kind else {
            panic!("legacy line engine");
        };
        cx.subscribe_in(state, window, |owner, _, event: &InputEvent, _, _| {
            if matches!(event, InputEvent::Change) {
                owner.legacy_source_edits = owner.legacy_source_edits.saturating_add(1);
            }
        })
        .detach();
    });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    probe.update_in(cx, |owner, window, cx| {
        owner.events.clear();
        owner.field.focus(window, cx);
    });
    cx.run_until_parked();
    cx.simulate_input("한글");
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    probe.update_in(cx, |owner, window, cx| owner.other.focus(window, cx));
    cx.run_until_parked();
    probe.read_with(cx, |owner, cx| {
        assert_eq!(owner.events.first(), Some(&Seen::Focus));
        assert_eq!(owner.events.last(), Some(&Seen::Blur("한글".into())));
        assert_eq!(
            owner
                .events
                .iter()
                .filter(|event| matches!(event, Seen::Focus))
                .count(),
            1
        );
        assert_eq!(
            owner
                .events
                .iter()
                .filter(|event| matches!(event, Seen::Blur(_)))
                .count(),
            1
        );
        assert_eq!(
            owner
                .events
                .iter()
                .filter(|event| matches!(event, Seen::Submit(_)))
                .count(),
            1
        );
        let submit = owner
            .events
            .iter()
            .position(|event| matches!(event, Seen::Submit(_)))
            .unwrap();
        assert_eq!(owner.events[submit], Seen::Submit("한글".into()));
        assert!(
            owner.events[..submit]
                .iter()
                .any(|event| *event == Seen::Edit("한글".into()))
        );
        let edits: Vec<_> = owner
            .events
            .iter()
            .filter_map(|event| match event {
                Seen::Edit(text) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(edits.len(), owner.source_edits);
        assert_eq!(edits.first(), Some(&"한"));
        assert_eq!(edits.last(), Some(&"한글"));
        assert!(edits.iter().all(|text| matches!(*text, "한" | "한글")));
        assert_eq!(owner.field.text(cx), "한글");
    });
    probe.update_in(cx, |owner, window, cx| {
        owner.events.clear();
        owner.legacy.focus(window, cx);
    });
    cx.run_until_parked();
    cx.simulate_input("legacy");
    cx.run_until_parked();
    probe.update_in(cx, |owner, window, cx| owner.other.focus(window, cx));
    cx.run_until_parked();
    probe.read_with(cx, |owner, _| {
        assert_eq!(owner.events.len(), owner.legacy_source_edits);
        assert!(
            owner
                .events
                .iter()
                .all(|event| matches!(event, Seen::Legacy(_)))
        );
        assert_eq!(owner.events.last(), Some(&Seen::Legacy("legacy".into())));
    });
}
