//! Interval form values and the cron input boundary. Units never round a
//! stored interval; every raw field becomes a typed value only at this edge.
//!
//! [Decision Log]
//! - Purpose: a number and unit a person recognises, without changing a rule.
//! - Constraint: ninety seconds cannot be rounded to two minutes; the node's
//!   cron grammar is the one parser, and its errors must keep their sources.
//! - Alternatives: raw seconds only, or the reference's rounded minutes.
//! - Choice: largest lossless form unit and checked conversion at the edge;
//!   human descriptions may decompose seconds without changing stored values.
//! - Why and cost: understandable controls with exact semantics; the UI must
//!   format unit-specific errors rather than showing diagnostic Display.

use vessel_proto::automation::{
    Cadence, CronExpr, CronField, EverySeconds, InvalidAutomationText, InvalidCron, OutOfRange,
};

/// Units a person can choose in the interval form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalUnit {
    /// Exact seconds, for intervals such as ninety seconds.
    Seconds,
    /// Sixty elapsed seconds.
    Minutes,
    /// 3,600 elapsed seconds.
    Hours,
    /// 86,400 elapsed seconds, independent of daylight saving.
    Days,
}

impl IntervalUnit {
    /// Largest to smallest, for lossless form decomposition.
    pub const DESCENDING: [Self; 4] = [Self::Days, Self::Hours, Self::Minutes, Self::Seconds];

    /// The reference offers minutes, hours and days. Seconds is retained only
    /// for an opened interval that would otherwise lose its exact value.
    #[must_use]
    pub fn choices(opened: Option<EverySeconds>) -> &'static [Self] {
        if opened.is_some_and(|value| IntervalParts::of(value).unit() == Self::Seconds) {
            &[Self::Seconds, Self::Minutes, Self::Hours, Self::Days]
        } else {
            &[Self::Minutes, Self::Hours, Self::Days]
        }
    }

    /// Elapsed seconds represented by one unit.
    #[must_use]
    pub const fn seconds(self) -> u32 {
        match self {
            Self::Seconds => 1,
            Self::Minutes => 60,
            Self::Hours => 3_600,
            Self::Days => 86_400,
        }
    }

    /// Inclusive limits for a whole-number field in this unit.
    #[must_use]
    pub fn limits(self) -> (u32, u32) {
        let min = EverySeconds::MIN
            .saturating_add(self.seconds().saturating_sub(1))
            .checked_div(self.seconds())
            .unwrap_or_default()
            .max(1);
        let max = EverySeconds::MAX
            .checked_div(self.seconds())
            .unwrap_or_default();
        (min, max)
    }
}

/// A stored interval opened in the largest unit that preserves every second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntervalParts {
    count: u32,
    unit: IntervalUnit,
}

impl IntervalParts {
    /// Ninety seconds stays ninety seconds; 7,200 seconds opens as two hours.
    #[must_use]
    pub fn of(seconds: EverySeconds) -> Self {
        for unit in IntervalUnit::DESCENDING {
            if seconds.get().checked_rem(unit.seconds()) == Some(0) {
                return Self {
                    count: seconds
                        .get()
                        .checked_div(unit.seconds())
                        .unwrap_or_default(),
                    unit,
                };
            }
        }
        // Seconds divide every representable interval; retaining the original
        // value is the safe total fallback if the unit list ever changes.
        Self {
            count: seconds.get(),
            unit: IntervalUnit::Seconds,
        }
    }

    /// The whole-number input.
    #[must_use]
    pub const fn count(self) -> u32 {
        self.count
    }

    /// The selected unit.
    #[must_use]
    pub const fn unit(self) -> IntervalUnit {
        self.unit
    }
}

/// Which part of one invalid cron field needs attention. This is derived from
/// that input field's notation at the edge, never from the node error's prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldPart {
    /// A literal/name is invalid.
    Value,
    /// A range is invalid.
    Range,
    /// A step is invalid.
    Step,
}

/// UI meaning of a cron grammar failure, apart from its retained source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CronIssue {
    /// More than the wire's byte limit.
    TooLong,
    /// Not five fields.
    FieldCount,
    /// Unknown shorthand.
    Shorthand,
    /// One field's notation is invalid.
    Field {
        /// Which field.
        field: CronField,
        /// Which notation.
        part: FieldPart,
    },
}

