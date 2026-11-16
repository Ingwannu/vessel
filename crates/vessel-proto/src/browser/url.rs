//! URL preparation is distinct for the address bar and an agent tool.
//!
//! Paseo defaults a tool's scheme to http but a bare address-bar hostname to
//! https. Keep those intentions explicit and never let a free-form scheme or
//! userinfo become a browser capability. Network reach is the engine's policy.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use url::Url;

/// A URL error does not retain the untrusted URL or a credential-bearing fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidBrowserUrl {
    /// The input must be an address before any process/network work starts.
    #[error("browser URL is empty")]
    Empty,
    /// Control/whitespace normalization must not hide an unsafe scheme.
    #[error("browser URL contains whitespace or control characters")]
    Whitespace,
    /// The engine cannot launch external handlers or local-file navigation.
    #[error("browser navigation supports only HTTP and HTTPS")]
    Scheme,
    /// Authentication belongs to the isolated browser profile, not the URL.
    #[error("browser URL must not contain user information")]
    Credentials,
    /// Numeric aliases must not conceal loopback or change interpretation later.
    #[error("browser URL must use canonical numeric host syntax")]
    NumericHost,
    /// Retain the parser's cause without reflecting the source address.
    #[error("browser URL could not be parsed")]
    Parse(#[source] url::ParseError),
}

/// A parsed HTTP(S) URL; Debug never turns a page's URL into a diagnostic leak.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct BrowserUrl(Url);

impl BrowserUrl {
    /// MCP keeps the reference's HTTP default for a scheme-less host.
    ///
    /// # Errors
    /// Refuses empty/whitespace, non-HTTP schemes, malformed hosts and userinfo.
    pub fn from_tool(text: &str) -> Result<Self, InvalidBrowserUrl> {
        if text.is_empty() {
            return Err(InvalidBrowserUrl::Empty);
        }
        if text.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(InvalidBrowserUrl::Whitespace);
        }
        if http_prefix(text) {
            return text.parse();
        }
        // A decimal port on a hostname is not an external-handler scheme.
        if let Some((scheme, _)) = text.split_once(':')
            && scheme.bytes().all(|byte| byte.is_ascii_alphabetic())
            && !text.split_once(':').is_some_and(|(_, tail)| {
                tail.bytes()
                    .next()
                    .is_some_and(|byte| byte.is_ascii_digit())
            })
        {
            return Err(InvalidBrowserUrl::Scheme);
        }
        format!("http://{text}").parse()
    }

    /// Address-bar intent keeps local hosts HTTP and general hostnames HTTPS.
    ///
    /// # Errors
    /// The same refusal boundary applies after the address is prepared.
    pub fn from_address(text: &str) -> Result<Self, InvalidBrowserUrl> {
        let text = text.trim();
        if text.is_empty() {
            return "https://example.com".parse();
        }
        if text.starts_with("//") {
            return format!("https:{text}").parse();
        }
        if text.contains("://") || text.starts_with("about:") {
            return text.parse();
        }
        let local = Self::from_tool(text)?;
        match local.0.host() {
            Some(url::Host::Ipv4(_) | url::Host::Ipv6(_) | url::Host::Domain("localhost")) => {
                Ok(local)
            }
            Some(url::Host::Domain(_)) => format!("https://{text}").parse(),
            None => Err(InvalidBrowserUrl::Parse(url::ParseError::EmptyHost)),
        }
    }

    /// Explicit page navigation needs the complete URL; never use it as a log field.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The parsed origin is available without reparsing page-controlled text.
    #[must_use]
    pub fn parsed(&self) -> &Url {
        &self.0
    }
}

fn http_prefix(text: &str) -> bool {
    text.get(.."http://".len())
        .is_some_and(|s| s.eq_ignore_ascii_case("http://"))
        || text
            .get(.."https://".len())
            .is_some_and(|s| s.eq_ignore_ascii_case("https://"))
}

impl FromStr for BrowserUrl {
    type Err = InvalidBrowserUrl;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if !http_prefix(text) {
            return Err(InvalidBrowserUrl::Scheme);
        }
        if text.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(InvalidBrowserUrl::Whitespace);
        }
        let parsed = Url::parse(text).map_err(InvalidBrowserUrl::Parse)?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host().is_none() {
            return Err(InvalidBrowserUrl::Scheme);
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(InvalidBrowserUrl::Credentials);
        }
        if let Some(url::Host::Ipv4(address)) = parsed.host() {
            let authority = text
                .split_once("://")
                .map(|(_, tail)| tail)
                .and_then(|tail| tail.split(['/', '?', '#']).next())
                .ok_or(InvalidBrowserUrl::NumericHost)?;
            let host = authority
                .split(':')
                .next()
                .ok_or(InvalidBrowserUrl::NumericHost)?;
            if host != address.to_string() {
                return Err(InvalidBrowserUrl::NumericHost);
            }
        }
        Ok(Self(parsed))
    }
}

impl fmt::Debug for BrowserUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BrowserUrl([redacted])")
    }
}

impl TryFrom<String> for BrowserUrl {
    type Error = InvalidBrowserUrl;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<BrowserUrl> for String {
    fn from(value: BrowserUrl) -> Self {
        value.0.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_bar_and_tool_scheme_defaults_are_distinct() {
        assert_eq!(
            BrowserUrl::from_tool("example.com").unwrap().as_str(),
            "http://example.com/"
        );
        assert_eq!(
            BrowserUrl::from_address("example.com").unwrap().as_str(),
            "https://example.com/"
        );
        for local in [
            "localhost:8080/path",
            "127.0.0.1:8080/path",
            "[::1]:8080/path",
        ] {
            assert_eq!(
                BrowserUrl::from_address(local).unwrap(),
                BrowserUrl::from_tool(local).unwrap()
            );
        }
        assert_eq!(
            BrowserUrl::from_address("//example.com").unwrap().as_str(),
            "https://example.com/"
        );
    }

    #[test]
    fn numeric_aliases_do_not_hide_the_actual_origin() {
        for alias in [
            "127.1",
            "2130706433",
            "0x7f.0.0.1",
            "0177.0.0.1",
            "127.0.0.1.",
        ] {
            assert!(BrowserUrl::from_tool(alias).is_err());
        }
        assert!(BrowserUrl::from_tool("127.0.0.1").is_ok());
        assert!(BrowserUrl::from_tool("[::ffff:127.0.0.1]").is_ok());
    }

    #[test]
    fn schemes_credentials_and_debug_cannot_bypass_the_boundary() {
        for bad in [
            "javascript:alert(1)",
            "file:///tmp/file",
            "data:text/plain,a",
            "about:blank",
            "https://user:secret@example.com",
            "https://exa mple.com",
        ] {
            assert!(BrowserUrl::from_tool(bad).is_err());
            assert!(bad.parse::<BrowserUrl>().is_err());
        }
        let url: BrowserUrl = "https://example.com/private-secret?token=secret#secret"
            .parse()
            .unwrap();
        assert!(!format!("{url:?}").contains("secret"));
        assert!(serde_json::from_str::<BrowserUrl>(r#""file:///tmp/file""#).is_err());
    }
}
