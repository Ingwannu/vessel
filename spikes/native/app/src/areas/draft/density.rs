//! How much of the composer's controls show, widest first: Paseo's
//! density ladder (`composer/agent-controls/layout.ts`), its floors, its
//! hysteresis and its presentations, for the three steps a wide window
//! meets.
//!
//! [Decision Log]
//! - Purpose: the owner's rule is Paseo's composer: on a wide window
//!   every feature is a control of its own beside the model, thinking and
//!   mode controls, with carets; as the composer narrows, carets and
//!   labels go and the features fold into one control that opens their
//!   sheet (0.0.0.0.6 always folded them).
//! - Choice: Paseo's numbers: a pointer control 28 high, its glyph 16, 8
//!   inside, 4 between glyph and label and between controls, a caret 14,
//!   12 of hysteresis.
//! - Differences, each with its reason:
//!   - The floors count the controls' own words as the text system draws
//!     them, where Paseo estimates (70 for a model, 48 for thinking, 96
//!     for a mode, 7 a character): here a model reads "Opus 5.5 Claude
//!     Code" and every feature carries its name, and the estimate kept
//!     the full step where the words did not fit.
//!   - No control's words are cut while a narrower step exists: the step
//!     drops the moment the row is under its floor, and the hysteresis
//!     holds only on the way back up (a fuller step needs its floor and
//!     12). Paseo keeps a step until 12 under its floor, cutting the
//!     words meanwhile, which costs it little (its features are icons by
//!     then); here the features carry words and a row of ellipses is what
//!     the owner sees (the coordinator, 0.0.0.0.7). Only `tight`, the last
//!     step, cuts the model's words (at Paseo's 56).
//!   - Paseo's two narrowest steps (`compact`: thinking, mode and features
//!     folded into the model control's sheet; `iconOnly`) are not here:
//!     under `tight` the controls stay as `tight` shows them, and under
//!     600 they are touch-sized as before. The font scale is 1 (the app
//!     has no text size setting yet).

/// A step of the ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Density {
    /// Carets, the thinking and mode labels, a control per feature.
    #[default]
    Full,
    /// No carets; thinking as its glyph; the features as one control.
    Condensed,
    /// The mode as its glyph too.
    Tight,
}

/// Widest first.
const LADDER: [Density; 3] = [Density::Full, Density::Condensed, Density::Tight];

impl Density {
    /// Paseo's presentation of this step.
    pub(crate) const fn carets(self) -> bool {
        matches!(self, Self::Full)
    }

    /// The thinking control shows its value.
    pub(crate) const fn thinking_label(self) -> bool {
        matches!(self, Self::Full)
    }

    /// The mode control shows its value.
    pub(crate) const fn mode_label(self) -> bool {
        matches!(self, Self::Full | Self::Condensed)
    }

    /// The features are one control opening their sheet.
    pub(crate) const fn aggregate_features(self) -> bool {
        !matches!(self, Self::Full)
    }
}

/// A feature as the floors count it, by its words' measured widths: every
/// control carries the feature's name and no glyph (Vessel's; Paseo's
/// on/off control is a square of the feature's icon and its choice shows
/// its icon and value; ACP sends no icon, and one glyph on every named
/// control says nothing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Feature {
    /// An on/off feature: its name's width.
    Toggle(f32),
    /// A choice: its name's and its value's widths.
    Select { name: f32, value: f32 },
}

/// What the composer has to show, its words measured (`kit::fonts::measure`).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Presence {
    /// The model control's words' width (the model and, when the machine
    /// has several agents, the gap and the agent); `None` for no control.
    pub(crate) model: Option<f32>,
    /// The thinking control's value's width.
    pub(crate) thinking: Option<f32>,
    /// The mode control's value's width.
    pub(crate) mode: Option<f32>,
    pub(crate) features: Vec<Feature>,
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

/// A feature's control's chrome: its padding either side (no glyph).
const FEATURE_CHROME: f32 = 16.0;

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

