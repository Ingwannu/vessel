use super::*;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::io::Cursor;

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(width, height, Rgba([10, 20, 30, 40])))
        .write_to(&mut bytes, ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

#[test]
fn decoded_pixels_are_transferred_once_in_gpui_channel_order() {
    let raster = decode_png(&png(2, 1)).unwrap();
    assert_eq!(raster.size(), gpui::size(2.into(), 1.into()));
    assert_eq!(raster.rgba(), &[10, 20, 30, 40, 10, 20, 30, 40]);
    let image = raster.into_render_image();
    assert_eq!(image.frame_count(), 1);
    assert_eq!(
        image.as_bytes(0),
        Some([30, 20, 10, 40, 30, 20, 10, 40].as_slice())
    );
}

#[test]
fn browser_width_does_not_relax_attachment_limits() {
    let bytes = png(MAX_SIDE, 1);
    assert!(decode_png(&bytes).is_ok());
    assert!(
        super::super::normalize(
            bytes,
            super::super::InputLimits {
                source: MAX_ENCODED_BYTES,
                result: MAX_ENCODED_BYTES,
            }
        )
        .is_err()
    );
    assert!(decode_png(&png(MAX_SIDE.saturating_add(1), 1)).is_err());
}

#[test]
fn encoded_and_rgba_limits_are_independent() {
    let excessive = vec![0; usize::try_from(MAX_ENCODED_BYTES.saturating_add(1)).unwrap()];
    assert!(matches!(decode_png(&excessive), Err(ImageProblem::Size)));
    assert!(decode_png(&png(4096, 2048)).is_ok());
    assert!(matches!(
        decode_png(&png(4096, 2049)),
        Err(ImageProblem::Size)
    ));
    assert!(decode_png(b"not a PNG").is_err());
    let mut jpeg = Cursor::new(Vec::new());
    DynamicImage::new_rgb8(1, 1)
        .write_to(&mut jpeg, ImageFormat::Jpeg)
        .unwrap();
    assert!(matches!(
        decode_png(&jpeg.into_inner()),
        Err(ImageProblem::Format)
    ));
}
