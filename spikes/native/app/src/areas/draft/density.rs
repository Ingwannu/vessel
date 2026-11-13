//! How much of the composer's controls show, widest first: Paseo's
//! density ladder (`composer/agent-controls/layout.ts`), its floors, its
//! hysteresis and its presentations, for the three steps a wide window
//! meets.
//!
//! [Decision Log]
//! - Purpose: the composer's row is the model-and-effort control and the
//!   mode control (the owner, 2026-11-13: one control for the model, the
//!   effort and fast mode; the other options in the model's list); as
//!   the composer narrows, the row drops its carets and the agent's name,
//!   then the effort's word and the mode's, before any word is cut.
//! - Choice: Paseo's numbers: a pointer control 28 high, its glyph 16, 8
//!   inside, 4 between glyph and label and between controls, a caret 14,
//!   12 of hysteresis.
//! - Differences, each with its reason:
//!   - The floors count the controls' own words as the text system draws
//!     them, where Paseo estimates (70 for a model, 48 for thinking, 96
//!     for a mode, 7 a character): here a model reads "Opus 5.5 Claude
//!     Code High", and the estimate kept the full step where the words
//!     did not fit. Measured at the interface size in effect.
//!   - No control's words are cut while a narrower step exists: the step
//!     drops the moment the row is under its floor, and the hysteresis
//!     holds only on the way back up (a fuller step needs its floor and
//!     12). Paseo keeps a step until 12 under its floor, cutting the
//!     words meanwhile, which costs it little (its controls are icons by
//!     then); here they carry words and a row of ellipses is what the
//!     owner sees (the coordinator, 0.0.0.0.7). Only `tight`, the last
//!     step, cuts the model's words (at Paseo's 56).
//!   - Paseo's two narrowest steps (`compact`, `iconOnly`) are not here:
//!     under `tight` the controls stay as `tight` shows them, and under
//!     600 they are touch-sized as before.

/// A step of the ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Density {
    /// Carets; the model, the agent's name and the effort's word; the
    /// mode's word.
    #[default]
    Full,
    /// No carets and no agent's name; the model and the effort's word; the
    /// mode's word.
    Condensed,
    /// The model alone, cut at its family's width; the mode as its glyph.
    Tight,
}

/// Widest first.
const LADDER: [Density; 3] = [Density::Full, Density::Condensed, Density::Tight];

impl Density {
    /// Paseo's presentation of this step.
    pub(crate) const fn carets(self) -> bool {
        matches!(self, Self::Full)
    }

    /// The model control shows the agent's name.
    pub(crate) const fn agent_label(self) -> bool {
        matches!(self, Self::Full)
    }

    /// The model control shows the effort's word.
    pub(crate) const fn effort_label(self) -> bool {
        matches!(self, Self::Full | Self::Condensed)
    }

    /// The mode control shows its value.
    pub(crate) const fn mode_label(self) -> bool {
        matches!(self, Self::Full | Self::Condensed)
    }
}

/// What the row has to show, its words measured (`kit::fonts::measure`).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Presence {
    /// The model's name's width; `None` for no control.
    pub(crate) model: Option<f32>,
    /// The agent's name's width, when the model control shows it.
    pub(crate) agent: Option<f32>,
    /// The effort's word's width, when the agent has levels.
    pub(crate) effort: Option<f32>,
    /// The mode control's value's width.
    pub(crate) mode: Option<f32>,
}

/// The controls' measures: Paseo's pointer geometry, or its touch
/// geometry (44 high, a glyph of 20).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Geometry {
    control: f32,
    glyph: f32,
}

impl Geometry {
    pub(crate) const fn of(touch: bool) -> Self {
        if touch {
            Self {
                control: 44.0,
                glyph: 20.0,
            }
        } else {
            Self {
                control: 28.0,
                glyph: 16.0,
            }
        }
    }
}

const GAP: f32 = 4.0;
const PADDING: f32 = 8.0;
const CARET: f32 = 14.0;
const HYSTERESIS: f32 = 12.0;

/// A labelled control's own chrome: its padding either side, its glyph,
/// and the gap to its label.
fn chrome(geometry: Geometry) -> f32 {
    [PADDING, PADDING, geometry.glyph, GAP].iter().sum()
}

