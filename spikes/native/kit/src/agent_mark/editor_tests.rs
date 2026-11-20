//! The editor marks: each is a whole vector with its ink edge to edge in
//! its box, and a black mark turns white on dark surfaces.

use super::{EDITORS, Scheme, Size, Space, VSCODE};

fn tree(bytes: &[u8]) -> resvg::usvg::Tree {
    resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default()).unwrap()
}

/// The side of the square a mark is drawn into for measuring.
const SIDE: u32 = 48;

/// The width and height of what `bytes` paints in a `SIDE` square: the
/// drawn pixels, not the paths' geometry (Microsoft's VS Code file draws
/// paths past its box and clips them with a mask).
fn painted(bytes: &[u8]) -> (u32, u32) {
    let tree = tree(bytes);
    let scale = 48.0_f32.mul_add(tree.size().width().recip(), 0.0);
    let mut pixels = resvg::tiny_skia::Pixmap::new(SIDE, SIDE).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixels.as_mut(),
    );
    let (mut left, mut top, mut right, mut bottom) = (SIDE, SIDE, 0, 0);
    for (y, row) in (0_u32..).zip(pixels.pixels().chunks(48)) {
        for (x, pixel) in (0_u32..).zip(row) {
            if pixel.alpha() > 0 {
                left = left.min(x);
                right = right.max(x);
                top = top.min(y);
                bottom = bottom.max(y);
            }
        }
    }
    (
        right.saturating_sub(left).saturating_add(1),
        bottom.saturating_sub(top).saturating_add(1),
    )
}

#[test]
fn every_editor_mark_fills_its_box_in_both_schemes() {
    for art in EDITORS {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let view = tree(art.image(scheme).bytes).size();
            assert!(
                view.height().mul_add(-1.0, view.width()).abs() < 0.01,
                "{}",
                art.light.path
            );
            let (width, height) = painted(art.image(scheme).bytes);
            // Edge to edge: at least four fifths of the square.
            assert!(
                width.max(height) >= 38,
                "{} {width}x{height}",
                art.light.path
            );
        }
        assert_eq!(
            art.image_size(Size::Sixteen),
            gpui::size(gpui::px(16.0), gpui::px(16.0))
        );
        assert_eq!(
            art.dimensions(Size::Sixteen, Space::Row),
            Some((32.0, 16.0))
        );
    }
}

#[test]
fn a_mark_drawn_black_on_light_is_white_on_dark_and_vs_code_is_microsoft_s_file() {
    for art in EDITORS {
        let light = std::str::from_utf8(art.image(Scheme::Light).bytes).unwrap();
        let dark = std::str::from_utf8(art.image(Scheme::Dark).bytes).unwrap();
        assert!(!dark.contains(r##"fill="#000000""##), "{}", art.dark.path);
        if light.contains(r##"fill="#000000""##) {
            assert!(dark.contains(r##"fill="#FFFFFF""##) || dark.contains(r##"fill="#3DDC84""##));
        }
    }
    let vscode = VSCODE.image(Scheme::Dark).bytes;
    assert_eq!(vscode, VSCODE.image(Scheme::Light).bytes);
    assert_eq!(vscode.len(), 4451);
    assert!(
        std::str::from_utf8(vscode)
            .unwrap()
            .starts_with(r#"<svg width="100" height="100" viewBox="0 0 100 100""#)
    );
}
