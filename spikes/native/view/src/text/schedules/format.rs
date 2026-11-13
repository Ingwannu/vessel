//! Human cadence descriptions, following Paseo's schedule-format cases.

use chrono::{Timelike as _, Weekday};
use vessel_proto::automation::{Cadence, EverySeconds};

use crate::Lang;
use crate::schedules::cadence::{IntervalParts, IntervalUnit};
use crate::schedules::description::{CronShape, Days, cron_shape};
use crate::schedules::{Action, Product};

impl Lang {
    /// A new-session schedule and an existing chat's recurring prompt have
    /// different product names, as Paseo's Schedule/Heartbeat distinction.
    #[must_use]
    pub const fn schedule_product(self, product: Product) -> &'static str {
        match (self, product) {
            (Self::En, Product::Schedule) => "Schedule",
            (Self::Ko, Product::Schedule) => "예약",
            (Self::En, Product::RecurringPrompt) => "Recurring prompt",
            (Self::Ko, Product::RecurringPrompt) => "반복 프롬프트",
            (Self::En, Product::Unknown) => "Automation",
            (Self::Ko, Product::Unknown) => "자동 실행",
        }
    }

    /// Menu labels use the same product meaning as the editor's title.
    #[must_use]
    pub fn schedule_action(self, product: Product, action: Action) -> String {
        if product == Product::Schedule {
            return self.schedules().action(action).to_owned();
        }
        if action == Action::RunNow {
            return self.schedules().run_now.to_owned();
        }
        let noun = match (self, product) {
            (Self::En, Product::RecurringPrompt) => "recurring prompt",
            (Self::Ko, Product::RecurringPrompt) => "반복 프롬프트",
            (Self::En, Product::Unknown) => "automation",
            (Self::Ko, Product::Unknown) => "자동 실행",
            (Self::En | Self::Ko, Product::Schedule) => {
                return self.schedules().action(action).to_owned();
            }
        };
        let verb = match (self, action) {
            (Self::En, Action::Edit) => "Edit",
            (Self::En, Action::Pause) => "Pause",
            (Self::En, Action::Resume) => "Resume",
            (Self::En, Action::Delete) => "Delete",
            (Self::Ko, Action::Edit) => "수정",
            (Self::Ko, Action::Pause) => "일시 정지",
            (Self::Ko, Action::Resume) => "재개",
            (Self::Ko, Action::Delete) => "삭제",
            (Self::En | Self::Ko, Action::RunNow) => return self.schedules().run_now.to_owned(),
        };
        match self {
            Self::En => format!("{verb} {noun}"),
            Self::Ko => format!("{noun} {verb}"),
        }
    }
    /// Human cadence copy; the typed expression is only a fallback for shapes
    /// the reference does not describe. No interval is rounded for readability.
    #[must_use]
    pub fn schedule_cadence(self, cadence: &Cadence) -> String {
        match cadence {
            Cadence::Every { seconds, .. } => self.interval_label(*seconds),
            Cadence::Cron { expr, zone } => match cron_shape(expr) {
                Some(CronShape::EveryMinute) => match self {
                    Self::En => "Every minute",
                    Self::Ko => "매분",
                }
                .to_owned(),
                Some(CronShape::Hourly(at)) => match (self, at.minute()) {
                    (Self::En, 0) => "Every hour".to_owned(),
                    (Self::Ko, 0) => "매시 정각".to_owned(),
                    (Self::En, minute) => format!("Every hour at :{minute:02}"),
                    (Self::Ko, minute) => format!("매시 {minute}분"),
                },
                Some(CronShape::Fixed { at, days }) => {
                    let day = self.schedule_days(days);
                    let time = at.format("%H:%M");
                    match self {
                        Self::En => format!("{day} at {time} {}", zone.as_str()),
                        Self::Ko => format!("{day} {time} ({})", zone.as_str()),
                    }
                }
                None => expr.as_str().to_owned(),
            },
            Cadence::Unknown => self.schedules().newer_machine.to_owned(),
        }
    }

    fn interval_label(self, seconds: EverySeconds) -> String {
        let mut parts = Vec::with_capacity(4);
        let whole = IntervalParts::of(seconds);
        if whole.unit() == IntervalUnit::Seconds {
            let mut remaining = seconds.get();
            for unit in IntervalUnit::DESCENDING {
                let count = remaining.checked_div(unit.seconds()).unwrap_or_default();
                remaining = remaining.checked_rem(unit.seconds()).unwrap_or_default();
                if count > 0 {
                    parts.push((count, unit));
                }
            }
        } else {
            // Preserve the reference's whole units (25 hours, 61 minutes).
            // A seconds remainder needs decomposition to avoid its rounding.
            parts.push((whole.count(), whole.unit()));
        }
        if let [(1, unit)] = parts.as_slice() {
            return match (self, unit) {
                (Self::En, IntervalUnit::Seconds) => "Every second",
                (Self::En, IntervalUnit::Minutes) => "Every minute",
                (Self::En, IntervalUnit::Hours) => "Every hour",
                (Self::En, IntervalUnit::Days) => "Every day",
                (Self::Ko, IntervalUnit::Seconds) => "매초",
                (Self::Ko, IntervalUnit::Minutes) => "매분",
                (Self::Ko, IntervalUnit::Hours) => "매시간",
                (Self::Ko, IntervalUnit::Days) => "매일",
            }
            .to_owned();
        }
        let text = parts
            .into_iter()
            .map(|(count, unit)| self.interval_part(count, unit))
            .collect::<Vec<_>>()
            .join(" ");
        match self {
            Self::En => format!("Every {text}"),
            Self::Ko => format!("{text}마다"),
        }
    }

    fn interval_part(self, count: u32, unit: IntervalUnit) -> String {
        match self {
            Self::En => {
                let unit = self.interval_unit(unit);
                let plural = if count == 1 { "" } else { "s" };
                format!("{count} {unit}{plural}")
            }
            Self::Ko => format!("{count}{}", self.interval_unit(unit)),
        }
    }

    /// The unit selector's human label (also used in range errors).
    #[must_use]
    pub const fn interval_unit(self, unit: IntervalUnit) -> &'static str {
        match (self, unit) {
            (Self::En, IntervalUnit::Seconds) => "second",
            (Self::En, IntervalUnit::Minutes) => "minute",
            (Self::En, IntervalUnit::Hours) => "hour",
            (Self::En, IntervalUnit::Days) => "day",
            (Self::Ko, IntervalUnit::Seconds) => "초",
            (Self::Ko, IntervalUnit::Minutes) => "분",
            (Self::Ko, IntervalUnit::Hours) => "시간",
            (Self::Ko, IntervalUnit::Days) => "일",
        }
    }

    const fn schedule_days(self, days: Days) -> &'static str {
        match (self, days) {
            (Self::En, Days::Daily) => "Daily",
            (Self::Ko, Days::Daily) => "매일",
            (Self::En, Days::Weekdays) => "Weekdays",
            (Self::Ko, Days::Weekdays) => "평일",
            (Self::En, Days::Weekends) => "Weekends",
            (Self::Ko, Days::Weekends) => "주말",
            (Self::En, Days::On(day)) => match day {
                Weekday::Sun => "Sundays",
                Weekday::Mon => "Mondays",
                Weekday::Tue => "Tuesdays",
                Weekday::Wed => "Wednesdays",
                Weekday::Thu => "Thursdays",
                Weekday::Fri => "Fridays",
                Weekday::Sat => "Saturdays",
            },
            (Self::Ko, Days::On(day)) => match day {
                Weekday::Sun => "일요일",
                Weekday::Mon => "월요일",
                Weekday::Tue => "화요일",
                Weekday::Wed => "수요일",
                Weekday::Thu => "목요일",
                Weekday::Fri => "금요일",
                Weekday::Sat => "토요일",
            },
        }
    }
}
