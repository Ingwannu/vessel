//! Every line glyph is also served at the command centre's row stroke (2.2):
//! the same vectors, only the root stroke changed; other drawings keep their
//! own path.
use super::{ALL, Assets, COMMAND_WEIGHT, Glyph, LINE_ROOT};
use gpui::AssetSource as _;
use resvg::usvg;

/// Stroked vectors under `group`, each asserted unfilled at `width`.
fn stroked(group: &usvg::Group, width: f32) -> usize {
    group.children().iter().fold(0_usize, |count, node| {
        count.saturating_add(match node {
            usvg::Node::Path(path) => {
                assert!(path.fill().is_none(), "an unfilled line glyph");
                assert_eq!(
                    path.stroke().unwrap().width().get().to_bits(),
                    width.to_bits()
                );
                1
            }
            usvg::Node::Group(group) => stroked(group, width),
            usvg::Node::Image(_) | usvg::Node::Text(_) => panic!("vector line paths only"),
        })
    })
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
        let base = Assets.load(glyph.path()).unwrap().unwrap();
        assert_eq!(
            stroked(parse(&data).root(), 2.2),
            stroked(parse(&base).root(), 1.5),
            "{glyph:?}"
        );
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
    ] {
        assert!(
            glyph.command_weight_path().starts_with(COMMAND_WEIGHT),
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
