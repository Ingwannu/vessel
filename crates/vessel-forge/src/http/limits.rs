//! A forge's limits as this node keeps them: each host's quota per
//! resource, its cooldown after a limit answer, and the background's
//! token bucket; and how a limit answer and a single sign-on refusal are
//! told from a plain refusal.
//!
//! [Decision Log]
//! - Purpose: a forge counts every request (GitHub: per resource, and a
//!   secondary limit per minute); the watcher must slow down before the
//!   quota runs out and stop when told, while a person's call still goes.
//! - Choice: a floor of [`FLOOR_PERCENT`] of a resource's quota under
//!   which only a person's request is sent (GOAT's rule); a cooldown per
//!   host after a limit answer, from its `retry-after` or reset when it
//!   gives one, otherwise [`COOLDOWN_FIRST`] seconds doubling to
//!   [`COOLDOWN_CAP`] (agent-orchestrator's backoff), which every request
//!   meets, a person's too (the forge would refuse it anyway); a token
//!   bucket for the background only.
//! - Must stay true: the times here are wall-clock (`DateTime`) where the
//!   forge gives one, monotonic for the bucket; an overflow keeps `now`,
//!   never panics.

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use reqwest::StatusCode;
use reqwest::header::HeaderMap;
use vessel_proto::forge::{ForgeHost, ForgeUrl, RefusedReason};

use crate::provider::Priority;

/// The background's rate per host: a bucket of this many, one more every
/// [`REFILL`].
const BUCKET: u32 = 10;
const REFILL: Duration = Duration::from_millis(200);
/// Background reads stop under this share of a resource's quota (GOAT).
const FLOOR_PERCENT: u64 = 10;
/// A cooldown with nothing to go by: this long, doubling to the cap.
const COOLDOWN_FIRST: i64 = 60;
const COOLDOWN_CAP: i64 = 15 * 60;

/// Which of a forge's quotas a request counts against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Resource {
    /// REST.
    Core,
    /// GraphQL.
    Graphql,
    /// Search (ADR 0088): its own, smaller quota.
    Search,
}

impl Resource {
    fn of(name: &str) -> Option<Self> {
        match name {
            "core" => Some(Self::Core),
            "graphql" => Some(Self::Graphql),
            "search" => Some(Self::Search),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Quota {
    limit: u64,
    remaining: u64,
    reset: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
struct Cooldown {
    until: DateTime<Utc>,
    next: i64,
}

#[derive(Debug, Clone, Copy)]
struct Bucket {
    tokens: u32,
    at: tokio::time::Instant,
}

/// What a limit answer holds (ADR 0088): the whole host, or only its
/// search, whose quota (30 a minute) is its own; a person typing a search
/// must not stop the pull requests being watched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Hold {
    /// Every request to the host (a secondary limit is the account's).
    Host,
    /// Only searches.
    Search,
}

impl Hold {
    /// What a limit answer to a request counted against `asked` holds:
    /// search alone for search's own spent quota, else the host.
    pub(crate) fn of(asked: Resource, headers: &HeaderMap) -> Self {
        let header = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
        };
        let searched = asked == Resource::Search
            && header("x-ratelimit-resource").is_none_or(|resource| resource == "search");
        if searched && header("x-ratelimit-remaining") == Some("0") {
            Self::Search
        } else {
            Self::Host
        }
    }

    /// The holds a request counted against `resource` waits for.
    const fn for_request(resource: Resource) -> &'static [Self] {
        match resource {
            Resource::Search => &[Self::Host, Self::Search],
            Resource::Core | Resource::Graphql => &[Self::Host],
        }
    }
}

/// Whether a request may go now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Admit {
    /// Send it.
    Go,
    /// Send it after this wait (the background's bucket).
    After(Duration),
    /// Held until then (a cooldown, or the background under the floor).
    Held(DateTime<Utc>),
}

