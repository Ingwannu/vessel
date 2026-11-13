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
//!   inside, 4 between glyph and label and between controls, a caret 14;
//!   a label estimated at 7 per character; a step is kept until the width
//!   is 12 under its floor and taken back only 12 over it.
//! - Differences: Paseo's two narrowest steps (`compact`: thinking, mode
//!   and features folded into the model control's sheet; `iconOnly`) are
//!   not here: under `tight` the controls stay as `tight` shows them, and
//!   under 600 they are touch-sized as before. The font scale is 1 (the
//!   app has no text size setting yet).

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

/// A feature as the floors count it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Feature {
    /// An on/off feature: a square.
    Toggle,
    /// A choice, with this many characters in its value.
    Select(usize),
}

/// What the composer has to show.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Presence {
    pub(crate) model: bool,
    pub(crate) thinking: bool,
    pub(crate) mode: bool,
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
/// The same, below a floor.
const KEEP: f32 = -12.0;
const PER_CHARACTER: f32 = 7.0;

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

/// A label's estimated width.
fn label(characters: usize) -> f32 {
    times(characters, PER_CHARACTER)
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
        Density::Full => {
            if presence.model {
                widths.push([chrome, CARET, 70.0].iter().sum());
            }
            if presence.thinking {
                widths.push([chrome, GAP, CARET, 48.0].iter().sum());
            }
            if presence.mode {
                widths.push([chrome, GAP, CARET, 96.0].iter().sum());
            }
            for feature in &presence.features {
                widths.push(match feature {
                    Feature::Toggle => geometry.control,
                    Feature::Select(characters) => {
                        [geometry.control, GAP, PADDING, PADDING, label(*characters)]
                            .iter()
                            .sum()
                    }
                });
            }
        }
        Density::Condensed => {
            if presence.model {
                widths.push([chrome, 60.0].iter().sum());
            }
            if presence.thinking {
                widths.push(geometry.control);
            }
            if presence.mode {
                widths.push([chrome, 96.0].iter().sum());
            }
            if !presence.features.is_empty() {
                widths.push(geometry.control);
            }
        }
        Density::Tight => {
            if presence.model {
                widths.push([chrome, 56.0].iter().sum());
            }
            for shown in [
                presence.thinking,
                presence.mode,
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
/// current one needs its floor and the hysteresis, the current one keeps
/// until the width is the hysteresis under its floor.
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
        let required = match step.cmp(&current_step) {
            std::cmp::Ordering::Less => floor.mul_add(1.0, HYSTERESIS),
            std::cmp::Ordering::Equal => floor.mul_add(1.0, KEEP),
            std::cmp::Ordering::Greater => floor,
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

    /// Claude Code as the preview node offers it: a model, thinking, mode,
    /// and three choices ("Off", "Off", "Default").
    fn claude() -> Presence {
        Presence {
            model: true,
            thinking: true,
            mode: true,
            features: vec![Feature::Select(3), Feature::Select(3), Feature::Select(7)],
        }
    }

    fn near(value: f32, expected: f32) {
        assert!(
            value > expected.mul_add(1.0, -0.01) && value < expected.mul_add(1.0, 0.01),
            "{value} is not {expected}"
        );
    }

    #[test]
    fn floors_are_pasos() {
        let pointer = Geometry::of(false);
        // 120 + 102 + 150 + (69 + 69 + 97) + 5 gaps of 4.
        near(floor(Density::Full, &claude(), pointer), 627.0);
        // 96 + 28 + 132 + 28 + 3 gaps.
        near(floor(Density::Condensed, &claude(), pointer), 296.0);
        // 92 + 28 + 28 + 28 + 3 gaps.
        near(floor(Density::Tight, &claude(), pointer), 188.0);
    }

    #[test]
    fn a_wide_composer_is_full_and_narrowing_steps_down() {
        let pointer = Geometry::of(false);
        let presence = claude();
        assert_eq!(
            resolve(760.0, Density::Full, &presence, pointer),
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
    fn a_width_near_a_floor_does_not_flip() {
        let pointer = Geometry::of(false);
        let presence = claude();
        // 620 is under Full's floor (627) but within the hysteresis: kept.
        assert_eq!(
            resolve(620.0, Density::Full, &presence, pointer),
            Density::Full
        );
        // From Condensed, Full comes back only at its floor plus 12.
        assert_eq!(
            resolve(630.0, Density::Condensed, &presence, pointer),
            Density::Condensed
        );
        assert_eq!(
            resolve(640.0, Density::Condensed, &presence, pointer),
            Density::Full
        );
    }
}
