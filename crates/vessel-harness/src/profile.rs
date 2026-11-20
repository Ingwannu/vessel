//! Harness profiles (ADR 0011): one harness, several accounts or
//! configurations, chosen per session.
//!
//! A [`Profile`] extends one harness with environment variables to set, to
//! remove, and extra terminal arguments. Every harness has an implicit
//! default profile named after its command (`claude`, `codex`, `gemini`,
//! `opencode`), which is the node's environment as is: launching with it is
//! exactly the launch from before profiles. The node defines the others in
//! `profiles.toml`, read by [`parse_profiles`]:
//!
//! ```toml
//! [profiles.claude-alt]
//! harness = "claude"                 # the harness it extends
//! label = "Claude (Alt)"
//! env = { CLAUDE_CONFIG_DIR = "~/.claude-alt" }
//! unset = ["ANTHROPIC_API_KEY"]      # removed from the inherited environment
//! args = ["--model", "opus"]         # terminal mode only, before the session's own
//!
//! [profiles.claude-qwen]
//! harness = "claude"
//! model = "qwen-coder"               # a route of `models.toml` (ADR 0015)
//! agent_terminals = true             # its agents may open terminals (ADR 0031)
//! ```
//!
//! A profile with `model` runs its harness against the node's model proxy,
//! pinned to the route's models (`routing`); only harnesses that can be
//! pointed there may have one, and such a profile may not set the
//! variables the route sets or removes.
//!
//! `agent_terminals` lets the agents of a session with this profile open
//! terminals for it, or for a session they started whose profile sets it
//! too (ADR 0031); from protocol 40 such a terminal opens in that
//! session's workspace (TRACKER 832). It is off for every default profile and
//! unless written: a terminal the node starts is outside the harness's own
//! sandbox, so a person turns it on knowing that sandbox.
//!
//! Values may be secrets (an API key in `env`): `Debug` and
//! [`Profile::info`] show variable names only, and parse errors never quote
//! the file.
//!
//! [Decision Log]
//! - Purpose: a pure parser, so the node owns reading the file (path,
//!   permissions, reload) and this crate owns what the text means.
//! - Format: TOML through the workspace's `toml_edit` (its `serde` feature
//!   reads the file into typed structs that refuse unknown keys), so there
//!   is one TOML implementation in the workspace.
//! - Errors: the TOML library's message with line and column, but not its
//!   source excerpt, which would quote the line, and a line of `env` may
//!   hold a secret that `profile.reload` would then send to a client. The
//!   checks after typing name the profile and the key.
//! - Variable names are compared ignoring ASCII case (two `env` entries,
//!   two `unset` entries, or one in both): Windows treats them as one
//!   variable, and a file that means different things on different
//!   systems is worse than refusing `Foo` beside `FOO` on Unix.
//! - A profile name that parses as a harness (`claude`, `Claude-Code`, ...)
//!   is reserved, so `--profile claude` always means the default profile.
//!   Duplicate names are refused ignoring case: clients and people type
//!   them, and two profiles one letter-case apart are a mistake.

mod parse;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use vessel_proto::{
    EnvName, HarnessKind, HarnessRef, ModelRouteName, ProfileInfo, ProfileName, ResolvedValue,
    SecretName,
};

pub use parse::{Position, ProfileProblem, ProfilesError, parse_profiles, parse_profiles_with};
pub(crate) use parse::{expand, position, without_values};

use crate::added::{AddedAgent, AddedAgents};
use crate::error::HarnessError;
use crate::harnesses::descriptor;

/// A harness with environment changes and terminal arguments.
#[derive(Clone, PartialEq, Eq)]
pub struct Profile {
    name: ProfileName,
    base: Base,
    label: Option<String>,
    env: BTreeMap<EnvName, String>,
    unset: BTreeSet<EnvName>,
    args: Vec<String>,
    model: Option<ModelRouteName>,
    agent_terminals: bool,
    /// The values of the secrets an added agent's definition names, filled
    /// in for one launch ([`Profile::with_secrets`]); empty otherwise.
    secrets: Arc<BTreeMap<SecretName, ResolvedValue>>,
}

/// The agent a profile extends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Base {
    /// One of the four.
    Known(HarnessKind),
    /// One added on the node by its command (ADR 0032).
    Added(Arc<AddedAgent>),
}

