//! The node parses the same bounded rule types and computes the preview.
//! Keeping the original elapsed rule lets edits away from and back to its
//! interval retain its phase.

use crate::localization::Translator;
use vessel_proto::automation::{
    Cadence, CronExpr, EverySeconds, InvalidAutomationText, InvalidCron, OutOfRange, TimeZoneName,
};

use super::super::cadence::{IntervalParts, IntervalUnit};

/// Wall-clock options, followed by the explicit elapsed and custom rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// Every wall-clock minute.
    Minute,
    /// At minute zero of every hour.
    Hour,
    /// Every day at 09:00.
    Daily,
    /// Weekdays at 09:00.
    Weekdays,
    /// Mondays at 09:00.
    Mondays,
    /// Exact elapsed interval.
    Interval,
    /// A custom five-field expression.
    Custom,
}

impl Choice {
    /// The standard preset order from Paseo, then elapsed and custom.
    pub const ALL: [Self; 7] = [
        Self::Minute,
        Self::Hour,
        Self::Daily,
        Self::Weekdays,
        Self::Mondays,
        Self::Interval,
        Self::Custom,
    ];

    const fn expression(self) -> Option<&'static str> {
        match self {
            Self::Minute => Some("* * * * *"),
            Self::Hour => Some("0 * * * *"),
            Self::Daily => Some("0 9 * * *"),
            Self::Weekdays => Some("0 9 * * 1-5"),
            Self::Mondays => Some("0 9 * * 1"),
            Self::Interval | Self::Custom => None,
        }
    }

    /// The localized choice name.
    #[must_use]
    pub fn label(self, catalog: &Translator) -> &str {
        let words = catalog.schedules();
        match self {
            Self::Minute => words.preset_minute(),
            Self::Hour => words.preset_hour(),
            Self::Daily => words.preset_daily(),
            Self::Weekdays => words.preset_weekdays(),
            Self::Mondays => words.preset_mondays(),
            Self::Interval => words.form_interval(),
            Self::Custom => words.form_custom(),
        }
    }
}

