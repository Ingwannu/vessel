//! Reading `profiles.toml` ([`parse_profiles_with`]): the file's shape,
//! its errors, and the checks of each profile (ADR 0011). Since protocol 11
//! a profile may extend an agent added on the node (ADR 0032), with `env`
//! and `unset` only.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;
use vessel_proto::{
    AgentName, EnvName, HarnessKind, HarnessRef, InvalidName, ModelRouteName, ProfileName,
};

use super::{Base, Profile, Profiles};
use crate::added::AddedAgents;
use crate::harnesses::descriptor;
use crate::routing::{overriding, routed_variables};

/// `profiles.toml` as typed: the file is `[profiles.<name>]` tables and
/// nothing else.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    #[serde(default)]
    profiles: BTreeMap<String, RawProfile>,
}

/// One `[profiles.<name>]` table. Names are checked after typing, so the
/// errors can name the profile and the key.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProfile {
    harness: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    unset: Vec<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    agent_terminals: bool,
}

/// `profiles.toml` could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProfilesError {
    /// Not TOML, or not the shape of `profiles.toml` (an unknown key, a
    /// missing `harness`, a value of the wrong type). The message names the
    /// key and the position, and never quotes the file.
    #[error("profiles.toml, {}: {message}", position.map_or_else(|| "unknown position".to_owned(), |at| at.to_string()))]
    Toml {
        /// Where, if the TOML library said.
        position: Option<Position>,
        /// The TOML library's message.
        message: String,
    },
    /// One profile is wrong.
    #[error("profiles.toml, profile `{profile}`: {problem}")]
    Profile {
        /// The profile's name as written.
        profile: String,
        /// What is wrong with it.
        problem: ProfileProblem,
    },
}

/// What is wrong with one profile.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProfileProblem {
    /// The name breaks the profile name rule.
    #[error("{0}")]
    InvalidName(InvalidName),
    /// The name is a harness's, so it is its default profile's.
    #[error("the name is reserved for the {harness} harness's default profile")]
    Reserved {
        /// The harness the name means.
        harness: HarnessKind,
    },
    /// The name is an added agent's (ignoring case), so it is that agent's
    /// default profile's (ADR 0032).
    #[error("the name is reserved for the default profile of the added agent `{agent}`")]
    ReservedForAgent {
        /// The agent.
        agent: String,
    },
    /// Another profile has the same name ignoring case.
    #[error("profile `{other}` has the same name (profile names are compared ignoring case)")]
    Duplicate {
        /// The other profile, as written.
        other: String,
    },
    /// `harness` names no harness.
    #[error(
        "`harness`: unknown harness `{value}` (one of claude, codex, gemini, opencode, or an agent added on the node)"
    )]
    UnknownHarness {
        /// The value given.
        value: String,
    },
    /// A name in `env` or `unset` is not a variable name.
    #[error("`{key}`: `{name}` is not a valid environment variable name ({})", vessel_proto::NameKind::EnvVar.rule())]
    InvalidVariable {
        /// `env` or `unset`.
        key: &'static str,
        /// The name given.
        name: String,
    },
    /// A variable is listed twice under one key (ignoring case).
    #[error("`{key}`: `{name}` is listed twice (variable names are compared ignoring case)")]
    RepeatedVariable {
        /// `env` or `unset`.
        key: &'static str,
        /// The second spelling.
        name: EnvName,
    },
    /// A variable is both set and removed.
    #[error("`{name}` is both in `env` and in `unset`")]
    SetAndUnset {
        /// The variable.
        name: EnvName,
    },
    /// A value starts with `~` but the node user's home is unknown or is
    /// not text.
    #[error(
        "`{key}`: `~` needs the node user's home directory, which is unknown or not valid text"
    )]
    NoHome {
        /// `env.<NAME>` or `args`.
        key: String,
    },
    /// `args` on a profile whose harness takes no terminal arguments.
    #[error("`args`: {harness} takes no extra terminal arguments")]
    ArgsUnsupported {
        /// The harness: an added agent never does (it has no terminal).
        harness: HarnessRef,
    },
    /// `model` is not a route name.
    #[error("`model`: {0}")]
    InvalidRoute(InvalidName),
    /// `model` on a profile whose harness cannot be pointed at the node's
    /// model proxy.
    #[error("`model`: {harness} cannot use a model route yet (Claude Code and Codex can)")]
    NotRoutable {
        /// The harness: an added agent never can (Vessel does not know how
        /// to point it at the proxy).
        harness: HarnessRef,
    },
    /// A routed profile sets a variable its route sets or removes.
    #[error("`env.{name}`: the profile's model route sets this variable")]
    RoutedVariable {
        /// The variable.
        name: EnvName,
    },
    /// A routed profile's argument would choose another model, provider or
    /// configuration profile.
    #[error("`args`: `{argument}` would take the profile's sessions off its model route")]
    RoutedArgument {
        /// The argument.
        argument: String,
    },
}

