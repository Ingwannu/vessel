//! Paseo's sheet-header search row: its rails, the shared engine's input, and
//! the same row on the compact sheet.
use super::*;
use crate::{Scheme, SchemeChoice, SearchField};
use gpui::{Modifiers, Render, TestAppContext, VisualTestContext};

struct Probe {
    state: DialogState,
    search: SearchField,
    query: String,
    narrow: bool,
}

impl Probe {
    fn changed(&mut self, text: String, _cx: &mut Context<'_, Self>) {
        self.query = text;
    }
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let content = DialogContent {
            title: "Labels".into(),
            body: div().h(px(120.0)).into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Normal,
        }
        .with_header_search(self.search.header_search(&tokens, cx));
        let modal = dialog(
            &self.state,
            content,
            &tokens,
            window,
            self.narrow,
            |_: &mut Self, _, _| {},
            cx,
        );
        div().size_full().child(modal)
    }
}

fn init(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
}

fn open(cx: &mut TestAppContext, narrow: bool) -> (gpui::Entity<Probe>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(move |window, cx| Probe {
        state: DialogState::new(cx),
        search: SearchField::new("Search labels", Probe::changed, window, cx),
        query: String::new(),
        narrow,
    });
    cx.run_until_parked();
    (view, cx)
}

#[gpui::test]
fn the_search_row_sits_under_the_title_on_paseos_rails(cx: &mut TestAppContext) {
    init(cx);
    for narrow in [false, true] {
        let (_view, cx) = open(cx, narrow);
        let title = cx.debug_bounds("dialog-header").unwrap();
        let row = cx.debug_bounds("dialog-header-search").unwrap();
        let input = cx.debug_bounds("dialog-header-search-input").unwrap();
        // Directly under the title row, across the same width.
        assert_eq!(row.top(), title.bottom());
        assert_eq!(row.size.width, title.size.width);
        // 24 inset, the 16-point mark, a gap of 8, then the input to the inset.
        assert_eq!(apart(row.left(), input.left()), 48.0_f32.to_bits());
        assert_eq!(apart(input.right(), row.right()), 24.0_f32.to_bits());
        // A 20-point line with 8 above and below, then 12 under the row.
        assert_eq!(input.size.height, px(36.0));
        assert_eq!(row.size.height, px(48.0));
    }
}

#[gpui::test]
fn typing_in_the_header_search_reaches_the_owner_through_the_engine(cx: &mut TestAppContext) {
    init(cx);
    let (view, cx) = open(cx, false);
    let input = cx.debug_bounds("dialog-header-search-input").unwrap();
    cx.simulate_click(input.center(), Modifiers::none());
    cx.simulate_input("bug");
    cx.run_until_parked();
    view.read_with(cx, |probe, cx| {
        assert_eq!(probe.query, "bug");
        assert_eq!(probe.search.text(cx), "bug");
    });
}

/// The distance from `from` to `to` (`mul_add`: no arithmetic operator).
fn apart(from: gpui::Pixels, to: gpui::Pixels) -> u32 {
    f32::from(from).mul_add(-1.0, f32::from(to)).to_bits()
}
