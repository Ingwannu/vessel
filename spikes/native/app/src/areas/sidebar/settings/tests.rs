//! Real settings controls drive the same inventory that draws the Search row.
use super::{Position, row};
use crate::shell::{Shell, language_tests};
use gpui::prelude::*;
use gpui::{
    Bounds, Entity, Modifiers, Pixels, TestAppContext, VisualTestContext, Window, div, point, px,
    size,
};
use std::{cell::RefCell, rc::Rc};
use vessel_app_kit::{Scheme, SchemeChoice, Tokens};
use vessel_app_view::{
    Lang,
    localization::{Locale, Translator},
    sidebar::navigation::{Builtin, Section},
};

#[derive(Clone, Copy)]
enum Surface {
    Row(Builtin),
    Search,
}
struct Probe {
    shell: Entity<Shell>,
    surface: Surface,
    scheme: Scheme,
    seen: Rc<RefCell<Vec<Bounds<Pixels>>>>,
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let surface = self.surface;
        let seen = Rc::clone(&self.seen);
        self.shell.update(cx, |shell, cx| {
            let body = match surface {
                Surface::Search => div().children(crate::areas::palette::sidebar_row(
                    shell, &tokens, window, cx,
                )),
                Surface::Row(builtin) => {
                    let section = builtin.section();
                    let all = super::super::navigation::inventory(shell);
                    let entries = all
                        .iter()
                        .filter(|entry| entry.section == section)
                        .collect::<Vec<_>>();
                    let at = entries
                        .iter()
                        .position(|entry| entry.item.key.builtin() == Some(builtin))
                        .unwrap();
                    row(
                        section,
                        entries[at],
                        Position {
                            at,
                            count: entries.len(),
                        },
                        shell,
                        &tokens,
                        vessel_app_view::panels::sidebar_form(f32::from(
                            window.viewport_size().width,
                        )) == vessel_app_view::panels::SidebarForm::Sheet,
                        cx,
                    )
                }
            };
            body.w_full()
                .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
        })
    }
}
fn setup(
    cx: &mut TestAppContext,
    width: f32,
    scheme: Scheme,
    lang: Lang,
    builtin: Builtin,
) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| vessel_app_kit::init(cx, SchemeChoice::Fixed(scheme)).unwrap());
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            shell.t = lang.text();
            shell.words = Translator::get(
                Locale::from_tag(if lang == Lang::En { "en" } else { "ko" }).unwrap(),
            )
            .unwrap();
            shell
        });
        cx.observe(&shell, |_, _, cx| cx.notify()).detach();
        Probe {
            shell,
            surface: Surface::Row(builtin),
            scheme,
            seen: Rc::default(),
        }
    });
    cx.simulate_resize(size(px(width), px(450.0)));
    cx.run_until_parked();
    (probe, cx)
}
#[derive(Clone, Copy)]
enum Control {
    Up,
    Down,
    Toggle,
}
fn click_control(probe: &Entity<Probe>, cx: &mut VisualTestContext, control: Control, width: f32) {
    let bounds = probe.read_with(cx, |probe, _| {
        *probe.seen.borrow().get(1).expect("actual action bounds")
    });
    let offset = match (control, width < 600.0) {
        (Control::Up, false) => 16.0,
        (Control::Up, true) => 22.0,
        (Control::Down, false) => 56.0,
        (Control::Down, true) => 74.0,
        (Control::Toggle, false) => 97.0,
        (Control::Toggle, true) => 126.0,
    };
    cx.simulate_click(
        point(
            px(f32::from(bounds.left()).mul_add(1.0, offset)),
            bounds.center().y,
        ),
        Modifiers::none(),
    );
    cx.run_until_parked();
}
fn show(probe: &Entity<Probe>, cx: &mut VisualTestContext, surface: Surface) {
    probe.update(cx, |probe, cx| {
        probe.surface = surface;
        cx.notify();
    });
    cx.run_until_parked();
}
#[gpui::test]
fn actual_reorder_buttons_obey_boundaries_and_source_row_density(cx: &mut TestAppContext) {
    for scheme in [Scheme::Light, Scheme::Dark] {
        for lang in [Lang::En, Lang::Ko] {
            for (width, height, controls) in [(700.0, 32.0, 114.0), (390.0, 44.0, 148.0)] {
                let (probe, cx) = setup(cx, width, scheme, lang, Builtin::NewWorkspace);
                probe.read_with(cx, |probe, _| {
                    let seen = probe.seen.borrow();
                    assert_eq!(seen.len(), 2, "one line of main and controls");
                    let actions = seen[1];
                    assert_eq!(actions.size.width, px(controls));
                    assert_eq!(actions.size.height, px(height));
                    assert_eq!(seen[0].center().y, actions.center().y);
                });
                let before = probe.read_with(cx, |probe, cx| {
                    super::super::navigation::items(probe.shell.read(cx), Section::Header)
                });
                click_control(&probe, cx, Control::Up, width);
                probe.read_with(cx, |probe, cx| {
                    assert_eq!(
                        super::super::navigation::items(probe.shell.read(cx), Section::Header),
                        before
                    )
                });
                click_control(&probe, cx, Control::Down, width);
                probe.read_with(cx, |probe, cx| {
                    let items =
                        super::super::navigation::items(probe.shell.read(cx), Section::Header);
                    assert_eq!(items.first().unwrap().key.builtin(), Some(Builtin::History));
                    assert_eq!(
                        items.get(1).unwrap().key.builtin(),
                        Some(Builtin::NewWorkspace)
                    );
                });
            }
        }
    }
}
#[gpui::test]
fn actual_visibility_switch_hides_and_restores_the_shared_search_opener(cx: &mut TestAppContext) {
    for scheme in [Scheme::Light, Scheme::Dark] {
        for lang in [Lang::En, Lang::Ko] {
            for (width, height) in [(700.0, 28.0), (390.0, 36.0)] {
                let (probe, cx) = setup(cx, width, scheme, lang, Builtin::Search);
                let before = probe.read_with(cx, |probe, cx| {
                    super::super::navigation::items(probe.shell.read(cx), Section::Header)
                });
                click_control(&probe, cx, Control::Down, width);
                probe.read_with(cx, |probe, cx| {
                    assert_eq!(
                        super::super::navigation::items(probe.shell.read(cx), Section::Header),
                        before
                    )
                });
                click_control(&probe, cx, Control::Toggle, width);
                show(&probe, cx, Surface::Search);
                probe.read_with(cx, |probe, _| {
                    assert!(probe.seen.borrow().is_empty(), "hidden Search has no row")
                });
                show(&probe, cx, Surface::Row(Builtin::Search));
                click_control(&probe, cx, Control::Toggle, width);
                show(&probe, cx, Surface::Search);
                let bounds = probe.read_with(cx, |probe, _| {
                    let seen = probe.seen.borrow();
                    assert_eq!(seen.len(), 1);
                    assert_eq!(seen[0].size.height, px(height));
                    seen[0]
                });
                cx.simulate_click(bounds.center(), Modifiers::none());
                cx.run_until_parked();
                probe.read_with(cx, |probe, cx| {
                    assert!(
                        probe.shell.read(cx).areas.palette.is_open(),
                        "existing real opener runs"
                    )
                });
            }
        }
    }
}
#[gpui::test]
fn existing_settings_entry_registers_sidebar_page(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        shell.open_settings(
            Some(vessel_app_view::settings::Section::Sidebar),
            window,
            cx,
        );
        assert_eq!(
            shell.areas.settings.section,
            vessel_app_view::settings::Section::Sidebar
        );
    });
}