/// Read `profiles.toml` from its text. `user_home` is what `~/` expands
/// to; `None` if the node user has none (a value using `~` is then an
/// error).
///
/// Pure: the caller reads the file (and checks it is owner-only).
///
/// # Errors
/// The first problem found, naming the profile and key; see
/// [`ProfilesError`].
pub fn parse_profiles(text: &str, user_home: Option<&Path>) -> Result<Profiles, ProfilesError> {
    parse_profiles_with(text, user_home, AddedAgents::default())
}

/// [`parse_profiles`], beside the agents `agents.toml` added (ADR 0032):
/// a profile may extend one (`harness = "<name>"`), and may not take one's
/// name, which is that agent's default profile's.
///
/// # Errors
/// As [`parse_profiles`].
pub fn parse_profiles_with(
    text: &str,
    user_home: Option<&Path>,
    added: AddedAgents,
) -> Result<Profiles, ProfilesError> {
    let raw: RawFile = toml_edit::de::from_str(text).map_err(|error| ProfilesError::Toml {
        position: position(text, error.span()),
        message: without_values(error.message()),
    })?;
    let home = user_home.and_then(Path::to_str);
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    let mut defined = BTreeMap::new();
    for (written, raw_profile) in raw.profiles {
        let fail = |problem| ProfilesError::Profile {
            profile: written.clone(),
            problem,
        };
        let name: ProfileName = written
            .parse()
            .map_err(|error| fail(ProfileProblem::InvalidName(error)))?;
        if let Ok(harness) = written.parse::<HarnessKind>() {
            return Err(fail(ProfileProblem::Reserved { harness }));
        }
        if added.has_name(&written) {
            return Err(fail(ProfileProblem::ReservedForAgent {
                agent: written.clone(),
            }));
        }
        if let Some(other) = seen.insert(written.to_ascii_lowercase(), written.clone()) {
            return Err(fail(ProfileProblem::Duplicate { other }));
        }
        let profile = profile(name, raw_profile, home, &added).map_err(fail)?;
        let _new = defined.insert(profile.name.clone(), profile);
    }
    Ok(Profiles::with(defined, added))
}

/// What `harness = "..."` names: one of the four by any of its names,
/// otherwise an agent `added` defines.
fn base(written: &str, added: &AddedAgents) -> Result<Base, ProfileProblem> {
    let unknown = || ProfileProblem::UnknownHarness {
        value: written.to_owned(),
    };
    if let Ok(kind) = written.parse::<HarnessKind>() {
        return Ok(Base::Known(kind));
    }
    let name: AgentName = written.parse().map_err(|_invalid| unknown())?;
    added
        .get(&name)
        .map(|agent| Base::Added(Arc::clone(agent)))
        .ok_or_else(unknown)
}

