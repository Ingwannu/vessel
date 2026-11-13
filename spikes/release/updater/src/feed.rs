//! Where the feed is, and reading from it over HTTPS.
//!
//! The feed is a directory on any static host: `<base>/<channel>.json`
//! (the signed manifest) and the artifacts at the paths it names. The
//! base is a build-time setting, so no host name is in the repository.
//! Nothing here trusts what it reads: the manifest is checked by
//! [`crate::manifest::Manifest::open`], an artifact by the download.
//!
//! The client is rustls with `ring` and the system's roots, as every
//! HTTPS client of the main workspace is (ADR 0005's amendment), so the
//! app gains no second TLS stack and Windows needs no `CMake` or `NASM`.

use std::{sync::Arc, time::Duration};

use crate::{
    channel::Channel,
    manifest::{ArtifactPath, MAX_ENVELOPE_BYTES},
};

/// How long connecting may take.
const CONNECT: Duration = Duration::from_secs(15);

/// How long a whole manifest request may take.
const MANIFEST_TIME: Duration = Duration::from_secs(30);

/// How long the body may stall between two chunks of an artifact; the
/// whole download has no limit, as its size and the line's speed decide.
pub(crate) const READ_STALL: Duration = Duration::from_secs(60);

/// The feed's base address: `https://`, or `http://` to this machine
/// only (tests, and a feed served locally to try a build).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedUrl(reqwest::Url);

/// Why a text is not a feed's address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FeedUrlError {
    /// Not a URL.
    #[error("the feed's address is not a URL")]
    Url,
    /// Not `https://`, nor `http://` to a loopback address.
    #[error("the feed's address must be https:// (http:// only to this machine)")]
    Scheme,
    /// Carries a query, a fragment or credentials.
    #[error("the feed's address is a plain base, without query, fragment or credentials")]
    Extras,
}

impl FeedUrl {
    /// Parse a base address. A trailing `/` is added, so names join under
    /// it rather than replacing its last segment.
    ///
    /// # Errors
    /// [`FeedUrlError`] for what is not a plain `https://` base (or
    /// `http://` to a loopback address).
    pub fn parse(text: &str) -> Result<Self, FeedUrlError> {
        let mut url = reqwest::Url::parse(text).map_err(|_bad| FeedUrlError::Url)?;
        let loopback = url.host_str().is_some_and(|host| {
            let bare = host.trim_start_matches('[').trim_end_matches(']');
            host == "localhost"
                || bare
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|address| address.is_loopback())
        });
        match url.scheme() {
            "https" => {}
            "http" if loopback => {}
            _ => return Err(FeedUrlError::Scheme),
        }
        if url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(FeedUrlError::Extras);
        }
        if !url.path().ends_with('/') {
            let path = format!("{}/", url.path());
            url.set_path(&path);
        }
        Ok(Self(url))
    }

    /// The channel's manifest: `<base>/<channel>.json`.
    #[must_use]
    pub fn manifest(&self, channel: &Channel) -> reqwest::Url {
        self.join(&format!("{channel}.json"))
    }

    /// An artifact: `<base>/<path>`.
    #[must_use]
    pub fn artifact(&self, path: &ArtifactPath) -> reqwest::Url {
        self.join(path.as_str())
    }

    fn join(&self, relative: &str) -> reqwest::Url {
        // A channel and an artifact path are plain names by construction,
        // so the join stays under the base; if it ever could not, the base
        // itself is asked for, which serves no manifest.
        self.0
            .join(relative)
            .unwrap_or_else(|_never| self.0.clone())
    }
}

impl std::fmt::Display for FeedUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// Why reading from the feed failed.
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    /// TLS could not be set up.
    #[error("TLS could not be set up")]
    Tls,
    /// No answer, or the connection broke.
    #[error("the feed could not be reached")]
    Unreachable(#[source] reqwest::Error),
    /// An answer that is not 200.
    #[error("the feed answered {status}")]
    Status {
        /// The status.
        status: reqwest::StatusCode,
    },
    /// More bytes than allowed.
    #[error("the feed sent more than {limit} bytes")]
    TooLarge {
        /// The most that was allowed.
        limit: u64,
    },
    /// The body stopped arriving.
    #[error("the feed stopped sending")]
    Stalled,
}

/// The HTTP client the updater reads the feed with.
#[derive(Debug, Clone)]
pub struct FeedClient {
    client: reqwest::Client,
}

