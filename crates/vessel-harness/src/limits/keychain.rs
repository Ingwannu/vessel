//! Claude Code's service/account derivation, after its saved-login convention.
//! A custom config home never falls back to the default service: that could
//! read a different account. The OS helper is fixed, never supplied by RPC.
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;
const DIGITS: &[u8; 16] = b"0123456789abcdef";

pub(super) fn service(config_dir: Option<&str>) -> String {
    let base = "Claude Code-credentials";
    match config_dir.filter(|value| !value.is_empty()) {
        None => base.to_owned(),
        Some(directory) => {
            let normalized: String = directory.nfc().collect();
            let hash = Sha256::digest(normalized.as_bytes());
            let mut suffix = String::with_capacity(8);
            for byte in hash.iter().take(4) {
                for nibble in [byte >> 4, byte & 0x0f] {
                    suffix.push(char::from(
                        DIGITS.get(usize::from(nibble)).copied().unwrap_or(b'0'),
                    ));
                }
            }
            format!("{base}-{suffix}")
        }
    }
}
pub(super) fn account(user: Option<&str>) -> &str {
    user.filter(|value| {
        !value.is_empty()
            && value.len() <= 256
            && value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
    })
    .unwrap_or("claude-code-user")
}
#[cfg(any(test, target_os = "macos"))]
pub(super) fn arguments(account: Option<&str>, service: &str) -> Vec<String> {
    let mut result = vec!["find-generic-password".to_owned()];
    if let Some(account) = account {
        result.extend(["-a".to_owned(), account.to_owned()]);
    }
    result.extend(["-w".to_owned(), "-s".to_owned(), service.to_owned()]);
    result
}
#[cfg(target_os = "macos")]
pub(super) async fn read(service: &str, account: &str) -> Option<super::credential::Credential> {
    use std::time::Duration;
    use tokio::io::AsyncReadExt;
    use vessel_proc::CommandSpec;
    use vessel_proto::limits::LimitLoginStore;
    use zeroize::Zeroizing;
    for who in [Some(account), None] {
        let command = CommandSpec::new("/usr/bin/security").args(arguments(who, service));
        let Ok(mut child) = vessel_proc::spawn(&command) else {
            continue;
        };
        drop(child.stdin);
        let read = async {
            let mut body = Zeroizing::new(Vec::new());
            let mut errors = Zeroizing::new(Vec::new());
            let (out, err) = tokio::join!(
                child
                    .stdout
                    .take(super::credential::MAX.saturating_add(1))
                    .read_to_end(&mut body),
                child
                    .stderr
                    .take(super::credential::MAX.saturating_add(1))
                    .read_to_end(&mut errors)
            );
            if out.is_err()
                || err.is_err()
                || u64::try_from(body.len()).ok()? > super::credential::MAX
            {
                return None;
            }
            if child.child.wait().await.ok()? != Some(0) {
                return None;
            }
            super::credential::decode(&body, LimitLoginStore::ClaudeCode).ok()
        };
        if let Ok(Some(credential)) = tokio::time::timeout(Duration::from_secs(2), read).await {
            return Some(credential);
        }
    }
    None
}
