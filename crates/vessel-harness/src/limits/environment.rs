//! Only nonsecret route facts and path inputs are captured by the binary.
//! Libraries never inspect process environment or discover other accounts.
use crate::{HarnessEnv, Profile};
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf};
use vessel_proto::{
    EnvName,
    limits::{LimitFailure, LimitLoginStore},
};

const VARIABLES: [&str; 6] = [
    "ANTHROPIC_BASE_URL",
    "OPENAI_BASE_URL",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "USER",
    "XDG_DATA_HOME",
];
/// Nonsecret route facts and local path inputs, captured by the binary.
/// An empty value is safe for temporary test homes; no process environment
/// is inspected by the library.
#[derive(Debug, Clone, Default)]
pub struct LimitEnvironment {
    vars: BTreeMap<&'static str, Fact>,
}
#[derive(Debug, Clone)]
enum Fact {
    Compatible,
    Foreign,
    Enabled,
    Disabled,
    Path(PathBuf),
    User(String),
}
impl LimitEnvironment {
    /// No inherited routing variables; safe for a temporary test home.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Snapshot known facts only. Only binaries pass their process reader.
    #[must_use]
    pub fn from_vars(read: impl Fn(&str) -> Option<OsString>) -> Self {
        Self {
            vars: VARIABLES
                .into_iter()
                .filter_map(|name| {
                    read(name)
                        .and_then(|value| fact(name, &value.to_string_lossy()))
                        .map(|value| (name, value))
                })
                .collect(),
        }
    }
    fn value(&self, name: &str, profile: &Profile) -> Option<Fact> {
        let variable: EnvName = name.parse().ok()?;
        if let Some(value) = profile.get(&variable) {
            return fact(name, value);
        }
        if profile.unsets(&variable) {
            return None;
        }
        self.vars.get(name).cloned()
    }
    pub(super) fn resolve(
        &self,
        homes: &HarnessEnv,
        profile: &Profile,
        source: LimitLoginStore,
    ) -> Result<Binding, LimitFailure> {
        if profile.model().is_some() {
            return Err(LimitFailure::DifferentRoute);
        }
        let home = effective_home(homes, profile)?;
        let resolved_homes = homes.clone();
        // The profile may change HOME as well as the provider-specific home.
        // Resolve the latter against precisely that home, without reading it.
        let path = match source {
            LimitLoginStore::ClaudeCode => {
                if matches!(
                    self.value("ANTHROPIC_BASE_URL", profile),
                    Some(Fact::Foreign)
                ) || ["CLAUDE_CODE_USE_BEDROCK", "CLAUDE_CODE_USE_VERTEX"]
                    .into_iter()
                    .any(|name| matches!(self.value(name, profile), Some(Fact::Enabled)))
                {
                    return Err(LimitFailure::DifferentRoute);
                }
                let path = crate::home(profile, &home, |name| {
                    resolved_homes.var(name).map(ToOwned::to_owned)
                })
                .ok_or(LimitFailure::UnsafeLoginStore)?;
                path.join(".credentials.json")
            }
            LimitLoginStore::Codex => {
                if matches!(self.value("OPENAI_BASE_URL", profile), Some(Fact::Foreign)) {
                    return Err(LimitFailure::DifferentRoute);
                }
                crate::home(profile, &home, |name| {
                    resolved_homes.var(name).map(ToOwned::to_owned)
                })
                .ok_or(LimitFailure::UnsafeLoginStore)?
                .join("auth.json")
            }
            LimitLoginStore::OpenCode => {
                let data = match self.value("XDG_DATA_HOME", profile) {
                    Some(Fact::Path(path)) if path.is_absolute() => path,
                    Some(Fact::Path(_)) => return Err(LimitFailure::UnsafeLoginStore),
                    None
                    | Some(
                        Fact::Compatible
                        | Fact::Foreign
                        | Fact::Enabled
                        | Fact::Disabled
                        | Fact::User(_),
                    ) => home.join(".local").join("share"),
                };
                data.join("opencode").join("auth.json")
            }
        };
        let config: EnvName = "CLAUDE_CONFIG_DIR"
            .parse()
            .map_err(|_invalid| LimitFailure::UnsafeLoginStore)?;
        let custom = profile
            .get(&config)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                (!profile.unsets(&config))
                    .then(|| {
                        homes
                            .var("CLAUDE_CONFIG_DIR")
                            .map(|value| value.to_string_lossy().into_owned())
                    })
                    .flatten()
            });
        let user = match self.value("USER", profile) {
            Some(Fact::User(user)) => Some(user),
            None
            | Some(
                Fact::Compatible | Fact::Foreign | Fact::Enabled | Fact::Disabled | Fact::Path(_),
            ) => None,
        };
        Ok(Binding {
            source,
            path,
            keychain_service: super::keychain::service(custom.as_deref()),
            keychain_account: super::keychain::account(user.as_deref()).to_owned(),
        })
    }
}
fn effective_home(homes: &HarnessEnv, profile: &Profile) -> Result<PathBuf, LimitFailure> {
    let variable: EnvName = if cfg!(windows) { "USERPROFILE" } else { "HOME" }
        .parse()
        .map_err(|_invalid| LimitFailure::UnsafeLoginStore)?;
    if profile.unsets(&variable) {
        return Err(LimitFailure::UnsafeLoginStore);
    }
    match profile.get(&variable).filter(|value| !value.is_empty()) {
        Some(value) => {
            let path = PathBuf::from(value);
            if path.is_absolute() {
                Ok(path)
            } else {
                Err(LimitFailure::UnsafeLoginStore)
            }
        }
        None => Ok(homes.user_home().to_owned()),
    }
}
fn fact(name: &str, text: &str) -> Option<Fact> {
    if text.is_empty() {
        return None;
    }
    Some(match name {
        "ANTHROPIC_BASE_URL" => {
            if text.trim_end_matches('/') == "https://api.anthropic.com" {
                Fact::Compatible
            } else {
                Fact::Foreign
            }
        }
        "OPENAI_BASE_URL" => Fact::Foreign,
        "CLAUDE_CODE_USE_BEDROCK" | "CLAUDE_CODE_USE_VERTEX" => {
            if text == "1" || text.eq_ignore_ascii_case("true") || text.eq_ignore_ascii_case("yes")
            {
                Fact::Enabled
            } else {
                Fact::Disabled
            }
        }
        "XDG_DATA_HOME" => Fact::Path(PathBuf::from(text)),
        "USER" => Fact::User(super::keychain::account(Some(text)).to_owned()),
        _ => return None,
    })
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Binding {
    pub(super) source: LimitLoginStore,
    pub(super) path: PathBuf,
    pub(super) keychain_service: String,
    pub(super) keychain_account: String,
}