/// Invalid input retains the original parse error at the boundary.
#[derive(Debug, thiserror::Error)]
pub enum Issue {
    /// An unknown IANA zone or no selected zone.
    #[error("invalid IANA zone: {0}")]
    Zone(#[source] InvalidAutomationText),
    /// The existing cron parser refused the expression.
    #[error("invalid cron expression: {0}")]
    Cron(#[source] InvalidCron),
    /// An elapsed number is invalid.
    #[error("invalid interval count: {0}")]
    Count(#[source] std::num::ParseIntError),
    /// Conversion to elapsed seconds overflowed.
    #[error("interval exceeds the wire representation")]
    Overflow,
    /// The node's typed elapsed bounds refused the value.
    #[error("invalid interval: {0}")]
    Interval(#[source] OutOfRange),
}

impl Issue {
    /// The view uses whole messages for invalid rules.
    #[must_use]
    pub fn label<'a>(&self, catalog: &'a Translator) -> &'a str {
        let words = catalog.schedules();
        match self {
            Self::Zone(_) => words.form_zone_invalid(),
            Self::Cron(_) => words.form_cron_invalid(),
            Self::Count(_) | Self::Overflow | Self::Interval(_) => words.form_interval_invalid(),
        }
    }
}

/// Edit fields with a cached checked cadence for save and preview.
#[derive(Debug)]
pub struct Draft {
    choice: Choice,
    expression: String,
    zone: String,
    count: String,
    unit: IntervalUnit,
    opened: Option<Cadence>,
    explicit_zone: bool,
    checked: Result<Cadence, Issue>,
}

impl Draft {
    /// A new schedule starts at the next wall-clock hour.
    #[must_use]
    pub fn create(zone: Option<&TimeZoneName>) -> Self {
        let expression = "0 * * * *".to_owned();
        let zone = zone.map_or_else(String::new, ToString::to_string);
        let checked = cron(&expression, &zone);
        Self {
            choice: Choice::Hour,
            expression,
            zone,
            count: "1".to_owned(),
            unit: IntervalUnit::Hours,
            opened: None,
            explicit_zone: false,
            checked,
        }
    }

    /// Open the exact stored cadence, including its original elapsed anchor.
    pub(super) fn open(cadence: &Cadence, zone: Option<&TimeZoneName>) -> Option<Self> {
        let mut draft = Self::create(zone);
        match cadence {
            Cadence::Cron { expr, zone } => {
                draft.expression = expr.as_str().to_owned();
                draft.zone = zone.to_string();
                draft.explicit_zone = true;
                draft.choice = Choice::ALL
                    .into_iter()
                    .find(|choice| choice.expression() == Some(expr.as_str()))
                    .unwrap_or(Choice::Custom);
            }
            Cadence::Every { seconds, .. } => {
                let parts = IntervalParts::of(*seconds);
                draft.choice = Choice::Interval;
                draft.count = parts.count().to_string();
                draft.unit = parts.unit();
            }
            Cadence::Unknown => return None,
        }
        draft.opened = Some(cadence.clone());
        draft.checked = Ok(cadence.clone());
        Some(draft)
    }

    /// Selected rule shape.
    #[must_use]
    pub const fn choice(&self) -> Choice {
        self.choice
    }

    /// Advanced expression shown to the user.
    #[must_use]
    pub fn expression(&self) -> &str {
        &self.expression
    }

    /// Cron zone field, possibly empty until the device or machine has a zone.
    #[must_use]
    pub fn zone(&self) -> &str {
        &self.zone
    }

    /// Raw elapsed magnitude.
    #[must_use]
    pub fn count(&self) -> &str {
        &self.count
    }

    /// Current elapsed unit.
    #[must_use]
    pub const fn unit(&self) -> IntervalUnit {
        self.unit
    }

    /// Seconds remains available for legacy intervals that require it.
    #[must_use]
    pub fn units(&self) -> &'static [IntervalUnit] {
        const STANDARD: &[IntervalUnit] =
            &[IntervalUnit::Minutes, IntervalUnit::Hours, IntervalUnit::Days];
        if matches!(self.opened, Some(Cadence::Every { seconds, .. })
            if IntervalParts::of(seconds).unit() == IntervalUnit::Seconds)
        {
            &IntervalUnit::DESCENDING
        } else {
            STANDARD
        }
    }

    /// The preview and save edges consume the same bounded value.
    /// # Errors
    /// Returns the last input validation issue.
    pub fn checked(&self) -> Result<&Cadence, &Issue> {
        self.checked.as_ref()
    }

    /// Change the preset without replacing the selected zone.
    pub fn select(&mut self, choice: Choice) {
        self.choice = choice;
        if let Some(expression) = choice.expression() {
            self.expression = expression.to_owned();
        }
        self.validate();
    }

    /// Commit text from the advanced expression field.
    pub fn set_expression(&mut self, text: String) {
        self.expression = text;
        self.validate();
    }

    /// Commit text from the IANA zone field.
    pub fn set_zone(&mut self, text: String) {
        self.zone = text;
        self.explicit_zone = true;
        self.validate();
    }

    /// Fill a zone from the device or machine before the user edits it.
    pub fn default_zone(&mut self, zone: Option<&TimeZoneName>) {
        if !self.explicit_zone {
            self.zone = zone.map_or_else(String::new, ToString::to_string);
            self.validate();
        }
    }

    pub(super) fn host_changed(&mut self) {
        self.default_zone(None);
    }

    /// Commit text from the elapsed count field.
    pub fn set_count(&mut self, text: String) {
        self.count = text;
        self.validate();
    }

    /// Choose a different elapsed unit for the current count.
    pub fn set_unit(&mut self, unit: IntervalUnit) {
        self.unit = unit;
        self.validate();
    }

    fn validate(&mut self) {
        self.checked = if self.choice == Choice::Interval {
            interval(&self.count, self.unit, self.opened.as_ref())
        } else {
            cron(&self.expression, &self.zone)
        };
    }
}

fn cron(expression: &str, zone: &str) -> Result<Cadence, Issue> {
    let expr = expression.trim().parse::<CronExpr>().map_err(Issue::Cron)?;
    let zone = zone.trim().parse().map_err(Issue::Zone)?;
    Ok(Cadence::Cron { expr, zone })
}

fn interval(count: &str, unit: IntervalUnit, opened: Option<&Cadence>) -> Result<Cadence, Issue> {
    let count = count.trim().parse::<u32>().map_err(Issue::Count)?;
    let seconds = count.checked_mul(unit.seconds()).ok_or(Issue::Overflow)?;
    let seconds = EverySeconds::try_from(seconds).map_err(Issue::Interval)?;
    if let Some(opened @ Cadence::Every { seconds: previous, .. }) = opened
        && *previous == seconds
    {
        return Ok(opened.clone());
    }
    Ok(Cadence::Every { seconds, anchor: None })
}
