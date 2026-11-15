//! Local sign-ins are a sensitive edge. Raw serde/I/O failures are
//! retained privately until the public probe projects an actionable
//! category. Their sources may echo a malformed credential: never log
//! them or expose their chains beyond this format boundary.
use serde::Deserialize;
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};
use vessel_proto::limits::{LimitFailure, LimitLoginStore};
use zeroize::Zeroizing;

pub(super) const MAX: u64 = 1_048_576;
#[derive(thiserror::Error)]
pub(super) enum CredentialError {
    #[error("no saved OAuth sign-in was found")]
    Missing,
    #[error("the saved sign-in could not be read")]
    Io(#[source] io::Error),
    #[error("the saved sign-in shape could not be read")]
    Decode(#[source] serde_json::Error),
    #[error("the saved sign-in could not be read safely")]
    Unsafe,
    #[error("the local sign-in check was cancelled")]
    Cancelled,
}
impl std::fmt::Debug for CredentialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl CredentialError {
    pub(super) fn failure(&self) -> LimitFailure {
        match self {
            Self::Missing | Self::Decode(_) => LimitFailure::NoLogin,
            Self::Cancelled => LimitFailure::Unsupported,
            Self::Io(error) if error.kind() == io::ErrorKind::NotFound => LimitFailure::NoLogin,
            Self::Io(_) | Self::Unsafe => LimitFailure::UnsafeLoginStore,
        }
    }
}

#[derive(Deserialize)]
#[serde(from = "String")]
struct Secret(Zeroizing<String>);
impl From<String> for Secret {
    fn from(value: String) -> Self {
        Self(Zeroizing::new(value))
    }
}
// No Debug or Serialize: neither a malformed sign-in nor an error's source
// may accidentally make its contents available to diagnostics.
#[derive(Deserialize)]
struct Claude {
    #[serde(rename = "claudeAiOauth")]
    oauth: Option<ClaudeOauth>,
}
#[derive(Deserialize)]
struct ClaudeOauth {
    #[serde(rename = "accessToken")]
    access: Option<Secret>,
    #[serde(rename = "expiresAt")]
    expires: Option<i64>,
}
#[derive(Deserialize)]
struct Codex {
    tokens: Option<CodexTokens>,
}
#[derive(Deserialize)]
struct CodexTokens {
    access_token: Option<Secret>,
    account_id: Option<Secret>,
}
#[derive(Deserialize)]
struct OpenCode {
    openai: Option<OpenAi>,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum AuthKind {
    Oauth,
    #[serde(other)]
    Other,
}
#[derive(Deserialize)]
struct OpenAi {
    #[serde(rename = "type")]
    kind: AuthKind,
    access: Option<Secret>,
    account_id: Option<Secret>,
    #[serde(rename = "accountId")]
    camel_account_id: Option<Secret>,
    expires: Option<i64>,
}

pub(super) struct Credential {
    pub(super) token: Zeroizing<String>,
    pub(super) account: Option<Zeroizing<String>>,
    pub(super) expires_ms: Option<i64>,
}

pub(super) fn read(
    path: &Path,
    guard: Option<&Path>,
    store: LimitLoginStore,
) -> Result<Credential, CredentialError> {
    let path = if let Some(root) = guard {
        let actual = path.canonicalize().map_err(CredentialError::Io)?;
        let root = root.canonicalize().map_err(CredentialError::Io)?;
        if !actual.starts_with(root) {
            return Err(CredentialError::Unsafe);
        }
        actual
    } else {
        path.to_owned()
    };
    let body = bounded(&path)?;
    decode(&body, store)
}
pub(super) fn bounded(path: &Path) -> Result<Zeroizing<Vec<u8>>, CredentialError> {
    if !std::fs::metadata(path)
        .map_err(CredentialError::Io)?
        .is_file()
    {
        return Err(CredentialError::Unsafe);
    }
    let file = File::open(path).map_err(CredentialError::Io)?;
    if !file.metadata().map_err(CredentialError::Io)?.is_file() {
        return Err(CredentialError::Unsafe);
    }
    let mut body = Zeroizing::new(Vec::new());
    let _read = file
        .take(MAX.saturating_add(1))
        .read_to_end(&mut body)
        .map_err(CredentialError::Io)?;
    if u64::try_from(body.len()).map_or(true, |size| size > MAX) {
        return Err(CredentialError::Unsafe);
    }
    Ok(body)
}
pub(super) fn decode(body: &[u8], store: LimitLoginStore) -> Result<Credential, CredentialError> {
    let (token, account, expires_ms) = match store {
        LimitLoginStore::ClaudeCode => {
            let parsed: Claude = serde_json::from_slice(body).map_err(CredentialError::Decode)?;
            let oauth = parsed.oauth.ok_or(CredentialError::Missing)?;
            (oauth.access, None, oauth.expires)
        }
        LimitLoginStore::Codex => {
            let parsed: Codex = serde_json::from_slice(body).map_err(CredentialError::Decode)?;
            let tokens = parsed.tokens.ok_or(CredentialError::Missing)?;
            (tokens.access_token, tokens.account_id, None)
        }
        LimitLoginStore::OpenCode => {
            let parsed: OpenCode = serde_json::from_slice(body).map_err(CredentialError::Decode)?;
            let oauth = parsed.openai.ok_or(CredentialError::Missing)?;
            match oauth.kind {
                AuthKind::Oauth => {}
                AuthKind::Other => return Err(CredentialError::Missing),
            }
            (
                oauth.access,
                oauth.camel_account_id.or(oauth.account_id),
                oauth.expires,
            )
        }
    };
    let token = token
        .map(|value| value.0)
        .filter(|token| {
            !token.is_empty() && token.len() <= 65_536 && !token.chars().any(char::is_control)
        })
        .ok_or(CredentialError::Missing)?;
    let account = match account.map(|value| value.0) {
        Some(value) if value.is_empty() => None,
        Some(value) if value.len() <= 4096 && !value.chars().any(char::is_control) => Some(value),
        Some(_) => return Err(CredentialError::Unsafe),
        None => None,
    };
    Ok(Credential {
        token,
        account,
        expires_ms,
    })
}
