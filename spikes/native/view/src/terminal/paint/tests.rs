//! Colours as drawn: the table past the 16, bold in bright colours,
//! inverse with default colours, faint and concealed ink.

use super::{Ink, Painted, Palette, indexed, paint};
use crate::terminal::grid::{Colour, Concealment, Pen, Polarity, Rgb, Weight};

const fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    Rgb { r, g, b }
}

/// A palette whose ANSI colour `n` is `(n, n, n)`, to tell them apart.
fn palette() -> Palette {
    let mut ansi = [Rgb::default(); 16];
    for (n, colour) in (0_u8..).zip(ansi.iter_mut()) {
        *colour = rgb(n, n, n);
    }
    Palette {
        ansi,
        foreground: rgb(200, 201, 202),
        background: rgb(10, 11, 12),
        cursor: rgb(1, 2, 3),
    }
}

#[test]
fn the_table_past_sixteen_is_xterms_cube_and_grey_ramp() {
    let palette = palette();
    assert_eq!(indexed(5, &palette), rgb(5, 5, 5));
    assert_eq!(indexed(16, &palette), rgb(0, 0, 0));
    assert_eq!(indexed(21, &palette), rgb(0, 0, 255));
    assert_eq!(indexed(67, &palette), rgb(95, 135, 175));
    assert_eq!(indexed(196, &palette), rgb(255, 0, 0));
    assert_eq!(indexed(231, &palette), rgb(255, 255, 255));
    assert_eq!(indexed(232, &palette), rgb(8, 8, 8));
    assert_eq!(indexed(255, &palette), rgb(238, 238, 238));
}

#[test]
fn bold_draws_the_eight_ansi_colours_bright_and_nothing_else() {
    let palette = palette();
    let bold = |fg| Pen {
        fg,
        weight: Weight::Bold,
        ..Pen::PLAIN
    };
    assert_eq!(paint(bold(Colour::Indexed(1)), &palette).fg, rgb(9, 9, 9));
    assert_eq!(paint(bold(Colour::Indexed(9)), &palette).fg, rgb(9, 9, 9));
    assert_eq!(
        paint(bold(Colour::Indexed(196)), &palette).fg,
        rgb(255, 0, 0)
    );
    assert_eq!(
        paint(bold(Colour::Default), &palette).fg,
        palette.foreground
    );
    assert_eq!(
        paint(bold(Colour::Rgb(rgb(1, 1, 1))), &palette).fg,
        rgb(1, 1, 1)
    );
}

#[test]
fn inverse_swaps_and_a_default_colour_takes_the_palettes() {
    let palette = palette();
    let inverse = Pen {
        polarity: Polarity::Inverse,
        ..Pen::PLAIN
    };
    assert_eq!(
        paint(inverse, &palette),
        Painted {
            fg: palette.background,
            bg: Some(palette.foreground),
            ink: Ink::Full,
        }
    );
    let red_on_blue = Pen {
        fg: Colour::Indexed(1),
        bg: Colour::Rgb(rgb(0, 0, 9)),
        ..inverse
    };
    assert_eq!(paint(red_on_blue, &palette).fg, rgb(0, 0, 9));
    assert_eq!(paint(red_on_blue, &palette).bg, Some(rgb(1, 1, 1)));
    // Plain: the default background is not drawn over.
    assert_eq!(paint(Pen::PLAIN, &palette).bg, None);
}

#[test]
fn inverse_swaps_before_bold_brightens_as_xterm_js_does() {
    let palette = palette();
    let bold_inverse = Pen {
        weight: Weight::Bold,
        polarity: Polarity::Inverse,
        ..Pen::PLAIN
    };
    // `ESC [ 1 ; 7 ; 31 ; 44 m`: bright blue (12) on red (1).
    let red_on_blue = Pen {
        fg: Colour::Indexed(1),
        bg: Colour::Indexed(4),
        ..bold_inverse
    };
    let drawn = paint(red_on_blue, &palette);
    assert_eq!((drawn.fg, drawn.bg), (rgb(12, 12, 12), Some(rgb(1, 1, 1))));
    // `ESC [ 1 ; 7 ; 44 m`: bright blue on the palette's foreground.
    let default_on_blue = Pen {
        bg: Colour::Indexed(4),
        ..bold_inverse
    };
    let drawn = paint(default_on_blue, &palette);
    assert_eq!(
        (drawn.fg, drawn.bg),
        (rgb(12, 12, 12), Some(palette.foreground))
    );
}

#[test]
fn faint_is_half_ink_and_concealed_is_none() {
    let palette = palette();
    let faint = Pen {
        weight: Weight::Faint,
        ..Pen::PLAIN
    };
    assert_eq!(paint(faint, &palette).ink, Ink::Faint);
    let hidden = Pen {
        concealment: Concealment::Concealed,
        ..faint
    };
    assert_eq!(paint(hidden, &palette).ink, Ink::Hidden);
}
