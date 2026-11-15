//! Validated proxy origin metadata; it cannot authorize a network connection.

use std::{fmt, str::FromStr, sync::Arc};

use crate::workspace::{InvalidServicePort, ServicePort};

/// Bound parser allocation before a client-controlled URL is decoded.
pub const MAX_PROXY_URL_BYTES: usize = 2048;
/// A generated service hostname must fit each DNS label.
pub const MAX_PROXY_LABEL_BYTES: usize = 63;
/// A complete generated service hostname must fit the DNS text form.
pub const MAX_PROXY_HOST_BYTES: usize = 253;

/// A credential-free HTTP(S) origin for display, never a Preview admission.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct ServiceProxyUrl(Arc<url::Url>);

/// Input-free diagnostics keep submitted credentials out of every source chain.
#[derive(Debug, thiserror::Error)]
pub enum InvalidServiceProxyUrl {
    /// Check text size before parsing or retaining any submitted content.
    #[error("a service URL is at most {max} encoded bytes", max = MAX_PROXY_URL_BYTES)]
    Length,
    /// The parser must not repair control/whitespace or Windows separators.
    #[error("a service URL has no whitespace, control character or backslash")]
    Text,
    /// Preserve the standard parser's concrete, input-free cause.
    #[error("the service URL does not parse")]
    Url(#[source] url::ParseError),
    /// No other transport is a service proxy origin.
    #[error("a service URL uses http or https")]
    Scheme,
    /// Even empty userinfo is not accepted by the display boundary.
    #[error("a service URL has no userinfo or credentials")]
    Credentials,
    /// Paths and query/fragment material are not an origin.
    #[error("a service URL names only an origin, with at most its root slash")]
    Origin,
    /// Both aggregate and per-label bounds use the same parser owner.
    #[error("a service hostname has at most {host} bytes and {label} per label", host = MAX_PROXY_HOST_BYTES, label = MAX_PROXY_LABEL_BYTES)]
    Host,
    /// A zero explicit port is not a service endpoint.
    #[error("the service URL port is invalid")]
    Port(#[source] InvalidServicePort),
}

impl ServiceProxyUrl {
    /// Canonical text is safe to display; it carries no connection authority.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Classify literal/reserved loopback metadata without DNS or a second parse.
    /// This does not prove that a service listener exists on the node.
    #[must_use]
    pub(super) fn is_node_loopback(&self) -> bool {
        match self.0.host() {
            Some(url::Host::Ipv4(address)) => address.is_loopback(),
            Some(url::Host::Ipv6(address)) => address.is_loopback(),
            Some(url::Host::Domain(domain)) => {
                domain == "localhost" || domain.ends_with(".localhost")
            }
            None => false,
        }
    }
}

impl FromStr for ServiceProxyUrl {
    type Err = InvalidServiceProxyUrl;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() > MAX_PROXY_URL_BYTES {
            return Err(InvalidServiceProxyUrl::Length);
        }
        if text
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace() || ch == '\\')
        {
            return Err(InvalidServiceProxyUrl::Text);
        }
        let parsed = url::Url::parse(text).map_err(InvalidServiceProxyUrl::Url)?;
        match parsed.scheme() {
            "http" | "https" => {}
            _ => return Err(InvalidServiceProxyUrl::Scheme),
        }
        original_origin(text)?;
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(InvalidServiceProxyUrl::Credentials);
        }
        if parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() {
            return Err(InvalidServiceProxyUrl::Origin);
        }
        let Some(host) = parsed.host() else {
            return Err(InvalidServiceProxyUrl::Host);
        };
        if let url::Host::Domain(domain) = host {
            check_hostname(domain)?;
        }
        let Some(port) = parsed.port_or_known_default() else {
            return Err(InvalidServiceProxyUrl::Scheme);
        };
        let _port = ServicePort::try_from(port).map_err(InvalidServiceProxyUrl::Port)?;
        if parsed.as_str().len() > MAX_PROXY_URL_BYTES {
            return Err(InvalidServiceProxyUrl::Length);
        }
        Ok(Self(Arc::new(parsed)))
    }
}

fn original_origin(text: &str) -> Result<(), InvalidServiceProxyUrl> {
    let (_, rest) = text
        .split_once("://")
        .ok_or(InvalidServiceProxyUrl::Origin)?;
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    if authority.contains('@') {
        return Err(InvalidServiceProxyUrl::Credentials);
    }
    // URL parsing normalizes dot segments; refuse the authored path first.
    if authority.is_empty() || !path.is_empty() {
        return Err(InvalidServiceProxyUrl::Origin);
    }
    Ok(())
}

fn check_hostname(domain: &str) -> Result<(), InvalidServiceProxyUrl> {
    if domain.len() > MAX_PROXY_HOST_BYTES
        || domain
            .split('.')
            .any(|label| label.is_empty() || label.len() > MAX_PROXY_LABEL_BYTES)
    {
        return Err(InvalidServiceProxyUrl::Host);
    }
    Ok(())
}

impl fmt::Display for ServiceProxyUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

crate::key::serde_via_text!(ServiceProxyUrl);

#[cfg(test)]
mod tests;
