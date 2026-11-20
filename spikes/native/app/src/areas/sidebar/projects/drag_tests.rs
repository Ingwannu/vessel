//! Dragging a project's header (TRACKER 811) in the real sidebar: a press
//! that moves less than 6 folds, one that moves more lifts the block; the
//! order follows the block under the pointer and a release writes it;
//! Escape restores it; the hover card closes as a drag starts; held near
//! the list's bottom edge the list scrolls. Then dnd-kit's geometry alone.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "finite measured fixture pixels and ticks"
)]
use std::time::Duration;

use gpui::{Bounds, Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext};
use vessel_app_kit::ui::{point, px, size};
use vessel_app_view::prefs::ProjectKey;

use super::{TICK, block_place, closest, edge_scroll, moved};
use crate::areas::sidebar::projects_tests::{project_id, setup};
use crate::shell::Shell;

fn key(index: u16) -> ProjectKey {
    ProjectKey::new("fixture", project_id(index))
}

/// Where `index`'s block was last drawn.
fn block(cx: &mut VisualTestContext, shell: &gpui::Entity<Shell>, index: u16) -> Bounds<Pixels> {
    cx.run_until_parked();
    shell
        .read_with(cx, |shell, _| {
            shell.areas.sidebar.place(&block_place(&key(index))).get()
        })
        .unwrap()
}

/// Inside a block's header: 10 below its top, 40 from its left.
fn on_header(block: Bounds<Pixels>) -> Point<Pixels> {
    point(block.left() + px(40.0), block.top() + px(10.0))
}

fn press(cx: &mut VisualTestContext, at: Point<Pixels>) {
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
}

fn drag_to(cx: &mut VisualTestContext, at: Point<Pixels>) {
    cx.simulate_mouse_move(at, Some(MouseButton::Left), Modifiers::none());
    cx.run_until_parked();
}

fn release(cx: &mut VisualTestContext, at: Point<Pixels>) {
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
}

/// The order the drag would leave now, if one is under way.
fn dragged_order(cx: &mut VisualTestContext, shell: &gpui::Entity<Shell>) -> Option<Vec<ProjectKey>> {
    shell.read_with(cx, |shell, _| {
        shell
            .areas
            .sidebar
            .projects
            .dragging
            .as_ref()
            .map(|drag| drag.order.keys().to_vec())
    })
}

fn stored(cx: &mut VisualTestContext, shell: &gpui::Entity<Shell>) -> Vec<ProjectKey> {
    shell.read_with(cx, |shell, _| {
        shell.prefs.settings.project_order.keys().to_vec()
    })
}

#[gpui::test]
fn a_press_that_moves_less_than_six_folds_and_drags_nothing(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx, 2);
    // By name: docs (1), notes (2), shop (0).
    let docs = on_header(block(cx, &shell, 1));
    press(cx, docs);
    drag_to(cx, point(docs.x, docs.y + px(5.0)));
    assert_eq!(dragged_order(cx, &shell), None);
    release(cx, point(docs.x, docs.y + px(5.0)));
    shell.read_with(cx, |shell, _| {
        assert!(shell.prefs.settings.folded_projects.contains("fixture", project_id(1)));
        assert!(shell.prefs.settings.project_order.is_empty());
    });
}

#[gpui::test]
fn a_dragged_block_goes_where_it_is_held_and_the_release_keeps_the_order(
    cx: &mut TestAppContext,
) {
    let (shell, cx) = setup(cx, 2);
    let docs = on_header(block(cx, &shell, 1));
    let shop = block(cx, &shell, 0);
    press(cx, docs);
    drag_to(cx, point(docs.x, docs.y + px(10.0)));
    // Lifted, nothing moved yet.
    assert_eq!(dragged_order(cx, &shell), Some(vec![key(1), key(2), key(0)]));
    // Held over shop's centre: docs goes after it, the rest close up.
    drag_to(cx, point(docs.x, shop.center().y));
    assert_eq!(dragged_order(cx, &shell), Some(vec![key(2), key(0), key(1)]));
    release(cx, point(docs.x, shop.center().y));
    assert_eq!(dragged_order(cx, &shell), None);
    assert_eq!(stored(cx, &shell), vec![key(2), key(0), key(1)]);
    // The release did not fold docs, and the tree draws the new order.
    shell.read_with(cx, |shell, _| {
        assert!(!shell.prefs.settings.folded_projects.contains("fixture", project_id(1)));
    });
    let (notes, shop, docs) = (block(cx, &shell, 2), block(cx, &shell, 0), block(cx, &shell, 1));
    assert!(notes.top() < shop.top() && shop.top() < docs.top());
}

