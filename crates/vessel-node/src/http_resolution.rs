//! reqwest's resolver hook over one tracked native-resolution scope.
//!
//! [Decision Log] (2026-11-16, ADR 0104 "HTTP consumer" and the usage supplement)
//! - Purpose: an HTTP client the node owns resolves names only through the
//!   shared tracked primitive, so a cancelled request cannot leave an
//!   untracked `getaddrinfo` behind a stop the node calls normal.
//! - Constraint: reqwest's only extension point is its `Resolve` trait object
//!   (boxed future and iterator); without one it silently uses Gai.
//! - Alternatives: a resolver per client crate (a second implementation and
//!   quota, rejected by ADR 0104); wrapping Gai in a lease held by the async
//!   waiter (the lease ends before the native work does).
//! - Choice: one thin generic adapter here. It validates the host, delegates
//!   to the scope's weak handle with port 0 (reqwest supplies the URL or
//!   scheme port) and forwards the primitive's safe failure. It holds no
//!   quota, cache, retry or fallback of its own.
//! - Cost: the boxed future and iterator are reqwest's, not ours; a copy of
//!   the bounded result (at most `MAX_RESOLVED_ADDRESSES`) is made per lookup.
//! - Consumers: the usage client (limits/dns.rs). Dictation and NodeLink wire
//!   their own scopes on the same node-root budget when their owners land.
use std::net::SocketAddr;
use vessel_relay::resolution::{HostName, ResolutionHandle, ResolutionPort};

/// reqwest's hook for one scope. Client clones share this one weak handle; it
/// cannot create, reopen or replace the scope behind it.
pub(crate) struct HttpResolver<Hold>(ResolutionHandle<Hold>);

impl<Hold: Send + Sync + 'static> HttpResolver<Hold> {
    pub(crate) fn new(handle: ResolutionHandle<Hold>) -> Self {
        Self(handle)
    }
}

impl<Hold: Send + Sync + 'static> reqwest::dns::Resolve for HttpResolver<Hold> {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let handle = self.0.clone();
        // Validated before the future exists; the request URL, headers and
        // body never reach the resolver.
        let host = HostName::new(name.as_str());
        Box::pin(async move {
            let addresses = handle.resolve(&host?, ResolutionPort(0)).await?;
            let found: Vec<SocketAddr> = addresses.iter().collect();
            Ok::<reqwest::dns::Addrs, Box<dyn std::error::Error + Send + Sync>>(Box::new(
                found.into_iter(),
            ))
        })
    }
}

#[cfg(test)]
mod tests;
