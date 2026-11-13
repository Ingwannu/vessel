//! The faces, embedded in the binary, and the type scale (DESIGN.md
//! section 5).
//!
//! Geist for Latin, then Pretendard for Hangul and every script Geist
//! lacks, as the page's stack has it; the owner's rule (no small text
//! that looks like pixel type, Pretendard before any generic family)
//! holds because no system face is ever asked for: both faces ship in the
//! binary and every style names them. Embedded rather than read from
//! files beside the binary: one file to install, nothing to find at
//! start, at about 5 MB (Pretendard's three weights are most of it; the
//! real build subsets them, as the page does).

use std::borrow::Cow;

use gpui::{App, FontFallbacks, FontWeight, SharedString};

/// The sans family's name, as registered.
pub const SANS: &str = "Geist";
/// The mono family's name.
pub const MONO: &str = "Geist Mono";
/// The fallback for every script Geist lacks.
pub const HANGUL: &str = "Pretendard";

static FACES: [&[u8]; 7] = [
    include_bytes!("../../assets/fonts/Geist-Regular.ttf"),
    include_bytes!("../../assets/fonts/Geist-Medium.ttf"),
    include_bytes!("../../assets/fonts/Geist-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/GeistMono-Regular.ttf"),
    include_bytes!("../../assets/fonts/Pretendard-Regular.otf"),
    include_bytes!("../../assets/fonts/Pretendard-Medium.otf"),
    include_bytes!("../../assets/fonts/Pretendard-SemiBold.otf"),
];

/// Register the faces with the text system.
///
/// # Errors
/// A face could not be read (only a broken build could cause it).
pub fn register(cx: &App) -> Result<(), FontError> {
    cx.text_system()
        .add_fonts(FACES.iter().map(|face| Cow::Borrowed(*face)).collect())
        .map_err(|error| FontError(error.to_string()))
}

/// A face could not be registered.
#[derive(Debug, thiserror::Error)]
#[error("registering the app's fonts: {0}")]
pub struct FontError(String);

/// The fallback list every style carries: Pretendard after Geist.
#[must_use]
pub fn fallbacks() -> FontFallbacks {
    FontFallbacks::from_fonts(vec![HANGUL.to_owned()])
}

/// One step of the type scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// `--t-title`: 16 / 20 / 600, a page's heading.
    Title,
    /// `--t-body`: 14 / 21 / 400.
    Body,
    /// `--t-label`: 14 / 20 / 500, sidebar rows.
    Label,
    /// `--t-meta`: 12 / 16 / 400, secondary chrome.
    Meta,
    /// `--t-kicker`: 11 / 16 / 500, column headers.
    Kicker,
    /// `--t-card`: 11.5 / 14.4 / 600, a card's title.
    Card,
    /// `--t-2xs`: 10.5 / 14 / 400, a card's lines.
    Tiny,
    /// `--t-mono`: 12 / 20 / 400, ids, paths.
    Mono,
}

impl Type {
    /// Size and line height in pixels, and weight.
    #[must_use]
    pub const fn metrics(self) -> (f32, f32, FontWeight) {
        match self {
            Self::Title => (16.0, 20.0, FontWeight::SEMIBOLD),
            Self::Body => (14.0, 21.0, FontWeight::NORMAL),
            Self::Label => (14.0, 20.0, FontWeight::MEDIUM),
            Self::Meta => (12.0, 16.0, FontWeight::NORMAL),
            Self::Kicker => (11.0, 16.0, FontWeight::MEDIUM),
            Self::Card => (11.5, 14.4, FontWeight::SEMIBOLD),
            Self::Tiny => (10.5, 14.0, FontWeight::NORMAL),
            Self::Mono => (12.0, 20.0, FontWeight::NORMAL),
        }
    }

    /// The family.
    #[must_use]
    pub fn family(self) -> SharedString {
        match self {
            Self::Mono => MONO.into(),
            Self::Title
            | Self::Body
            | Self::Label
            | Self::Meta
            | Self::Kicker
            | Self::Card
            | Self::Tiny => SANS.into(),
        }
    }
}

/// How wide `text` is drawn in `step` (shaped by the window's text
/// system, the same shaping the screen draws with), rounded up to a whole
/// point so that room of this width never cuts the words: for a layout
/// that has to know whether words fit before it draws them (the
/// composer's controls choose their step of Paseo's ladder from it).
#[must_use]
pub fn measure(window: &gpui::Window, text: &str, step: Type) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let (size, _, weight) = step.metrics();
    let run = gpui::TextRun {
        len: text.len(),
        font: gpui::Font {
            family: step.family(),
            features: gpui::FontFeatures::default(),
            fallbacks: Some(fallbacks()),
            weight,
            style: gpui::FontStyle::Normal,
        },
        color: gpui::black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let line =
        window
            .text_system()
            .shape_line(text.to_owned().into(), gpui::px(size), &[run], None);
    f32::from(line.width).ceil()
}
