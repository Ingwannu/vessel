//! The command centre's root icons keep Paseo's explicit strokes (2.4 for its
//! Plus, 2.2 for the others) on canonical Lucide 1.50.0 shapes, distinct from
//! the provider-weight (1.5) roles drawn elsewhere.
use super::panel_role_tests::{paint, tree};
use super::{Assets, Glyph, view_box};
use gpui::AssetSource as _;
use resvg::usvg;

/// Each role, its stroke width and its stroked vector count after usvg
/// turns circles and rects into paths.
const ROLES: [(Glyph, f32, usize); 10] = [
    (Glyph::CommandPlus, 2.4, 2),
    (Glyph::CommandAddProject, 2.2, 3),
    (Glyph::CommandSettings, 2.2, 2),
    (Glyph::CommandHistory, 2.2, 3),
    (Glyph::CommandSchedules, 2.2, 6),
    (Glyph::CommandKeyboard, 2.2, 9),
    (Glyph::CommandHome, 2.2, 2),
    (Glyph::CommandFolder, 2.2, 1),
    (Glyph::CommandCircleDashed, 2.2, 8),
    (Glyph::CommandPanelLeft, 2.2, 2),
];

fn stroked(group: &usvg::Group, width: f32) -> usize {
    group.children().iter().fold(0_usize, |count, node| {
        count.saturating_add(match node {
            usvg::Node::Path(path) => {
                assert!(path.fill().is_none(), "an unfilled line glyph");
                assert_eq!(path.stroke().unwrap().width().get().to_bits(), width.to_bits());
                1
            }
            usvg::Node::Group(group) => stroked(group, width),
            usvg::Node::Image(_) | usvg::Node::Text(_) => panic!("vector line paths only"),
        })
    })
}

#[test]
fn command_roles_have_one_asset_each_and_paseos_explicit_strokes() {
    for (glyph, width, count) in ROLES {
        assert!(glyph.path().starts_with("kit/command-"), "{glyph:?}");
        assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 24.0, 24.0]));
        assert_eq!(
            Assets
                .list("kit/")
                .unwrap()
                .iter()
                .filter(|entry| entry.as_ref() == glyph.path())
                .count(),
            1
        );
        assert_eq!(stroked(tree(glyph).root(), width), count, "{glyph:?}");
    }
}

#[test]
fn command_plus_is_heavier_than_the_shared_plus() {
    // The shared Plus keeps the provider's 1.5 for every other surface.
    assert_eq!(stroked(tree(Glyph::Plus).root(), 1.5), 2);
    assert_eq!(stroked(tree(Glyph::CommandPlus).root(), 2.4), 2);
    let shared = paint(Glyph::Plus, 16, 4);
    let command = paint(Glyph::CommandPlus, 16, 4);
    let ink = |image: &image::RgbaImage| {
        image
            .pixels()
            .map(|pixel| u64::from(pixel.0[3]))
            .sum::<u64>()
    };
    assert!(ink(&command) > ink(&shared));
}

#[test]
fn command_roles_keep_ink_at_the_row_size_over_fractional_scales() {
    // Lucide's frames reach 0.9 of a unit from the box at these strokes, as
    // Paseo's do, so no clear rim is required; each role must still draw.
    for (glyph, _, _) in ROLES {
        for scale in [4_u16, 5, 6, 8] {
            let image = paint(glyph, 16, scale);
            assert!(image.pixels().any(|pixel| pixel.0[3] > 0), "{glyph:?}/{scale}");
        }
    }
}
