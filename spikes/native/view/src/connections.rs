//! Settings › Connections for the chosen machine, as Paseo's
//! `ConnectionsSection` (`screens/settings/host-page.tsx`): every way the
//! book reaches the machine, in the order the app tries them, each named
//! by its kind and its endpoint (Paseo's `formatHostConnectionLabel`:
//! "Relay (relay.example:443)", "TCP (127.0.0.1:7077)").
//!
//! [Decision Log]
//! - Purpose: Paseo lists a host's saved connections; Vessel's book keeps
//!   one address per machine, which for a paired machine holds several
//!   routes (its own addresses, then the relays it registers with).
//! - Constraint: a route is a URL from the book; only its host and port
//!   are shown (Paseo shows endpoints), never a path, a query or a user
//!   part, so nothing a URL might carry beyond where it goes reaches the
//!   screen.
//! - Choice: the routes as typed values the link fills from the book
//!   ([`Route`]), endpoints cut at the input ([`Endpoint::of_url`]).
//! - Cost: two routes to the same host on different paths read the same.

/// Where a route goes: `host[:port]`, nothing else of its URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint(String);

impl Endpoint {
    /// The host and port of `url` (`ws://`, `wss://` or none): no scheme,
    /// no `user@`, no path, query or fragment.
    #[must_use]
    pub fn of_url(url: &str) -> Self {
        let rest = url.split_once("://").map_or(url, |(_scheme, rest)| rest);
        let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
        let host = authority
            .rsplit_once('@')
            .map_or(authority, |(_user, host)| host);
        Self(host.to_owned())
    }

    /// An SSH target's `[user@]host[:port]`, as the book names it.
    #[must_use]
    pub fn of_ssh(authority: &str) -> Self {
        Self(authority.to_owned())
    }

    /// The text shown.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One way the book reaches a machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// Its websocket with a token (Paseo's direct TCP connection).
    Tcp(Endpoint),
    /// A paired machine's own address, over the secure channel.
    Direct(Endpoint),
    /// A relay the paired machine registers with.
    Relay(Endpoint),
    /// `ssh` to the machine (Paseo's remote SSH).
    Ssh(Endpoint),
}

impl Route {
    /// Where it goes.
    #[must_use]
    pub const fn endpoint(&self) -> &Endpoint {
        match self {
            Self::Tcp(endpoint)
            | Self::Direct(endpoint)
            | Self::Relay(endpoint)
            | Self::Ssh(endpoint) => endpoint,
        }
    }
}