impl FeedClient {
    /// A client trusting the system's roots.
    ///
    /// # Errors
    /// [`FetchError::Tls`] when TLS cannot be set up,
    /// [`FetchError::Unreachable`] when the client cannot be built.
    pub fn new() -> Result<Self, FetchError> {
        let found = rustls_native_certs::load_native_certs();
        let mut roots = rustls::RootCertStore::empty();
        let (_added, _ignored) = roots.add_parsable_certificates(found.certs);
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut tls = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|_refused| FetchError::Tls)?
            .with_root_certificates(roots)
            .with_no_client_auth();
        tls.alpn_protocols = vec![b"http/1.1".to_vec()];
        let client = reqwest::Client::builder()
            .use_preconfigured_tls(tls)
            // A static feed has no reason to send the app elsewhere; a
            // redirect to another host is refused rather than followed.
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(CONNECT)
            .user_agent(concat!("vessel-app-updater/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(FetchError::Unreachable)?;
        Ok(Self { client })
    }

    /// The channel's envelope, at most [`MAX_ENVELOPE_BYTES`].
    ///
    /// # Errors
    /// [`FetchError`] for every way the request can fail.
    pub async fn envelope(&self, feed: &FeedUrl, channel: &Channel) -> Result<Vec<u8>, FetchError> {
        let response = self
            .client
            .get(feed.manifest(channel))
            // A cache between the app and the host must not hide a
            // publish; the manifest is small, so asking anew costs little.
            .header(reqwest::header::CACHE_CONTROL, "no-cache")
            .timeout(MANIFEST_TIME)
            .send()
            .await
            .map_err(FetchError::Unreachable)?;
        let mut response = ok(response)?;
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(FetchError::Unreachable)? {
            let len = u64::try_from(body.len().saturating_add(chunk.len())).unwrap_or(u64::MAX);
            if len > MAX_ENVELOPE_BYTES {
                return Err(FetchError::TooLarge {
                    limit: MAX_ENVELOPE_BYTES,
                });
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }

    /// Start downloading an artifact; the body is read by the caller
    /// ([`crate::stage`]), which bounds and checks it.
    ///
    /// # Errors
    /// [`FetchError`] for every way the request can fail.
    pub(crate) async fn artifact(
        &self,
        feed: &FeedUrl,
        path: &ArtifactPath,
    ) -> Result<reqwest::Response, FetchError> {
        let response = self
            .client
            .get(feed.artifact(path))
            .send()
            .await
            .map_err(FetchError::Unreachable)?;
        ok(response)
    }
}

fn ok(response: reqwest::Response) -> Result<reqwest::Response, FetchError> {
    let status = response.status();
    if status == reqwest::StatusCode::OK {
        Ok(response)
    } else {
        Err(FetchError::Status { status })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_base_is_https_or_loopback_http() {
        let feed = FeedUrl::parse("https://example.org/vessel/app").unwrap();
        assert_eq!(
            feed.manifest(&Channel::preview()).as_str(),
            "https://example.org/vessel/app/preview.json"
        );
        let path: ArtifactPath = "0.0.0.0.1/vessel.exe".parse().unwrap();
        assert_eq!(
            feed.artifact(&path).as_str(),
            "https://example.org/vessel/app/0.0.0.0.1/vessel.exe"
        );
        assert!(FeedUrl::parse("http://127.0.0.1:8080/").is_ok());
        assert!(FeedUrl::parse("http://localhost/feed").is_ok());
        assert!(FeedUrl::parse("http://[::1]/").is_ok());
        assert_eq!(
            FeedUrl::parse("http://example.org/"),
            Err(FeedUrlError::Scheme)
        );
        assert_eq!(
            FeedUrl::parse("ftp://example.org/"),
            Err(FeedUrlError::Scheme)
        );
        assert_eq!(
            FeedUrl::parse("file:///tmp/feed"),
            Err(FeedUrlError::Scheme)
        );
        assert_eq!(
            FeedUrl::parse("https://example.org/?a=1"),
            Err(FeedUrlError::Extras)
        );
        assert_eq!(
            FeedUrl::parse("https://u:p@example.org/"),
            Err(FeedUrlError::Extras)
        );
        assert_eq!(FeedUrl::parse("not a url"), Err(FeedUrlError::Url));
    }
}
