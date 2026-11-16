//! Every line glyph is also served at the command centre's row stroke (2.2):
//! the same vectors, only the root stroke changed; other drawings keep their
//! own path.
use super::{ALL, Assets, COMMAND_WEIGHT, Glyph, LINE_ROOT};
use gpui::AssetSource as _;
use resvg::usvg;

/// The stroke width of every vector under `group`, in order, each asserted
/// unfilled.
fn widths(group: &usvg::Group) -> Vec<u32> {
    group
        .children()
        .iter()
        .flat_map(|node| match node {
            usvg::Node::Path(path) => {
                assert!(path.fill().is_none(), "an unfilled line glyph");
                vec![path.stroke().unwrap().width().get().to_bits()]
            }
            usvg::Node::Group(group) => widths(group),
            usvg::Node::Image(_) | usvg::Node::Text(_) => panic!("vector line paths only"),
        })
        .collect()
}

fn parse(data: &[u8]) -> usvg::Tree {
    usvg::Tree::from_data(data, &usvg::Options::default()).unwrap()
}

#[test]
fn every_line_glyph_has_a_command_weight_asset_with_the_same_vectors_at_2_2() {
    let mut lines = 0_usize;
    for glyph in ALL {
        if !glyph.svg().starts_with(LINE_ROOT) {
            continue;
        }
        lines = lines.saturating_add(1);
        let path = glyph.command_weight_path();
        let file = glyph.path().strip_prefix("kit/").unwrap();
        assert_eq!(
            path.as_ref(),
            format!("{COMMAND_WEIGHT}{file}"),
            "{glyph:?}"
        );
        let data = Assets.load(&path).unwrap().unwrap();
        // Only the root's stroke differs from the shared drawing.
        assert_eq!(
            String::from_utf8(data.to_vec()).unwrap(),
            glyph
                .svg()
                .replacen(r#"stroke-width="1.5""#, r#"stroke-width="2.2""#, 1),
            "{glyph:?}"
        );
        // The same vectors; each one drawn at the root's 1.5 is now at 2.2,
        // and a child's own stroke (the spinner's arc at 2) is kept.
        let base = Assets.load(glyph.path()).unwrap().unwrap();
        let expected: Vec<u32> = widths(parse(&base).root())
            .into_iter()
            .map(|width| {
                if width == 1.5_f32.to_bits() {
                    2.2_f32.to_bits()
                } else {
                    width
                }
            })
            .collect();
        assert_eq!(widths(parse(&data).root()), expected, "{glyph:?}");
    }
    assert!(lines > 80, "{lines}");
}

#[test]
fn other_drawings_keep_their_own_path() {
    for glyph in ALL {
        if glyph.svg().starts_with(LINE_ROOT) {
            continue;
        }
        assert_eq!(
            glyph.command_weight_path().as_ref(),
            glyph.path(),
            "{glyph:?}"
        );
        let file = glyph.path().strip_prefix("kit/").unwrap();
        assert!(
            Assets
                .load(&format!("{COMMAND_WEIGHT}{file}"))
                .unwrap()
                .is_none(),
            "{glyph:?}"
        );
    }
    // Paseo's own command-centre roots are drawn at their stroke already.
    for glyph in [Glyph::CommandPlus, Glyph::CommandSettings] {
        assert_eq!(glyph.command_weight_path().as_ref(), glyph.path());
    }
    assert!(
        Assets
            .load(&format!("{COMMAND_WEIGHT}missing.svg"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn the_vessel_only_rows_are_derived() {
    // Add a machine and each machine, the board, and the theme, session and
    // settings rows Paseo's list does not have.
    for glyph in [
        Glyph::Machine,
        Glyph::Board,
        Glyph::Monitor,
        Glyph::Sun,
        Glyph::Moon,
        Glyph::Message,
        Glyph::Terminal,
        Glyph::Palette,
        Glyph::Keyboard,
        // The chosen theme row's check.
        Glyph::Check,
    ] {
        let path = glyph.command_weight_path();
        let data = Assets.load(&path).unwrap().unwrap();
        let drawn = widths(parse(&data).root());
        assert!(!drawn.is_empty(), "{glyph:?}");
        assert!(
            drawn.iter().all(|width| *width == 2.2_f32.to_bits()),
            "{glyph:?}"
        );
        assert!(
            path.starts_with(COMMAND_WEIGHT),
            "{glyph:?}"
        );
    }
}

#[test]
fn the_shared_assets_list_stays_one_entry_per_glyph() {
    let listed = Assets.list("kit/").unwrap();
    let shared = ALL
        .into_iter()
        .filter(|glyph| glyph.path().starts_with("kit/"))
        .count();
    assert_eq!(listed.len(), shared);
    assert!(listed.iter().all(|path| !path.starts_with(COMMAND_WEIGHT)));
}
