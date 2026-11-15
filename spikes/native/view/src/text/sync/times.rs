//! Last report is historical; it is never called a live connection or a
//! last successful sync. Local dates use the viewer's explicit offset.
use super::{Lang, SyncWords};
use crate::devices::time::{Age, exact_time};
use chrono::{DateTime, Datelike as _, FixedOffset, Utc};

impl SyncWords {
    /// Compact age value used only in a member's single metadata line.
    pub(super) fn report_age(&self, age: Age) -> String {
        match (self.lang, age) {
            (Lang::En, Age::Now) => "just now".into(),
            (Lang::En, Age::Minutes(1)) => "1 minute ago".into(),
            (Lang::En, Age::Minutes(n)) => format!("{n} minutes ago"),
            (Lang::En, Age::Hours(1)) => "1 hour ago".into(),
            (Lang::En, Age::Hours(n)) => format!("{n} hours ago"),
            (Lang::En, Age::Days(1)) => "1 day ago".into(),
            (Lang::En, Age::Days(n)) => format!("{n} days ago"),
            (Lang::En, Age::Date(at)) => at.format("%b %-d").to_string(),
            (Lang::Ko, Age::Now) => "방금".into(),
            (Lang::Ko, Age::Minutes(n)) => format!("{n}분 전"),
            (Lang::Ko, Age::Hours(n)) => format!("{n}시간 전"),
            (Lang::Ko, Age::Days(n)) => format!("{n}일 전"),
            (Lang::Ko, Age::Date(at)) => format!("{}월 {}일", at.month(), at.day()),
        }
    }
    /// Exact member tooltip, viewer-local to seconds with a named offset.
    #[must_use]
    pub fn exact_report(&self, at: Option<DateTime<Utc>>, offset: FixedOffset) -> String {
        match (self.lang, at) {
            (Lang::En, Some(at)) => format!("Report: {}", exact_time(at, offset)),
            (Lang::Ko, Some(at)) => format!("보고: {}", exact_time(at, offset)),
            (Lang::En, None) => "No report time received.".to_owned(),
            (Lang::Ko, None) => "보고받은 시각이 없습니다.".to_owned(),
        }
    }
}
