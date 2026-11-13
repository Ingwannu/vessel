//! Device-row sentences and action failures.

use super::{DevicesWords, Lang};
use crate::devices::{
    DeviceState, Problem,
    time::{Age, exact_time},
};
use chrono::{DateTime, Datelike as _, FixedOffset, Utc};

impl DevicesWords {
    /// A short credential-state badge, distinct from its explanation.
    #[must_use]
    pub const fn state(&self, state: DeviceState) -> &'static str {
        match (self.lang, state) {
            (Lang::En, DeviceState::Paired) => "Paired",
            (Lang::En, DeviceState::Unseen) => "Not seen",
            (Lang::En, DeviceState::Revoked) => "Revoked",
            (Lang::Ko, DeviceState::Paired) => "페어링됨",
            (Lang::Ko, DeviceState::Unseen) => "접속 전",
            (Lang::Ko, DeviceState::Revoked) => "해제됨",
        }
    }
    /// The exact English outputs of Paseo's formatTimeAgo test cases.
    #[must_use]
    pub fn compact_age(&self, age: Age) -> String {
        match (self.lang, age) {
            (Lang::En, Age::Now) => "just now".to_owned(),
            (Lang::Ko, Age::Now) => "방금".to_owned(),
            (Lang::En, Age::Minutes(n)) => format!("{n}m ago"),
            (Lang::En, Age::Hours(n)) => format!("{n}h ago"),
            (Lang::En, Age::Days(n)) => format!("{n}d ago"),
            (Lang::Ko, Age::Minutes(n)) => format!("{n}분 전"),
            (Lang::Ko, Age::Hours(n)) => format!("{n}시간 전"),
            (Lang::Ko, Age::Days(n)) => format!("{n}일 전"),
            (Lang::En, Age::Date(at)) => at.format("%b %-d").to_string(),
            (Lang::Ko, Age::Date(at)) => format!("{}월 {}일", at.month(), at.day()),
        }
    }
    /// Settings has room for words instead of dense-list abbreviations.
    #[must_use]
    pub fn prose_age(&self, age: Age) -> String {
        match (self.lang, age) {
            (Lang::En, Age::Minutes(1)) => "1 minute ago".to_owned(),
            (Lang::En, Age::Minutes(n)) => format!("{n} minutes ago"),
            (Lang::En, Age::Hours(1)) => "1 hour ago".to_owned(),
            (Lang::En, Age::Hours(n)) => format!("{n} hours ago"),
            (Lang::En, Age::Days(1)) => "1 day ago".to_owned(),
            (Lang::En, Age::Days(n)) => format!("{n} days ago"),
            (Lang::En, Age::Now | Age::Date(_))
            | (
                Lang::Ko,
                Age::Now | Age::Minutes(_) | Age::Hours(_) | Age::Days(_) | Age::Date(_),
            ) => self.compact_age(age),
        }
    }
    /// Explanation below the device name. Counts are numeric; dates use the viewer's offset.
    /// This has room for full words and must not use the badge's short label.
    #[must_use]
    pub fn last_seen(&self, age: Option<Age>) -> String {
        match (self.lang, age) {
            (Lang::En, None) => "No connection recorded".to_owned(),
            (Lang::Ko, None) => "기록된 접속이 없습니다".to_owned(),
            (Lang::En, Some(Age::Now)) => "Last seen just now".to_owned(),
            (Lang::En, Some(Age::Minutes(1))) => "Last seen 1 minute ago".to_owned(),
            (Lang::En, Some(Age::Minutes(n))) => format!("Last seen {n} minutes ago"),
            (Lang::En, Some(Age::Hours(1))) => "Last seen 1 hour ago".to_owned(),
            (Lang::En, Some(Age::Hours(n))) => format!("Last seen {n} hours ago"),
            (Lang::En, Some(Age::Days(1))) => "Last seen 1 day ago".to_owned(),
            (Lang::En, Some(Age::Days(n))) => format!("Last seen {n} days ago"),
            (Lang::En, Some(Age::Date(at))) => format!("Last seen on {}", at.format("%b %-d")),
            (Lang::Ko, Some(Age::Now)) => "마지막 접속: 방금".to_owned(),
            (Lang::Ko, Some(Age::Minutes(n))) => format!("마지막 접속: {n}분 전"),
            (Lang::Ko, Some(Age::Hours(n))) => format!("마지막 접속: {n}시간 전"),
            (Lang::Ko, Some(Age::Days(n))) => format!("마지막 접속: {n}일 전"),
            (Lang::Ko, Some(Age::Date(at))) => {
                let (month, day) = (at.month(), at.day());
                format!("마지막 접속: {month}월 {day}일")
            }
        }
    }
    /// Pairing history below the device name. Counts are numeric; dates use the viewer's offset.
    /// Each arm is a complete translation unit rather than an age fragment.
    #[must_use]
    pub fn paired_ago(&self, age: Age) -> String {
        match (self.lang, age) {
            (Lang::En, Age::Now) => "Paired just now".to_owned(),
            (Lang::En, Age::Minutes(1)) => "Paired 1 minute ago".to_owned(),
            (Lang::En, Age::Minutes(n)) => format!("Paired {n} minutes ago"),
            (Lang::En, Age::Hours(1)) => "Paired 1 hour ago".to_owned(),
            (Lang::En, Age::Hours(n)) => format!("Paired {n} hours ago"),
            (Lang::En, Age::Days(1)) => "Paired 1 day ago".to_owned(),
            (Lang::En, Age::Days(n)) => format!("Paired {n} days ago"),
            (Lang::En, Age::Date(at)) => format!("Paired on {}", at.format("%b %-d")),
            (Lang::Ko, Age::Now) => "페어링: 방금".to_owned(),
            (Lang::Ko, Age::Minutes(n)) => format!("페어링: {n}분 전"),
            (Lang::Ko, Age::Hours(n)) => format!("페어링: {n}시간 전"),
            (Lang::Ko, Age::Days(n)) => format!("페어링: {n}일 전"),
            (Lang::Ko, Age::Date(at)) => {
                let (month, day) = (at.month(), at.day());
                format!("페어링: {month}월 {day}일")
            }
        }
    }
    /// Hover tooltip: viewer-local instants to the second, with a named
    /// offset and no fractions. The body can wrap; this is not a badge.
    #[must_use]
    pub fn exact_times(
        &self,
        paired: DateTime<Utc>,
        seen: Option<DateTime<Utc>>,
        offset: FixedOffset,
    ) -> String {
        let paired = exact_time(paired, offset);
        match (self.lang, seen) {
            (Lang::En, None) => format!("Paired: {paired}\nLast seen: Never"),
            (Lang::Ko, None) => format!("페어링: {paired}\n마지막 접속: 접속한 적 없음"),
            (Lang::En, Some(seen)) => {
                let seen = exact_time(seen, offset);
                format!("Paired: {paired}\nLast seen: {seen}")
            }
            (Lang::Ko, Some(seen)) => {
                let seen = exact_time(seen, offset);
                format!("페어링: {paired}\n마지막 접속: {seen}")
            }
        }
    }
    /// Failed reads have their own place and action context.
    #[must_use]
    pub fn list_problem(&self, problem: Problem) -> String {
        match self.lang {
            Lang::En => format!("Could not list devices. {}", self.problem(problem)),
            Lang::Ko => format!("기기를 불러오지 못했습니다. {}", self.problem(problem)),
        }
    }
    /// A failed invitation never uses the credential in its explanation.
    #[must_use]
    pub fn invite_problem(&self, problem: Problem) -> String {
        match self.lang {
            Lang::En => format!("Could not make a pairing link. {}", self.problem(problem)),
            Lang::Ko => format!("페어링 링크를 만들지 못했습니다. {}", self.problem(problem)),
        }
    }
    /// A lost response cannot prove the revoke did not take effect.
    #[must_use]
    pub fn revoke_problem(&self, problem: Problem) -> String {
        match self.lang {
            Lang::En => format!(
                "Could not confirm access was revoked. {}",
                self.problem(problem)
            ),
            Lang::Ko => format!(
                "접근 해제 결과를 확인하지 못했습니다. {}",
                self.problem(problem)
            ),
        }
    }
}
