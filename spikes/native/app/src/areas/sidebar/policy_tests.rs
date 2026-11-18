//! Real shell rendering at the sidebar boundary, which the phone chrome
//! shares (600, TRACKER 798). The shared pure budget tests cover every
//! numeric branch.
use super::arrangement;
use crate::routes::Route;
use crate::shell::{Shell, language_tests};
use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext, size};
use std::{cell::RefCell, rc::Rc};
use vessel_app_kit::ui::px;
use vessel_app_view::{panels::SidebarLayout, prefs::Panel};

pub(super) fn setup(
    cx: &mut TestAppContext,
    width: f32,
    route: Route,
) -> (Entity<Shell>, &mut VisualTestContext) {
    let saved = Rc::new(RefCell::new(None));
    let window = cx.update(|cx| {
        cx.open_window(gpui::WindowOptions::default(), {
            let saved = Rc::clone(&saved);
            move |window, cx| {
                let shell = cx.new(|cx| {
                    let mut shell = Shell::without_io(window, cx);
                    shell.route = route;
                    shell
                });
                *saved.borrow_mut() = Some(shell.clone());
                vessel_app_kit::root(shell, window, cx)
            }
        })
        .unwrap()
    });
    let cx = VisualTestContext::from_window(window.into(), cx).into_mut();
    cx.simulate_resize(size(px(width), px(600.0)));
    cx.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let shell = saved.borrow().as_ref().unwrap().clone();
    (shell, cx)
}

#[gpui::test]
fn root_docks_requested_sidebar_at_exact_six_hundred_without_explorer_or_settings(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = setup(cx, 600.0, Route::Draft);
    cx.update(|window, cx| {
        shell.read_with(cx, |shell, _| {
            assert_eq!(shell.prefs.settings.layout.sidebar, Panel::Shown);
            assert_eq!(shell.code_placement(window), None);
            assert!(matches!(
                arrangement(shell, window),
                SidebarLayout::Docked { width, .. } if width.to_bits() == 200.0_f32.to_bits()
            ));
            assert!(shell.areas.sidebar.layout.scroll.bounds().size.height > px(0.0));
        });
    });
    shell.update_in(cx, Shell::toggle_sidebar);
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.prefs.settings.layout.sidebar, Panel::Hidden);
    });
    cx.simulate_resize(size(px(590.0), px(600.0)));
    cx.run_until_parked();
    shell.update_in(cx, Shell::toggle_sidebar);
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(shell.sidebar_open());
        assert_eq!(shell.prefs.settings.layout.sidebar, Panel::Hidden);
    });
    cx.simulate_resize(size(px(650.0), px(600.0)));
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(!shell.sidebar_open());
        assert_eq!(shell.prefs.settings.layout.sidebar, Panel::Hidden);
    });
}

#[gpui::test]
fn settings_takes_its_phone_form_below_the_one_compact_width(cx: &mut TestAppContext) {
    language_tests::init(cx);
    // Settings folds at the one compact width, as every route (TRACKER 798).
    for (width, sheet) in [(590.0, true), (650.0, false), (800.0, false)] {
        let (shell, cx) = setup(cx, width, Route::Settings);
        cx.update(|window, cx| {
            shell.read_with(cx, |shell, _| {
                assert_eq!(
                    matches!(arrangement(shell, window), SidebarLayout::Sheet),
                    sheet
                );
            });
        });
    }
}

#[gpui::test]
fn at_the_least_width_the_foot_keeps_labels_on_one_line(cx: &mut TestAppContext) {
    language_tests::init(cx);
    // 600: the sidebar docks at its least, 200 (TRACKER 798's 600 frame).
    let (_shell, cx) = setup(cx, 600.0, Route::Draft);
    let labels = cx.debug_bounds("sidebar-foot-labels").unwrap();
    // One line, wider than tall; squeezed, it stood one letter a line.
    assert!(labels.size.width > labels.size.height, "{labels:?}");
}
