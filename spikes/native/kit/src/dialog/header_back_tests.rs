//! Paseo's sheet-header back arrow: before the title on the header's rails,
//! on the compact sheet and the desktop dialog, and the owner's handler.
use super::*;
use crate::{Scheme, SchemeChoice};
use gpui::{Modifiers, Render, TestAppContext, VisualTestContext};

struct Probe {
    state: DialogState,
    backs: u32,
    closes: u32,
    narrow: bool,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let content = DialogContent {
            title: "Edit label".into(),
            body: div().h(px(120.0)).into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Normal,
        }
        .with_header_back(HeaderBack::new(cx.listener(
            |probe: &mut Self, _event, _window, _cx| {
                probe.backs = probe.backs.saturating_add(1);
            },
        )));
        let modal = dialog(
            &self.state,
            content,
            &tokens,
            window,
            self.narrow,
            |probe: &mut Self, _, _| probe.closes = probe.closes.saturating_add(1),
            cx,
        );
        div().size_full().child(modal)
    }
}

fn init(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
}

fn open(cx: &mut TestAppContext, narrow: bool) -> (gpui::Entity<Probe>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(move |_window, cx| Probe {
        state: DialogState::new(cx),
        backs: 0,
        closes: 0,
        narrow,
    });
    cx.run_until_parked();
    (view, cx)
}

#[gpui::test]
fn the_back_arrow_leads_the_title_on_the_header_rails(cx: &mut TestAppContext) {
    init(cx);
    for narrow in [false, true] {
        let (_view, cx) = open(cx, narrow);
        let header = cx.debug_bounds("dialog-header").unwrap();
        let back = cx.debug_bounds("dialog-header-back").unwrap();
        let title = cx.debug_bounds("dialog-title").unwrap();
        // The 18-point glyph box at the 24 inset, then the row's gap of 8.
        assert_eq!(back.size.width, px(header_back::SIZE));
        assert_eq!(back.size.height, px(header_back::SIZE));
        assert_eq!(apart(header.left(), back.left()), 24.0_f32.to_bits());
        assert_eq!(apart(back.right(), title.left()), 8.0_f32.to_bits());
        assert_eq!(back.center().y, header.center().y);
        // The close button keeps its place at the end of the row.
        let close = cx.debug_bounds("dialog-close").unwrap();
        assert_eq!(apart(close.right(), header.right()), 24.0_f32.to_bits());
    }
}

#[gpui::test]
fn the_back_arrow_runs_the_owners_handler_and_does_not_close(cx: &mut TestAppContext) {
    init(cx);
    let (view, cx) = open(cx, false);
    let back = cx.debug_bounds("dialog-header-back").unwrap();
    cx.simulate_click(back.center(), Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |probe, _cx| {
        assert_eq!(probe.backs, 1);
        assert_eq!(probe.closes, 0);
    });
}

/// The distance from `from` to `to` (`mul_add`: no arithmetic operator).
fn apart(from: gpui::Pixels, to: gpui::Pixels) -> u32 {
    f32::from(from).mul_add(-1.0, f32::from(to)).to_bits()
}
