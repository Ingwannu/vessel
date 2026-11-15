//! Signing in to a forge (ADR 0029): which token goes to which host, where
//! it comes from, and how it is kept.
//!
//! [Decision Log]
//! - Purpose: the node reads and opens pull requests as the user, with a
//!   token the machine already holds, and never keeps one of its own
//!   (agent-orchestrator and GOAT read the same sources; Paseo lets `gh`
//!   make every call).
//! - Choice, in order, for `github.com`: `GH_TOKEN`, then `GITHUB_TOKEN`
//!   (handed over by the binary: a library reads no environment); for
//!   any other host, `GH_ENTERPRISE_TOKEN` or `GITHUB_ENTERPRISE_TOKEN`
//!   only when `GH_HOST` names that host (GOAT's trap: an enterprise
//!   token must not go to a host taken from a remote); then the machine's
//!   `gh auth token --hostname <host>` (`gh`). A host with no token gets
//!   no request at all.
//! - Kept: in memory only, five minutes, a miss ten seconds; dropped on a
//!   401 and asked again at most once a minute per host. [`Token`]'s
//!   `Debug` never shows it; it leaves this crate only as an
//!   `Authorization` header marked sensitive.

mod identity;

pub use identity::CredentialIdentity;

use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use tokio::time::Instant;
use vessel_proto::forge::{ForgeHost, GhState, TokenSource};

use crate::gh::Gh;

/// A forge token. Never printed: its `Debug` says only that it is one.
#[derive(Clone)]
pub struct Token(Arc<str>);

impl Token {
    /// A token from `text`, `None` when it is blank or not one header-safe
    /// line.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        let text = text.trim();
        let printable = text.bytes().all(|byte| byte.is_ascii_graphic());
        (!text.is_empty() && printable).then(|| Self(Arc::from(text)))
    }

    /// The `Authorization` header's value, marked sensitive (so an HTTP
    /// library's debug output leaves it out).
    pub(crate) fn header(&self) -> Option<reqwest::header::HeaderValue> {
        let mut value =
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.0)).ok()?;
        value.set_sensitive(true);
        Some(value)
    }

    /// Compare cached credentials without deriving a loggable id. This
    /// is cache association, never an authentication check.
    pub(crate) fn same_as(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(..)")
    }
}

/// The variables a token for a forge is read from, by [`EnvTokens::from_vars`]:
/// the node's sign-in when they are set, so whatever the node starts in a
/// repository's folder (a terminal, a script, setup and teardown) runs
/// without them (the join of wave 2's first half; ADRs 0029 and 0031).
pub const SIGN_IN_VARIABLES: [&str; 4] = [
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "GH_ENTERPRISE_TOKEN",
    "GITHUB_ENTERPRISE_TOKEN",
];

/// Tokens the binary found in the node's environment. A library reads no
/// environment (AGENTS.md section 8): `vesseld` fills this.
#[derive(Debug, Clone, Default)]
pub struct EnvTokens {
    /// `GH_TOKEN`, else `GITHUB_TOKEN`: for `github.com` only.
    pub github: Option<Token>,
    /// `GH_ENTERPRISE_TOKEN`, else `GITHUB_ENTERPRISE_TOKEN`, with the
    /// host `GH_HOST` names: for that host only.
    pub enterprise: Option<(ForgeHost, Token)>,
}

impl EnvTokens {
    /// The tokens the variables `read` gives name (the binary passes its
    /// environment's reader). Empty or unreadable values are none; an
    /// enterprise token needs `GH_HOST` to name a host other than
    /// `github.com`.
    pub fn from_vars(read: impl Fn(&str) -> Option<OsString>) -> Self {
        let [gh, github, gh_enterprise, github_enterprise] = SIGN_IN_VARIABLES;
        let text = |name: &str| read(name).and_then(|value| value.into_string().ok());
        let token = |names: [&str; 2]| {
            names
                .into_iter()
                .find_map(|name| text(name).and_then(|value| Token::new(value.trim())))
        };
        let enterprise = text("GH_HOST")
            .and_then(|host| host.trim().to_ascii_lowercase().parse::<ForgeHost>().ok())
            .filter(|host| host.as_str() != "github.com")
            .and_then(|host| Some((host, token([gh_enterprise, github_enterprise])?)));
        Self {
            github: token([gh, github]),
            enterprise,
        }
    }
}

/// What asking for a host's token found.
#[derive(Debug, Clone)]
pub enum Found {
    /// A token, and where it came from.
    Token(Token, TokenSource),
    /// None: what `gh` said.
    None(GhState),
}

