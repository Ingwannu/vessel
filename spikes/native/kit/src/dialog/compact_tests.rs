//! Compact opt-in must retain the desktop profile and existing shared defaults.

use super::{BodySpacing, CompactLayout, DesktopHeight, DialogLayout, DialogSize};
use crate::sheet::snap::{Percentage, SnapPoints};

#[test]
fn normal_and_wide_accept_source_adaptive_stops_without_copying_desktop_geometry() {
    let points = SnapPoints::new(65, 90).unwrap();
    let cap = Percentage::new(85).unwrap();
    for (original, width) in [(DialogSize::Normal, 520), (DialogSize::Wide, 640)] {
        let size = original.with_compact(CompactLayout::Draggable(points));
        assert!(matches!(size, DialogSize::Configured(_)));
        let layout = size.layout();
        assert_eq!(layout.width().get(), width);
        assert_eq!(layout.height(), DesktopHeight::AtMost(cap));
        assert_eq!(layout.body_spacing(), BodySpacing::Inset);
        assert_eq!(layout.compact(), CompactLayout::Draggable(points));
        assert_eq!(points.compact().value(), 65);
        assert_eq!(points.expanded().value(), 90);
    }
}

#[test]
fn configured_exact_height_and_caller_spacing_survive_compact_replacement() {
    let cap = Percentage::new(85).unwrap();
    let prior = DialogLayout::new(
        620,
        DesktopHeight::Exact(cap),
        CompactLayout::Fixed(Percentage::FULL),
    )
    .unwrap()
    .with_body_spacing(BodySpacing::Caller);
    let points = SnapPoints::new(65, 90).unwrap();
    let first = DialogSize::Configured(prior).with_compact(CompactLayout::Draggable(points));
    let next = first.with_compact(CompactLayout::Fixed(cap)).layout();
    assert_eq!(first.layout().width().get(), 620);
    assert_eq!(first.layout().height(), DesktopHeight::Exact(cap));
    assert_eq!(first.layout().body_spacing(), BodySpacing::Caller);
    assert_eq!(first.layout().compact(), CompactLayout::Draggable(points));
    assert_eq!(next.width(), prior.width());
    assert_eq!(next.height(), prior.height());
    assert_eq!(next.body_spacing(), BodySpacing::Caller);
    assert_eq!(next.compact(), CompactLayout::Fixed(cap));
}

#[test]
fn compact_opt_in_leaves_unconfigured_dialog_and_snap_defaults_unchanged() {
    let points = SnapPoints::new(65, 90).unwrap();
    let _configured = DialogSize::Normal.with_compact(CompactLayout::Draggable(points));
    let fitting = Percentage::new(90).unwrap();
    assert_eq!(
        DialogSize::Normal.layout().compact(),
        CompactLayout::Fixed(fitting)
    );
    assert_eq!(
        DialogSize::Wide.layout().compact(),
        CompactLayout::Fixed(fitting)
    );
    assert_eq!(SnapPoints::default().compact().value(), 60);
    assert_eq!(SnapPoints::default().expanded().value(), 90);
    assert!(SnapPoints::new(90, 65).is_none());
    assert!(SnapPoints::new(0, 90).is_none());
}
