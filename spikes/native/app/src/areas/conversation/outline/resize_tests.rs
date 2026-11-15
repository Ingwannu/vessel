//! The rail follows a real window resize across Paseo's 918, through the
//! cached timeline region the app draws it in.
//!
//! The 0.0.0.0.20 probe passed its own reading but its notification was
//! made while the window drew, which GPUI drops: the smoke kept the rail
//! after a resize from 1280 to 800, and a session opened at 1280 never
//! showed it. This draws frames the way the platform does (a resize, then
//! frames until nothing is left to draw) and reads where the rail painted.
use gpui::{AppContext as _, Render, TestAppContext, VisualTestContext, size};
use vessel_app_kit::ui::{
    Context, Entity, IntoElement, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Scheme, Tokens};
use vessel_proto::{AgentEvent, ContentBlock, EventRecord, EventSeq, SessionEvent, SessionId};

use crate::shell::{Shell, language_tests};

struct Probe {
    shell: Entity<Shell>,
}

/// Isolate timer ownership from the separate cached-region resize contract.
struct TimerProbe {
    shell: Entity<Shell>,
}

impl Render for TimerProbe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            let list = shell
                .areas
                .conversation
                .list
                .element(|_, _, _| div().into_any_element());
            div()
                .size_full()
                .relative()
                .child(list)
                .children(super::rail(shell, &Tokens::of(Scheme::Dark), window, cx))
        })
    }
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let timeline = self.shell.update(cx, |shell, cx| {
            let _started = crate::region::begin(shell, cx);
            crate::region::timeline(shell, &Tokens::of(Scheme::Dark), window, cx)
        });
        div().size_full().flex().flex_col().child(timeline)
    }
}

fn session() -> SessionId {
    "00000000-0000-4000-8000-000000000918".parse().unwrap()
}

fn prompt(seq: u64, text: &str) -> EventRecord {
    EventRecord {
        seq: EventSeq::new(seq),
        at: "2026-11-15T09:00:00Z".parse().unwrap(),
        event: SessionEvent::Agent {
            event: AgentEvent::TurnStarted {
                content: vec![ContentBlock::Text {
                    text: text.to_owned(),
                }],
            },
        },
    }
}

/// Frames until the window has nothing left to draw, as the platform
/// draws them: each frame's effects (a deferred notification) run after it.
fn settle(cx: &mut VisualTestContext) {
    for _ in 0..4 {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
    }
}

fn rail_shown(cx: &mut VisualTestContext, width: f32) -> bool {
    cx.simulate_resize(size(px(width), px(700.0)));
    settle(cx);
    cx.debug_bounds("outline-rail").is_some()
}

fn seed(shell: &mut Shell, cx: &mut Context<'_, Shell>) {
    let area = &mut shell.areas.conversation;
    area.open("machine", session());
    let records = [prompt(0, "first"), prompt(1, "second")];
    assert!(area.take(&session(), &records));
    assert_eq!(area.outline.len(), 2);
    // A3's local fallback still paints an exact target for stale-click checks.
    shell.prompt_index_refresh();
}

#[gpui::test]
fn the_rail_follows_a_resize_across_918_in_the_cached_timeline(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (_probe, cx) = cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            seed(&mut shell, cx);
            shell
        });
        Probe { shell }
    });
    // Opened wide: the list's first layout is zero wide, so the rail
    // appears only when the probe's notification survives the frame.
    assert!(rail_shown(cx, 1280.0), "opened at 1280 without the rail");
    // Narrowed past 918 in one step, as the smoke's `windowsize` does.
    assert!(!rail_shown(cx, 800.0), "the rail stayed at 800");
    assert!(rail_shown(cx, 1280.0), "the rail did not come back at 1280");
    // Close to the line on either side, the decision still follows the
    // list's own width (the probe and the rail read the same measure).
    assert!(rail_shown(cx, 1000.0));
    assert!(!rail_shown(cx, 900.0));
}

#[gpui::test]
fn a_replaced_resources_hover_timer_cannot_activate_the_current_pointer(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            seed(&mut shell, cx);
            shell
        });
        TimerProbe { shell }
    });
    assert!(rail_shown(cx, 1280.0));
    probe.update(cx, |probe, cx| {
        probe
            .shell
            .update(cx, |shell, cx| shell.outline_point(0, cx));
    });
    cx.run_until_parked();
    let half = super::OUTLINE_INTENT.checked_div(2).unwrap();
    cx.executor().advance_clock(half);
    probe.update(cx, |probe, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell.areas.conversation = super::super::ConversationArea::new(cx);
            seed(shell, cx);
        });
    });
    settle(cx);
    probe.update(cx, |probe, cx| {
        probe
            .shell
            .update(cx, |shell, cx| shell.outline_point(0, cx));
    });
    cx.run_until_parked();
    cx.executor().advance_clock(half);
    cx.run_until_parked();
    probe.update(cx, |probe, cx| {
        probe.shell.update(cx, |shell, _cx| {
            let pointer = &shell.areas.conversation.outline_pointer;
            assert!(
                pointer.settling(),
                "old timer must not consume a new ticket"
            );
            assert!(pointer.hover.attention().is_none());
            assert!(pointer.shown.is_none());
        });
    });
    cx.executor().advance_clock(half);
    cx.run_until_parked();
    probe.update(cx, |probe, cx| {
        probe.shell.update(cx, |shell, _cx| {
            let pointer = &shell.areas.conversation.outline_pointer;
            assert!(!pointer.settling());
            assert_eq!(pointer.hover.attention().unwrap().index(), 0);
            assert_eq!(pointer.shown, Some(0));
        });
    });
}