/// `count` times `each` (`mul_add`: no arithmetic operator).
fn times(count: usize, each: f32) -> f32 {
    let count = u16::try_from(count).unwrap_or(u16::MAX);
    f32::from(count).mul_add(each, 0.0)
}

/// Controls side by side, with the gaps between them.
fn row(widths: &[f32]) -> f32 {
    let gaps = times(widths.len().saturating_sub(1), GAP);
    widths.iter().sum::<f32>().mul_add(1.0, gaps)
}

/// A word after a gap, when there is one.
fn after_gap(word: Option<f32>) -> f32 {
    word.map_or(0.0, |word| word.mul_add(1.0, GAP))
}

fn floor(density: Density, presence: &Presence, geometry: Geometry) -> f32 {
    let chrome = chrome(geometry);
    let mut widths = Vec::new();
    match density {
        // A control's chrome, then its words, each after a gap, and its
        // caret after a gap.
        Density::Full => {
            if let Some(model) = presence.model {
                widths.push(
                    [
                        chrome,
                        model,
                        after_gap(presence.agent),
                        after_gap(presence.effort),
                        GAP,
                        CARET,
                    ]
                    .iter()
                    .sum(),
                );
            }
            if let Some(mode) = presence.mode {
                widths.push([chrome, mode, GAP, CARET].iter().sum());
            }
        }
        Density::Condensed => {
            if let Some(model) = presence.model {
                widths.push([chrome, model, after_gap(presence.effort)].iter().sum());
            }
            if let Some(mode) = presence.mode {
                widths.push([chrome, mode].iter().sum());
            }
        }
        Density::Tight => {
            // The model at its family's width (Paseo's 56), cut after it.
            if presence.model.is_some() {
                widths.push([chrome, 56.0].iter().sum());
            }
            if presence.mode.is_some() {
                widths.push(geometry.control);
            }
        }
    }
    row(&widths)
}

/// The widest step whose floor fits `available`: a step wider than the
/// current one needs its floor and the hysteresis, the current one and
/// narrower ones their floor (under it, words would be cut).
pub(crate) fn resolve(
    available: f32,
    current: Density,
    presence: &Presence,
    geometry: Geometry,
) -> Density {
    let current_step = LADDER
        .iter()
        .position(|step| *step == current)
        .unwrap_or_default();
    for (step, density) in LADDER.iter().enumerate() {
        let floor = floor(*density, presence, geometry);
        let required = if step < current_step {
            floor.mul_add(1.0, HYSTERESIS)
        } else {
            floor
        };
        if available >= required {
            return *density;
        }
    }
    Density::Tight
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Claude Code at 7 a character: "Opus 5.5", "Claude Code", "High",
    /// "Manual".
    fn claude() -> Presence {
        Presence {
            model: Some(56.0),
            agent: Some(77.0),
            effort: Some(28.0),
            mode: Some(42.0),
        }
    }

    fn near(value: f32, expected: f32) {
        assert!(
            value > expected.mul_add(1.0, -0.01) && value < expected.mul_add(1.0, 0.01),
            "{value} is not {expected}"
        );
    }

    #[test]
    fn floors_count_every_word_and_its_chrome() {
        let pointer = Geometry::of(false);
        // 36 + 56 + 81 + 32 + 18, then 36 + 42 + 18, a gap.
        near(floor(Density::Full, &claude(), pointer), 323.0);
        // 36 + 56 + 32, then 36 + 42, a gap.
        near(floor(Density::Condensed, &claude(), pointer), 206.0);
        // 92, then 28, a gap.
        near(floor(Density::Tight, &claude(), pointer), 124.0);
    }

    #[test]
    fn a_step_drops_under_its_floor_and_comes_back_over_it_and_twelve() {
        let pointer = Geometry::of(false);
        let presence = claude();
        assert_eq!(
            resolve(323.0, Density::Full, &presence, pointer),
            Density::Full
        );
        assert_eq!(
            resolve(322.0, Density::Full, &presence, pointer),
            Density::Condensed
        );
        assert_eq!(
            resolve(334.0, Density::Condensed, &presence, pointer),
            Density::Condensed
        );
        assert_eq!(
            resolve(335.0, Density::Condensed, &presence, pointer),
            Density::Full
        );
        assert_eq!(
            resolve(150.0, Density::Full, &presence, pointer),
            Density::Tight
        );
    }
}
