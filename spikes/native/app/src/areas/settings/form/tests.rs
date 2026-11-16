//! Real kit-root navigation, not another field or window fixture.
use std::time::Duration;

use gpui::{Modifiers, TestAppContext, VisualTestContext, size};
use vessel_app_kit::ui::px;
use vessel_app_view::{prefs::Panel, settings::Section};

use super::*;
use crate::areas::settings::choices::List;
use crate::areas::sidebar::policy_tests::setup;
use crate::shell::language_tests;

fn settled(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
}

#[gpui::test]
fn settings_uses_one_reachable_navigation_at_actual_sharing_boundaries(cx: &mut TestAppContext) {
    language_tests::init(cx);
    for (width, stacked) in [
        (390.0, true),
        (600.0, true),
        (650.0, true),
        (767.0, true),
        (800.0, true),
        (931.0, true),
        (932.0, false),
        (1280.0, false),
    ] {
        let (shell, cx) = setup(cx, width, Route::Settings);
        settled(cx);
        cx.update(|window, cx| {
            shell.read_with(cx, |shell, _| {
                assert_eq!(current(shell, window) == Form::Stacked, stacked);
            });
        });
        assert_eq!(cx.debug_bounds("settings-nav-list").is_some(), stacked);
        assert_eq!(cx.debug_bounds("settings-nav-docked").is_some(), !stacked);
        assert_eq!(cx.debug_bounds("settings-section-page").is_some(), !stacked);
    }
}

#[gpui::test]
fn a_narrow_desktop_clicks_section_then_back_to_list_and_prior_workspace(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = setup(cx, 800.0, Route::Draft);
    shell.update_in(cx, |shell, window, cx| {
        shell.open_settings(None, window, cx);
    });
    settled(cx);
    let row = cx
        .debug_bounds("settings-section-general")
        .expect("actual list row");
    cx.simulate_click(row.center(), Modifiers::none());
    settled(cx);
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.route, Route::Settings);
        assert!(shell.areas.settings.pushed);
        assert_eq!(shell.areas.settings.section, Section::General);
    });
    assert!(cx.debug_bounds("settings-nav-list").is_none());
    assert!(cx.debug_bounds("settings-nav-docked").is_none());
    assert!(cx.debug_bounds("settings-section-page").is_some());
    let back = cx
        .debug_bounds("settings-compact-back")
        .expect("actual back action");
    cx.simulate_click(back.center(), Modifiers::none());
    settled(cx);
    shell.read_with(cx, |shell, _| assert!(!shell.areas.settings.pushed));
    assert!(cx.debug_bounds("settings-nav-list").is_some());
    let back = cx
        .debug_bounds("settings-compact-back")
        .expect("prior workspace action");
    cx.simulate_click(back.center(), Modifiers::none());
    settled(cx);
    shell.read_with(cx, |shell, _| assert_eq!(shell.route, Route::Draft));
}

#[gpui::test]
fn explicit_section_survives_resize_and_keeps_body_density_independent(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = setup(cx, 1280.0, Route::Draft);
    shell.update_in(cx, |shell, window, cx| {
        shell.open_settings(Some(Section::General), window, cx);
    });
    settled(cx);
    let desktop_height = shell.read_with(cx, |shell, _| {
        shell
            .areas
            .settings
            .anchors
            .of(List::Delivery)
            .get()
            .unwrap()
            .size
            .height
    });
    cx.simulate_resize(size(px(800.0), px(600.0)));
    settled(cx);
    shell.read_with(cx, |shell, _| {
        assert!(shell.areas.settings.pushed);
        assert_eq!(shell.areas.settings.section, Section::General);
        assert_eq!(
            shell
                .areas
                .settings
                .anchors
                .of(List::Delivery)
                .get()
                .unwrap()
                .size
                .height,
            desktop_height
        );
    });
    assert!(cx.debug_bounds("settings-compact-back").is_some());
    cx.simulate_resize(size(px(650.0), px(600.0)));
    settled(cx);
    shell.read_with(cx, |shell, _| {
        assert!(
            shell
                .areas
                .settings
                .anchors
                .of(List::Delivery)
                .get()
                .unwrap()
                .size
                .height
                > desktop_height
        );
    });
    cx.simulate_resize(size(px(1280.0), px(600.0)));
    shell.update_in(cx, |shell, _window, cx| {
        shell.prefs.settings.layout.sidebar = Panel::Hidden;
        cx.notify();
    });
    settled(cx);
    assert!(cx.debug_bounds("settings-nav-docked").is_none());
    assert!(cx.debug_bounds("settings-compact-back").is_some());
    shell.update_in(cx, Shell::toggle_sidebar);
    settled(cx);
    assert!(cx.debug_bounds("settings-nav-docked").is_some());
    assert!(cx.debug_bounds("settings-compact-back").is_none());
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.areas.settings.section, Section::General);
        assert!(shell.areas.settings.pushed);
    });
}

#[gpui::test]
fn changing_form_closes_the_old_popup_anchor_without_applying_a_choice(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = setup(cx, 1280.0, Route::Draft);
    shell.update_in(cx, |shell, window, cx| {
        shell.open_settings(Some(Section::General), window, cx);
    });
    settled(cx);
    let kept = shell.read_with(cx, |shell, _| shell.prefs.settings.composer.delivery);
    shell.update_in(cx, |shell, window, cx| {
        shell.open_list(List::Delivery, window, cx);
    });
    settled(cx);
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.areas.settings.showing(), Some(List::Delivery));
    });
    cx.simulate_resize(size(px(800.0), px(600.0)));
    settled(cx);
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.areas.settings.showing(), None);
        assert_eq!(shell.prefs.settings.composer.delivery, kept);
    });
}

#[gpui::test]
fn changing_form_closes_language_without_selecting_another_preference(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = setup(cx, 1280.0, Route::Draft);
    shell.update_in(cx, |shell, window, cx| {
        shell.open_settings(Some(Section::General), window, cx);
    });
    settled(cx);
    let kept = shell.read_with(cx, |shell, _| shell.prefs.settings.language);
    let trigger = cx
        .debug_bounds("settings-language-trigger")
        .expect("actual language trigger");
    cx.simulate_click(trigger.center(), Modifiers::none());
    settled(cx);
    shell.update_in(cx, |shell, window, cx| {
        let tokens = vessel_app_kit::tokens(cx);
        assert!(super::super::language::picker(shell, &tokens, window, cx).is_some());
    });
    cx.simulate_resize(size(px(800.0), px(600.0)));
    settled(cx);
    shell.update_in(cx, |shell, window, cx| {
        let tokens = vessel_app_kit::tokens(cx);
        assert!(super::super::language::picker(shell, &tokens, window, cx).is_none());
        assert_eq!(shell.prefs.settings.language, kept);
    });
}