#[gpui::test]
fn escape_puts_the_order_back_and_ends_the_drag(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx, 2);
    let docs = on_header(block(cx, &shell, 1));
    let shop = block(cx, &shell, 0);
    press(cx, docs);
    drag_to(cx, point(docs.x, docs.y + px(10.0)));
    drag_to(cx, point(docs.x, shop.center().y));
    assert_eq!(dragged_order(cx, &shell), Some(vec![key(2), key(0), key(1)]));
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(dragged_order(cx, &shell), None);
    assert!(!cx.update(|_, cx| cx.has_active_drag()));
    release(cx, point(docs.x, shop.center().y));
    assert!(stored(cx, &shell).is_empty());
    let (docs, notes) = (block(cx, &shell, 1), block(cx, &shell, 2));
    assert!(docs.top() < notes.top());
}

#[gpui::test]
fn the_hover_card_closes_as_a_drag_starts(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx, 2);
    // shop's first session row, under shop's header.
    let shop = block(cx, &shell, 0);
    cx.simulate_mouse_move(
        point(shop.left() + px(60.0), shop.top() + px(50.0)),
        None,
        Modifiers::none(),
    );
    cx.run_until_parked();
    assert!(shell.read_with(cx, |shell, _| shell.areas.sidebar.cards.shown().is_some()));
    let docs = on_header(block(cx, &shell, 1));
    press(cx, docs);
    drag_to(cx, point(docs.x, docs.y + px(10.0)));
    assert!(dragged_order(cx, &shell).is_some());
    assert!(shell.read_with(cx, |shell, _| shell.areas.sidebar.cards.shown().is_none()));
    release(cx, point(docs.x, docs.y + px(10.0)));
}

#[gpui::test]
fn held_near_the_bottom_edge_the_list_scrolls(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx, 12);
    cx.simulate_resize(size(px(1280.0), px(520.0)));
    cx.run_until_parked();
    let viewport = shell.read_with(cx, |shell, _| shell.areas.sidebar.layout.scroll.bounds());
    let docs = on_header(block(cx, &shell, 1));
    press(cx, docs);
    drag_to(cx, point(docs.x, docs.y + px(10.0)));
    drag_to(cx, point(docs.x, viewport.bottom() - px(2.0)));
    let before = shell.read_with(cx, |shell, _| shell.areas.sidebar.layout.scroll.offset().y);
    cx.executor().advance_clock(TICK * 10);
    cx.run_until_parked();
    let after = shell.read_with(cx, |shell, _| shell.areas.sidebar.layout.scroll.offset().y);
    assert!(after < before, "{before:?} → {after:?}");
    release(cx, point(docs.x, viewport.bottom() - px(2.0)));
    // The drag ended: the ticks stop.
    cx.executor().advance_clock(Duration::from_millis(200));
    cx.run_until_parked();
    let rested = shell.read_with(cx, |shell, _| shell.areas.sidebar.layout.scroll.offset().y);
    cx.executor().advance_clock(Duration::from_millis(200));
    cx.run_until_parked();
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.areas.sidebar.layout.scroll.offset().y),
        rested
    );
}

#[test]
fn the_nearest_centre_is_the_one_held_over() {
    let centres = [px(20.0), px(60.0), px(140.0)];
    assert_eq!(closest(&centres, px(0.0)), Some(0));
    assert_eq!(closest(&centres, px(79.0)), Some(1));
    assert_eq!(closest(&centres, px(101.0)), Some(2));
    assert_eq!(closest(&[], px(10.0)), None);
}

#[test]
fn a_move_takes_one_key_out_and_puts_it_back_where_it_lands() {
    let keys = [key(0), key(1), key(2)];
    assert_eq!(moved(&keys, 0, 2), [key(1), key(2), key(0)]);
    assert_eq!(moved(&keys, 2, 0), [key(2), key(0), key(1)]);
    assert_eq!(moved(&keys, 1, 1), keys);
    assert_eq!(moved(&keys, 3, 0), keys);
}

#[test]
fn the_edges_scroll_faster_the_deeper_the_pointer_is() {
    // A list 100 high from 100: zones 20 high at either edge.
    let list = Bounds::new(point(px(0.0), px(100.0)), size(px(200.0), px(100.0)));
    assert_eq!(edge_scroll(list, px(150.0)), px(0.0));
    assert_eq!(edge_scroll(list, px(120.0)), px(0.0));
    assert_eq!(edge_scroll(list, px(110.0)), px(-16.0));
    assert_eq!(edge_scroll(list, px(100.0)), px(-32.0));
    assert_eq!(edge_scroll(list, px(190.0)), px(16.0));
    assert_eq!(edge_scroll(list, px(200.0)), px(32.0));
}