impl Profile {
    /// The implicit default profile of `harness`: no changes, so launching
    /// with it is the launch from before profiles. Its name is the harness's
    /// command (`claude` for Claude Code).
    #[must_use]
    pub fn default_for(harness: HarnessKind) -> Self {
        Self::plain(default_name(harness), Base::Known(harness))
    }

    /// The implicit default profile of an added agent: its definition as
    /// written, under the agent's own name.
    #[must_use]
    pub fn default_added(agent: Arc<AddedAgent>) -> Self {
        Self::plain(agent.name().profile().clone(), Base::Added(agent))
    }

    fn plain(name: ProfileName, base: Base) -> Self {
        Self {
            name,
            base,
            label: None,
            env: BTreeMap::new(),
            unset: BTreeSet::new(),
            args: Vec::new(),
            model: None,
            agent_terminals: false,
            secrets: Arc::default(),
        }
    }

    /// This profile with the value of every secret its added agent names,
    /// for one launch; a profile of one of the four is returned as it is.
    ///
    /// # Errors
    /// The first secret `value` has no value for.
    pub fn with_secrets(
        mut self,
        value: impl Fn(&SecretName) -> Option<ResolvedValue>,
    ) -> Result<Self, SecretName> {
        let Base::Added(agent) = &self.base else {
            return Ok(self);
        };
        let mut secrets = BTreeMap::new();
        for name in agent.secrets() {
            let resolved = value(name).ok_or_else(|| name.clone())?;
            let _none = secrets.insert(name.clone(), resolved);
        }
        self.secrets = Arc::new(secrets);
        Ok(self)
    }

    /// The secrets filled in by [`Profile::with_secrets`].
    pub(crate) fn secret_values(&self) -> &BTreeMap<SecretName, ResolvedValue> {
        &self.secrets
    }

    pub(crate) const fn base(&self) -> &Base {
        &self.base
    }

    /// The one of the four it extends, if it does.
    #[must_use]
    pub const fn known(&self) -> Option<HarnessKind> {
        match &self.base {
            Base::Known(kind) => Some(*kind),
            Base::Added(_) => None,
        }
    }

    /// The added agent it extends, if it does.
    #[must_use]
    pub fn added(&self) -> Option<&Arc<AddedAgent>> {
        match &self.base {
            Base::Known(_) => None,
            Base::Added(agent) => Some(agent),
        }
    }

    /// The default profile of `harness`, for a caller given no profile:
    /// one of the four's ([`Profile::default_for`]). An added agent's
    /// default profile carries its definition, which only the node's set of
    /// profiles has, so it is refused here.
    ///
    /// # Errors
    /// [`HarnessError::NotDefined`] for an added agent.
    pub fn default_of(harness: &HarnessRef) -> Result<Self, HarnessError> {
        match harness {
            HarnessRef::Known(kind) => Ok(Self::default_for(*kind)),
            HarnessRef::Added(name) => Err(HarnessError::NotDefined { name: name.clone() }),
        }
    }

    /// Its name.
    #[must_use]
    pub const fn name(&self) -> &ProfileName {
        &self.name
    }

    /// The harness it extends: one of the four, or an added agent.
    #[must_use]
    pub fn harness(&self) -> HarnessRef {
        match &self.base {
            Base::Known(kind) => HarnessRef::Known(*kind),
            Base::Added(agent) => HarnessRef::Added(agent.name().clone()),
        }
    }

    /// Its label for people, if any.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Whether this is a harness's implicit default profile.
    #[must_use]
    pub fn is_default(&self) -> bool {
        match &self.base {
            Base::Known(kind) => self.name == default_name(*kind),
            Base::Added(agent) => &self.name == agent.name().profile(),
        }
    }

    /// The variables it sets, with their values (`~/` already expanded).
    /// The values may be secrets: they go into the harness's environment
    /// and nowhere else.
    pub fn env(&self) -> impl Iterator<Item = (&EnvName, &str)> {
        self.env.iter().map(|(name, value)| (name, value.as_str()))
    }

    /// The value it gives `name`, if it sets it. Names compare as this
    /// system's environment does ([`same_variable`]).
    #[must_use]
    pub fn get(&self, name: &EnvName) -> Option<&str> {
        self.env
            .iter()
            .find(|(key, _)| same_variable(key.as_str(), name.as_str()))
            .map(|(_, value)| value.as_str())
    }

    /// The inherited variables it removes.
    pub fn unset(&self) -> impl Iterator<Item = &EnvName> {
        self.unset.iter()
    }