impl CronIssue {
    fn of(source: &InvalidCron, input: &str) -> Self {
        match source {
            InvalidCron::TooLong => Self::TooLong,
            InvalidCron::FieldCount { .. } => Self::FieldCount,
            InvalidCron::Shorthand => Self::Shorthand,
            InvalidCron::Field { field, .. } => {
                let fields = [
                    CronField::Minute,
                    CronField::Hour,
                    CronField::DayOfMonth,
                    CronField::Month,
                    CronField::DayOfWeek,
                ];
                let text = fields
                    .into_iter()
                    .zip(input.split_whitespace())
                    .find(|(kind, _)| kind == field)
                    .map(|(_, text)| text)
                    .unwrap_or_default();
                let part = if text.contains('/') {
                    FieldPart::Step
                } else if text.contains('-') {
                    FieldPart::Range
                } else {
                    FieldPart::Value
                };
                Self::Field {
                    field: *field,
                    part,
                }
            }
        }
    }
}

/// Typed errors retain diagnostic sources. Display is log copy; a screen calls
/// `Lang::cadence_error` for the person's language instead.
#[derive(Debug, thiserror::Error)]
pub enum CadenceError {
    /// Not an integer.
    #[error("interval is not a whole number: {source}")]
    Number {
        /// Unit whose field was entered.
        unit: IntervalUnit,
        /// Original parse failure.
        #[source]
        source: std::num::ParseIntError,
    },
    /// Multiplication exceeds the integer representation.
    #[error("interval conversion overflow: {source}")]
    Overflow {
        /// Unit whose field was entered.
        unit: IntervalUnit,
        /// Original conversion failure.
        #[source]
        source: std::num::TryFromIntError,
    },
    /// Outside the node's interval bounds.
    #[error("interval out of range: {source}")]
    Range {
        /// Unit whose limits the screen must name.
        unit: IntervalUnit,
        /// Original bounded-value failure.
        #[source]
        source: OutOfRange,
    },
    /// Empty custom rule input.
    #[error("cron expression is empty")]
    EmptyExpression,
    /// The existing grammar refused the rule.
    #[error("cron syntax: {source}")]
    Cron {
        /// UI interpretation, derived without parsing source prose.
        issue: CronIssue,
        /// Original grammar error.
        #[source]
        source: InvalidCron,
    },
    /// Malformed IANA zone shape; existence is the node's to decide.
    #[error("time zone name: {0}")]
    Zone(#[source] InvalidAutomationText),
}

/// Raw form fields, parsed once before becoming a stored cadence.
#[derive(Debug, Clone, Copy)]
pub enum CadenceInput<'a> {
    /// A number and its explicit unit.
    Every {
        /// The number as entered.
        count: &'a str,
        /// The person's chosen unit.
        unit: IntervalUnit,
    },
    /// Custom cron, the form's last choice.
    Cron {
        /// The expression as entered.
        expression: &'a str,
        /// Explicit IANA zone.
        zone: &'a str,
    },
}

impl CadenceInput<'_> {
    /// Parse the form edge, preserving exact intervals and typed grammar errors.
    ///
    /// # Errors
    /// A number, range, conversion, cron syntax or zone-name failure.
    pub fn parse(self) -> Result<Cadence, CadenceError> {
        match self {
            Self::Every { count, unit } => {
                let count = count
                    .trim()
                    .parse::<i128>()
                    .map_err(|source| CadenceError::Number { unit, source })?;
                // Clamp only the conversion operand to a value that still
                // cannot pass EverySeconds; the user's rule is never rounded.
                let seconds = count.saturating_mul(i128::from(unit.seconds()));
                let seconds = u32::try_from(seconds)
                    .map_err(|source| CadenceError::Overflow { unit, source })?;
                let seconds = EverySeconds::try_from(seconds)
                    .map_err(|source| CadenceError::Range { unit, source })?;
                Ok(Cadence::Every {
                    seconds,
                    anchor: None,
                })
            }
            Self::Cron { expression, zone } => {
                let expr = parse_expression(expression)?;
                let zone = zone.parse().map_err(CadenceError::Zone)?;
                Ok(Cadence::Cron { expr, zone })
            }
        }
    }
}

/// Parse expression text once while retaining a zone already chosen from the
/// typed picker. The raw-zone input boundary delegates here after its own parse.
/// # Errors
/// Empty or invalid expressions retain the original grammar failure.
pub fn cron(
    expression: &str,
    zone: vessel_proto::automation::TimeZoneName,
) -> Result<Cadence, CadenceError> {
    let expr = parse_expression(expression)?;
    Ok(Cadence::Cron { expr, zone })
}
fn parse_expression(expression: &str) -> Result<CronExpr, CadenceError> {
    if expression.trim().is_empty() {
        return Err(CadenceError::EmptyExpression);
    }
    expression
        .parse::<CronExpr>()
        .map_err(|source| CadenceError::Cron {
            issue: CronIssue::of(&source, expression),
            source,
        })
}
