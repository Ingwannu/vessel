//! The faces, embedded in the binary, and the type scale (DESIGN.md
//! section 5).
//!
//! Geist for Latin, then Pretendard for Hangul and every script Geist
//! lacks, as the page's stack has it; the owner's rule (no small text
//! that looks like pixel type, Pretendard before any generic family)
//! holds because no system face is ever asked for: both faces ship in the
//! binary and every style names them. Embedded rather than read from
//! files beside the binary: one file to install, nothing to find at
//! start, at about 6.7 MB (Pretendard's four weights are most of it; the
//! real build subsets them, as the page does).
//!
//! [Decision Log: form paths]
//! - Purpose: match GOAT's `GitActionsControl` inherited text-sm14/20 mono rows.
//! - Constraint: existing Mono12/20 callers and fallbacks must stay unchanged.
//! - Alternatives: area font literals, or a UI-scaled label with a mono family.
//! - Choice: one named MonoBody14/20 normal step on the code scale and face.
//! - Why: metrics cannot mix independent UI/code preferences; `TypeExt` already
//!   consumes the shared metrics/family and retains its fallback list.
//! - Cost: one additional public type step; no font asset or editing engine.

use std::borrow::Cow;
use std::sync::atomic::{AtomicU8, Ordering};

use gpui::{App, FontFallbacks, FontWeight, SharedString};

/// The sans family's name, as registered.
pub const SANS: &str = "Geist";
/// The mono family's name.
pub const MONO: &str = "Geist Mono";
/// The fallback for every script Geist lacks.
pub const HANGUL: &str = "Pretendard";

