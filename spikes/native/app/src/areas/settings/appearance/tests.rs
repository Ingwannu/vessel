//! Actual field events through the existing editor, isolated from link I/O.
use super::*;
use gpui::{AppContext as _, Entity, IntoElement, Render, TestAppContext};
use vessel_app_kit::field::AppearanceProfile;
use vessel_app_view::{
    appearance::{SyntaxTheme, form::Field},
    prefs::{ContentSize, ContentWidth},
};

struct Probe {
    shell: Entity<Shell>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            let fields = &shell.areas.settings.appearance.fields;
            div().flex().flex_col().children(Field::ALL.map(|field| {
                fields.of(field).appearance_element(
                    AppearanceProfile::Number,
                    "Appearance input".into(),
                    &Tokens::of(Scheme::Dark),
                    cx,
                )
            }))
        })
    }
}
fn setup(window: &mut Window, cx: &mut Context<'_, Probe>) -> Probe {
    window.activate_window();
    let shell = cx.new(|cx| {
        let mut shell = Shell::without_io(window, cx);
        sync(&mut shell, window, cx);
        shell
    });
    Probe { shell }
}
#[gpui::test]
fn numeric_focus_selects_all_live_preview_is_local_and_blur_commits_one_axis(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(setup);
    cx.run_until_parked();
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell
                .areas
                .settings
                .appearance
                .fields
                .of(Field::CodeSize)
                .focus(window, cx);
        });
    });
    cx.run_until_parked();
    cx.simulate_input("19abc");
    cx.run_until_parked();
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let state = &shell.areas.settings.appearance;
            assert_eq!(state.fields.of(Field::CodeSize).text(cx), "19");
            assert_eq!(state.preview.as_ref().unwrap().code_size.get(), 19);
            assert_eq!(shell.prefs.settings.appearance.code_size.get(), 12);
            state.fields.of(Field::InterfaceSize).focus(window, cx);
        });
    });
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        probe.shell.read_with(cx, |shell, cx| {
            assert_eq!(shell.prefs.settings.appearance.code_size.get(), 19);
            assert_eq!(shell.prefs.settings.appearance.content_size.get(), 15);
            assert_eq!(shell.prefs.settings.appearance.interface_size.get(), 14);
            assert_eq!(
                shell
                    .areas
                    .settings
                    .appearance
                    .fields
                    .of(Field::CodeSize)
                    .text(cx),
                "19"
            );
        });
    });
}
#[gpui::test]
fn theme_changes_preserve_other_drafts_while_external_axis_changes_resync_current_value(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(setup);
    cx.run_until_parked();
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let fields = &shell.areas.settings.appearance.fields;
            fields
                .of(Field::MonoFamily)
                .set_text("'unfinished", window, cx);
            fields.of(Field::CodeSize).set_text("20", window, cx);
            shell.prefer(&PreferenceChange::Theme(Theme::Zinc), window, cx);
            assert_eq!(
                shell
                    .areas
                    .settings
                    .appearance
                    .fields
                    .of(Field::MonoFamily)
                    .text(cx),
                "'unfinished"
            );
            assert_eq!(
                shell
                    .areas
                    .settings
                    .appearance
                    .fields
                    .of(Field::CodeSize)
                    .text(cx),
                "20"
            );
            shell.prefer(
                &PreferenceChange::ContentSize(ContentSize::try_from(18).unwrap()),
                window,
                cx,
            );
            assert_eq!(
                shell
                    .areas
                    .settings
                    .appearance
                    .fields
                    .of(Field::ContentSize)
                    .text(cx),
                "18"
            );
            assert_eq!(
                shell
                    .areas
                    .settings
                    .appearance
                    .fields
                    .of(Field::CodeSize)
                    .text(cx),
                "20"
            );
            shell.appearance_commit(Field::ContentSize, "", window, cx);
            assert_eq!(
                shell
                    .areas
                    .settings
                    .appearance
                    .fields
                    .of(Field::ContentSize)
                    .text(cx),
                "18"
            );
            shell.prefer(
                &PreferenceChange::SyntaxTheme(SyntaxTheme::Nord),
                window,
                cx,
            );
            assert_eq!(
                vessel_app_kit::syntax::selected(),
                vessel_app_kit::syntax::SyntaxPalette::Nord
            );
            shell.appearance_commit(Field::Width, "0x400", window, cx);
            assert_eq!(
                shell.prefs.settings.appearance.content_width.points(),
                Some(1024)
            );
            shell.prefer(
                &PreferenceChange::ContentWidth(ContentWidth::default()),
                window,
                cx,
            );
            assert_eq!(
                shell
                    .areas
                    .settings
                    .appearance
                    .fields
                    .of(Field::Width)
                    .text(cx),
                "820"
            );
            assert!(!shell.prefs.settings.appearance.content_width.resettable());
        });
    });
}

#[gpui::test]
fn language_refresh_keeps_family_draft_selection_and_legacy_width_as_presentation(
    cx: &mut TestAppContext,
) {
    use crate::shell::language_tests;
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, window, cx| {
            window.activate_window();
            shell.prefs.settings.appearance.content_width = ContentWidth::Full;
            sync(shell, window, cx);
            assert_eq!(
                shell
                    .areas
                    .settings
                    .appearance
                    .fields
                    .of(Field::Width)
                    .text(cx),
                ""
            );
        },
        |shell| shell.areas.settings.appearance.fields.of(Field::MonoFamily),
        |shell| {
            assert_eq!(
                shell.prefs.settings.appearance.content_width,
                ContentWidth::Full
            );
            assert_eq!(
                shell
                    .areas
                    .settings
                    .appearance
                    .saved
                    .as_ref()
                    .unwrap()
                    .content_width,
                ContentWidth::Full
            );
        },
    );
}
