//! Last report is historical; it is never called a live connection or a
//! last successful sync. Local dates use the viewer's explicit offset.
use super::{Lang, SyncWords};
use crate::devices::time::{Age, exact_time};
use chrono::{DateTime, Datelike as _, FixedOffset, Utc};

impl SyncWords {
    /// Full last-report sentence under a member badge; numeric ages, local dates.
    #[must_use]
    pub fn reported(&self, age: Option<Age>) -> String {
        match (self.lang, age) {
            (Lang::En, None) => "No report time received.".to_owned(),
            (Lang::Ko, None) => "보고받은 시각이 없습니다.".to_owned(),
            (Lang::En, Some(Age::Now)) => "Last reported just now.".to_owned(),
            (Lang::En, Some(Age::Minutes(1))) => "Last reported 1 minute ago.".to_owned(),
            (Lang::En, Some(Age::Minutes(n))) => format!("Last reported {n} minutes ago."),
            (Lang::En, Some(Age::Hours(1))) => "Last reported 1 hour ago.".to_owned(),
            (Lang::En, Some(Age::Hours(n))) => format!("Last reported {n} hours ago."),
            (Lang::En, Some(Age::Days(1))) => "Last reported 1 day ago.".to_owned(),
            (Lang::En, Some(Age::Days(n))) => format!("Last reported {n} days ago."),
            (Lang::En, Some(Age::Date(at))) => format!("Last reported on {}.", at.format("%b %-d")),
            (Lang::Ko, Some(Age::Now)) => "마지막 보고: 방금.".to_owned(),
            (Lang::Ko, Some(Age::Minutes(n))) => format!("마지막 보고: {n}분 전."),
            (Lang::Ko, Some(Age::Hours(n))) => format!("마지막 보고: {n}시간 전."),
            (Lang::Ko, Some(Age::Days(n))) => format!("마지막 보고: {n}일 전."),
            (Lang::Ko, Some(Age::Date(at))) => {
                format!("마지막 보고: {}월 {}일.", at.month(), at.day())
            }
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