fn floor(density: Density, presence: &Presence, geometry: Geometry) -> f32 {
    let chrome = chrome(geometry);
    let mut widths = Vec::new();
    match density {
        // A control's chrome, then: the model's words and its caret (a gap
        // before it); a value and its caret; a feature's name, and a
        // choice's value after a gap.
        Density::Full => {
            if let Some(model) = presence.model {
                widths.push([chrome, model, GAP, CARET].iter().sum());
            }
            if let Some(thinking) = presence.thinking {
                widths.push([chrome, thinking, GAP, CARET].iter().sum());
            }
            if let Some(mode) = presence.mode {
                widths.push([chrome, mode, GAP, CARET].iter().sum());
            }
            for feature in &presence.features {
                widths.push(match feature {
                    Feature::Toggle(name) => [FEATURE_CHROME, *name].iter().sum(),
                    Feature::Select { name, value } => {
                        [FEATURE_CHROME, *name, GAP, *value].iter().sum()
                    }
                });
            }
        }
        Density::Condensed => {
            if let Some(model) = presence.model {
                widths.push([chrome, model].iter().sum());
            }
            if presence.thinking.is_some() {
                widths.push(geometry.control);
            }
            if let Some(mode) = presence.mode {
                widths.push([chrome, mode].iter().sum());
            }
            if !presence.features.is_empty() {
                widths.push(geometry.control);
            }
        }
        Density::Tight => {
            // The model at its family's width (Paseo's 56), cut after it.
            if presence.model.is_some() {
                widths.push([chrome, 56.0].iter().sum());
            }
            for shown in [
                presence.thinking.is_some(),
                presence.mode.is_some(),
                !presence.features.is_empty(),
            ] {
                if shown {
                    widths.push(geometry.control);
                }
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

    /// Claude Code as the preview node offers it, at 7 a character: a
    /// model ("Opus 5.5", a gap, "Claude Code"), thinking ("High"), mode
    /// ("Manual"), and three choices ("Fast mode" "Off", "Ultracode" "Off",
    /// "Agent" "Default").
    fn claude() -> Presence {
        Presence {
            model: Some(137.0),
            thinking: Some(28.0),
            mode: Some(42.0),
            features: vec![
                Feature::Select {
                    name: 63.0,
                    value: 21.0,
                },
                Feature::Select {
                    name: 63.0,
                    value: 21.0,
                },
                Feature::Select {
                    name: 35.0,
                    value: 49.0,
                },
            ],
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
        // 191 + 82 + 96 + three named choices of 104 (no glyph) + 5 gaps.
        near(floor(Density::Full, &claude(), pointer), 701.0);
        // 173 + 28 + 78 + 28 + 3 gaps.
        near(floor(Density::Condensed, &claude(), pointer), 319.0);
        // 92 + 28 + 28 + 28 + 3 gaps.
        near(floor(Density::Tight, &claude(), pointer), 188.0);
    }

    #[test]
    fn a_wide_composer_is_full_and_narrowing_steps_down() {
        let pointer = Geometry::of(false);
        let presence = claude();
        assert_eq!(
            resolve(900.0, Density::Full, &presence, pointer),
            Density::Full
        );
        assert_eq!(
            resolve(400.0, Density::Full, &presence, pointer),
            Density::Condensed
        );
        assert_eq!(
            resolve(250.0, Density::Full, &presence, pointer),
            Density::Tight
        );
    }

    #[test]
    fn a_step_drops_under_its_floor_and_comes_back_over_it_and_twelve() {
        let pointer = Geometry::of(false);
        let presence = claude();
        // At its floor (701) Full keeps; a point under, it drops at once
        // (its words would be cut).
        assert_eq!(
            resolve(701.0, Density::Full, &presence, pointer),
            Density::Full
        );
        assert_eq!(
            resolve(700.0, Density::Full, &presence, pointer),
            Density::Condensed
        );
        // From Condensed, Full comes back only at its floor plus 12.
        assert_eq!(
            resolve(712.0, Density::Condensed, &presence, pointer),
            Density::Condensed
        );
        assert_eq!(
            resolve(713.0, Density::Condensed, &presence, pointer),
            Density::Full
        );
        // Condensed itself drops at its floor too.
        assert_eq!(
            resolve(318.0, Density::Condensed, &presence, pointer),
            Density::Tight
        );
    }
}
