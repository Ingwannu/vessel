//! [`Roots`]: the certificates `wss://` relays are checked against.
//!
//! [Decision Log]
//! - Purpose: a paired node behind a relay is reached over `wss://`, and
//!   the relay's certificate is checked against the system's roots.
//! - Constraint: `vessel-relay` asks `rustls-native-certs`, which reads
//!   OpenSSL's file locations on Unix. Android keeps its roots elsewhere:
//!   in the Conscrypt module (`/apex/com.android.conscrypt/cacerts`,
//!   updated by the Play system updates since Android 14) and, before
//!   that, in the system image (`/system/etc/security/cacerts`); one file
//!   per certificate, PEM followed by a text dump. Without them every
//!   `wss://` relay would fail on a phone.
//! - Alternatives: `rustls-platform-verifier` (Android's own trust manager
//!   through JNI: roots the user added and revocation too, but a Java
//!   class in the app and JNI handed over before the first dial; the
//!   right answer if this shell is kept); setting `SSL_CERT_DIR` for
//!   `rustls-native-certs` (changes the process's environment, which is
//!   `unsafe` in this edition, and the workspace forbids it).
//! - Choice: on Android, read those directories ourselves (the first one
//!   holding any certificate) into the root store `vessel-relay`'s
//!   `TlsConfig::with_roots` takes; elsewhere (the shell on a desktop, the
//!   tests) the system's roots as the CLI loads them.
//! - Cost: certificates a user added in Android's settings are not
//!   trusted (Android keeps them where an app cannot read); revocation is
//!   not checked (it is not on the desktop either). Read once, when the
//!   bridge starts: a root store updated while the app runs is seen at the
//!   next start.

use std::path::Path;

use rustls::RootCertStore;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject as _;
use vessel_client::secure::{TlsError, WsDialer};
use vessel_proto::RouteUrl;
use vessel_relay::TlsConfig;

/// Where Android keeps its root certificates, newest first.
const ANDROID_ROOTS: [&str; 2] = [
    "/apex/com.android.conscrypt/cacerts",
    "/system/etc/security/cacerts",
];

/// The roots this app checks `wss://` relays against, loaded once.
#[derive(Debug, Clone)]
pub struct Roots {
    /// `None`: no root could be loaded; `ws://` routes still work.
    tls: Option<TlsConfig>,
}

/// Why a dialer could not be made for a node's routes.
#[derive(Debug, thiserror::Error)]
pub enum RootsError {
    /// A route is `wss://` and this phone gave no root certificate to
    /// check it against.
    #[error("no root certificates could be read on this device, so a wss:// relay cannot be checked")]
    NoRoots,
    /// rustls refused the configuration.
    #[error("TLS could not be configured")]
    Tls(#[source] TlsError),
}

impl Roots {
    /// The system's roots: Android's store on Android, the CLI's way
    /// elsewhere. Reads files; call it off the async runtime's workers.
    #[must_use]
    pub fn system() -> Self {
        if cfg!(target_os = "android") {
            Self::from_dirs(ANDROID_ROOTS.iter().map(Path::new))
        } else {
            Self {
                tls: TlsConfig::native_roots().ok(),
            }
        }
    }

    /// The certificates of the first of `dirs` that holds any (each file
    /// read as PEM; anything else in it is skipped). Reads files.
    #[must_use]
    pub fn from_dirs<'a>(dirs: impl IntoIterator<Item = &'a Path>) -> Self {
        let store = dirs
            .into_iter()
            .map(read_dir)
            .find(|store| !store.is_empty());
        let tls = store.and_then(|store| match TlsConfig::with_roots(store) {
            Ok(tls) => Some(tls),
            Err(error) => {
                tracing::warn!(%error, "the device's root certificates were refused");
                None
            }
        });
        Self { tls }
    }

    /// No roots: only `ws://` routes can be dialled (tests).
    #[must_use]
    pub const fn none() -> Self {
        Self { tls: None }
    }

    /// Whether any root was loaded.
    #[must_use]
    pub const fn loaded(&self) -> bool {
        self.tls.is_some()
    }

    /// A dialer for `routes`, as `WsDialer::for_routes` makes one: with
    /// no roots, `ws://` routes still work and a `wss://` route is an
    /// error now rather than at every dial.
    ///
    /// # Errors
    /// Some route is `wss://` and no root was loaded, or rustls refused
    /// an empty store.
    pub fn dialer<'a>(
        &self,
        mut routes: impl Iterator<Item = &'a RouteUrl>,
    ) -> Result<WsDialer, RootsError> {
        match &self.tls {
            Some(tls) => Ok(WsDialer::new(tls.clone())),
            None if routes.any(|route| route.as_str().starts_with("wss://")) => {
                Err(RootsError::NoRoots)
            }
            None => TlsConfig::with_roots(RootCertStore::empty())
                .map(WsDialer::new)
                .map_err(RootsError::Tls),
        }
    }
}

/// Every certificate in the files of `dir` that parses; a directory that
/// cannot be read gives none.
fn read_dir(dir: &Path) -> RootCertStore {
    let mut store = RootCertStore::empty();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return store;
    };
    for entry in entries.flatten() {
        let Ok(bytes) = std::fs::read(entry.path()) else {
            continue;
        };
        for certificate in CertificateDer::pem_slice_iter(&bytes).flatten() {
            if let Err(error) = store.add(certificate) {
                tracing::debug!(%error, "a root certificate was skipped");
            }
        }
    }
    tracing::debug!(dir = %dir.display(), roots = store.len(), "read root certificates");
    store
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A public root (ISRG Root X1), as Android stores one: the PEM, then
    /// the text dump `openssl x509 -text` writes.
    const ANDROID_FILE: &str = include_str!("../tests/fixtures/android-root.0");

    fn route(text: &str) -> RouteUrl {
        text.parse().unwrap()
    }

    #[test]
    fn the_first_directory_with_a_certificate_is_used_and_the_text_dump_skipped() {
        let empty = tempfile::tempdir().unwrap();
        let full = tempfile::tempdir().unwrap();
        std::fs::write(full.path().join("6187b673.0"), ANDROID_FILE).unwrap();
        std::fs::write(full.path().join("junk.txt"), "not a certificate").unwrap();
        let missing = empty.path().join("missing");
        let roots = Roots::from_dirs([missing.as_path(), empty.path(), full.path()]);
        assert!(roots.loaded());
        assert_eq!(read_dir(full.path()).len(), 1);
        let relay = route("wss://relay.example:443");
        assert!(roots.dialer([&relay].into_iter()).is_ok());
    }

    #[test]
    fn without_roots_only_plain_routes_get_a_dialer() {
        let roots = Roots::from_dirs([tempfile::tempdir().unwrap().path()]);
        assert!(!roots.loaded());
        let direct = route("ws://192.0.2.1:7070");
        let relay = route("wss://relay.example:443");
        assert!(roots.dialer([&direct].into_iter()).is_ok());
        assert!(matches!(
            roots.dialer([&direct, &relay].into_iter()),
            Err(RootsError::NoRoots)
        ));
    }
}
