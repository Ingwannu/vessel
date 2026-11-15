//! Exact local candidates only. No client supplies a path, vendor URL, header or sign-in material.
use super::AccountSourceAccess;
use crate::account_usage::Number;
use chrono::{DateTime, Utc};
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;
use vessel_proto::{limits::LimitFailure, usage_reports::BuiltinUsageSource};
use zeroize::Zeroizing;
#[derive(Clone, Copy, Default)]
pub(super) enum Region {
    #[default]
    Global,
    China,
}
impl Region {
    pub(super) const fn identity(self) -> &'static [u8] {
        match self {
            Self::Global => b"global",
            Self::China => b"china",
        }
    }
}
pub(super) struct Credential {
    pub(super) token: Zeroizing<String>,
    pub(super) expires: Option<DateTime<Utc>>,
    pub(super) region: Region,
}
impl Credential {
    fn token(token: Zeroizing<String>) -> Self {
        Self {
            token,
            expires: None,
            region: Region::Global,
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
fn environment(access: &AccountSourceAccess, names: &[&str]) -> Option<Credential> {
    names
        .iter()
        .find_map(|name| access.value(name))
        .map(Credential::token)
}
fn file<T: DeserializeOwned>(path: &Path, guard: Option<&Path>) -> Result<Option<T>, LimitFailure> {
    let path = guarded(path, guard)?;
    let body = match super::super::credential::bounded(&path) {
        Ok(body) => body,
        Err(error) if error.failure() == LimitFailure::NoLogin => return Ok(None),
        Err(error) => return Err(error.failure()),
    };
    Ok(serde_json::from_slice(&body).ok())
}
fn guarded(path: &Path, guard: Option<&Path>) -> Result<PathBuf, LimitFailure> {
    let Some(root) = guard else {
        return Ok(path.to_owned());
    };
    // Missing candidates remain missing; existing candidates cannot escape a fixture root through links.
    if !path.exists() {
        return Ok(path.to_owned());
    }
    let actual = path
        .canonicalize()
        .map_err(|_private| LimitFailure::UnsafeLoginStore)?;
    let root = root
        .canonicalize()
        .map_err(|_private| LimitFailure::UnsafeLoginStore)?;
    if !actual.starts_with(&root) {
        return Err(LimitFailure::UnsafeLoginStore);
    }
    Ok(actual)
}
pub(super) fn read(
    access: &AccountSourceAccess,
    source: BuiltinUsageSource,
    guard: Option<&Path>,
    stop: &CancellationToken,
) -> Result<Credential, LimitFailure> {
    if stop.is_cancelled() {
        return Err(LimitFailure::Unsupported);
    }
    let credential = match source {
        BuiltinUsageSource::Copilot => copilot(access, guard),
        BuiltinUsageSource::Cursor => cursor(access, guard),
        BuiltinUsageSource::Grok => grok(access, guard),
        BuiltinUsageSource::Kimi => kimi(access, guard),
        BuiltinUsageSource::MiniMax => minimax(access, guard),
        BuiltinUsageSource::OpenCodeGo => go(access, guard),
        BuiltinUsageSource::Zai => Ok(environment(access, &["ZAI_API_KEY", "GLM_API_KEY"])),
        BuiltinUsageSource::Claude | BuiltinUsageSource::Codex => Err(LimitFailure::Unsupported),
    }?;
    if stop.is_cancelled() {
        return Err(LimitFailure::Unsupported);
    }
    credential
        .filter(|credential| !credential.token.trim().is_empty())
        .ok_or(LimitFailure::NoLogin)
}
fn copilot(
    access: &AccountSourceAccess,
    guard: Option<&Path>,
) -> Result<Option<Credential>, LimitFailure> {
    if access
        .value("GH_HOST")
        .is_some_and(|host| host.as_str() != "github.com")
        || access
            .value("GITHUB_API_URL")
            .is_some_and(|url| url.trim_end_matches('/') != "https://api.github.com")
    {
        return Err(LimitFailure::DifferentRoute);
    }
    if let Some(credential) = environment(access, &["COPILOT_TOKEN", "GITHUB_TOKEN", "GITHUB_PAT"])
    {
        return Ok(Some(credential));
    }
    let base = if cfg!(windows) {
        access
            .absolute("APPDATA", access.home.join("AppData").join("Roaming"))?
            .join("GitHub CLI")
    } else {
        access.home.join(".config").join("gh")
    };
    let path = guarded(&base.join("hosts.yml"), guard)?;
    let body = match super::super::credential::bounded(&path) {
        Ok(body) => body,
        Err(error) if error.failure() == LimitFailure::NoLogin => return Ok(None),
        Err(error) => return Err(error.failure()),
    };
    let text = std::str::from_utf8(&body).map_err(|_private| LimitFailure::NoLogin)?;
    let mut github = false;
    for line in text.lines() {
        if !line.starts_with(char::is_whitespace) {
            github = line.trim() == "github.com:";
        }
        if github && let Some(token) = line.trim().strip_prefix("oauth_token:") {
            let token = token.trim().trim_matches(['\'', '"']);
            if !token.is_empty()
                && token
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            {
                return Ok(Some(Credential::token(Zeroizing::new(token.to_owned()))));
            }
        }
    }
    Ok(None)
}
#[derive(Deserialize)]
struct CursorAuth {
    #[serde(rename = "accessToken")]
    access_token: Option<Secret>,
}
fn cursor(
    access: &AccountSourceAccess,
    guard: Option<&Path>,
) -> Result<Option<Credential>, LimitFailure> {
    if let Some(credential) = environment(access, &["CURSOR_ACCESS_TOKEN", "CURSOR_TOKEN"]) {
        return Ok(Some(credential));
    }
    let base = if cfg!(windows) {
        access
            .absolute("APPDATA", access.home.join("AppData").join("Roaming"))?
            .join("Cursor")
    } else if cfg!(target_os = "macos") {
        access
            .home
            .join("Library")
            .join("Application Support")
            .join("Cursor")
    } else {
        access.home.join(".config").join("Cursor")
    };
    let path = guarded(
        &base.join("User").join("globalStorage").join("state.vscdb"),
        guard,
    )?;
    if path.exists()
        && let Ok(Some(token)) = (access.cursor)(&path)
    {
        return Ok(Some(Credential::token(token)));
    }
    Ok(file::<CursorAuth>(
        &access.home.join(".config").join("cursor").join("auth.json"),
        guard,
    )?
    .and_then(|auth| auth.access_token)
    .map(|token| Credential::token(token.0)))
}
#[derive(Deserialize)]
struct GrokAuth {
    access_token: Option<Secret>,
    #[serde(flatten)]
    entries: BTreeMap<String, MaybeNested>,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum MaybeNested {
    Known { key: Option<Secret> },
    Unknown(serde::de::IgnoredAny),
}
fn grok(
    access: &AccountSourceAccess,
    guard: Option<&Path>,
) -> Result<Option<Credential>, LimitFailure> {
    if let Some(credential) = environment(access, &["GROK_API_KEY", "GROK_TOKEN"]) {
        return Ok(Some(credential));
    }
    let Some(mut auth) = file::<GrokAuth>(&access.home.join(".grok").join("auth.json"), guard)?
    else {
        return Ok(None);
    };
    if let Some(token) = auth.access_token.take() {
        return Ok(Some(Credential::token(token.0)));
    }
    let preferred = auth
        .entries
        .keys()
        .any(|key| key.starts_with("https://auth.x.ai::"));
    for (key, value) in auth.entries {
        if preferred && !key.starts_with("https://auth.x.ai::") {
            continue;
        }
        if let MaybeNested::Known { key: Some(token) } = value {
            return Ok(Some(Credential::token(token.0)));
        }
    }
    Ok(None)
}
#[derive(Deserialize)]
struct KimiAuth {
    access_token: Option<Secret>,
    expires_at: Option<Number>,
}
#[expect(
    clippy::float_arithmetic,
    reason = "Kimi explicitly reports epoch seconds; expiration is not inferred from receipt time"
)]
fn epoch_seconds(value: Number) -> Option<DateTime<Utc>> {
    const MILLIS_PER_SECOND: f64 = 1_000.0;
    let millis = (value.0.get() * MILLIS_PER_SECOND)
        .trunc()
        .to_string()
        .parse::<i64>()
        .ok()?;
    DateTime::from_timestamp_millis(millis)
}
fn kimi(
    access: &AccountSourceAccess,
    guard: Option<&Path>,
) -> Result<Option<Credential>, LimitFailure> {
    if let Some(credential) = environment(access, &["KIMI_TOKEN", "KIMI_API_KEY"]) {
        return Ok(Some(credential));
    }
    let code = access.absolute("KIMI_CODE_HOME", access.home.join(".kimi-code"))?;
    for base in [code, access.home.join(".kimi")] {
        if let Some(auth) =
            file::<KimiAuth>(&base.join("credentials").join("kimi-code.json"), guard)?
            && let Some(token) = auth.access_token
        {
            return Ok(Some(Credential {
                token: token.0,
                expires: auth.expires_at.and_then(epoch_seconds),
                region: Region::Global,
            }));
        }
    }
    Ok(None)
}
#[derive(Deserialize)]
struct MiniCredentials {
    access_token: Option<Secret>,
    expires_at: Option<String>,
    resource_url: Option<String>,
}
#[derive(Deserialize)]
struct MiniConfig {
    api_key: Option<Secret>,
    region: Option<String>,
    base_url: Option<String>,
    oauth: Option<MiniCredentials>,
}
fn region(url: Option<&str>, region: Option<&str>) -> Result<Region, LimitFailure> {
    match url.map(|url| url.trim_end_matches('/')) {
        Some("https://api.minimax.io") => Ok(Region::Global),
        Some("https://api.minimaxi.com") => Ok(Region::China),
        Some(_) => Err(LimitFailure::DifferentRoute),
        None => Ok(if region == Some("cn") {
            Region::China
        } else {
            Region::Global
        }),
    }
}
fn mini_oauth(auth: MiniCredentials, fallback: Region) -> Option<Credential> {
    let region = auth
        .resource_url
        .as_deref()
        .map(|url| region(Some(url), None))
        .transpose()
        .ok()?
        .unwrap_or(fallback);
    Some(Credential {
        token: auth.access_token?.0,
        expires: auth
            .expires_at
            .and_then(|at| DateTime::parse_from_rfc3339(&at).ok())
            .map(|at| at.with_timezone(&Utc)),
        region,
    })
}
fn minimax(
    access: &AccountSourceAccess,
    guard: Option<&Path>,
) -> Result<Option<Credential>, LimitFailure> {
    if let Some(mut credential) = environment(access, &["MINIMAX_API_KEY"]) {
        credential.region = region(
            access
                .value("MINIMAX_BASE_URL")
                .as_deref()
                .map(String::as_str),
            None,
        )?;
        return Ok(Some(credential));
    }
    let base = access.home.join(".mmx");
    if let Some(auth) = file::<MiniCredentials>(&base.join("credentials.json"), guard)?
        && let Some(credential) = mini_oauth(auth, Region::Global)
    {
        return Ok(Some(credential));
    }
    let Some(config) = file::<MiniConfig>(&base.join("config.json"), guard)? else {
        return Ok(None);
    };
    let region = region(config.base_url.as_deref(), config.region.as_deref())?;
    if let Some(token) = config.api_key {
        return Ok(Some(Credential {
            token: token.0,
            expires: None,
            region,
        }));
    }
    Ok(config.oauth.and_then(|auth| mini_oauth(auth, region)))
}
#[derive(Deserialize)]
struct GoAuth {
    #[serde(rename = "opencode-go")]
    go: Option<GoKey>,
}
#[derive(Deserialize)]
struct GoKey {
    r#type: String,
    key: Option<Secret>,
}
fn go(
    access: &AccountSourceAccess,
    guard: Option<&Path>,
) -> Result<Option<Credential>, LimitFailure> {
    let data = access.absolute("XDG_DATA_HOME", access.home.join(".local").join("share"))?;
    Ok(
        file::<GoAuth>(&data.join("opencode").join("auth.json"), guard)?
            .and_then(|auth| auth.go)
            .filter(|auth| auth.r#type == "api")
            .and_then(|auth| auth.key)
            .map(|token| Credential::token(token.0)),
    )
}