/// The weights the design draws: 300 (Paseo's desktop screen title,
/// `headers/screen-title.tsx`, track G2's `screen_header`), 400, 500 and
/// 600, each in Geist and in Pretendard so Hangul keeps the Latin's
/// weight (a missing weight would be drawn at the nearest one, or
/// synthesized). The 300 pair costs 1,725,412 bytes (Geist Light 125,444,
/// Pretendard Light 1,599,968), from the same packages (`scripts/fonts.sh`)
/// under the same licences.
static FACES: [&[u8]; 9] = [
    include_bytes!("../../assets/fonts/Geist-Light.ttf"),
    include_bytes!("../../assets/fonts/Geist-Regular.ttf"),
    include_bytes!("../../assets/fonts/Geist-Medium.ttf"),
    include_bytes!("../../assets/fonts/Geist-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/GeistMono-Regular.ttf"),
    include_bytes!("../../assets/fonts/Pretendard-Light.otf"),
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

/// The interface's text size the scale is drawn at (Settings ›
/// Appearance, the page's `data-ui-size`): its steps are written for 14.
///
/// [Decision Log]
/// - Purpose: the interface and code sizes are applied everywhere at once
///   (the page's `ui/styles/appearance.css`: every sans step scales by the
///   size over 14, code is the code size over 12; spacing and control
///   heights stay, as GOAT's interface size changes text, not the layout).
/// - Choice: one place, read by every step ([`Type::metrics`]) and by
///   [`ui`] and [`code`] for the few sizes not on a step: two numbers kept
///   process-wide, since a step is applied where no context is at hand
///   (`TypeExt::type_`). Set by [`crate::set_text_sizes`], which draws the
///   windows again.
/// - Alternatives: a GPUI global (every `type_` call would need the app's
///   context); a size passed down every screen (each screen would have to
///   remember it).
static INTERFACE: AtomicU8 = AtomicU8::new(INTERFACE_BASE);

/// The code size the scale is drawn at (the page's `data-code-size`).
static CODE: AtomicU8 = AtomicU8::new(CODE_BASE);

/// The interface size the steps are written for.
const INTERFACE_BASE: u8 = 14;

/// The code size the code steps are written for.
const CODE_BASE: u8 = 12;

/// Draw the scale at `interface` and `code` points (bounded by the
/// preferences' ranges). Whether either changed.
pub(crate) fn set_sizes(interface: u8, code: u8) -> bool {
    let before = INTERFACE.swap(interface, Ordering::Relaxed);
    let before_code = CODE.swap(code, Ordering::Relaxed);
    before != interface || before_code != code
}

/// `points` of interface text (written for 14) at the chosen size.
#[must_use]
pub fn ui(points: f32) -> f32 {
    scaled(points, INTERFACE.load(Ordering::Relaxed), INTERFACE_BASE)
}

/// `points` of code (written for 12) at the chosen code size.
#[must_use]
pub fn code(points: f32) -> f32 {
    scaled(points, CODE.load(Ordering::Relaxed), CODE_BASE)
}

/// `points` times `size` over `base`.
fn scaled(points: f32, size: u8, base: u8) -> f32 {
    let ratio = f32::from(size).mul_add(f32::from(base).recip(), 0.0);
    points.mul_add(ratio, 0.0)
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
    /// GOAT's form path row: 14 / 20 / 400, on the code scale and face.
    MonoBody,
}

impl Type {
    /// Size and line height in pixels, and weight, at the chosen sizes
    /// ([`ui`], [`code`]).
    #[must_use]
    pub fn metrics(self) -> (f32, f32, FontWeight) {
        self.metrics_with(ui, code)
    }

    // Injectable resolvers exercise the same production path with unequal
    // preferences without racing process-wide font settings in parallel tests.
    fn metrics_with(
        self,
        interface: fn(f32) -> f32,
        code: fn(f32) -> f32,
    ) -> (f32, f32, FontWeight) {
        let (size, line, weight) = self.written();
        if matches!(self, Self::Mono | Self::MonoBody) {
            (code(size), code(line), weight)
        } else {
            (interface(size), interface(line), weight)
        }
    }

    /// Size, line height and weight as written (the interface at 14, code
    /// at 12).
    const fn written(self) -> (f32, f32, FontWeight) {
        match self {
            Self::Title => (16.0, 20.0, FontWeight::SEMIBOLD),
            Self::Body => (14.0, 21.0, FontWeight::NORMAL),
            Self::Label => (14.0, 20.0, FontWeight::MEDIUM),
            Self::Meta => (12.0, 16.0, FontWeight::NORMAL),
            Self::Kicker => (11.0, 16.0, FontWeight::MEDIUM),
            Self::Card => (11.5, 14.4, FontWeight::SEMIBOLD),
            Self::Tiny => (10.5, 14.0, FontWeight::NORMAL),
            Self::Mono => (12.0, 20.0, FontWeight::NORMAL),
            Self::MonoBody => (14.0, 20.0, FontWeight::NORMAL),
        }
    }

    /// The family.
    #[must_use]
    pub fn family(self) -> SharedString {
        match self {
            Self::Mono | Self::MonoBody => MONO.into(),
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

/// `text` cut in its middle with "…" so it is at most `width` points in
/// `step`'s face; the whole text when it fits. A path keeps its start (the
/// root) and its end (the file's name), which a cut at either end loses.
///
/// [Decision Log]
/// - Purpose: a path in a settings row was clipped mid-word with no mark
///   that anything was missing (the coordinator's review of Settings ›
///   Agents); the brief's rule for a path is its own element of a set
///   width, cut in the middle, the whole of it in a tooltip.
/// - Constraint: GPUI truncates only at a text's end; the cut has to be
///   chosen before drawing, from measured widths.
/// - Choice: the longest middle cut that fits, found by halving the kept
///   length ([`elide_middle_by`]), measured with [`measure`].
/// - Cost: a few line shapings per drawn path (about log2 of its length).
#[must_use]
pub fn elide_middle(window: &gpui::Window, text: &str, step: Type, width: f32) -> String {
    elide_middle_by(text, |candidate| measure(window, candidate, step) <= width)
}

/// The longest middle cut of `text` that `fits` (the whole text when it
/// does): as many characters kept at the start as at the end, one more at
/// the end when odd (a file's name is at the end). `fits` must hold for
/// every shorter cut of a cut it holds for.
#[must_use]
pub fn elide_middle_by(text: &str, fits: impl Fn(&str) -> bool) -> String {
    if fits(text) {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let cut = |keep: usize| -> String {
        // Half, rounded down: the extra character goes to the end.
        let head = keep >> 1;
        let tail = keep.saturating_sub(head);
        chars
            .iter()
            .take(head)
            .chain(std::iter::once(&'…'))
            .chain(chars.iter().skip(chars.len().saturating_sub(tail)))
            .collect()
    };
    let (mut low, mut high) = (0_usize, chars.len().saturating_sub(1));
    while low < high {
        // The upper middle, so the search always moves.
        let middle = low.saturating_add(high).saturating_add(1) >> 1;
        if fits(&cut(middle)) {
            low = middle;
        } else {
            high = middle.saturating_sub(1);
        }
    }
    cut(low)
}

#[cfg(test)]
mod mono_body_tests {
    use super::*;

    fn same(points: f32) -> f32 {
        points
    }
    fn code_eighteen(points: f32) -> f32 {
        points.mul_add(1.5, 0.0)
    }

    #[test]
    fn form_paths_add_the_exact_reference_step_without_changing_existing_mono() {
        assert_eq!(
            Type::Mono.metrics_with(same, same),
            (12.0, 20.0, FontWeight::NORMAL)
        );
        assert_eq!(
            Type::MonoBody.metrics_with(same, same),
            (14.0, 20.0, FontWeight::NORMAL)
        );
        assert_eq!(Type::Mono.family(), SharedString::from(MONO));
        assert_eq!(Type::MonoBody.family(), SharedString::from(MONO));
        assert_eq!(Type::Body.family(), SharedString::from(SANS));
    }

    #[test]
    fn both_code_steps_keep_size_and_line_on_code_while_body_stays_on_interface() {
        assert_eq!(
            Type::Mono.metrics_with(same, code_eighteen),
            (18.0, 30.0, FontWeight::NORMAL)
        );
        assert_eq!(
            Type::MonoBody.metrics_with(same, code_eighteen),
            (21.0, 30.0, FontWeight::NORMAL)
        );
        assert_eq!(
            Type::Body.metrics_with(same, code_eighteen),
            (14.0, 21.0, FontWeight::NORMAL)
        );
    }
}

#[cfg(test)]
mod elide_tests {
    use super::elide_middle_by;

    fn at_most(count: usize) -> impl Fn(&str) -> bool {
        move |text| text.chars().count() <= count
    }

    #[test]
    fn a_text_that_fits_is_kept_whole() {
        assert_eq!(
            elide_middle_by("/usr/bin/codex", at_most(14)),
            "/usr/bin/codex"
        );
    }

    #[test]
    fn a_long_path_keeps_its_root_and_its_name() {
        let path = "/tmp/build/vessel-target-m12-agents/debug/fake-acp";
        let cut = elide_middle_by(path, at_most(20));
        assert_eq!(cut.chars().count(), 20);
        assert_eq!(cut, "/data/bui…g/fake-acp");
        assert!(cut.starts_with("/data/") && cut.ends_with("fake-acp"));
    }

    #[test]
    fn nothing_fits_but_the_mark_itself() {
        assert_eq!(elide_middle_by("abcdef", at_most(1)), "…");
    }
}
