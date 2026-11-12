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

use std::collections::HashMap;
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

    /// A short name for the token, for telling two apart in a log without
    /// showing either: never part of it.
    #[cfg(test)]
    pub(crate) fn same_as(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(..)")
    }
}

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

/// What asking for a host's token found.
#[derive(Debug, Clone)]
pub enum Found {
    /// A token, and where it came from.
    Token(Token, TokenSource),
    /// None: what `gh` said.
    None(GhState),
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
}

/// The node's tokens, per host: from its environment, else from `gh`.
#[derive(Debug)]
pub struct Credentials {
    env: EnvTokens,
    gh: Option<Gh>,
    cache: Mutex<HashMap<ForgeHost, Cached>>,
    dropped: Mutex<HashMap<ForgeHost, Instant>>,
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
        let _previous = self.cache().insert(
            host.clone(),
            Cached {
                found: found.clone(),
                until: now.checked_add(keep).unwrap_or(now),
            },
        );
        found
    }

    /// The forge said 401: forget `host`'s token, so the next call asks
    /// again, at most once a minute (a token that stays bad is not asked
    /// for on every read).
    pub fn refused(&self, host: &ForgeHost) {
        let now = Instant::now();
        let mut dropped = self.dropped.lock().unwrap_or_else(PoisonError::into_inner);
        if dropped
            .get(host)
            .is_some_and(|at| now.saturating_duration_since(*at) < ASK_AGAIN)
        {
            return;
        }
        let _at = dropped.insert(host.clone(), now);
        drop(dropped);
        let _forgotten = self.cache().remove(host);
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

    #[test]
    fn a_token_never_shows_itself() {
        let token = Token::new("ghp_secretvalue").unwrap();
        assert_eq!(format!("{token:?}"), "Token(..)");
        let header = token.header().unwrap();
        assert!(header.is_sensitive());
        assert!(Token::new("  ").is_none());
        assert!(Token::new("two\nlines").is_none());
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