impl Found {
    // Concurrent gh cache misses may complete with the same credential.
    // Preserve that generation so an unchanged refresh does not split
    // coalesced page reads. A changed value or source gets a new marker.
    fn same_credential(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Token(left, from_left), Self::Token(right, from_right)) => {
                from_left == from_right && left.same_as(right)
            }
            (Self::None(left), Self::None(right)) => left == right,
            (Self::None(_), Self::Token(..)) | (Self::Token(..), Self::None(_)) => false,
        }
    }
}

/// How long a token is kept, and a miss.
const KEEP: Duration = Duration::from_secs(300);
const KEEP_MISS: Duration = Duration::from_secs(10);
/// How often a dropped token is asked for again.
const ASK_AGAIN: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
struct Cached {
    found: Found,
    until: Instant,
    generation: Arc<()>,
}

/// The most hosts whose answer from `gh` (or whose 401) is kept: remotes
/// an agent controls can name any number of hosts.
const MAX_CACHED: usize = 64;

/// The node's tokens, per host: from its environment, else from `gh`.
#[derive(Debug)]
pub struct Credentials {
    env: EnvTokens,
    gh: Option<Gh>,
    cache: Mutex<HashMap<ForgeHost, Cached>>,
    dropped: Mutex<HashMap<ForgeHost, Instant>>,
    configured_generation: Arc<()>,
    /// Every known refusal invalidates page caches, even when asking gh
    /// again is held back by `ASK_AGAIN`. One marker, not a host-sized map.
    invalidated: Mutex<Arc<()>>,
}

impl Credentials {
    /// Tokens from `env`, else from `gh` (`None`: no `gh` on this machine).
    #[must_use]
    pub fn new(env: EnvTokens, gh: Option<Gh>) -> Self {
        Self {
            env,
            gh,
            cache: Mutex::new(HashMap::new()),
            dropped: Mutex::new(HashMap::new()),
            configured_generation: Arc::new(()),
            invalidated: Mutex::new(Arc::new(())),
        }
    }

    /// The token for `host`, from the environment or `gh`, kept a while.
    pub async fn token(&self, host: &ForgeHost) -> Found {
        if let Some(token) = self.env_token(host) {
            return Found::Token(token, TokenSource::Environment);
        }
        let now = Instant::now();
        if let Some(cached) = self.cache().get(host)
            && cached.until > now
        {
            return cached.found.clone();
        }
        let found = match &self.gh {
            None => Found::None(GhState::Missing),
            Some(gh) => match gh.token(host).await {
                Ok(token) => Found::Token(token, TokenSource::Gh),
                Err(state) => Found::None(state),
            },
        };
        let keep = match found {
            Found::Token(..) => KEEP,
            Found::None(_) => KEEP_MISS,
        };
        let mut cache = self.cache();
        // Hosts come from remotes an agent can set: the cache is bounded,
        // the expired first, else the one that expires soonest.
        if !cache.contains_key(host) && cache.len() >= MAX_CACHED {
            cache.retain(|_, cached| cached.until > now);
            if cache.len() >= MAX_CACHED
                && let Some(soonest) = cache
                    .iter()
                    .min_by_key(|(_, cached)| cached.until)
                    .map(|(host, _)| host.clone())
            {
                let _dropped = cache.remove(&soonest);
            }
        }
        let generation = cache
            .get(host)
            .filter(|cached| cached.found.same_credential(&found))
            .map_or_else(|| Arc::new(()), |cached| Arc::clone(&cached.generation));
        let _previous = cache.insert(
            host.clone(),
            Cached {
                found: found.clone(),
                until: now.checked_add(keep).unwrap_or(now),
                generation,
            },
        );
        drop(cache);
        found
    }

    /// The forge said 401: forget `host`'s token, so the next call asks
    /// again, at most once a minute (a token that stays bad is not asked
    /// for on every read).
    pub fn refused(&self, host: &ForgeHost) {
        self.invalidate();
        let now = Instant::now();
        let mut dropped = self.dropped.lock().unwrap_or_else(PoisonError::into_inner);
        if dropped
            .get(host)
            .is_some_and(|at| now.saturating_duration_since(*at) < ASK_AGAIN)
        {
            return;
        }
        // Bounded as the cache is: a drop older than a minute holds
        // nothing back any more.
        dropped.retain(|_, at| now.saturating_duration_since(*at) < ASK_AGAIN);
        if dropped.len() < MAX_CACHED {
            let _at = dropped.insert(host.clone(), now);
        }
        drop(dropped);
        let _forgotten = self.cache().remove(host);
    }

    // A repository refusal invalidates page associations too, without
    // changing gh's cache/retry policy. Only the existing 401 path calls
    // refused(host), which separately decides when to forget a gh token.
    pub(crate) fn invalidate(&self) {
        *self
            .invalidated
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Arc::new(());
    }

