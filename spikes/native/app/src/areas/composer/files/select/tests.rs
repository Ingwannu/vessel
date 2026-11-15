//! Native menu acquisition and key paste share the same typed intake.
use super::*;
use crate::shell::language_tests;
use gpui::{ClipboardItem, Image, ImageFormat, TestAppContext};

#[gpui::test]
fn clipboard_menu_attaches_the_image_and_restores_the_target_field(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    let bytes = include_bytes!("clipboard.png").to_vec();
    cx.write_to_clipboard(ClipboardItem::new_image(&Image::from_bytes(
        ImageFormat::Png,
        bytes.clone(),
    )));
    shell.update_in(cx, |shell, window, cx| {
        shell.areas.composer.attach_for = Some(DraftKey::New);
        shell.composer_choose(Acquire::ClipboardImage, window, cx);
        assert!(shell.composer_attachments_blocked(DraftKey::New));
        assert!(
            shell
                .areas
                .draft
                .composer
                .focus_handle(cx)
                .is_focused(window)
        );
    });
    cx.run_until_parked();
    shell.update_in(cx, |shell, _window, cx| {
        assert!(!shell.composer_attachments_blocked(DraftKey::New));
        let tray = shell.areas.composer.trays.get(DraftKey::New).unwrap();
        assert_eq!(tray.items().len(), 1);
        assert_eq!(tray.items()[0].name.as_str(), "image.png");
        assert!(matches!(&tray.items()[0].origin, vessel_app_view::composer::tray::Origin::Pasted(actual) if actual.as_ref() == bytes.as_slice()));
    });
}
#[gpui::test]
fn empty_image_clipboard_is_a_visible_error_without_inserting_its_text(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    cx.write_to_clipboard(ClipboardItem::new_string("clipboard text".to_owned()));
    shell.update_in(cx, |shell, window, cx| {
        shell
            .areas
            .draft
            .composer
            .set_text("kept draft", window, cx);
        shell.areas.composer.attach_for = Some(DraftKey::New);
        shell.composer_choose(Acquire::ClipboardImage, window, cx);
        assert!(!shell.composer_has_files(DraftKey::New));
        assert_eq!(shell.areas.draft.composer.text(cx), "kept draft");
        assert_eq!(
            shell.areas.toasts.list().last().unwrap().text,
            shell.words.composer().image_clipboard_empty()
        );
    });
}
