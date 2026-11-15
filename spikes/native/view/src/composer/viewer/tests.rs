use super::*;

fn extent(w: f32, h: f32) -> Extent {
    Extent::new(w, h).expect("finite nonempty extent")
}
fn position(x: f32, y: f32) -> Position {
    Position::new(x, y).expect("finite point")
}
fn viewport() -> Viewport {
    Viewport::new(extent(1600.0, 800.0), extent(800.0, 600.0))
}

#[test]
fn fit_preserves_aspect_with_padding_and_presentation_caps() {
    let view = viewport();
    assert_eq!(view.size(), extent(768.0, 384.0));
    let large = Viewport::new(extent(1600.0, 800.0), extent(2000.0, 1400.0));
    assert_eq!(large.size(), extent(MAX_WIDTH, 480.0));
    assert_eq!(large.scale(), MIN_SCALE);
    assert!(!view.contains(position(400.0, 10.0)));
    assert!(view.contains(position(400.0, 300.0)));
}

#[test]
fn a_portrait_fits_on_the_phone_and_resets_on_resize() {
    let mut view = Viewport::new(extent(400.0, 800.0), extent(390.0, 844.0));
    assert_eq!(view.size(), extent(320.0, MAX_HEIGHT));
    view.zoom_center(MAX_SCALE);
    view.resize(extent(800.0, 600.0));
    assert_eq!(view.scale(), MIN_SCALE);
    assert_eq!(view.size(), extent(284.0, 568.0));
}

#[test]
fn zoom_keeps_the_focal_point_and_clamps_scale_and_pan() {
    let mut view = viewport();
    let focal = position(500.0, 300.0);
    view.zoom(2.0, focal);
    assert_eq!(view.left(), -468.0);
    view.drag(position(0.0, 0.0), position(10000.0, -10000.0));
    assert_eq!(view.left(), 0.0);
    assert_eq!(view.top(), -168.0);
    view.zoom_center(f32::MAX);
    assert_eq!(view.scale(), MAX_SCALE);
    view.zoom_center(f32::MIN_POSITIVE);
    assert_eq!(view.scale(), MIN_SCALE);
    assert_eq!(view.left(), 16.0);
    assert_eq!(view.top(), 108.0);
}

#[test]
fn invalid_platform_data_cannot_poison_viewport() {
    assert!(Extent::new(f32::NAN, 10.0).is_none());
    assert!(Extent::new(0.0, 10.0).is_none());
    assert!(Position::new(f32::INFINITY, 0.0).is_none());
    let mut view = viewport();
    let before = view;
    view.zoom(f32::NAN, position(1.0, 1.0));
    view.wheel(f32::INFINITY, position(1.0, 1.0));
    assert_eq!(view, before);
    view.zoom_center(ZOOM_IN);
    view.reset();
    assert_eq!(view, before);
}
