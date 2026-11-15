use super::*;

#[test]
fn growth_waits_for_a_frame_and_stops_scheduling_after_completion() {
    let mut kept = Kept::begin("a", 0);
    assert_eq!(
        kept.follow("a", 0, TextPhase::Streaming, Duration::ZERO),
        (1, false)
    );
    let full = format!("a{}", "b".repeat(300));
    assert_eq!(
        kept.follow(&full, 1, TextPhase::Streaming, Duration::ZERO),
        (1, true)
    );
    let (end, pending) = kept.follow(&full, 1, TextPhase::Streaming, Duration::from_millis(17));
    assert!(end > 1 && end < full.len());
    assert!(pending);
    assert_eq!(
        kept.follow(&full, 1, TextPhase::Complete, Duration::from_millis(18)),
        (full.len(), false)
    );
    assert_eq!(
        kept.follow(&full, 1, TextPhase::Complete, Duration::from_secs(5)),
        (full.len(), false)
    );
    assert_eq!(full.len(), 301);
}

use gpui::{Context, Render, TestAppContext};
use std::cell::RefCell;
use std::rc::Rc;
use vessel_app_kit::ui::{IntoElement as _, ParentElement as _, div};

struct Probe {
    text: String,
    revision: u32,
    phase: TextPhase,
    shown: Rc<RefCell<String>>,
}
impl Render for Probe {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        let end = visible_end(
            "pace-probe".into(),
            &self.text,
            self.revision,
            self.phase,
            window,
            cx,
        );
        let prefix = self.text.get(..end).unwrap();
        *self.shown.borrow_mut() = prefix.to_owned();
        div().child(prefix.to_owned()).into_any_element()
    }
}

fn draw(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
}

#[gpui::test]
fn mounted_rows_request_frames_only_for_backlog_and_motion_policy_completes(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    cx.update(|cx| cx.set_reduce_motion(false));
    let shown = Rc::new(RefCell::new(String::new()));
    let observed = Rc::clone(&shown);
    let (view, cx) = cx.add_window_view(move |_, _| Probe {
        text: "a".to_owned(),
        revision: 0,
        phase: TextPhase::Streaming,
        shown,
    });
    draw(cx);
    assert_eq!(observed.borrow().as_str(), "a");
    let _startup = cx.update(|window, cx| window.simulate_next_frame(cx));
    view.update_in(cx, |probe, _, cx| {
        probe.text.push_str(&"b".repeat(300));
        probe.revision += 1;
        cx.notify();
    });
    draw(cx);
    assert_eq!(
        observed.borrow().as_str(),
        "a",
        "arrival changes only the target"
    );
    let scheduled = cx.update(|window, cx| window.simulate_next_frame(cx));
    assert!(scheduled > 0);
    draw(cx);
    assert!(observed.borrow().len() > 1 && observed.borrow().len() < 301);
    let _old = cx.update(|window, cx| window.simulate_next_frame(cx));
    view.update_in(cx, |probe, _, cx| {
        probe.phase = TextPhase::Complete;
        cx.notify();
    });
    draw(cx);
    assert_eq!(observed.borrow().len(), 301);
    assert_eq!(cx.update(|window, cx| window.simulate_next_frame(cx)), 0);
    view.update_in(cx, |probe, _, cx| {
        probe.text.push_str("reduced");
        probe.revision += 1;
        probe.phase = TextPhase::Streaming;
        cx.set_reduce_motion(true);
        cx.notify();
    });
    draw(cx);
    assert_eq!(observed.borrow().len(), 308);
    assert_eq!(cx.update(|window, cx| window.simulate_next_frame(cx)), 0);
}
