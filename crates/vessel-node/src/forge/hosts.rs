//! The forges a node reaches and how it is signed in to each (ADR 0029):
//! the provider, the credentials, and per host who the token is, kept five
//! minutes; what `forge.status` answers.
//!
//! [Decision Log]
//! - Purpose: a card must say why a session's pull request cannot be seen
//!   (signed out, another login, no provider), and `forge.status` must
//!   say it for the node, without a request to a host the node has no
//!   token for.
//! - Choice: one provider for every host (GitHub's, which reaches
//!   github.com, `*.ghe.com` and GitHub Enterprise Server by their API
//!   addresses), the token deciding: a host with none is `signed_out` for
//!   github.com, `not_supported` for any other (a token is never sent to a
//!   host a remote names unless the person set it up for that host). A
//!   test's in-memory forge replaces the provider and needs no token.
//! - Bounded: at most [`ForgeStatus::MAX_HOSTS`] hosts are remembered
//!   (remotes an agent controls could name any number); past that a host
//!   is still answered, not listed.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use chrono::{TimeDelta, Utc};
use vessel_forge::{Credentials, Forge, ForgeError, Gh, GitHub, TokenFound};
use vessel_proto::forge::{
    ForgeAccess, ForgeHost, ForgeKind, ForgeLogin, ForgeStatus, HostStatus, TokenSource,
};

use super::ForgeSetup;

/// How long who a token is stays known.
const KEEP_LOGIN: Duration = Duration::from_secs(300);
/// A host that could not be reached is said to be retried this much later.
const RETRY: TimeDelta = TimeDelta::seconds(60);

/// What the node knows of one host.
#[derive(Debug, Clone, Default)]
struct Host {
    /// Who the token is, and since when.
    login: Option<(Option<ForgeLogin>, Instant)>,
}

/// Who a host's token is, as kept.
#[derive(Debug)]
enum Cached {
    /// Not asked, or asked too long ago.
    Missing,
    /// Asked: the login (none for a token without a user).
    Login(Option<ForgeLogin>),
}

/// The provider, the credentials and every host's sign-in.
#[derive(Debug)]
pub(crate) struct Hosts {
    /// `None` when the HTTP client could not be built (no TLS roots): the
    /// node then watches nothing, and says so.
    forge: Option<Arc<dyn Forge>>,
    /// A test's in-memory forge: no token is asked for.
    custom: bool,
    credentials: Arc<Credentials>,
    known: Mutex<BTreeMap<ForgeHost, Host>>,
}

impl Hosts {
    /// The provider and credentials `setup` describes; `gh` runs in `data`,
    /// the node's own folder, never in a repository.
    pub(crate) fn new(setup: ForgeSetup, data: PathBuf) -> Self {
        let credentials = Arc::new(Credentials::new(
            setup.env,
            setup.gh.map(|program| Gh::new(program, data)),
        ));
        let custom = setup.custom.is_some();
        let forge = match setup.custom {
            Some(forge) => Some(forge),
            None => match GitHub::new(Arc::clone(&credentials)) {
                Ok(github) => {
                    let github: Arc<dyn Forge> = Arc::new(github);
                    Some(github)
                }
                Err(error) => {
                    tracing::warn!(%error, "no forge client; pull requests are not watched");
                    None
                }
            },
        };
        let mut hosts = BTreeMap::new();
        if let Ok(github) = "github.com".parse() {
            let _new = hosts.insert(github, Host::default());
        }
        Self {
            forge,
            custom,
            credentials,
            known: Mutex::new(hosts),
        }
    }

    /// The provider, when there is one.
    pub(crate) fn forge(&self) -> Option<Arc<dyn Forge>> {
        self.forge.clone()
    }

