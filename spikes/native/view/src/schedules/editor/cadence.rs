//! The form's cadence choices keep legacy intervals exact. Text is parsed only
//! by the edit edge; changing a typed zone never reparses the expression.
//!
//! [Decision Log]
//! - Purpose: presets first, elapsed interval when needed, raw cron last.
//! - Constraint: opening90s must not round it or change its anchored phase.
//! - Choice: private raw fields with a cached typed cadence/error. Only setters
//!   replace that cache; an opened cadence is already parsed by the wire.
//! - Alternative: reparse every render, or convert every stored interval to
//!   a wall-clock cron. The latter silently changes elapsed/DST semantics.
//! - Cost: raw text and its validation coexist behind one mutation boundary.
use super::{CadenceError, CadenceInput, EditError, IntervalParts, IntervalUnit};
use crate::localization::Translator;
use vessel_proto::automation::{Cadence, EverySeconds, TimeZoneName};

/// Approved preset order from schedule-cadence-options.test.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// Every wall-clock minute.
    Minute,
    /// At minute zero of every hour.
    Hour,
    /// Every day at09:00.
    Daily,
    /// Monday through Friday at09:00.
    Weekdays,
    /// Monday at09:00.
    Mondays,
}
impl Preset {
    /// The reference menu order; custom is a separate last choice.
    pub const ALL: [Self; 5] = [
        Self::Minute,
        Self::Hour,
        Self::Daily,
        Self::Weekdays,
        Self::Mondays,
    ];
    /// Literal input handed once to the existing protocol parser at selection.
    #[must_use]
    pub const fn expression(self) -> &'static str {
        match self {
            Self::Minute => "* * * * *",
            Self::Hour => "0 * * * *",
            Self::Daily => "0 9 * * *",
            Self::Weekdays => "0 9 * * 1-5",
            Self::Mondays => "0 9 * * 1",
        }
    }
    /// Short picker words, distinct from a saved rule's full description.
    #[must_use]
    pub fn label(self, catalog: &Translator) -> &str {
        let words = catalog.schedules();
        match self {
            Self::Minute => words.preset_minute(),
            Self::Hour => words.preset_hour(),
            Self::Daily => words.preset_daily(),
            Self::Weekdays => words.preset_weekdays(),
            Self::Mondays => words.preset_mondays(),
        }
    }
    fn of(cadence: &Cadence) -> Option<Self> {
        match cadence {
            Cadence::Cron { expr, .. } => Self::ALL
                .into_iter()
                .find(|preset| preset.expression() == expr.as_str().trim()),
            Cadence::Every { .. } | Cadence::Unknown => None,
        }
    }
}
/// New forms start at the next wall-clock hour in the explicitly known viewer
/// zone, as the reference normalizes its default one-hour interval. An absent
/// viewer zone must be selected before calling this; UTC is not inferred.
/// # Errors
/// Preserve the existing protocol parser's typed failure.
pub fn initial(zone: TimeZoneName) -> Result<Cadence, CadenceError> {
    super::super::cadence::cron(Preset::Hour.expression(), zone)
}
/// Which control the form exposes; custom never precedes the presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// A familiar fixed-time shape.
    Preset(Preset),
    /// Exact elapsed number/unit, preserving opened legacy values.
    Interval,
    /// Advanced expression input.
    Custom,
}
#[derive(Debug)]
enum Fields {
    Cron(String),
    Interval { count: String, unit: IntervalUnit },
}
/// One cadence edit, independently owned by its form visit.
#[derive(Debug)]
pub struct Draft {
    fields: Fields,
    checked: Result<Cadence, CadenceError>,
    zone: TimeZoneName,
    choice: Choice,
    opened_interval: Option<EverySeconds>,
}
impl Draft {
    /// Open a typed rule without reparsing or losing its phase. The viewer zone
    /// is used only when an interval switches to a cron rule.
    /// # Errors
    /// Unknown wire variants cannot be replaced by this client.
    pub fn open(cadence: Cadence, viewer: TimeZoneName) -> Result<Self, EditError> {
        let (fields, zone, choice, opened_interval) = match &cadence {
            Cadence::Cron { expr, zone } => (
                Fields::Cron(expr.as_str().to_owned()),
                zone.clone(),
                Preset::of(&cadence).map_or(Choice::Custom, Choice::Preset),
                None,
            ),
            Cadence::Every { seconds, .. } => {
                let parts = IntervalParts::of(*seconds);
                (
                    Fields::Interval {
                        count: parts.count().to_string(),
                        unit: parts.unit(),
                    },
                    viewer,
                    Choice::Interval,
                    Some(*seconds),
                )
            }
            Cadence::Unknown => return Err(EditError::Unknown),
        };
        Ok(Self {
            fields,
            checked: Ok(cadence),
            zone,
            choice,
            opened_interval,
        })
    }
    /// Current picker identity, including an explicit custom selection.
    #[must_use]
    pub const fn choice(&self) -> Choice {
        self.choice
    }
    /// The already validated selected zone; never a text field.
    #[must_use]
    pub const fn zone(&self) -> &TimeZoneName {
        &self.zone
    }
    /// Existing unit policy: seconds only when the opened value needs them.
    #[must_use]
    pub fn units(&self) -> &'static [IntervalUnit] {
        IntervalUnit::choices(self.opened_interval)
    }
    /// Current input text, for a field constructed/reset outside painting.
    #[must_use]
    pub fn text(&self) -> &str {
        match &self.fields {
            Fields::Cron(text) | Fields::Interval { count: text, .. } => text,
        }
    }
    /// Interval unit only while the interval field is visible.
    #[must_use]
    pub const fn unit(&self) -> Option<IntervalUnit> {
        match self.fields {
            Fields::Cron(_) => None,
            Fields::Interval { unit, .. } => Some(unit),
        }
    }
    /// Validated rule, never a raw expression string sent to the node.
    #[must_use]
    pub fn checked(&self) -> Result<&Cadence, &CadenceError> {
        self.checked.as_ref()
    }
    /// Select a reference preset without replacing an existing cron zone.
    pub fn preset(&mut self, preset: Preset) {
        self.cron(preset.expression().to_owned());
        self.choice = Choice::Preset(preset);
    }
    /// The advanced option is explicit even when its input matches a preset.
    pub fn custom(&mut self) {
        if matches!(self.fields, Fields::Interval { .. }) {
            self.cron(Preset::Hour.expression().to_owned());
        }
        self.choice = Choice::Custom;
    }
    /// One raw expression edit, through the node's single grammar parser.
    pub fn cron(&mut self, text: String) {
        self.checked = super::super::cadence::cron(&text, self.zone.clone());
        self.fields = Fields::Cron(text);
        self.choice = Choice::Custom;
    }
    /// One exact elapsed-number/unit edit. A changed interval intentionally
    /// drops the opened anchor; the node preview supplies its new canonical one.
    pub fn interval(&mut self, count: String, unit: IntervalUnit) {
        self.checked = CadenceInput::Every {
            count: &count,
            unit,
        }
        .parse();
        self.fields = Fields::Interval { count, unit };
        self.choice = Choice::Interval;
    }
    /// A chosen typed zone changes neither an interval nor parsed cron fields.
    pub fn set_zone(&mut self, zone: TimeZoneName) {
        if let Ok(Cadence::Cron { zone: current, .. }) = &mut self.checked {
            *current = zone.clone();
        }
        self.zone = zone;
    }
}
#[cfg(test)]
mod tests;