    /// The generation of this host's currently cached credential, without
    /// revealing or hashing token contents. A page loader compares this
    /// before and after its read, and never publishes across a change.
    /// None means no stable token is cached: do not reuse a page then.
    /// Existing token expiry, host restrictions and gh retry policy apply.
    pub async fn identity(&self, host: &ForgeHost) -> Option<CredentialIdentity> {
        if !matches!(self.token(host).await, Found::Token(..)) {
            return None;
        }
        self.cached_identity(host)
    }

    /// This host's currently selected cached credential, without a refresh
    /// or subprocess. None includes expired and negative entries. An owner
    /// rechecks this at cache admission after asynchronous preparation, so
    /// a queued completion cannot reuse facts invalidated in the meantime.
    #[must_use]
    pub fn cached_identity(&self, host: &ForgeHost) -> Option<CredentialIdentity> {
        let loaded = if self.env_token(host).is_some() {
            Arc::clone(&self.configured_generation)
        } else {
            let cache = self.cache();
            let cached = cache.get(host)?;
            if cached.until <= Instant::now() || !matches!(&cached.found, Found::Token(..)) {
                return None;
            }
            Arc::clone(&cached.generation)
        };
        let invalidated = Arc::clone(
            &self
                .invalidated
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
        Some(CredentialIdentity::new(host.clone(), loaded, invalidated))
    }

    fn env_token(&self, host: &ForgeHost) -> Option<Token> {
        if host.as_str() == "github.com" {
            return self.env.github.clone();
        }
        match &self.env.enterprise {
            Some((named, token)) if named == host => Some(token.clone()),
            Some(_) | None => None,
        }
    }

    fn cache(&self) -> std::sync::MutexGuard<'_, HashMap<ForgeHost, Cached>> {
        self.cache.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(name: &str) -> ForgeHost {
        name.parse().unwrap()
    }

    /// Remotes can name any number of hosts: what is kept of their
    /// answers stays bounded.
    #[tokio::test]
    async fn the_answers_kept_are_bounded() {
        let credentials = Credentials::new(EnvTokens::default(), None);
        for n in 0..(MAX_CACHED * 2) {
            let named = host(&format!("h{n}.example"));
            assert!(matches!(
                credentials.token(&named).await,
                Found::None(GhState::Missing)
            ));
            credentials.refused(&named);
        }
        assert!(credentials.cache().len() <= MAX_CACHED);
        assert!(
            credentials
                .dropped
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .len()
                <= MAX_CACHED
        );
    }

    #[test]
    fn a_token_never_shows_itself() {
        let token = Token::new("ghp_secretvalue").unwrap();
        assert_eq!(format!("{token:?}"), "Token(..)");
        let header = token.header().unwrap();
        assert!(header.is_sensitive());
        assert!(Token::new("  ").is_none());
        assert!(Token::new("two\nlines").is_none());
    }

    #[test]
    fn the_environment_names_each_token_s_host() {
        let vars = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| OsString::from(*value))
            }
        };
        let found = EnvTokens::from_vars(vars(&[
            ("GITHUB_TOKEN", "second"),
            ("GH_TOKEN", " first "),
            ("GH_HOST", "GHE.Example.org"),
            ("GITHUB_ENTERPRISE_TOKEN", "enterprise"),
        ]));
        assert!(found.github.unwrap().same_as(&Token::new("first").unwrap()));
        let (host, token) = found.enterprise.unwrap();
        assert_eq!(host.as_str(), "ghe.example.org");
        assert!(token.same_as(&Token::new("enterprise").unwrap()));
        // No host named: an enterprise token goes nowhere.
        let alone = EnvTokens::from_vars(vars(&[("GH_ENTERPRISE_TOKEN", "x"), ("GH_TOKEN", "")]));
        assert!(alone.enterprise.is_none() && alone.github.is_none());
    }

    #[tokio::test]
    async fn each_token_goes_only_to_its_own_host() {
        let github = Token::new("github-token").unwrap();
        let enterprise = Token::new("enterprise-token").unwrap();
        let credentials = Credentials::new(
            EnvTokens {
                github: Some(github.clone()),
                enterprise: Some((host("git.corp.example"), enterprise.clone())),
            },
            None,
        );
        let Found::Token(found, TokenSource::Environment) =
            credentials.token(&host("github.com")).await
        else {
            panic!("github.com takes GH_TOKEN");
        };
        assert!(found.same_as(&github));
        let Found::Token(found, TokenSource::Environment) =
            credentials.token(&host("git.corp.example")).await
        else {
            panic!("GH_HOST's host takes the enterprise token");
        };
        assert!(found.same_as(&enterprise));
        // Any other host: neither token, and without `gh`, none at all.
        assert!(matches!(
            credentials.token(&host("evil.example")).await,
            Found::None(GhState::Missing)
        ));
    }
}