    /// Whether it removes `name`. Names compare as this system's
    /// environment does ([`same_variable`]).
    #[must_use]
    pub fn unsets(&self, name: &EnvName) -> bool {
        self.unset
            .iter()
            .any(|key| same_variable(key.as_str(), name.as_str()))
    }

    /// Arguments added before the session's own, in terminal mode only.
    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// The model route its sessions are pointed at through the node's
    /// proxy (ADR 0015), if any.
    #[must_use]
    pub const fn model(&self) -> Option<&ModelRouteName> {
        self.model.as_ref()
    }

    /// Whether the agents of its sessions may open terminals beside them
    /// (ADR 0031). Off unless the file turns it on.
    #[must_use]
    pub const fn agent_terminals(&self) -> bool {
        self.agent_terminals
    }

    /// What a node reports about it: names, never values.
    #[must_use]
    pub fn info(&self) -> ProfileInfo {
        ProfileInfo {
            name: self.name.clone(),
            label: self.label.clone(),
            harness: self.harness(),
            sets: self.env.keys().cloned().collect(),
            model: self.model.clone(),
        }
    }
}

/// Names only: values may be secrets.
impl fmt::Debug for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Profile")
            .field("name", &self.name)
            .field("harness", &self.harness())
            .field("label", &self.label)
            .field("sets", &self.env.keys().collect::<Vec<_>>())
            .field("unset", &self.unset)
            .field("args", &self.args)
            .field("model", &self.model)
            .field("agent_terminals", &self.agent_terminals)
            .finish_non_exhaustive()
    }
}

/// Whether two variable names are one variable on this system: ignoring
/// ASCII case on Windows, as its environment does, and exactly elsewhere.
/// Every lookup of a profile's variables (its home, its removals, the MCP
/// check) goes through this, so they agree with what the harness gets.
/// Parsing is stricter on every system (two names one case apart are
/// refused), so a file never means different things on two systems.
#[must_use]
pub fn same_variable(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

/// The name of `harness`'s default profile.
#[expect(
    clippy::expect_used,
    reason = "the four names are constants, and a test parses each of them"
)]
fn default_name(harness: HarnessKind) -> ProfileName {
    descriptor(harness)
        .profile
        .parse()
        .expect("a default profile name follows the profile name rule")
}

/// The profiles of one node: every harness's default (the four's, then
/// each added agent's) and those defined in `profiles.toml`. The defaults
/// are built once, with the set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profiles {
    defaults: Vec<Profile>,
    defined: BTreeMap<ProfileName, Profile>,
    added: AddedAgents,
}

/// The default profiles only (no `profiles.toml`, no `agents.toml`).
impl Default for Profiles {
    fn default() -> Self {
        Self::with(BTreeMap::new(), AddedAgents::default())
    }
}

impl Profiles {
    fn with(defined: BTreeMap<ProfileName, Profile>, added: AddedAgents) -> Self {
        Self {
            defaults: HarnessKind::ALL
                .into_iter()
                .map(Profile::default_for)
                .chain(added.all().cloned().map(Profile::default_added))
                .collect(),
            defined,
            added,
        }
    }

    /// The agents added on the node, read with the profiles.
    #[must_use]
    pub const fn added(&self) -> &AddedAgents {
        &self.added
    }

    /// The default profile of `harness`: one of the four's, or an added
    /// agent's when the node has it.
    #[must_use]
    pub fn default_of(&self, harness: &HarnessRef) -> Option<Profile> {
        match harness {
            HarnessRef::Known(kind) => Some(Profile::default_for(*kind)),
            HarnessRef::Added(name) => self
                .added
                .get(name)
                .map(|agent| Profile::default_added(Arc::clone(agent))),
        }
    }

    /// The profile named `name`: a defined one, or a harness's default
    /// (names do not overlap: a defined one may not take a harness's name).
    #[must_use]
    pub fn get(&self, name: &ProfileName) -> Option<&Profile> {
        self.defined
            .get(name)
            .or_else(|| self.defaults.iter().find(|profile| profile.name() == name))
    }

    /// Every profile: the defaults (in harness order), then the defined
    /// ones by name.
    pub fn all(&self) -> impl Iterator<Item = &Profile> {
        self.defaults.iter().chain(self.defined.values())
    }

    /// The profiles defined in the file, by name.
    pub fn defined(&self) -> impl Iterator<Item = &Profile> {
        self.defined.values()
    }
}
