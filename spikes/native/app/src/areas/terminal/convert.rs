//! Between the view's terminal types and the kit's: the palette the model
//! answers OSC colour queries with and paints by, and the painted rows the
//! grid control draws. `app` is the only crate that sees both, as neither
//! may depend on the other (ADR 0073).

use gpui::{FontStyle, FontWeight};
use vessel_app_kit::terminal_grid::{
    GridFill, GridRow, GridRun, Ink as KitInk, Strike as KitStrike, Underline as KitUnderline,
};
use vessel_app_kit::theme::terminal::TerminalColors;
use vessel_app_kit::ui::{Hsla, rgb};
use vessel_app_view::terminal::grid::{Rgb, Slant, Strike, Underline, Weight};
use vessel_app_view::terminal::paint::{Ink, PaintedRow, Palette, RunInk};

/// The kit's palette as the model's: the same bytes.
pub(super) fn palette(colors: &TerminalColors) -> Palette {
    let mut ansi = [Rgb::default(); 16];
    for (colour, word) in ansi.iter_mut().zip(colors.ansi) {
        *colour = rgb_of(word);
    }
    Palette {
        ansi,
        foreground: rgb_of(colors.foreground),
        background: rgb_of(colors.background),
        cursor: rgb_of(colors.cursor),
    }
}

/// `0xRRGGBB` as red, green and blue.
const fn rgb_of(word: u32) -> Rgb {
    let [_, r, g, b] = word.to_be_bytes();
    Rgb { r, g, b }
}

/// A drawn colour.
pub(super) fn hsla(colour: Rgb) -> Hsla {
    rgb(u32::from_be_bytes([0, colour.r, colour.g, colour.b])).into()
}

/// A painted row as the control draws it.
pub(super) fn row(row: &PaintedRow) -> GridRow {
    GridRow {
        fills: row
            .fills
            .iter()
            .map(|fill| GridFill {
                col: fill.col,
                cells: fill.cells,
                color: hsla(fill.color),
            })
            .collect(),
        runs: row
            .runs
            .iter()
            .map(|run| GridRun {
                col: run.col,
                text: run.text.clone().into(),
                ink: ink(run.ink),
            })
            .collect(),
    }
}

/// A run's look: faint is half the colour's alpha (xterm.js draws it at
/// half opacity); the grid draws a straight or a wavy underline, so
/// double, dotted and dashed are drawn straight.
fn ink(run: RunInk) -> KitInk {
    let mut color = hsla(run.fg);
    match run.ink {
        Ink::Full => {}
        Ink::Faint => color.a = 0.5,
        Ink::Hidden => color.a = 0.0,
    }
    KitInk {
        color,
        weight: match run.weight {
            Weight::Bold => FontWeight::BOLD,
            Weight::Regular | Weight::Faint => FontWeight::NORMAL,
        },
        style: match run.slant {
            Slant::Italic => FontStyle::Italic,
            Slant::Upright => FontStyle::Normal,
        },
        underline: match run.underline {
            Underline::None => KitUnderline::None,
            Underline::Curly => KitUnderline::Wavy,
            Underline::Single | Underline::Double | Underline::Dotted | Underline::Dashed => {
                KitUnderline::Straight
            }
        },
        strike: match run.strike {
            Strike::None => KitStrike::None,
            Strike::Through => KitStrike::Through,
        },
    }
}
