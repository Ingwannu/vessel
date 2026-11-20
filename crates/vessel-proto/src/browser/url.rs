//! An `http` or `https` URL a browser may open.
//!
//! Named `BrowserUrl` because [`crate::HttpUrl`] is the fleet manifest
//! URL, which refuses userinfo. A page URL may carry userinfo; `Debug`
//! redacts it, and a refusal never quotes the text.
//!
//! A tool argument with no scheme and no whitespace becomes `http://`
//! plus the text, as Paseo's `normalizeHttpUrlInput` does
//! (`browser-tools/tools.ts`). `javascript:`, `file:`, `data:` and
//! embedded whitespace are rejected here, so the wire value cannot
//! represent them.

use std::fmt;
use std::str::FromStr;

use super::InvalidInput;

/// An `http` or `https` URL with a host.
#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct BrowserUrl(String);

impl BrowserUrl {
    /// The URL a tool argument names, with a missing scheme filled in.
    ///
    /// # Errors
    /// Empty, whitespace, a non-http scheme, or no host. The error does
    /// not contain the text.
    pub fn parse_tool(text: &str) -> Result<Self, InvalidInput> {
        if text.is_empty() {
            return Err(InvalidInput::UrlEmpty);
        }
        if text.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(InvalidInput::UrlSpace);
        }
        if let Some((scheme, rest)) = text.split_once("://") {
            if scheme_is_http(scheme) {
                return if host_ok(rest) {
                    Ok(Self(text.to_owned()))
                } else {
                    Err(InvalidInput::UrlHost)
                };
            }
            if looks_like_scheme(scheme) {
                return Err(InvalidInput::UrlScheme);
            }
        }
        let prefixed = format!("http://{text}");
        let Some((_, rest)) = prefixed.split_once("://") else {
            return Err(InvalidInput::UrlScheme);
        };
        if host_ok(rest) {
            Ok(Self(prefixed))
        } else {
            Err(InvalidInput::UrlScheme)
        }
    }

    /// The URL text, as it was accepted.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BrowserUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for BrowserUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BrowserUrl({})", redacted(&self.0))
    }
}

impl FromStr for BrowserUrl {
    type Err = InvalidInput;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let Some((scheme, rest)) = text.split_once("://") else {
            return Err(InvalidInput::UrlScheme);
        };
        if !scheme_is_http(scheme) {
            return Err(InvalidInput::UrlScheme);
        }
        if text.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(InvalidInput::UrlSpace);
        }
        if host_ok(rest) {
            Ok(Self(text.to_owned()))
        } else {
            Err(InvalidInput::UrlHost)
        }
    }
}

crate::key::serde_via_text!(BrowserUrl);

fn scheme_is_http(scheme: &str) -> bool {
    scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
}

fn looks_like_scheme(scheme: &str) -> bool {
    let mut chars = scheme.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        Some(_) | None => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
}

/// `rest` is the URL after `scheme://`. The host is the authority, after
/// any userinfo, with a numeric port when a colon is present.
fn host_ok(rest: &str) -> bool {
    let authority = match rest.split(['/', '?', '#']).next() {
        Some(authority) => authority,
        None => return false,
    };
    if authority.is_empty() {
        return false;
    }
    let hostport = match authority.rsplit_once('@') {
        Some((_, host)) => host,
        None => authority,
    };
    if let Some(host) = hostport.strip_prefix('[') {
        let Some((inside, after)) = host.split_once(']') else {
            return false;
        };
        if inside.is_empty() {
            return false;
        }
        return match after.strip_prefix(':') {
            None => after.is_empty(),
            Some(port) => !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()),
        };
    }
    match hostport.rsplit_once(':') {
        None => !hostport.is_empty(),
        Some((host, port)) => {
            !host.is_empty()
                && !host.contains(':')
                && !port.is_empty()
                && port.chars().all(|c| c.is_ascii_digit())
        }
    }
}

/// The URL with userinfo replaced, for `Debug` only.
fn redacted(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    match authority.rsplit_once('@') {
        Some((_, host)) => format!("{scheme}://<redacted>@{host}{tail}"),
        None => url.to_owned(),
    }
}
