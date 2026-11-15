//! Construct HTTP links from explicit hosts and observed listeners. A remote
//! loopback address must never become a link to a different local machine.
use std::num::NonZeroU16;

/// A host chosen explicitly by the person; no URL, credentials or path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewHost(String);

/// Browser destinations cannot carry arbitrary schemes or credentials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewUrl(String);

/// Why a port cannot safely be opened in the desktop browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PreviewProblem {
    /// No explicit destination is known.
    #[error("choose the service host before opening a preview")]
    MissingHost,
    /// A DNS name or IP literal is required.
    #[error("the service host must be a DNS name or IP address without credentials or a path")]
    InvalidHost,
    /// A remote host cannot expose its loopback socket to this browser.
    #[error("this remote listener is loopback-only; use an explicit local tunnel host")]
    RemoteLoopback,
    /// The observed socket changed after the row was drawn.
    #[error("refresh the listener before opening a preview")]
    Stale,
    /// Zero is not a listening TCP port.
    #[error("a preview needs a nonzero TCP port")]
    ZeroPort,
}

/// DNS wire names fit this many ASCII bytes, excluding a trailing dot.
const MAX_HOST_BYTES: usize = 253;
/// One DNS label's bound, also guarding malformed user input.
const MAX_LABEL_BYTES: usize = 63;

impl PreviewHost {
    /// Validate an explicit host before allowing the browser to receive it.
    /// # Errors
    /// Refuses URL syntax, whitespace, control characters and malformed hosts.
    pub fn parse(text: &str) -> Result<Self, PreviewProblem> {
        if text.is_empty() {
            return Err(PreviewProblem::MissingHost);
        }
        let ip = text.parse::<core::net::IpAddr>().ok();
        if let Some(ip) = ip {
            return Ok(Self(ip.to_string()));
        }
        let valid = text.len() <= MAX_HOST_BYTES
            && text.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= MAX_LABEL_BYTES
                    && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                    && !label.starts_with('-')
                    && !label.ends_with('-')
            });
        if valid {
            Ok(Self(text.to_ascii_lowercase()))
        } else {
            Err(PreviewProblem::InvalidHost)
        }
    }

    /// Whether an explicit browser host denotes this desktop's loopback.
    #[must_use]
    pub fn is_loopback(&self) -> bool {
        self.0 == "localhost"
            || self
                .0
                .parse::<core::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    }
    /// Build a destination only from the observed port, never a declared override.
    /// # Errors
    /// Refuses port zero from a malformed peer.
    pub fn url(&self, port: u16) -> Result<PreviewUrl, PreviewProblem> {
        let port = NonZeroU16::new(port).ok_or(PreviewProblem::ZeroPort)?;
        let host = if self.0.contains(':') {
            format!("[{}]", self.0)
        } else {
            self.0.clone()
        };
        Ok(PreviewUrl(format!("http://{host}:{port}/")))
    }
}
impl PreviewUrl {
    /// The fully displayed destination passed to the platform opener.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_explicit_http_hosts_and_observed_nonzero_ports_are_openable() {
        for bad in [
            "",
            "https://example.com",
            "user@host",
            "host/path",
            "x?token=secret",
            "a b",
            "a\nx",
            "-x",
            "x:",
            ".",
        ] {
            assert!(PreviewHost::parse(bad).is_err());
        }
        let v6 = PreviewHost::parse("::1").unwrap();
        assert_eq!(v6.url(3000).unwrap().as_str(), "http://[::1]:3000/");
        assert_eq!(v6.url(0), Err(PreviewProblem::ZeroPort));
        assert_eq!(
            PreviewHost::parse("EXAMPLE.com")
                .unwrap()
                .url(80)
                .unwrap()
                .as_str(),
            "http://example.com:80/"
        );
    }
}
