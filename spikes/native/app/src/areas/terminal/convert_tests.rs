//! The kit's palette reaches the model byte for byte, and a run's look
//! reaches the grid as the stated rules say (faint is half ink, a curly
//! underline is wavy, the other underlines straight).

use gpui::{FontStyle, FontWeight};
use vessel_app_kit::terminal_grid::{Strike as KitStrike, Underline as KitUnderline};
use vessel_app_kit::theme::terminal::TerminalColors;
use vessel_app_view::terminal::grid::{Rgb, Slant, Strike, Underline, Weight};
use vessel_app_view::terminal::paint::{Ink, PaintedFill, PaintedRow, PaintedRun, RunInk};

use super::convert::{hsla, palette, row};

const RED: Rgb = Rgb {
    r: 0xcd,
    g: 0x31,
    b: 0x31,
};

fn run(ink: RunInk) -> PaintedRow {
    PaintedRow {
        fills: Vec::new(),
        runs: vec![PaintedRun {
            col: 3,
            text: "ab".to_owned(),
            ink,
        }],
    }
}

fn plain() -> RunInk {
    RunInk {
        fg: RED,
        ink: Ink::Full,
        weight: Weight::Regular,
        slant: Slant::Upright,
        underline: Underline::None,
        strike: Strike::None,
    }
}

#[test]
fn the_palette_is_the_tokens_bytes() {
    for colors in [TerminalColors::light(), TerminalColors::dark()] {
        let painted = palette(&colors);
        for (colour, word) in painted.ansi.iter().zip(colors.ansi) {
            assert_eq!(u32::from_be_bytes([0, colour.r, colour.g, colour.b]), word);
        }
        let bytes = |c: Rgb| u32::from_be_bytes([0, c.r, c.g, c.b]);
        assert_eq!(bytes(painted.foreground), colors.foreground);
        assert_eq!(bytes(painted.background), colors.background);
        assert_eq!(bytes(painted.cursor), colors.cursor);
    }
}

#[test]
fn a_run_keeps_its_column_text_and_colour() {
    let drawn = row(&run(plain()));
    let [only] = drawn.runs.as_slice() else {
        panic!("one run expected, got {}", drawn.runs.len());
    };
    assert_eq!(only.col, 3);
    assert_eq!(only.text.as_ref(), "ab");
    assert_eq!(only.ink.color, hsla(RED));
    assert!((only.ink.color.a - 1.0).abs() < f32::EPSILON);
    assert_eq!(only.ink.weight, FontWeight::NORMAL);
    assert_eq!(only.ink.style, FontStyle::Normal);
    assert_eq!(only.ink.underline, KitUnderline::None);
    assert_eq!(only.ink.strike, KitStrike::None);
}

#[test]
fn faint_is_half_ink_and_bold_italic_take_their_faces() {
    let drawn = row(&run(RunInk {
        ink: Ink::Faint,
        weight: Weight::Bold,
        slant: Slant::Italic,
        strike: Strike::Through,
        ..plain()
    }));
    let [only] = drawn.runs.as_slice() else {
        panic!("one run expected");
    };
    assert!((only.ink.color.a - 0.5).abs() < f32::EPSILON);
    assert_eq!(only.ink.weight, FontWeight::BOLD);
    assert_eq!(only.ink.style, FontStyle::Italic);
    assert_eq!(only.ink.strike, KitStrike::Through);
}

#[test]
fn curly_is_wavy_and_every_other_underline_straight() {
    let drawn = |underline| {
        let painted = row(&run(RunInk {
            underline,
            ..plain()
        }));
        painted.runs.first().map(|run| run.ink.underline)
    };
    assert_eq!(drawn(Underline::Curly), Some(KitUnderline::Wavy));
    for underline in [
        Underline::Single,
        Underline::Double,
        Underline::Dotted,
        Underline::Dashed,
    ] {
        assert_eq!(drawn(underline), Some(KitUnderline::Straight));
    }
}

#[test]
fn a_fill_keeps_its_cells_and_colour() {
    let drawn = row(&PaintedRow {
        fills: vec![PaintedFill {
            col: 7,
            cells: 4,
            color: RED,
        }],
        runs: Vec::new(),
    });
    let [only] = drawn.fills.as_slice() else {
        panic!("one fill expected");
    };
    assert_eq!((only.col, only.cells), (7, 4));
    assert_eq!(only.color, hsla(RED));
}