/// Every host's limits.
#[derive(Debug, Default)]
pub(crate) struct Limits {
    quotas: HashMap<(ForgeHost, Resource), Quota>,
    cooldowns: HashMap<(ForgeHost, Hold), Cooldown>,
    buckets: HashMap<ForgeHost, Bucket>,
}

impl Limits {
    /// Whether a request to `host` counting against `resource` may go.
    pub(crate) fn admit(
        &mut self,
        host: &ForgeHost,
        resource: Resource,
        priority: Priority,
        now: DateTime<Utc>,
    ) -> Admit {
        for hold in Hold::for_request(resource) {
            if let Some(cooldown) = self.cooldowns.get(&(host.clone(), *hold))
                && cooldown.until > now
            {
                return Admit::Held(cooldown.until);
            }
        }
        if priority == Priority::Person {
            return Admit::Go;
        }
        if let Some(quota) = self.quotas.get(&(host.clone(), resource))
            && quota.reset > now
            && quota.remaining.saturating_mul(100) < quota.limit.saturating_mul(FLOOR_PERCENT)
        {
            return Admit::Held(quota.reset);
        }
        self.take_token(host).map_or(Admit::Go, Admit::After)
    }

    /// Keep what an answer's `x-ratelimit-*` headers say of its quota.
    pub(crate) fn note(&mut self, host: &ForgeHost, asked: Resource, headers: &HeaderMap) {
        let number = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .and_then(|text| text.trim().parse::<u64>().ok())
        };
        let (Some(limit), Some(remaining), Some(reset)) = (
            number("x-ratelimit-limit"),
            number("x-ratelimit-remaining"),
            number("x-ratelimit-reset"),
        ) else {
            return;
        };
        let resource = headers
            .get("x-ratelimit-resource")
            .and_then(|value| value.to_str().ok())
            .and_then(Resource::of)
            .unwrap_or(asked);
        let Some(reset) = i64::try_from(reset)
            .ok()
            .and_then(|seconds| DateTime::<Utc>::from_timestamp(seconds, 0))
        else {
            return;
        };
        let _previous = self.quotas.insert(
            (host.clone(), resource),
            Quota {
                limit,
                remaining,
                reset,
            },
        );
    }

    /// A limit answer: hold `host` until the time it gives, or for the
    /// next step of the backoff. Answers until when.
    pub(crate) fn cool_down(
        &mut self,
        host: &ForgeHost,
        headers: &HeaderMap,
        now: DateTime<Utc>,
    ) -> DateTime<Utc> {
        self.cool_down_for(host, Hold::Host, headers, now)
    }

    /// [`Self::cool_down`] for what `hold` holds of `host`.
    pub(crate) fn cool_down_for(
        &mut self,
        host: &ForgeHost,
        hold: Hold,
        headers: &HeaderMap,
        now: DateTime<Utc>,
    ) -> DateTime<Utc> {
        let key = (host.clone(), hold);
        let step = self
            .cooldowns
            .get(&key)
            .map_or(COOLDOWN_FIRST, |cooldown| cooldown.next);
        let said = headers
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|text| text.trim().parse::<i64>().ok())
            .and_then(|seconds| later(now, seconds.clamp(1, COOLDOWN_CAP)))
            .or_else(|| {
                let remaining = headers.get("x-ratelimit-remaining")?.to_str().ok()?;
                (remaining.trim() == "0").then_some(())?;
                let reset = headers.get("x-ratelimit-reset")?.to_str().ok()?;
                DateTime::<Utc>::from_timestamp(reset.trim().parse().ok()?, 0)
            });
        let until = said.or_else(|| later(now, step)).unwrap_or(now);
        let _previous = self.cooldowns.insert(
            key,
            Cooldown {
                until,
                next: step.saturating_mul(2).min(COOLDOWN_CAP),
            },
        );
        until
    }

    /// A good answer after a cooldown ended: the next one starts from the
    /// first step again.
    pub(crate) fn cooled(&mut self, host: &ForgeHost, now: DateTime<Utc>) {
        for hold in [Hold::Host, Hold::Search] {
            let key = (host.clone(), hold);
            if self
                .cooldowns
                .get(&key)
                .is_some_and(|cooldown| cooldown.until <= now)
            {
                let _ended = self.cooldowns.remove(&key);
            }
        }
    }

    /// One token of `host`'s bucket, or how long until there is one.
    fn take_token(&mut self, host: &ForgeHost) -> Option<Duration> {
        let now = tokio::time::Instant::now();
        let bucket = self.buckets.entry(host.clone()).or_insert(Bucket {
            tokens: BUCKET,
            at: now,
        });
        let gained = now
            .saturating_duration_since(bucket.at)
            .as_millis()
            .checked_div(REFILL.as_millis())
            .unwrap_or(0);
        let gained = u32::try_from(gained).unwrap_or(u32::MAX);
        if gained > 0 {
            bucket.tokens = bucket.tokens.saturating_add(gained).min(BUCKET);
            bucket.at = now;
        }
        if bucket.tokens > 0 {
            bucket.tokens = bucket.tokens.saturating_sub(1);
            return None;
        }
        Some(REFILL.saturating_sub(now.saturating_duration_since(bucket.at)))
    }
}

