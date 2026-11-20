//! The editor marks: each is a whole vector with its ink edge to edge in
//! its box, and a black mark turns white on dark surfaces.

use super::{EDITORS, Scheme, Size, Space, VSCODE};

fn tree(bytes: &[u8]) -> resvg::usvg::Tree {
    resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default()).unwrap()
}

#[test]
fn every_editor_mark_fills_its_box_in_both_schemes() {
    for art in EDITORS {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let tree = tree(art.image(scheme).bytes);
            let view = tree.size();
            let ink = tree.root().abs_bounding_box();
            assert!(
                ink.left() >= -0.01 && ink.top() >= -0.01,
                "{}",
                art.light.path
            );
            assert!(
                ink.right() <= view.width().mul_add(1.0, 0.01),
                "{}",
                art.light.path
            );
            assert!(
                ink.bottom() <= view.height().mul_add(1.0, 0.01),
                "{}",
                art.light.path
            );
            let longest = ink.width().max(ink.height());
            assert!(
                longest >= view.width().mul_add(0.8, 0.0),
                "{}",
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
