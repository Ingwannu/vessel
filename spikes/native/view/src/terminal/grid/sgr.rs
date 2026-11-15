//! SGR (`CSI … m`): the parser's groups applied to the pen, in order. A
//! group that is unknown or malformed is skipped and the rest still
//! apply (xterm's rule). The parser keeps at most 32 values, so a list is
//! short whatever the program wrote.

use super::pen::{Colour, Concealment, Pen, Polarity, Rgb, Slant, Strike, Underline, Weight};
use crate::terminal::parse::Group;

/// Apply `groups` to `pen`.
pub(super) fn apply(pen: &mut Pen, groups: &[Group]) {
    let mut rest = groups;
    while let Some((group, after)) = rest.split_first() {
        rest = after;
        let Some((&code, subs)) = group.split_first() else {
            continue;
        };
        match code {
            0 => *pen = Pen::PLAIN,
            1 => pen.weight = Weight::Bold,
            2 => pen.weight = Weight::Faint,
            22 => pen.weight = Weight::Regular,
            3 => pen.slant = Slant::Italic,
            23 => pen.slant = Slant::Upright,
            4 => pen.underline = underline(subs),
            21 => pen.underline = Underline::Double,
            24 => pen.underline = Underline::None,
            7 => pen.polarity = Polarity::Inverse,
            27 => pen.polarity = Polarity::Normal,
            8 => pen.concealment = Concealment::Concealed,
            28 => pen.concealment = Concealment::Shown,
            9 => pen.strike = Strike::Through,
            29 => pen.strike = Strike::None,
            30..=37 => pen.fg = indexed(code, 30, 0),
            90..=97 => pen.fg = indexed(code, 90, 8),
            39 => pen.fg = Colour::Default,
            40..=47 => pen.bg = indexed(code, 40, 0),
            100..=107 => pen.bg = indexed(code, 100, 8),
            49 => pen.bg = Colour::Default,
            // 58 is the underline's colour: read so its values are not
            // taken for groups of their own, then dropped.
            38 | 48 | 58 => {
                let (colour, used) = extended(subs, rest);
                rest = rest.get(used..).unwrap_or_default();
                if let Some(colour) = colour {
                    match code {
                        38 => pen.fg = colour,
                        48 => pen.bg = colour,
                        _ => {}
                    }
                }
            }
            // Blink, overline and the rest: dropped (see `Pen`).
            _ => {}
        }
    }
}

/// `4` alone, or with its style as a sub-parameter (`4:0` … `4:5`).
fn underline(subs: &[u16]) -> Underline {
    match subs.first() {
        Some(0) => Underline::None,
        Some(2) => Underline::Double,
        Some(3) => Underline::Curly,
        Some(4) => Underline::Dotted,
        Some(5) => Underline::Dashed,
        // `4`, `4:1`, and an unknown style, which still underlines
        // (xterm.js draws it single).
        None | Some(_) => Underline::Single,
    }
}

/// Palette index `offset + (code − base)`; `code` is within `base..base+8`.
fn indexed(code: u16, base: u16, offset: u8) -> Colour {
    let step = code
        .checked_sub(base)
        .and_then(|step| u8::try_from(step).ok())
        .unwrap_or(0);
    Colour::Indexed(offset.saturating_add(step))
}

/// An extended colour: in the colon form from `subs` (`38:5:n`,
/// `38:2:r:g:b`, `38:2:space:r:g:b`), in the semicolon form from the
/// groups after it (`38;5;n`, `38;2;r;g;b`). Returns the colour, if it
/// was well formed, and how many of `rest`'s groups it used.
fn extended(subs: &[u16], rest: &[Group]) -> (Option<Colour>, usize) {
    if subs.is_empty() {
        let values: Vec<u16> = rest
            .iter()
            .take(4)
            .map(|group| group.first().copied().unwrap_or(0))
            .collect();
        return match values.split_first() {
            Some((5, tail)) => (palette(tail.first().copied()), 2.min(rest.len())),
            Some((2, tail)) => (rgb(tail), 4.min(rest.len())),
            Some(_) | None => (None, 1.min(rest.len())),
        };
    }
    let colour = match subs.split_first() {
        Some((5, tail)) => palette(tail.first().copied()),
        // With a colour space id: `2:space:r:g:b`.
        Some((2, tail)) if tail.len() >= 4 => rgb(tail.get(1..).unwrap_or_default()),
        Some((2, tail)) => rgb(tail),
        Some(_) | None => None,
    };
    (colour, 0)
}

/// A 256-colour index, if it is one.
fn palette(value: Option<u16>) -> Option<Colour> {
    value
        .and_then(|value| u8::try_from(value).ok())
        .map(Colour::Indexed)
}

/// Red, green and blue from the first three values, each at most 255.
fn rgb(values: &[u16]) -> Option<Colour> {
    let [r, g, b, ..] = values else {
        return None;
    };
    Some(Colour::Rgb(Rgb {
        r: u8::try_from(*r).ok()?,
        g: u8::try_from(*g).ok()?,
        b: u8::try_from(*b).ok()?,
    }))
}