/// `seconds` after `now`, `None` past chrono's range.
fn later(now: DateTime<Utc>, seconds: i64) -> Option<DateTime<Utc>> {
    now.checked_add_signed(TimeDelta::try_seconds(seconds)?)
}

/// Whether an answer is a rate limit (agent-orchestrator's reading): a
/// 429, or a 403 with nothing remaining, a `retry-after`, or the words.
pub(crate) fn is_limit(status: StatusCode, headers: &HeaderMap, body: &[u8]) -> bool {
    if status == StatusCode::TOO_MANY_REQUESTS {
        return true;
    }
    if status != StatusCode::FORBIDDEN {
        return false;
    }
    let none_left = headers
        .get("x-ratelimit-remaining")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|text| text.trim() == "0");
    let text = String::from_utf8_lossy(body).to_ascii_lowercase();
    none_left
        || headers.contains_key("retry-after")
        || text.contains("rate limit")
        || text.contains("abuse")
}

/// A 403 asking for single sign-on (`x-github-sso: required; url=...`).
pub(crate) fn sso(headers: &HeaderMap) -> Option<RefusedReason> {
    let value = headers.get("x-github-sso")?.to_str().ok()?;
    let url = value
        .split(';')
        .find_map(|part| part.trim().strip_prefix("url="))
        .and_then(ForgeUrl::lossy);
    Some(RefusedReason::Sso { url })
}

#[cfg(test)]
mod tests {
    use reqwest::header::HeaderValue;

    use super::*;

    fn host() -> ForgeHost {
        "github.com".parse().unwrap()
    }

    #[test]
    fn a_403_is_a_limit_only_by_its_headers_or_words() {
        let mut headers = HeaderMap::new();
        assert!(!is_limit(
            StatusCode::FORBIDDEN,
            &headers,
            b"Resource not accessible"
        ));
        assert!(is_limit(
            StatusCode::FORBIDDEN,
            &headers,
            b"API rate limit exceeded"
        ));
        assert!(is_limit(StatusCode::TOO_MANY_REQUESTS, &headers, b""));
        let _old = headers.insert("x-ratelimit-remaining", HeaderValue::from_static("0"));
        assert!(is_limit(StatusCode::FORBIDDEN, &headers, b""));
        assert!(!is_limit(StatusCode::UNAUTHORIZED, &headers, b""));
    }

