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

#[gpui::test]
fn the_rail_follows_a_resize_across_918_in_the_cached_timeline(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (_probe, cx) = cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            let area = &mut shell.areas.conversation;
            area.open("machine", session());
            let records = [prompt(0, "first"), prompt(1, "second")];
            assert!(area.take(&session(), &records));
            assert_eq!(area.outline.len(), 2);
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
