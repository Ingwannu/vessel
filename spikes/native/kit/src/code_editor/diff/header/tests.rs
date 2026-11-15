//! Header byte ranges and actual engine notifications across theme changes.
use super::*;
use crate::code_editor::diff::DiffLine;
use gpui::AppContext as _;

fn text() -> DiffText {
    DiffText::new(
        Arc::from("코드\n@@ 함수 @@\n\nnotice\n@@ end @@"),
        Arc::from([
            DiffLine {
                number: None,
                tone: DiffTone::Context,
            },
            DiffLine {
                number: None,
                tone: DiffTone::Header,
            },
            DiffLine {
                number: None,
                tone: DiffTone::Header,
            },
            DiffLine {
                number: None,
                tone: DiffTone::Notice,
            },
            DiffLine {
                number: None,
                tone: DiffTone::Header,
            },
        ]),
    )
    .unwrap()
}

#[test]
fn only_nonempty_header_text_is_decorated_in_utf8_byte_coordinates() {
    let text = text();
    let actual = ranges(&text);
    let second = "코드\n@@ 함수 @@\n\nnotice\n".len();
    assert_eq!(
        actual,
        vec![
            "코드\n".len().."코드\n@@ 함수 @@".len(),
            second..text.text.len()
        ]
    );
    assert_eq!(text.text.get(actual[0].clone()), Some("@@ 함수 @@"));
    assert_eq!(text.text.get(actual[1].clone()), Some("@@ end @@"));
}

#[gpui::test]
fn unchanged_header_projection_does_not_notify_and_clones_share_theme_updates(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(gpui_base::init);
    let cx = cx.add_empty_window();
    let text = text();
    let state = cx.update(|window, cx| {
        cx.new(|cx| EditorState::new(window, cx).default_value(text.text.to_string()))
    });
    let headers = cx.update(|_, cx| HeaderInk::new(&state, &text, cx));
    cx.run_until_parked();
    let notifications = Rc::new(Cell::new(0_usize));
    let counter = Rc::clone(&notifications);
    let _watch = cx.update(|_, cx| {
        cx.observe(&state, move |_, _| {
            counter.set(counter.get().saturating_add(1))
        })
    });
    let dark = crate::Tokens::of(crate::theme::Scheme::Dark)
        .neutral
        .foreground_muted;
    let light = crate::Tokens::of(crate::theme::Scheme::Light)
        .neutral
        .foreground_muted;
    assert_ne!(dark, light);
    cx.update(|_, cx| headers.apply(dark, cx));
    cx.run_until_parked();
    assert_eq!(notifications.get(), 1);
    let cloned = headers.clone();
    for _ in 0..3 {
        cx.update(|_, cx| cloned.apply(dark, cx));
        cx.run_until_parked();
    }
    assert_eq!(notifications.get(), 1);
    cx.update(|_, cx| cloned.apply(light, cx));
    cx.run_until_parked();
    assert_eq!(notifications.get(), 2);
    cx.update(|_, cx| headers.apply(light, cx));
    cx.run_until_parked();
    assert_eq!(notifications.get(), 2);
    let HeaderInk::Rows { collection, .. } = headers else {
        panic!("headers exist")
    };
    assert_eq!(cx.update(|_, cx| collection.get_ranges(cx)), ranges(&text));
    assert_eq!(
        state.read_with(cx, |state, _| state.value().to_string()),
        text.text.as_ref()
    );
}