    #[test]
    fn a_spent_search_quota_holds_only_search_and_a_secondary_limit_holds_all() {
        let mut limits = Limits::default();
        let now = DateTime::<Utc>::UNIX_EPOCH;
        let mut spent = HeaderMap::new();
        let _old = spent.insert("x-ratelimit-resource", HeaderValue::from_static("search"));
        let _old = spent.insert("x-ratelimit-remaining", HeaderValue::from_static("0"));
        let hold = Hold::of(Resource::Search, &spent);
        assert_eq!(hold, Hold::Search);
        let until = limits.cool_down_for(&host(), hold, &spent, now);
        assert_eq!(
            limits.admit(&host(), Resource::Search, Priority::Person, now),
            Admit::Held(until)
        );
        assert_eq!(
            limits.admit(&host(), Resource::Core, Priority::Person, now),
            Admit::Go,
            "watching pull requests goes on while search waits"
        );
        let mut secondary = HeaderMap::new();
        let _old = secondary.insert("retry-after", HeaderValue::from_static("30"));
        assert_eq!(Hold::of(Resource::Search, &secondary), Hold::Host);
        assert_eq!(Hold::of(Resource::Core, &spent), Hold::Host);
        limits.cooled(&host(), later(until, 1).unwrap());
        assert_eq!(
            limits.admit(&host(), Resource::Search, Priority::Person, now),
            Admit::Go
        );
    }

    #[test]
    fn a_cooldown_doubles_to_its_cap_and_holds_everyone() {
        let mut limits = Limits::default();
        let now = DateTime::<Utc>::UNIX_EPOCH;
        let first = limits.cool_down(&host(), &HeaderMap::new(), now);
        assert_eq!(first, later(now, COOLDOWN_FIRST).unwrap());
        assert_eq!(
            limits.admit(&host(), Resource::Core, Priority::Person, now),
            Admit::Held(first)
        );
        let second = limits.cool_down(&host(), &HeaderMap::new(), now);
        assert_eq!(second, later(now, COOLDOWN_FIRST * 2).unwrap());
        for _ in 0..10 {
            let _until = limits.cool_down(&host(), &HeaderMap::new(), now);
        }
        assert_eq!(
            limits.cool_down(&host(), &HeaderMap::new(), now),
            later(now, COOLDOWN_CAP).unwrap()
        );
        let after = later(now, COOLDOWN_CAP + 1).unwrap();
        limits.cooled(&host(), after);
        assert_eq!(
            limits.cool_down(&host(), &HeaderMap::new(), after),
            later(after, COOLDOWN_FIRST).unwrap()
        );
    }

    #[test]
    fn a_retry_after_is_taken_as_given() {
        let mut limits = Limits::default();
        let now = DateTime::<Utc>::UNIX_EPOCH;
        let mut headers = HeaderMap::new();
        let _old = headers.insert("retry-after", HeaderValue::from_static("7"));
        assert_eq!(
            limits.cool_down(&host(), &headers, now),
            later(now, 7).unwrap()
        );
    }

    #[test]
    fn under_the_floor_only_a_person_goes() {
        let mut limits = Limits::default();
        let now = DateTime::<Utc>::UNIX_EPOCH;
        let mut headers = HeaderMap::new();
        let _l = headers.insert("x-ratelimit-limit", HeaderValue::from_static("5000"));
        let _r = headers.insert("x-ratelimit-remaining", HeaderValue::from_static("499"));
        let _t = headers.insert("x-ratelimit-reset", HeaderValue::from_static("3600"));
        let _k = headers.insert("x-ratelimit-resource", HeaderValue::from_static("graphql"));
        limits.note(&host(), Resource::Core, &headers);
        assert_eq!(
            limits.admit(&host(), Resource::Graphql, Priority::Background, now),
            Admit::Held(later(now, 3600).unwrap())
        );
        assert_eq!(
            limits.admit(&host(), Resource::Graphql, Priority::Person, now),
            Admit::Go
        );
        // REST's quota is its own.
        assert_eq!(
            limits.admit(&host(), Resource::Core, Priority::Background, now),
            Admit::Go
        );
    }

    #[test]
    fn single_sign_on_names_its_page() {
        let mut headers = HeaderMap::new();
        let _old = headers.insert(
            "x-github-sso",
            HeaderValue::from_static("required; url=https://github.com/orgs/o/sso?x=1"),
        );
        assert!(matches!(
            sso(&headers),
            Some(RefusedReason::Sso { url: Some(_) })
        ));
    }
}
