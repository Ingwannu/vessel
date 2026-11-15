use super::{HoverChange, HoverTicket, OutlineHover, OutlinePoint, OutlineSlot};

fn slot(index: usize) -> OutlineSlot {
    OutlineSlot::new(index, 3).unwrap()
}
fn point(x: f32, y: f32) -> OutlinePoint {
    OutlinePoint::new(x, y).unwrap()
}
fn scheduled(change: HoverChange) -> HoverTicket {
    match change {
        HoverChange::Schedule(ticket) => ticket,
        other => panic!("expected timer: {other:?}"),
    }
}

#[test]
fn vertical_motion_keeps_the_original_deadline_and_activates_the_latest_slot() {
    let mut hover = OutlineHover::default();
    hover.enter(point(0.0, 0.0));
    let first = scheduled(hover.point_at(slot(0)));
    assert_eq!(hover.moved(point(2.0, 8.0)), HoverChange::None);
    assert_eq!(hover.point_at(slot(1)), HoverChange::None);
    assert_eq!(hover.attention(), None);
    assert_eq!(hover.activate(first), HoverChange::Changed);
    assert_eq!(hover.attention(), Some(slot(1)));
    assert_eq!(hover.point_at(slot(2)), HoverChange::Changed);
    assert_eq!(hover.attention(), Some(slot(2)));
    assert_eq!(hover.point_at(slot(2)), HoverChange::None);
    assert_eq!(hover.moved(point(100.0, 8.0)), HoverChange::None);
}

#[test]
fn horizontal_transit_accumulates_then_restarts_and_stale_timers_cannot_activate() {
    let mut hover = OutlineHover::default();
    hover.enter(point(0.0, 0.0));
    let first = scheduled(hover.point_at(slot(0)));
    assert_eq!(hover.moved(point(3.0, 0.0)), HoverChange::None);
    let second = scheduled(hover.moved(point(4.0, 0.0)));
    assert_eq!(hover.activate(first), HoverChange::None);
    assert_eq!(hover.point_at(slot(1)), HoverChange::None);
    assert_eq!(hover.activate(second), HoverChange::Changed);
    assert_eq!(hover.attention(), Some(slot(1)));
    assert_eq!(hover.activate(second), HoverChange::None);
}

#[test]
fn leaving_cancels_the_timer_and_each_new_entry_waits_again() {
    let mut hover = OutlineHover::default();
    let first = scheduled(hover.point_at(slot(0)));
    assert_eq!(hover.leave(), HoverChange::None);
    assert_eq!(hover.activate(first), HoverChange::None);
    let second = scheduled(hover.point_at(slot(1)));
    assert_eq!(hover.activate(second), HoverChange::Changed);
    assert_eq!(hover.leave(), HoverChange::Changed);
    assert_eq!(hover.attention(), None);
    assert_eq!(hover.activate(second), HoverChange::None);
    assert!(OutlinePoint::new(f32::NAN, 0.0).is_none());
    assert!(OutlinePoint::new(0.0, f32::INFINITY).is_none());
    assert!(OutlineSlot::new(3, 3).is_none());
    assert!(OutlineSlot::new(0, 0).is_none());
}
