use super::*;
use vessel_app_view::browser::{LogicalLength, PanePoint};
use vessel_proto::conversation::FiniteNumber;

// Independently authored synthetic-page capture, no reference asset or user data.
const PNG: &[u8] = include_bytes!("pipe-canary.png");
const CSS_ERROR_BOUND: f64 = 1e-9;

#[expect(
    clippy::float_arithmetic,
    reason = "the tolerance checks continuous CSS coordinates without exact floating equality"
)]
fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= CSS_ERROR_BOUND);
}

fn point(x: f64, y: f64) -> PanePoint {
    PanePoint {
        x: FiniteNumber::new(x).unwrap(),
        y: FiniteNumber::new(y).unwrap(),
    }
}

#[test]
fn decoded_frame_maps_its_associated_css_viewport_and_excludes_letterboxing() {
    let frame = PreparedFrame::decode(
        PNG,
        RasterSize::new(390, 844).unwrap(),
        PageSize::new(195, 422).unwrap(),
    )
    .unwrap();
    assert_eq!(frame.image.frame_count(), 1);
    assert_eq!(frame.image.as_bytes(0).unwrap().len(), 390 * 844 * 4);
    let map = frame
        .surface(PaneSize {
            width: LogicalLength::new(590.0).unwrap(),
            height: LogicalLength::new(844.0).unwrap(),
        })
        .unwrap();
    assert_eq!(map.origin(), point(100.0, 0.0));
    let center = map.page_point(point(295.0, 422.0)).unwrap();
    close(center.x.get(), 97.5);
    close(center.y.get(), 211.0);
    assert!(map.page_point(point(99.0, 100.0)).is_none());
    assert!(map.page_point(point(490.0, 100.0)).is_none());
    assert!(map.page_point(point(295.0, 844.0)).is_none());
}

#[test]
fn mismatched_metadata_and_invalid_pixels_cannot_form_a_frame() {
    let expected = RasterSize::new(390, 843).unwrap();
    let page = PageSize::new(390, 844).unwrap();
    assert!(matches!(
        PreparedFrame::decode(PNG, expected, page),
        Err(PreparationError::RasterMismatch)
    ));
    let error = PreparedFrame::decode(b"not a PNG", expected, page)
        .err()
        .unwrap();
    assert!(matches!(error, PreparationError::Image(_)));
    assert!(std::error::Error::source(&error).is_some());
    assert!(!error.to_string().contains("not a PNG"));
}
