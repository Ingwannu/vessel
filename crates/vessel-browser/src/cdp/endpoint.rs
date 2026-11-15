//! Launcher-private CDP endpoint parsing, without DNS or credential-bearing Debug.

use std::fmt;
use std::net::IpAddr;

const MAX_ENDPOINT_BYTES: usize = 512;
const BROWSER_PATH_PREFIX: &str = "/devtools/browser/";

/// A syntactically local endpoint; the launcher must separately prove child ownership.
#[derive(Clone)]
pub struct Endpoint(url::Url);
impl fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CdpEndpoint([redacted])")
    }
}

/// Parsing retains the source without reflecting the private endpoint in its message.
#[derive(Debug, thiserror::Error)]
pub enum InvalidEndpoint {
    /// No DNS name, userinfo, external host, fragment or arbitrary websocket route.
    #[error("CDP endpoint is not a bounded local browser endpoint")]
    Policy,
    /// Numeric parsing retains its original source without the selected endpoint.
    #[error("CDP endpoint address is malformed")]
    Address(#[source] std::net::AddrParseError),
    /// URL syntax errors do not change the trust boundary.
    #[error("CDP endpoint is malformed")]
    Parse(#[source] url::ParseError),
}
impl Endpoint {
    /// Use only with an endpoint emitted by the just-launched owned browser.
    /// This parser does not mint authority or qualify the runtime.
    ///
    /// # Errors
    /// Refuses DNS names/aliases, non-loopback hosts, credentials and non-browser routes.
    pub fn from_launcher(text: &str) -> Result<Self, InvalidEndpoint> {
        if text.len() > MAX_ENDPOINT_BYTES
            || text.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(InvalidEndpoint::Policy);
        }
        let url = url::Url::parse(text).map_err(InvalidEndpoint::Parse)?;
        let host = url.host_str().ok_or(InvalidEndpoint::Policy)?;
        let address: IpAddr = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse()
            .map_err(InvalidEndpoint::Address)?;
        // The exact spelling check also refuses URL's canonicalized numeric aliases.
        if !address.is_loopback()
            || url.as_str() != text
            || url.scheme() != "ws"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_none_or(|port| port == 0)
            || url.query().is_some()
            || url.fragment().is_some()
            || !url
                .path()
                .strip_prefix(BROWSER_PATH_PREFIX)
                .is_some_and(|suffix| !suffix.is_empty() && !suffix.contains('/'))
        {
            return Err(InvalidEndpoint::Policy);
        }
        Ok(Self(url))
    }
    pub(super) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launcher_route_is_local_canonical_and_redacted() {
        let endpoint =
            Endpoint::from_launcher("ws://127.0.0.1:12345/devtools/browser/private-canary")
                .unwrap();
        assert!(!format!("{endpoint:?}").contains("private-canary"));
        for invalid in [
            "ws://localhost:12345/devtools/browser/id",
            "ws://127.1:12345/devtools/browser/id",
            "ws://2130706433:12345/devtools/browser/id",
            "ws://127.0.0.1:12345/rpc",
            "ws://user:secret@127.0.0.1:12345/devtools/browser/id",
            "ws://203.0.113.1:12345/devtools/browser/id",
        ] {
            assert!(Endpoint::from_launcher(invalid).is_err());
        }
    }
}
