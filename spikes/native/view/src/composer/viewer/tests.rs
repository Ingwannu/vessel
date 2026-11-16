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
    assert_eq!(large.scale().to_bits(), FIT_SCALE.to_bits());
    assert!(!view.contains(position(400.0, 10.0)));
    assert!(view.contains(position(400.0, 300.0)));
}

#[test]
fn a_portrait_fits_on_the_phone_and_resets_on_resize() {
    let mut view = Viewport::new(extent(400.0, 800.0), extent(390.0, 844.0));
    assert_eq!(view.size(), extent(320.0, MAX_HEIGHT));
    view.zoom_center(MAX_SCALE);
    view.resize(extent(800.0, 600.0));
    assert_eq!(view.scale().to_bits(), FIT_SCALE.to_bits());
    assert_eq!(view.size(), extent(284.0, 568.0));
}

#[test]
fn zoom_keeps_the_focal_point_and_clamps_scale_and_pan() {
    let mut view = viewport();
    let focal = position(500.0, 300.0);
    view.zoom(2.0, focal);
    assert_eq!(view.left().to_bits(), (-468.0_f32).to_bits());
    view.drag(position(0.0, 0.0), position(10000.0, -10000.0));
    assert_eq!(view.left().to_bits(), (0.0_f32).to_bits());
    assert_eq!(view.top().to_bits(), (-168.0_f32).to_bits());
    view.zoom_center(f32::MAX);
    assert_eq!(view.scale().to_bits(), MAX_SCALE.to_bits());
    view.zoom_center(f32::MIN_POSITIVE);
    assert_eq!(view.scale().to_bits(), MIN_SCALE.to_bits());
    // A quarter of the 768 x 384 fit, centred: no pan below fit.
    assert_eq!(view.size(), extent(192.0, 96.0));
    assert_eq!(view.left().to_bits(), (304.0_f32).to_bits());
    assert_eq!(view.top().to_bits(), (252.0_f32).to_bits());
}

#[test]
fn toolbar_steps_reach_paseos_quarter_floor_and_eight_times_ceiling_from_fit() {
    let mut view = viewport();
    assert_eq!(view.scale().to_bits(), FIT_SCALE.to_bits());
    // Zoom out is available at fit: one 0.8 step from the centre.
    view.zoom_center(ZOOM_OUT);
    assert_eq!(view.scale().to_bits(), ZOOM_OUT.to_bits());
    assert!(view.contains(position(400.0, 300.0)));
    assert!(
        !view.contains(position(400.0, 120.0)),
        "smaller than the fit"
    );
    // 0.8 to the sixth power is still above the floor; the seventh step stops at it.
    for _ in 0..5 {
        view.zoom_center(ZOOM_OUT);
    }
    assert!(view.scale() > MIN_SCALE);
    view.zoom_center(ZOOM_OUT);
    assert_eq!(view.scale().to_bits(), MIN_SCALE.to_bits());
    view.zoom_center(ZOOM_OUT);
    assert_eq!(view.scale().to_bits(), MIN_SCALE.to_bits());
    view.reset();
    assert_eq!(view.scale().to_bits(), FIT_SCALE.to_bits());
    view.zoom_center(ZOOM_IN);
    assert_eq!(view.scale().to_bits(), ZOOM_IN.to_bits());
    // 1.25 to the ninth power is below the ceiling; the tenth step stops at 8.
    for _ in 0..8 {
        view.zoom_center(ZOOM_IN);
    }
    assert!(view.scale() < MAX_SCALE);
    view.zoom_center(ZOOM_IN);
    assert_eq!(view.scale().to_bits(), MAX_SCALE.to_bits());
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
