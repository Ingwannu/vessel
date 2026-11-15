//! `vessel-app-feed`: the release key and the signed feed of the native
//! app's builds (`docs/plans/backend-m12-release.md`, "Updates").
//!
//! ```text
//! vessel-app-feed keygen DIR
//! vessel-app-feed publish --key FILE --feed DIR --channel NAME --label LABEL
//!                         [--notes FILE] [--published TEXT]
//!                         --artifact TARGET:PACKAGING:PATH ...
//! vessel-app-feed sums --key FILE --label LABEL DIR
//! vessel-app-feed check --feed DIR --public KEY --channel NAME
//! vessel-app-feed check-sums --public KEY DIR
//! ```
//!
//! `keygen` makes the key pair once, outside any repository: the secret
//! half is created with mode 0600 in a directory of mode 0700 and never
//! printed; the public half is printed, as the app is built with it.
//! `publish` writes `<feed>/<channel>.json`, the signed manifest the app
//! reads, from artifacts already under the feed. `sums` writes
//! `SHA256SUMS` and its minisign signature for a label's directory, for
//! people checking a download by hand (`minisign -Vm SHA256SUMS -P KEY`).
//! `check` reads a feed as the app does, with the app's own code, and
//! checks every artifact's size and digest.
//!
//! The key is unencrypted: the release script signs without a person at
//! the keyboard. Its protection is the file's mode and where it lives;
//! when releases move to CI, the key moves to the CI's secret store and
//! the owner may choose a password (the plan lists it as theirs).

mod args;
mod publish;
mod sums;

use std::{io::Write as _, path::Path, process::ExitCode};

use args::Command;

fn main() -> ExitCode {
    let command = match args::parse(std::env::args_os().skip(1)) {
        Ok(command) => command,
        Err(error) => {
            report(&format!("{error}\n\n{}", args::USAGE));
            return ExitCode::from(2);
        }
    };
    let result = match command {
        Command::Keygen { dir } => keygen(&dir),
        Command::Publish(publish) => publish::publish(&publish).map(|line| say(&line)),
        Command::Sums { key, label, dir } => sums::write(&key, &label, &dir).map(|line| say(&line)),
        Command::CheckSums { public, dir } => sums::check(&public, &dir).map(|line| say(&line)),
        Command::Check {
            feed,
            public,
            channel,
        } => publish::check(&feed, &public, &channel).map(|line| say(&line)),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report(&error.to_string());
            ExitCode::FAILURE
        }
    }
}

/// Why a command failed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum FeedError {
    /// A file could not be read or written.
    #[error("{what}: {source}")]
    Io {
        /// Which file, and what was being done.
        what: String,
        /// The error.
        #[source]
        source: std::io::Error,
    },
    /// The key could not be made, read or used.
    #[error("{what}: {source}")]
    Key {
        /// What was being done.
        what: &'static str,
        /// The error.
        #[source]
        source: minisign::PError,
    },
    /// The key files exist already.
    #[error("{0} exists; a release key is made once")]
    Exists(String),
    /// A value does not parse.
    #[error("{0}")]
    Value(String),
    /// What was published does not open as the app opens it.
    #[error("the feed does not open as the app opens it: {0}")]
    Check(String),
}

fn io(what: impl std::fmt::Display) -> impl FnOnce(std::io::Error) -> FeedError {
    let what = what.to_string();
    move |source| FeedError::Io { what, source }
}

/// The two key files `keygen` writes in its directory.
pub(crate) const SECRET_FILE: &str = "vessel-app-update.key";
const PUBLIC_FILE: &str = "vessel-app-update.pub";

fn keygen(dir: &Path) -> Result<(), FeedError> {
    std::fs::create_dir_all(dir).map_err(io(format!("creating {}", dir.display())))?;
    restrict_dir(dir)?;
    let secret = dir.join(SECRET_FILE);
    let public = dir.join(PUBLIC_FILE);
    for path in [&secret, &public] {
        if path.exists() {
            return Err(FeedError::Exists(path.display().to_string()));
        }
    }
    let pair =
        minisign::KeyPair::generate_unencrypted_keypair().map_err(|source| FeedError::Key {
            what: "making the key pair",
            source,
        })?;
    let secret_text = pair
        .sk
        .to_box(Some("Vessel app release key (secret)"))
        .map_err(|source| FeedError::Key {
            what: "writing the secret key",
            source,
        })?
        .into_string();
    let public_text = pair
        .pk
        .to_box()
        .map_err(|source| FeedError::Key {
            what: "writing the public key",
            source,
        })?
        .into_string();
    write_new(&secret, secret_text.as_bytes(), true)?;
    write_new(&public, public_text.as_bytes(), false)?;
    say(&format!(
        "made {} (secret, 0600) and {}\npublic key (VESSEL_UPDATE_KEY): {}",
        secret.display(),
        public.display(),
        pair.pk.to_base64()
    ));
    Ok(())
}

/// Create `path`, which must not exist; a secret is created 0600 so it
/// never exists, even for a moment, readable by others.
fn write_new(path: &Path, bytes: &[u8], secret: bool) -> Result<(), FeedError> {
    let mut options = std::fs::OpenOptions::new();
    let _same = options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        let _same = options.mode(if secret { 0o600 } else { 0o644 });
    }
    #[cfg(not(unix))]
    let _unused = secret;
    let mut file = options
        .open(path)
        .map_err(io(format!("creating {}", path.display())))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(io(format!("writing {}", path.display())))
}

fn restrict_dir(dir: &Path) -> Result<(), FeedError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .map_err(io(format!("restricting {}", dir.display())))?;
    }
    #[cfg(not(unix))]
    let _unused = dir;
    Ok(())
}

/// Read a secret key file `keygen` wrote.
pub(crate) fn read_secret(path: &Path) -> Result<minisign::KeyPair, FeedError> {
    let text = std::fs::read_to_string(path).map_err(io(format!("reading {}", path.display())))?;
    let sk = minisign::SecretKeyBox::from_string(&text)
        .and_then(minisign::SecretKeyBox::into_unencrypted_secret_key)
        .map_err(|source| FeedError::Key {
            what: "reading the secret key",
            source,
        })?;
    let pk = minisign::PublicKey::from_secret_key(&sk).map_err(|source| FeedError::Key {
        what: "deriving the public key",
        source,
    })?;
    Ok(minisign::KeyPair { pk, sk })
}

fn say(line: &str) {
    let mut out = std::io::stdout().lock();
    let _ignored = writeln!(out, "{line}");
}

fn report(line: &str) {
    let mut err = std::io::stderr().lock();
    let _ignored = writeln!(err, "vessel-app-feed: {line}");
}
