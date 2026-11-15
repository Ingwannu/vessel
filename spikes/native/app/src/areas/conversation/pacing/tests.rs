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
    kept.wake = FrameWake::Ready;
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
    scope: ElementId,
    text: String,
    revision: u32,
    phase: TextPhase,
    shown: Rc<RefCell<Vec<String>>>,
    now: Duration,
}
impl Render for Probe {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        let now = self.now;
        let end = window.with_id(self.scope.clone(), |window| {
            visible_end_with_clock(
                "pace-probe".into(),
                &self.text,
                self.revision,
                self.phase,
                |_| now,
                window,
                cx,
            )
        });
        let prefix = self.text.get(..end).unwrap();
        self.shown.borrow_mut().push(prefix.to_owned());
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
    let shown = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&shown);
    let (view, cx) = cx.add_window_view(move |_, _| Probe {
        scope: "source-a".into(),
        text: "a".to_owned(),
        revision: 0,
        phase: TextPhase::Streaming,
        shown,
        now: Duration::ZERO,
    });
    draw(cx);
    assert_eq!(observed.borrow().last().unwrap(), "a");
    let _startup = cx.update(|window, cx| window.simulate_next_frame(cx));
    let before_growth = observed.borrow().len();
    view.update_in(cx, |probe, _, cx| {
        probe.text.push_str(&"b".repeat(300));
        probe.revision += 1;
        cx.notify();
    });
    draw(cx);
    assert_eq!(
        observed.borrow().get(before_growth).unwrap(),
        "a",
        "first arrival render changes only the target"
    );
    let scheduled = cx.update(|window, cx| window.simulate_next_frame(cx));
    assert_eq!(
        scheduled, 1,
        "one outstanding callback across repeated draws"
    );
    draw(cx);
    let shown = observed.borrow().last().unwrap().len();
    assert!(shown > 1 && shown < 301);
    let _old = cx.update(|window, cx| window.simulate_next_frame(cx));
    view.update_in(cx, |probe, _, cx| {
        probe.phase = TextPhase::Complete;
        cx.notify();
    });
    draw(cx);
    assert_eq!(observed.borrow().last().unwrap().len(), 301);
    let paints_before_drain = observed.borrow().len();
    let completed = cx.update(|window, cx| window.simulate_next_frame(cx));
    assert!(completed <= 1);
    assert_eq!(
        observed.borrow().len(),
        paints_before_drain,
        "settled callback cannot wake the row"
    );
    assert_eq!(cx.update(|window, cx| window.simulate_next_frame(cx)), 0);
    view.update_in(cx, |probe, _, cx| {
        probe.text.push_str("reduced");
        probe.revision += 1;
        probe.phase = TextPhase::Streaming;
        cx.set_reduce_motion(true);
        cx.notify();
    });
    draw(cx);
    assert_eq!(observed.borrow().last().unwrap().len(), 308);
    assert_eq!(cx.update(|window, cx| window.simulate_next_frame(cx)), 0);
}

#[gpui::test]
fn equal_entry_numbers_and_revisions_in_a_new_session_do_not_reuse_a_paced_prefix(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    cx.update(|cx| cx.set_reduce_motion(false));
    let shown = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&shown);
    let source_a = Shown {
        machine: "same-route".to_owned(),
        session: vessel_proto::SessionId::random(),
    };
    let source_b = Shown {
        machine: source_a.machine.clone(),
        session: vessel_proto::SessionId::random(),
    };
    assert_ne!(scope(&source_a), scope(&source_b));
    let (view, cx) = cx.add_window_view(move |_, _| Probe {
        scope: scope(&source_a),
        text: "a".to_owned(),
        revision: 0,
        phase: TextPhase::Streaming,
        shown,
        now: Duration::ZERO,
    });
    draw(cx);
    let _startup = cx.update(|window, cx| window.simulate_next_frame(cx));
    view.update_in(cx, |probe, _, cx| {
        probe.text.push_str(&"b".repeat(300));
        probe.revision = 1;
        cx.notify();
    });
    draw(cx);
    assert_eq!(observed.borrow().last().unwrap(), "a");
    let next_text = "새 세션".repeat(70);
    view.update_in(cx, |probe, _, cx| {
        probe.scope = scope(&source_b);
        probe.text.clone_from(&next_text);
        // A different source may reuse this revision and the same row number.
        probe.revision = 1;
        cx.notify();
    });
    draw(cx);
    assert_eq!(observed.borrow().last().unwrap(), &next_text);
    let draws = observed.borrow().len();
    let callbacks = cx.update(|window, cx| window.simulate_next_frame(cx));
    assert!(
        callbacks <= 1,
        "only the old source's bounded callback may drain"
    );
    assert_eq!(
        observed.borrow().len(),
        draws,
        "old weak callback cannot wake the new row"
    );
    assert_eq!(cx.update(|window, cx| window.simulate_next_frame(cx)), 0);
}

#[gpui::test]
fn readonly_history_with_an_open_turn_never_paces_or_hides_a_trailing_fragment(
    cx: &mut TestAppContext,
) {
    use vessel_proto::{AgentEvent, ContentBlock, EventRecord, EventSeq, SessionEvent};
    cx.update(|cx| {
        let mut area = ConversationArea::new(cx);
        let records = [
            (
                1,
                AgentEvent::TurnStarted {
                    content: Vec::new(),
                },
            ),
            (
                2,
                AgentEvent::MessageChunk {
                    content: ContentBlock::Text {
                        text: "flag🇺".to_owned(),
                    },
                },
            ),
        ]
        .map(|(seq, event)| EventRecord {
            seq: EventSeq::new(seq),
            at: chrono::DateTime::UNIX_EPOCH,
            event: SessionEvent::Agent { event },
        });
        assert!(area.fold_records(&records));
        let entry = area.timeline.entries().last().unwrap();
        assert!(
            area.timeline.in_turn(),
            "snapshot may omit its later closing record"
        );
        assert_eq!(phase(&area, entry), TextPhase::Complete);
        let reveal = TextReveal::begin("flag🇺");
        assert_eq!(
            reveal.visible_end("flag🇺", phase(&area, entry)),
            "flag🇺".len()
        );
    });
}
