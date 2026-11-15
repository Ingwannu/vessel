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
                    bytes: include_bytes!("../files/select/clipboard.png").to_vec()
                }],
                cx
            ),
            KeyAnswer::Taken
        );
        assert!(shell.composer_attachments_blocked(DraftKey::New));
        assert_eq!(shell.areas.draft.composer.text(cx), "");
    });
    cx.run_until_parked();
    shell.update(cx, |shell, cx| {
        assert!(!shell.composer_attachments_blocked(DraftKey::New));
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
fn invalid_image_paste_is_consumed_and_refused_before_creating_a_tray(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        shell
            .areas
            .draft
            .composer
            .set_text("retained text", window, cx);
        assert_eq!(
            shell.composer_pasted(
                true,
                vec![Pasted::Image {
                    mime: "image/png",
                    bytes: vec![1, 2, 3],
                }],
                cx
            ),
            KeyAnswer::Taken
        );
        assert!(shell.composer_attachments_blocked(DraftKey::New));
    });
    cx.run_until_parked();
    shell.update(cx, |shell, cx| {
        assert!(!shell.composer_attachments_blocked(DraftKey::New));
        assert!(!shell.composer_has_files(DraftKey::New));
        assert_eq!(shell.areas.draft.composer.text(cx), "retained text");
        assert_eq!(
            shell
                .areas
                .toasts
                .list()
                .last()
                .expect("visible refusal")
                .text,
            shell.words.composer().image_format_unsupported()
        );
    });
}
