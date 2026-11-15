//! The published owner state resets its one path across actual opening cycles.
use super::{PageDepth, PageId};
use crate::menu::MenuState;
use gpui::prelude::*;
use gpui::{Context, Render, TestAppContext, Window, div, point, px};

struct Probe(MenuState);
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        div().track_focus(&self.0.focus)
    }
}
#[gpui::test]
fn shared_menu_owner_reopens_at_root_and_uses_one_path_for_back_and_flyouts(
    cx: &mut TestAppContext,
) {
    let (probe, cx) = cx.add_window_view(|_, cx| Probe(MenuState::new(cx)));
    probe.update_in(cx, |probe, window, cx| {
        let menu = &mut probe.0;
        assert!(!menu.open_sub_page(PageId::new("hidden"), PageDepth::ROOT));
        menu.open(point(px(10.0), px(20.0)), window, cx);
        assert!(menu.open_pages().is_empty());
        assert!(menu.open_sub_page(PageId::new("show"), PageDepth::ROOT));
        assert!(menu.open_sub_page(PageId::new("colors"), PageDepth::at(1)));
        assert_eq!(
            menu.open_pages(),
            [PageId::new("show"), PageId::new("colors")]
        );
        assert_eq!(menu.current_page(), Some(PageId::new("colors")));
        assert!(menu.back_page());
        assert_eq!(menu.current_page(), Some(PageId::new("show")));
        assert!(menu.open_sub_page(PageId::new("colors"), PageDepth::at(1)));
        assert!(menu.prune_pages(|id| id == &PageId::new("show")));
        assert_eq!(menu.current_page(), Some(PageId::new("show")));
        menu.close();
        assert!(menu.open_pages().is_empty());
        assert!(menu.current_page().is_none());
        assert!(!menu.back_page());
        menu.open_end(
            gpui::Bounds::new(point(px(20.0), px(30.0)), gpui::size(px(40.0), px(20.0))),
            window,
            cx,
        );
        assert!(menu.open_pages().is_empty());
        assert!(menu.current_page().is_none());
        assert!(menu.open_sub_page(PageId::new("show"), PageDepth::ROOT));
        assert!(menu.close_sub_page(PageDepth::ROOT));
        assert!(menu.is_open());
        assert!(!menu.back_page());
    });
}
