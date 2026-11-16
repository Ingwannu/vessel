//! Actual shared-kit scene bounds protect approval metadata at phone width.
use super::*;
use crate::shell::language_tests;
use gpui::{Render, TestAppContext};
use vessel_app_kit::ui::IntoElement;
use vessel_app_kit::{Scheme, theme::spacing};
use vessel_app_view::Lang;
use vessel_proto::scripts::{ScriptEntry, ScriptKind};

struct Probe;
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let script = ScriptEntry::new(
            "preview-with-a-long-name-for-phone-layout".parse().unwrap(),
            ScriptKind::Service,
            "serve".parse().unwrap(),
            None,
            None,
        )
        .unwrap();
        div()
            .debug_selector(|| "approval-geometry-card".to_owned())
            .w_full()
            .p(px(spacing::S4))
            .child(script_list(
                Lang::En.workspace(),
                &[ScriptLine::Line {
                    change: Change::Same,
                    script,
                }],
                &Tokens::of(Scheme::Dark),
            ))
    }
}
#[gpui::test]
fn service_kind_stays_inside_the_approval_card_at_phone_width(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (_probe, cx) = cx.add_window_view(|_window, _cx| Probe);
    cx.simulate_resize(gpui::size(px(390.0), px(844.0)));
    cx.run_until_parked();
    let card = cx.debug_bounds("approval-geometry-card").unwrap();
    let kind = cx.debug_bounds("approval-script-kind").unwrap();
    assert!(kind.left() >= card.left(), "{kind:?} outside {card:?}");
    assert!(kind.right() <= card.right(), "{kind:?} outside {card:?}");
    assert!(kind.top() >= card.top());
    assert!(kind.bottom() <= card.bottom());
}
