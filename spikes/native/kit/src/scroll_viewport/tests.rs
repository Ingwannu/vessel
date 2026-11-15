//! A real bounded GPUI body, fixed siblings, wheel and keyboard focus.
//! Focus targets in this probe do not imply that every app row is accessible.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    Context, Entity, FocusHandle, Render, ScrollDelta, ScrollEvent, TestAppContext,
    VisualTestContext, Window, point, size,
};

use super::*;

const NAV: f32 = 160.0;
const FOOT: f32 = 80.0;
const HEADING: f32 = 24.0;
const PROJECT_ROW: f32 = 40.0;
const SESSION_ROW: f32 = 28.0;
const SESSIONS: u16 = 12;

struct Probe {
    scroll: ScrollState,
    projects: u16,
    nav_focus: FocusHandle,
    last_focus: FocusHandle,
    foot_focus: FocusHandle,
    frame: Rc<RefCell<Vec<Bounds<Pixels>>>>,
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let projects = div()
            .flex()
            .flex_col()
            .child(div().flex_none().h(px(HEADING)).child("Projects"))
            .children((0..self.projects).map(|index| {
                div()
                    .flex_none()
                    .h(px(PROJECT_ROW))
                    .child(format!("Project {index}"))
            }));
        let sessions = div()
            .flex()
            .flex_col()
            .child(div().flex_none().h(px(HEADING)).child("Sessions"))
            .children((0..SESSIONS.saturating_sub(1)).map(|index| {
                div()
                    .flex_none()
                    .h(px(SESSION_ROW))
                    .child(format!("Session {index}"))
            }))
            .child(self.scroll.focus_target(
                "last-item",
                &self.last_focus,
                div().h(px(SESSION_ROW)).child("Last item"),
            ));
        let frame = Rc::clone(&self.frame);
        div()
            .size_full()
            .flex()
            .flex_col()
            .on_children_prepainted(move |bounds, _, _| *frame.borrow_mut() = bounds)
            .child(
                div()
                    .id("nav")
                    .flex_none()
                    .h(px(NAV))
                    .track_focus(&self.nav_focus)
                    .child("Navigation"),
            )
            .child(
                ScrollViewport::new("body", &self.scroll)
                    .section(projects)
                    .section(sessions)
                    .element(),
            )
            .child(
                div()
                    .id("foot")
                    .flex_none()
                    .h(px(FOOT))
                    .track_focus(&self.foot_focus)
                    .child("Add machine / Settings"),
            )
    }
}

fn setup(
    cx: &mut TestAppContext,
    width: f32,
    height: f32,
    projects: u16,
) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(gpui_base::init);
    let saved = Rc::new(RefCell::new(None));
    let (_, cx) = cx.add_window_view({
        let saved = Rc::clone(&saved);
        move |window, cx| {
            let view = cx.new(|cx| Probe {
                scroll: ScrollState::new(),
                projects,
                nav_focus: cx.focus_handle().tab_stop(true),
                last_focus: cx.focus_handle().tab_stop(true),
                foot_focus: cx.focus_handle().tab_stop(true),
                frame: Rc::default(),
            });
            *saved.borrow_mut() = Some(view.clone());
            gpui_base::Root::new(view, window, cx)
        }
    });
    cx.simulate_resize(size(px(width), px(height)));
    cx.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let view = saved.borrow().as_ref().unwrap().clone();
    (view, cx)
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "finite fixture pixel bounds"
)]
fn assert_frame(view: &Entity<Probe>, cx: &VisualTestContext, height: f32, projects: u16) {
    view.read_with(cx, |probe, _| {
        let frame = probe.frame.borrow();
        assert_eq!(frame.len(), 3);
        assert_eq!(frame[0].size.height, px(NAV));
        assert_eq!(frame[2].size.height, px(FOOT));
        assert_eq!(frame[2].bottom(), px(height));
        assert_eq!(
            probe.scroll.bounds().size.height,
            px(height) - px(NAV) - px(FOOT)
        );
        assert_eq!(probe.scroll.bounds().bottom(), frame[2].top());
        let project = probe.scroll.section_bounds(SectionIndex::new(0)).unwrap();
        let sessions = probe.scroll.section_bounds(SectionIndex::new(1)).unwrap();
        assert_eq!(
            project.size.height,
            px(PROJECT_ROW.mul_add(f32::from(projects), HEADING))
        );
        assert_eq!(
            sessions.size.height,
            px(SESSION_ROW.mul_add(f32::from(SESSIONS), HEADING))
        );
        assert_eq!(project.bottom(), sessions.top());
        assert!(probe.scroll.section_bounds(SectionIndex::new(2)).is_none());
    });
}