/// One profile's checks, after its name.
fn profile(
    name: ProfileName,
    raw: RawProfile,
    home: Option<&str>,
    added: &AddedAgents,
) -> Result<Profile, ProfileProblem> {
    let base = base(&raw.harness, added)?;
    let harness_ref = || match &base {
        Base::Known(kind) => HarnessRef::Known(*kind),
        Base::Added(agent) => HarnessRef::Added(agent.name().clone()),
    };
    // An added agent has no terminal and no route Vessel knows of.
    let known = match &base {
        Base::Known(kind) => Some(*kind),
        Base::Added(_) => None,
    };
    if !raw.args.is_empty() && !known.is_some_and(|kind| descriptor(kind).terminal_args) {
        return Err(ProfileProblem::ArgsUnsupported {
            harness: harness_ref(),
        });
    }
    let model = raw
        .model
        .as_deref()
        .map(str::parse::<ModelRouteName>)
        .transpose()
        .map_err(ProfileProblem::InvalidRoute)?;
    let routed = match (&model, known) {
        (None, _) => Vec::new(),
        (Some(_), Some(kind)) if descriptor(kind).model_proxy.is_some() => routed_variables(kind),
        (Some(_), _) => {
            return Err(ProfileProblem::NotRoutable {
                harness: harness_ref(),
            });
        }
    };
    let mut set_folded = BTreeSet::new();
    let mut env = BTreeMap::new();
    for (written, value) in raw.env {
        let variable = variable("env", &written)?;
        if !set_folded.insert(written.to_ascii_uppercase()) {
            return Err(ProfileProblem::RepeatedVariable {
                key: "env",
                name: variable,
            });
        }
        if routed.iter().any(|name| written.eq_ignore_ascii_case(name)) {
            return Err(ProfileProblem::RoutedVariable { name: variable });
        }
        let value = expand(&value, home).ok_or_else(|| ProfileProblem::NoHome {
            key: format!("env.{written}"),
        })?;
        let _new = env.insert(variable, value);
    }
    let mut unset_folded = BTreeSet::new();
    let mut unset = BTreeSet::new();
    for written in raw.unset {
        let variable = variable("unset", &written)?;
        let folded = written.to_ascii_uppercase();
        if set_folded.contains(&folded) {
            return Err(ProfileProblem::SetAndUnset { name: variable });
        }
        if !unset_folded.insert(folded) {
            return Err(ProfileProblem::RepeatedVariable {
                key: "unset",
                name: variable,
            });
        }
        let _new = unset.insert(variable);
    }
    let args = raw
        .args
        .iter()
        .map(|arg| expand(arg, home))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| ProfileProblem::NoHome {
            key: "args".to_owned(),
        })?;
    if model.is_some()
        && let Some(kind) = known
        && let Some(argument) = overriding(kind, &args)
    {
        return Err(ProfileProblem::RoutedArgument { argument });
    }
    Ok(Profile {
        name,
        base,
        label: raw.label,
        env,
        unset,
        args,
        model,
        agent_terminals: raw.agent_terminals,
        secrets: Arc::default(),
    })
}

pub(super) fn variable(key: &'static str, written: &str) -> Result<EnvName, ProfileProblem> {
    written
        .parse()
        .map_err(|_invalid| ProfileProblem::InvalidVariable {
            key,
            name: written.to_owned(),
        })
}

/// `~` and a leading `~/` become the home; anything else is kept as text
/// (`~user` and a `~` later in the value too, as in a shell). `None` when
/// the value needs the home and there is none.
///
/// The path after `~/` is written with the system's separator, as the home
/// is: on Windows `~/.claude-work` is `C:\Users\me\.claude-work`, not a
/// path with both separators, which Windows accepts but which then differs
/// as text from the same path elsewhere (in a report, or an environment
/// variable compared by value).
pub(crate) fn expand(value: &str, home: Option<&str>) -> Option<String> {
    let rest = if value == "~" {
        ""
    } else if let Some(rest) = value.strip_prefix("~/") {
        rest
    } else {
        return Some(value.to_owned());
    };
    let home = home?;
    let rest = rest.replace('/', std::path::MAIN_SEPARATOR_STR);
    let separator = if home.ends_with(['/', std::path::MAIN_SEPARATOR]) || rest.is_empty() {
        ""
    } else {
        std::path::MAIN_SEPARATOR_STR
    };
    Some(format!("{home}{separator}{rest}"))
}

/// The TOML library's message without the value it found. Serde's type
/// and value errors quote it (``invalid type: integer `98765`, expected a
/// string``, `invalid type: string "--token=…", expected a sequence`),
/// and a value in `env` or `args` may be a secret that `profile.reload`
/// would send to a client. Only the kind found and what was expected stay.
/// Syntax errors, unknown and missing fields name keys and positions only
/// (checked against `toml_edit` 0.25), so they pass as they are.
pub(crate) fn without_values(message: &str) -> String {
    for prefix in ["invalid type: ", "invalid value: "] {
        let Some(at) = message.find(prefix) else {
            continue;
        };
        let (before, rest) = message.split_at(at);
        let rest = rest.get(prefix.len()..).unwrap_or_default();
        // What was expected is written by code, never taken from the file,
        // so the last separator is the real one even if the value holds it.
        return match rest.rsplit_once(", expected ") {
            Some((found, expected)) => {
                let kind = found.split(['`', '"']).next().unwrap_or_default().trim();
                format!("{before}{prefix}{kind}, expected {expected}")
            }
            None => format!("{before}{prefix}a value of another type"),
        };
    }
    message.to_owned()
}

/// A place in `profiles.toml`; both count from 1, the column in
/// characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// The line.
    pub line: usize,
    /// The column.
    pub column: usize,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// The position of a byte offset.
pub(crate) fn position(text: &str, span: Option<std::ops::Range<usize>>) -> Option<Position> {
    let before = span.and_then(|span| text.get(..span.start))?;
    let line = before.matches('\n').count().saturating_add(1);
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |line| line.chars().count())
        .saturating_add(1);
    Some(Position { line, column })
}
