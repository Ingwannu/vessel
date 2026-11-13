//! Refuse, at build time, a release setting that the app would refuse at
//! run time (`src/config.rs` lists them). Without this a typo in the
//! feed's address or the key builds fine and ships an app that never
//! updates, which the owner would find out only when the next build does
//! not arrive.
//!
//! The checks here are the shape only (the parsers live in `src/` and are
//! not reachable from a build script without copying them); the app's
//! parsers are the authority.

/// A check of one setting's text.
type Check = fn(&str) -> bool;

fn main() {
    for name in [
        "VESSEL_APP_LABEL",
        "VESSEL_UPDATE_FEED",
        "VESSEL_UPDATE_KEY",
        "VESSEL_UPDATE_CHANNEL",
    ] {
        println!("cargo::rerun-if-env-changed={name}");
    }
    let checks: [(&str, Check); 4] = [
        ("VESSEL_APP_LABEL", label),
        ("VESSEL_UPDATE_FEED", feed),
        ("VESSEL_UPDATE_KEY", key),
        ("VESSEL_UPDATE_CHANNEL", channel),
    ];
    for (name, check) in checks {
        if let Some(value) = std::env::var_os(name) {
            let fine = value.to_str().is_some_and(check);
            if !fine {
                // The value is not printed: it may be pasted from a file
                // that holds more than it should.
                println!("cargo::error={name} is set but is not in the form the app reads");
            }
        }
    }
}

fn label(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn feed(text: &str) -> bool {
    let local = ["http://127.0.0.1", "http://localhost", "http://[::1]"];
    text.starts_with("https://") || local.iter().any(|prefix| text.starts_with(prefix))
}

fn key(text: &str) -> bool {
    // A minisign public key is 42 bytes, 56 characters of base64.
    let text = text.trim();
    text.len() == 56
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
}

fn channel(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && text.len() <= 32
}