#[gpui::test]
fn natural_sections_never_push_fixed_siblings_out_at_either_window_height(cx: &mut TestAppContext) {
    for (width, height) in [(800.0, 600.0), (1280.0, 800.0)] {
        for projects in [8, 11] {
            let (view, cx) = setup(cx, width, height, projects);
            assert_frame(&view, cx, height, projects);
            view.read_with(cx, |probe, _| assert_eq!(probe.scroll.offset().y, px(0.0)));
        }
    }
}

#[gpui::test]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "finite fixture pixel bounds"
)]
fn wheel_reaches_the_last_item_and_a_fold_clamps_the_same_offset(cx: &mut TestAppContext) {
    for (width, height) in [(800.0, 600.0), (1280.0, 800.0)] {
        let (view, cx) = setup(cx, width, height, 11);
        let viewport = view.read_with(cx, |probe, _| probe.scroll.bounds());
        cx.simulate_event(ScrollEvent {
            position: viewport.center(),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-2000.0))),
            ..ScrollEvent::default()
        });
        cx.run_until_parked();
        assert_frame(&view, cx, height, 11);
        view.read_with(cx, |probe, _| {
            let last = probe.scroll.section_bounds(SectionIndex::new(1)).unwrap();
            assert_eq!(last.bottom() + probe.scroll.offset().y, viewport.bottom());
            assert!(probe.scroll.offset().y < px(0.0));
        });
        view.update(cx, |probe, cx| {
            probe.projects = 8;
            cx.notify();
        });
        cx.run_until_parked();
        assert_frame(&view, cx, height, 8);
        view.read_with(cx, |probe, _| {
            let last = probe.scroll.section_bounds(SectionIndex::new(1)).unwrap();
            assert_eq!(last.bottom() + probe.scroll.offset().y, viewport.bottom());
        });
    }
}

#[gpui::test]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "finite fixture pixel bounds"
)]
fn tab_focus_reveals_the_offscreen_last_target_and_then_the_fixed_footer(cx: &mut TestAppContext) {
    for (width, height) in [(800.0, 600.0), (1280.0, 800.0)] {
        let (view, cx) = setup(cx, width, height, 11);
        cx.update(|window, cx| {
            let focus = view.read(cx).nav_focus.clone();
            focus.focus(window, cx);
        });
        cx.run_until_parked();
        cx.simulate_keystrokes("tab");
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.read_with(cx, |probe, _| {
                assert!(probe.last_focus.is_focused(window));
                assert!(probe.scroll.offset().y < px(0.0));
                let last = probe.scroll.section_bounds(SectionIndex::new(1)).unwrap();
                assert_eq!(
                    last.bottom() + probe.scroll.offset().y,
                    probe.scroll.bounds().bottom()
                );
            });
        });
        assert_frame(&view, cx, height, 11);
        let viewport = view.read_with(cx, |probe, _| probe.scroll.bounds());
        cx.simulate_event(ScrollEvent {
            position: viewport.center(),
            delta: ScrollDelta::Pixels(point(px(0.0), px(2000.0))),
            ..ScrollEvent::default()
        });
        cx.run_until_parked();
        view.read_with(cx, |probe, _| assert_eq!(probe.scroll.offset().y, px(0.0)));
        cx.simulate_keystrokes("tab");
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.read_with(cx, |probe, _| {
                assert!(probe.foot_focus.is_focused(window));
            });
        });
        assert_frame(&view, cx, height, 11);
        cx.simulate_keystrokes("shift-tab");
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.read_with(cx, |probe, _| {
                assert!(probe.last_focus.is_focused(window));
                assert!(probe.scroll.offset().y < px(0.0));
            });
        });
    }
}

#[test]
fn oversized_focus_targets_align_at_the_start_without_oscillating() {
    let viewport = Bounds::new(point(px(0.0), px(100.0)), size(px(200.0), px(80.0)));
    let target = Bounds::new(point(px(0.0), px(40.0)), size(px(200.0), px(160.0)));
    let offset = point(px(0.0), px(-100.0));
    let maximum = point(px(0.0), px(300.0));
    let next = reveal_offset(viewport, target, offset, maximum);
    assert_eq!(next.y, px(-40.0));
    let moved = Bounds::new(point(px(0.0), px(100.0)), target.size);
    assert_eq!(reveal_offset(viewport, moved, next, maximum), next);
}
