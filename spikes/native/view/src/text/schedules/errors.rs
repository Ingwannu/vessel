//! Every form error's UI copy, separately from diagnostic Display. Never
//! translate the protocol's error prose or require the app to inspect variants.

use vessel_proto::automation::{CronExpr, CronField};

use crate::Lang;
use crate::schedules::cadence::{CadenceError, CronIssue, FieldPart, IntervalUnit};
use crate::schedules::editor::EditError;

impl Lang {
    /// Localised interval/cron/zone error; the source remains on the typed error.
    #[must_use]
    pub fn cadence_error(self, error: &CadenceError) -> String {
        match error {
            CadenceError::Number { unit, source } => {
                if source.kind() == &std::num::IntErrorKind::PosOverflow
                    || source.kind() == &std::num::IntErrorKind::NegOverflow
                {
                    self.interval_range(*unit)
                } else {
                    match self {
                        Self::En => "Enter a whole number".to_owned(),
                        Self::Ko => "정수를 입력하세요".to_owned(),
                    }
                }
            }
            CadenceError::Overflow { unit, .. } | CadenceError::Range { unit, .. } => {
                self.interval_range(*unit)
            }
            CadenceError::EmptyExpression => match self {
                Self::En => "Enter a cron expression",
                Self::Ko => "예약 규칙을 입력하세요",
            }
            .to_owned(),
            CadenceError::Zone(_) => match self {
                Self::En => "Enter an IANA time zone, such as Europe/London",
                Self::Ko => "Asia/Seoul 같은 IANA 시간대를 입력하세요",
            }
            .to_owned(),
            CadenceError::Cron { issue, .. } => self.cron_issue(*issue),
        }
    }

    /// Localised reason an opened definition cannot be replaced safely.
    #[must_use]
    pub const fn edit_error(self, error: EditError) -> &'static str {
        let words = self.schedules();
        match error {
            EditError::Unknown => words.newer_machine,
            EditError::Ended => words.edit_ended,
            EditError::SessionMode => words.structured_only,
        }
    }

    fn interval_range(self, unit: IntervalUnit) -> String {
        let (min, max) = unit.limits();
        let label = self.interval_unit(unit);
        match self {
            Self::En => format!("Enter {min} to {max} {label}s"),
            Self::Ko => format!("{min}~{max}{label} 사이로 입력하세요"),
        }
    }

    fn cron_issue(self, issue: CronIssue) -> String {
        match issue {
            CronIssue::TooLong => match self {
                Self::En => format!(
                    "Cron expressions must be at most {} bytes",
                    CronExpr::MAX_LEN
                ),
                Self::Ko => format!("예약 규칙은 {}바이트 이하여야 합니다", CronExpr::MAX_LEN),
            },
            CronIssue::FieldCount => match self {
                Self::En => "Cron expressions must have 5 fields",
                Self::Ko => "예약 규칙에는 항목이 5개 있어야 합니다",
            }
            .to_owned(),
            CronIssue::Shorthand => match self {
                Self::En => "Choose a supported cron shorthand",
                Self::Ko => "지원하는 예약 규칙 축약형을 선택하세요",
            }
            .to_owned(),
            CronIssue::Field { field, part } => {
                let field = self.cron_field(field);
                let part = match (self, part) {
                    (Self::En, FieldPart::Value) => "value",
                    (Self::En, FieldPart::Range) => "range",
                    (Self::En, FieldPart::Step) => "step",
                    (Self::Ko, FieldPart::Value) => "값이",
                    (Self::Ko, FieldPart::Range) => "범위가",
                    (Self::Ko, FieldPart::Step) => "간격이",
                };
                match self {
                    Self::En => format!("Invalid {field} {part}"),
                    Self::Ko => format!("{field} {part} 올바르지 않습니다"),
                }
            }
        }
    }

    const fn cron_field(self, field: CronField) -> &'static str {
        match (self, field) {
            (Self::En, CronField::Minute) => "minute",
            (Self::En, CronField::Hour) => "hour",
            (Self::En, CronField::DayOfMonth) => "day-of-month",
            (Self::En, CronField::Month) => "month",
            (Self::En, CronField::DayOfWeek) => "day-of-week",
            (Self::Ko, CronField::Minute) => "분",
            (Self::Ko, CronField::Hour) => "시간",
            (Self::Ko, CronField::DayOfMonth) => "날짜",
            (Self::Ko, CronField::Month) => "월",
            (Self::Ko, CronField::DayOfWeek) => "요일",
        }
    }
}
