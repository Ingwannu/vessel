//! Device-row sentences and action failures.

use super::{DevicesWords, Lang};
use crate::devices::{
    DeviceBadge, Problem,
    time::{Age, exact_time},
};
use chrono::{DateTime, Datelike as _, FixedOffset, Utc};

impl DevicesWords {
    /// A short credential-state badge, distinct from its explanation.
    #[must_use]
    pub const fn state(&self, state: DeviceBadge) -> &'static str {
        match (self.lang, state) {
            (Lang::En, DeviceBadge::ThisDevice) => "This device",
            (Lang::En, DeviceBadge::Unseen) => "Not seen",
            (Lang::En, DeviceBadge::Revoked) => "Revoked",
            (Lang::Ko, DeviceBadge::ThisDevice) => "이 기기",
            (Lang::Ko, DeviceBadge::Unseen) => "접속 전",
            (Lang::Ko, DeviceBadge::Revoked) => "해제됨",
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
