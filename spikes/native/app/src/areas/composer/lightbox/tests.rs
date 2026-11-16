//! These use the real shell controls with no node, updater or credentials.
use crate::shell::{Shell, language_tests};
use gpui::{ClipboardItem, Image, ImageFormat, TestAppContext};
use std::sync::Arc;
use vessel_app_kit::field::composer_hooks::{KeyAnswer, Pasted};
use vessel_app_view::composer::drafts::DraftKey;

#[gpui::test]
fn closing_the_image_restores_the_original_composer_focus(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        window.activate_window();
        shell.areas.draft.composer.focus(window, cx);
        shell
            .areas
            .draft
            .composer
            .replace_all("kept text", window, cx);
        shell.open_local_image(Arc::new(Image::empty()).into(), window, cx);
        assert!(shell.areas.composer.lightbox.focus.is_focused(window));
        shell.open_local_image(Arc::new(Image::empty()).into(), window, cx);
        shell.close_image(window, cx);
        assert!(
            shell
                .areas
                .draft
                .composer
                .focus_handle(cx)
                .is_focused(window)
        );
        assert_eq!(shell.areas.draft.composer.text(cx), "kept text");
        assert!(shell.areas.composer.lightbox.preview.is_none());
    });
}

#[gpui::test]
fn image_paste_is_attached_but_text_paste_remains_text(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        window.activate_window();
        shell.areas.draft.composer.focus(window, cx);
        assert_eq!(shell.composer_pasted(true, Vec::new(), cx), KeyAnswer::Pass);
        assert_eq!(
            shell.composer_pasted(
                true,
                vec![Pasted::Image {
                    mime: "image/png",
                    bytes: vec![1, 2, 3]
                }],
                cx
            ),
            KeyAnswer::Taken
        );
        let tray = shell
            .areas
            .composer
            .trays
            .get(DraftKey::New)
            .expect("image tray");
        assert_eq!(tray.items().len(), 1);
        assert_eq!(
            tray.items().first().expect("image").kind,
            vessel_proto::AttachmentKind::Image
        );
        assert_eq!(shell.areas.draft.composer.text(cx), "");
    });
    cx.write_to_clipboard(ClipboardItem::new_string("clipboard text".to_owned()));
    let paste = cx.read(|cx| {
        if cx.platform_info().is_mac() {
            "cmd-v"
        } else {
            "ctrl-v"
        }
    });
    cx.simulate_keystrokes(paste);
    cx.run_until_parked();
    shell.update(cx, |shell, cx| {
        assert_eq!(shell.areas.draft.composer.text(cx), "clipboard text");
    });
}

#[gpui::test]
fn escape_in_the_lightbox_cannot_submit_the_background_composer(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        window.activate_window();
        shell.areas.draft.composer.focus(window, cx);
        shell.open_local_image(
            Arc::new(Image::from_bytes(ImageFormat::Png, vec![1])).into(),
            window,
            cx,
        );
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    shell.update(cx, |shell, _| {
        assert!(shell.areas.composer.lightbox.preview.is_none());
    });
}

#[gpui::test]
fn ordinary_wheel_and_modifier_wheel_use_the_same_bounded_modal_route(cx: &mut TestAppContext) {
    use gpui::{Modifiers, ScrollDelta, ScrollEvent, point, px};
    use vessel_app_view::composer::viewer::{Extent, MAX_SCALE, MIN_SCALE, Viewport};
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        window.activate_window();
        shell.open_local_image(Arc::new(Image::empty()).into(), window, cx);
        let canvas = Extent::new(800.0, 600.0).unwrap();
        shell.areas.composer.lightbox.viewport = Some(Viewport::new(canvas, canvas));
    });
    cx.run_until_parked();
    // Dispatch real scroll events through the rendered modal listener, including
    // ordinary wheel input; a modifier-only guard would leave scale at fit.
    for modifiers in [
        Modifiers::default(),
        Modifiers {
            control: true,
            ..Default::default()
        },
        Modifiers {
            platform: true,
            ..Default::default()
        },
    ] {
        shell.update(cx, |shell, _cx| {
            shell
                .areas
                .composer
                .lightbox
                .viewport
                .as_mut()
                .unwrap()
                .reset();
        });
        cx.simulate_event(ScrollEvent {
            position: point(px(100.0), px(100.0)),
            delta: ScrollDelta::Lines(point(0.0, 2.0)),
            modifiers,
            ..Default::default()
        });
        cx.run_until_parked();
        shell.read_with(cx, |shell, _| {
            assert!(shell.areas.composer.lightbox.viewport.unwrap().scale() > MIN_SCALE);
        });
    }
    for (delta, expected) in [(100_000.0, MAX_SCALE), (-100_000.0, MIN_SCALE)] {
        for _step in 0..12 {
            cx.simulate_event(ScrollEvent {
                position: point(px(100.0), px(100.0)),
                delta: ScrollDelta::Lines(point(0.0, delta)),
                ..Default::default()
            });
            cx.run_until_parked();
        }
        shell.read_with(cx, |shell, _| {
            assert_eq!(
                shell
                    .areas
                    .composer
                    .lightbox
                    .viewport
                    .unwrap()
                    .scale()
                    .to_bits(),
                expected.to_bits()
            );
        });
    }
}