    /// Whether the node can reach `host` now, and as whom: `ready`, or why
    /// not. Asks `GET /user` at most once per five minutes per host, and
    /// nothing of a host it has no token for.
    pub(crate) async fn access(&self, host: &ForgeHost) -> ForgeAccess {
        self.remember(host);
        let Some(forge) = self.forge() else {
            return ForgeAccess::NotSupported { host: host.clone() };
        };
        if !self.custom {
            match self.credentials.token(host).await {
                TokenFound::Token(..) => {}
                TokenFound::None(gh) if host.as_str() == "github.com" => {
                    return ForgeAccess::SignedOut {
                        host: host.clone(),
                        gh,
                    };
                }
                TokenFound::None(_) => return ForgeAccess::NotSupported { host: host.clone() },
            }
        }
        if let Cached::Login(login) = self.cached_login(host) {
            return ForgeAccess::Ready { login };
        }
        match forge.whoami(host).await {
            Ok(login) => {
                if let Some(entry) = self.hosts().get_mut(host) {
                    entry.login = Some((login.clone(), Instant::now()));
                }
                ForgeAccess::Ready { login }
            }
            Err(error) => denied(host, &error, None),
        }
    }

    /// Who the token for `host` is, when known (for a refusal's message).
    pub(crate) fn login(&self, host: &ForgeHost) -> Option<ForgeLogin> {
        match self.cached_login(host) {
            Cached::Login(login) => login,
            Cached::Missing => None,
        }
    }

    /// Every remembered host and its sign-in (`forge.status`).
    pub(crate) async fn status(&self, watched: u32, limit: u32) -> ForgeStatus {
        let names: Vec<ForgeHost> = self.hosts().keys().cloned().collect();
        let mut hosts = Vec::with_capacity(names.len());
        for host in names {
            let access = self.access(&host).await;
            // Where the token came from, when one was used.
            let used = matches!(
                access,
                ForgeAccess::Ready { .. } | ForgeAccess::Refused { .. }
            );
            let source = if used && !self.custom {
                match self.credentials.token(&host).await {
                    TokenFound::Token(_, source) => Some(source),
                    TokenFound::None(_) => None,
                }
            } else {
                None
            };
            hosts.push(HostStatus {
                kind: ForgeKind::Github,
                host,
                access,
                source: source.filter(|source| *source != TokenSource::Unknown),
            });
        }
        ForgeStatus {
            hosts,
            watched,
            limit,
        }
    }

    /// Note `host`, within the bound.
    fn remember(&self, host: &ForgeHost) {
        let mut hosts = self.hosts();
        if !hosts.contains_key(host) && hosts.len() < ForgeStatus::MAX_HOSTS {
            let _new = hosts.insert(host.clone(), Host::default());
        }
    }

    fn cached_login(&self, host: &ForgeHost) -> Cached {
        let hosts = self.hosts();
        match hosts.get(host).and_then(|entry| entry.login.as_ref()) {
            Some((login, at)) if at.elapsed() < KEEP_LOGIN => Cached::Login(login.clone()),
            Some(_) | None => Cached::Missing,
        }
    }

    fn hosts(&self) -> MutexGuard<'_, BTreeMap<ForgeHost, Host>> {
        self.known.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What a failed request says about the node's access to `host`.
pub(crate) fn denied(
    host: &ForgeHost,
    error: &ForgeError,
    login: Option<ForgeLogin>,
) -> ForgeAccess {
    let host = host.clone();
    match error {
        ForgeError::SignedOut { gh } => ForgeAccess::SignedOut { host, gh: *gh },
        ForgeError::Refused { reason } => ForgeAccess::Refused {
            host,
            reason: reason.clone(),
            login,
        },
        ForgeError::RateLimited { until } => ForgeAccess::RateLimited {
            host,
            until: *until,
        },
        ForgeError::Unreachable(_)
        | ForgeError::Down
        | ForgeError::Server { .. }
        | ForgeError::NotFound
        | ForgeError::Unexpected { .. }
        | ForgeError::Busy => ForgeAccess::Unreachable {
            host,
            retry_at: Utc::now()
                .checked_add_signed(RETRY)
                .unwrap_or_else(Utc::now),
        },
    }
}
