//! The one existing list handles native wheel input with/without observation.
use super::*;
use gpui::prelude::*;
use gpui::{Context, Render, TestAppContext, div, point};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy)]
enum Listening {
    Absent,
    Present,
}
struct Probe {
    list: VirtualList,
    calls: Rc<Cell<usize>>,
    observed: Option<(usize, Pixels)>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        self.list.element(|index, _, _| {
            div()
                .w_full()
                .h(px(20.))
                .child(index.to_string())
                .into_any_element()
        })
    }
}
#[gpui::test]
fn existing_wheel_motion_is_unchanged_and_notification_never_reborrows_the_list(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap());
    let mut tops = Vec::new();
    for listening in [Listening::Absent, Listening::Present] {
        let (probe, cx) = cx.add_window_view(|_, _| {
            let list = VirtualList::new(false);
            list.reset(200);
            let calls = Rc::new(Cell::new(0_usize));
            if let Listening::Present = listening {
                let seen = Rc::clone(&calls);
                list.on_scroll(move |_, _| seen.set(seen.get().saturating_add(1)));
            }
            Probe {
                list,
                calls,
                observed: None,
            }
        });
        cx.run_until_parked();
        let before = probe.read_with(cx, |probe, _| {
            assert_eq!(probe.calls.get(), 0);
            probe.list.top()
        });
        cx.simulate_event(gpui::ScrollEvent {
            position: point(px(100.), px(100.)),
            delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-80.))),
            touch_phase: gpui::TouchPhase::Moved,
            ..Default::default()
        });
        cx.run_until_parked();
        let after = probe.read_with(cx, |probe, _| {
            // Query after input dispatch, never inside the borrowed callback.
            assert_ne!(probe.list.top(), before);
            match listening {
                Listening::Absent => assert_eq!(probe.calls.get(), 0),
                Listening::Present => assert_eq!(probe.calls.get(), 1),
            }
            probe.list.top()
        });
        tops.push(after);
    }
    assert_eq!(tops.len(), 2);
    assert_eq!(tops.first(), tops.last());
}

#[gpui::test]
fn deferred_owner_reads_the_actual_new_list_position_once(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap());
    let (probe, cx) = cx.add_window_view(|_, cx| {
        let list = VirtualList::new(false);
        list.reset(200);
        let owner = cx.entity().downgrade();
        list.on_scroll(move |window, cx| {
            // The owner and its ListState are read only after event dispatch
            // releases the backend's active list borrow.
            let owner = owner.clone();
            window.defer(cx, move |_, cx| {
                let _updated = owner.update(cx, |probe: &mut Probe, cx| {
                    probe.calls.set(probe.calls.get().saturating_add(1));
                    probe.observed = Some(probe.list.top());
                    cx.notify();
                });
            });
        });
        Probe {
            list,
            calls: Rc::new(Cell::new(0)),
            observed: None,
        }
    });
    let before = probe.read_with(cx, |probe, _| {
        assert_eq!(probe.calls.get(), 0);
        assert_eq!(probe.observed, None);
        probe.list.top()
    });
    cx.simulate_event(gpui::ScrollEvent {
        position: point(px(100.), px(100.)),
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-80.))),
        touch_phase: gpui::TouchPhase::Moved,
        ..Default::default()
    });
    cx.run_until_parked();
    probe.read_with(cx, |probe, _| {
        assert_ne!(probe.list.top(), before);
        assert_eq!(probe.calls.get(), 1);
        assert_eq!(probe.observed, Some(probe.list.top()));
    });
}
