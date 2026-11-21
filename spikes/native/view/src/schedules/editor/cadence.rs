//! The editor writes cron presets or an explicit cron expression. A stored
//! elapsed cadence remains a typed value until a person changes the rule.
use crate::localization::{FormatError, Number, Translator};
use vessel_proto::automation::{
    Cadence, CronExpr, InvalidAutomationText, InvalidCron, TimeZoneName,
};

/// The cron presets and the display identity of a custom expression.
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
    /// The visible expression does not match a preset.
    Custom,
}

impl Choice {
    /// Preset order from Paseo. Custom expressions are entered in the field.
    pub const PRESETS: [Self; 5] = [
        Self::Minute,
        Self::Hour,
        Self::Daily,
        Self::Weekdays,
        Self::Mondays,
    ];

    const fn expression(self) -> Option<&'static str> {
        match self {
            Self::Minute => Some("* * * * *"),
            Self::Hour => Some("0 * * * *"),
            Self::Daily => Some("0 9 * * *"),
            Self::Weekdays => Some("0 9 * * 1-5"),
            Self::Mondays => Some("0 9 * * 1"),
            Self::Custom => None,
        }
    }

    fn of(expression: &str) -> Self {
        Self::PRESETS
            .into_iter()
            .find(|choice| choice.expression() == Some(expression.trim()))
            .unwrap_or(Self::Custom)
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
            Self::Custom => words.form_custom(),
        }
    }
}

/// Invalid input retains its original parse error.
#[derive(Debug, thiserror::Error)]
pub enum Issue {
    /// A stored rule introduced by a newer protocol.
    #[error("unknown stored cadence")]
    Unknown,
    /// Missing or malformed IANA zone.
    #[error("invalid IANA zone: {0}")]
    Zone(#[source] InvalidAutomationText),
    /// The existing cron parser refused the expression.
    #[error("invalid cron expression: {0}")]
    Cron(#[source] InvalidCron),
}

impl Issue {
    /// The editor replaces the preview with one localized error.
    #[must_use]
    pub fn label<'a>(&self, catalog: &'a Translator) -> &'a str {
        let words = catalog.schedules();
        match self {
            Self::Unknown => words.newer_value(),
            Self::Zone(_) => words.form_zone_invalid(),
            Self::Cron(_) => words.form_cron_invalid(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZoneSource {
    Default,
    Stored,
    Manual,
}

/// Edit fields with a cached checked cadence for save and preview.
#[derive(Debug)]
pub struct Draft {
    choice: Choice,
    expression: String,
    zone: String,
    zone_source: ZoneSource,
    checked: Result<Cadence, Issue>,
}

impl Draft {
    /// A new schedule starts with the hourly cron preset.
    #[must_use]
    pub fn create(zone: Option<&TimeZoneName>) -> Self {
        let expression = "0 * * * *".to_owned();
        let zone = zone.map_or_else(String::new, ToString::to_string);
        let checked = cron(&expression, &zone);
        Self {
            choice: Choice::Hour,
            expression,
            zone,
            zone_source: ZoneSource::Default,
            checked,
        }
    }

    /// Open the exact stored rule. A legacy interval has no editable count
    /// or unit, and its original anchor survives a save of unrelated fields.
    pub(super) fn open(cadence: &Cadence, zone: Option<&TimeZoneName>) -> Self {
        let mut draft = Self::create(zone);
        match cadence {
            Cadence::Cron { expr, zone } => {
                expr.as_str().clone_into(&mut draft.expression);
                draft.zone = zone.to_string();
                draft.zone_source = ZoneSource::Stored;
                draft.choice = Choice::of(expr.as_str());
            }
            Cadence::Every { .. } => {
                draft.expression.clear();
                draft.choice = Choice::Custom;
            }
            Cadence::Unknown => {
                draft.expression.clear();
                draft.choice = Choice::Custom;
                draft.checked = Err(Issue::Unknown);
                return draft;
            }
        }
        draft.checked = Ok(cadence.clone());
        draft
    }

    /// The current preset identity.
    #[must_use]
    pub const fn choice(&self) -> Choice {
        self.choice
    }

    /// The visible cron expression, blank for a retained elapsed rule.
    #[must_use]
    pub fn expression(&self) -> &str {
        &self.expression
    }

    /// The rule's IANA zone, if available.
    #[must_use]
    pub fn zone(&self) -> &str {
        &self.zone
    }

    /// Show editable zone input only when a cron needs a manual choice.
    #[must_use]
    pub fn needs_zone_field(&self) -> bool {
        !self.is_elapsed() && (self.zone.is_empty() || self.zone_source == ZoneSource::Manual)
    }

    /// Whether the wire cadence is still the stored elapsed record.
    #[must_use]
    pub const fn is_elapsed(&self) -> bool {
        matches!(self.checked, Ok(Cadence::Every { .. }))
    }

    /// The preview and save edges consume the same validated value.
    /// # Errors
    /// Returns the last rule validation issue.
    pub fn checked(&self) -> Result<&Cadence, &Issue> {
        self.checked.as_ref()
    }

    /// Choosing a cron preset deliberately replaces the existing rule.
    pub fn select(&mut self, choice: Choice) {
        if let Some(expression) = choice.expression() {
            self.set_expression(expression.to_owned());
        }
    }

    /// Assigning a cron expression is the other explicit replacement path.
    pub fn set_expression(&mut self, text: String) {
        if self.is_elapsed() && text.trim().is_empty() {
            return;
        }
        self.choice = Choice::of(&text);
        self.expression = text;
        self.checked = cron(&self.expression, &self.zone);
    }

    /// Set a manual IANA name when neither the device nor machine can supply it.
    pub fn set_zone(&mut self, text: String) {
        self.zone = text;
        self.zone_source = ZoneSource::Manual;
        self.checked = cron(&self.expression, &self.zone);
    }

    /// Fill the device or machine's zone without changing an elapsed record.
    pub fn default_zone(&mut self, zone: Option<&TimeZoneName>) {
        if self.zone_source == ZoneSource::Default {
            self.zone = zone.map_or_else(String::new, ToString::to_string);
            if !self.is_elapsed() {
                self.checked = cron(&self.expression, &self.zone);
            }
        }
    }

    pub(super) fn host_changed(&mut self) {
        self.default_zone(None);
    }
}

fn cron(expression: &str, zone: &str) -> Result<Cadence, Issue> {
    let expr = expression.trim().parse::<CronExpr>().map_err(Issue::Cron)?;
    let zone = zone.trim().parse().map_err(Issue::Zone)?;
    Ok(Cadence::Cron { expr, zone })
}

/// Feedback for a rule, naming the raw count of a stored elapsed schedule.
/// # Errors
/// Locale formatting failed.
pub fn description(rule: &Cadence, catalog: &Translator) -> Result<String, FormatError> {
    if let Cadence::Every { seconds, .. } = rule {
        catalog
            .schedules()
            .form_elapsed_seconds(Number::count(seconds.get()))
    } else {
        super::super::locale::cadence(rule, catalog)
    }
}
