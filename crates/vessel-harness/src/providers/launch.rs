//! Inert provider command resolution and environment overlays.
//! Secrets resolve only from caller-supplied values; this module reads no process.

use crate::{HarnessEnv, Lookup, PASSED_ON, passed_on};
use std::{collections::BTreeMap, ffi::OsString};
use vessel_proc::CommandSpec;
use vessel_proto::{
    ConfigValue, EnvName, ResolvedValue, SecretName,
    agents::AgentProgram,
    providers::{
        Arguments, CommandEdit, DisallowedTools, LaunchConfigError, LaunchSettings,
        ProviderEnvironment,
    },
};

/// Claude's parent-session markers must be removed after every overlay.
pub const PARENT_SESSION_VARIABLES: &[&str] = &[
    "CLAUDECODE",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_SSE_PORT",
    "CLAUDE_AGENT_SDK_VERSION",
];

/// Why a command was chosen; useful without revealing any argument text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchOrigin {
    /// Reference launch prefix.
    Default,
    /// Reference program with added literal arguments.
    Append,
    /// Entire configured command prefix replaced.
    Override,
}

/// A program with a bounded literal argv, before executable lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPrefix {
    /// Program name or absolute path.
    pub program: AgentProgram,
    /// Literal argv, not shell text.
    pub args: Arguments,
}

/// Launch resolution refuses unsupported policy before yielding an executable.
#[derive(Debug, thiserror::Error)]
pub enum ProviderLaunchError {
    /// Default and append edits need a reference executable.
    #[error("default provider command is required unless replaced")]
    NoDefault,
    /// Relative paths would depend on the daemon's cwd.
    #[error("provider program must be a bare executable name or an absolute path")]
    RelativeProgram,
    /// Executable is not found; the raw command is not echoed.
    #[error("provider launcher is unavailable")]
    Unavailable,
    /// Secret resolution is the caller's responsibility.
    #[error("provider launch needs a configured secret")]
    MissingSecret,
    /// Driver cannot apply the requested deny policy.
    #[error("provider driver cannot enforce disallowed tools")]
    ToolPolicyUnsupported,
    /// A combination exceeds the same bound as a decoded setting.
    #[error(transparent)]
    Config(#[from] LaunchConfigError),
}

/// Whether the chosen driver will apply native deny patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolEnforcement {
    /// Driver cannot guarantee native denied-tool semantics.
    Unsupported,
    /// Driver owns the tested translation and enforcement of native patterns.
    Native,
}

/// Atomic input to a launch. A caller cannot obtain the command without
/// explicitly accepting responsibility for the denied tools it contains.
#[derive(Debug)]
pub struct ProviderLaunch {
    command: CommandSpec,
    denied: DisallowedTools,
    origin: LaunchOrigin,
}

impl ProviderLaunch {
    /// Provenance without exposing values or arguments.
    #[must_use]
    pub const fn origin(&self) -> LaunchOrigin {
        self.origin
    }

    /// Yield both the command and its policy only to a capable transport.
    ///
    /// # Errors
    /// Unsupported drivers refuse any nonempty deny list before process spawn.
    pub fn into_parts(
        self,
        enforcement: ToolEnforcement,
    ) -> Result<(CommandSpec, DisallowedTools), ProviderLaunchError> {
        if enforcement == ToolEnforcement::Unsupported && !self.denied.patterns().is_empty() {
            return Err(ProviderLaunchError::ToolPolicyUnsupported);
        }
        Ok((self.command, self.denied))
    }
}

/// Resolve default/append/replace without probing or starting an executable.
///
/// # Errors
/// `NoDefault` or Config refuses an unrepresentable command prefix.
pub fn resolve_prefix(
    default: Option<&CommandPrefix>,
    edit: &CommandEdit,
) -> Result<(CommandPrefix, LaunchOrigin), ProviderLaunchError> {
    match edit {
        CommandEdit::Replace { program, args } => Ok((
            CommandPrefix {
                program: program.clone(),
                args: args.clone(),
            },
            LaunchOrigin::Override,
        )),
        CommandEdit::Default => Ok((
            default.ok_or(ProviderLaunchError::NoDefault)?.clone(),
            LaunchOrigin::Default,
        )),
        CommandEdit::Append { args } => {
            let default = default.ok_or(ProviderLaunchError::NoDefault)?;
            let args = Arguments::new(
                default
                    .args
                    .all()
                    .iter()
                    .chain(args.all())
                    .cloned()
                    .collect(),
            )?;
            Ok((
                CommandPrefix {
                    program: default.program.clone(),
                    args,
                },
                LaunchOrigin::Append,
            ))
        }
    }
}

/// Settings inheritance is shallow for environment values and concatenates deny
/// patterns; command absence preserves the base edit rather than resetting it.
///
/// # Errors
/// Combining two bounded inputs can exceed the bound and is refused atomically.
pub fn merge_launch(
    base: &LaunchSettings,
    command: Option<&CommandEdit>,
    env: &ProviderEnvironment,
    denied: &DisallowedTools,
) -> Result<LaunchSettings, ProviderLaunchError> {
    let mut values = base.env.values().clone();
    for (name, value) in env.values() {
        values.retain(|old, _| !old.as_str().eq_ignore_ascii_case(name.as_str()));
        let _previous = values.insert(name.clone(), value.clone());
    }
    Ok(LaunchSettings {
        command: command.unwrap_or(&base.command).clone(),
        env: ProviderEnvironment::new(values)?,
        disallowed_tools: DisallowedTools::new(
            base.disallowed_tools
                .patterns()
                .iter()
                .chain(denied.patterns())
                .map(|pattern| pattern.as_str().to_owned())
                .collect(),
        )?,
    })
}

/// Resolve an explicitly configured provider in an empty environment, retaining
/// Vessel's ADR 0032 security floor. Parent markers are removed last even if a
/// definition tried to set them. Filesystem lookups require a blocking context.
///
/// # Errors
/// Missing executable/default/secret or invalid combined settings are refused.
pub fn provider_launch(
    default: Option<&CommandPrefix>,
    settings: &LaunchSettings,
    lookup: &Lookup,
    account: &HarnessEnv,
    secrets: &BTreeMap<SecretName, ResolvedValue>,
    inherited: impl Fn(&str) -> Option<OsString>,
) -> Result<ProviderLaunch, ProviderLaunchError> {
    let (prefix, origin) = resolve_prefix(default, &settings.command)?;
    let written = prefix.program.as_str();
    if written.contains(['/', '\\']) && !std::path::Path::new(written).is_absolute() {
        return Err(ProviderLaunchError::RelativeProgram);
    }
    let program = lookup
        .find(written)
        .ok_or(ProviderLaunchError::Unavailable)?;
    let mut command = CommandSpec::new(program)
        .args(prefix.args.all())
        .env_clear();
    if let Some(path) = lookup.search_path() {
        command = command.env("PATH", path);
    }
    let home = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    command = command.env(home, account.user_home());
    for name in PASSED_ON {
        if let Some(value) = inherited(name)
            && passed_on(name, &value)
        {
            command = command.env(name, value);
        }
    }
    for (name, value) in settings.env.values() {
        if parent_marker(name) {
            continue;
        }
        let value = match value {
            ConfigValue::Text(text) => text.as_str(),
            ConfigValue::Secret(name) => secrets
                .get(name)
                .ok_or(ProviderLaunchError::MissingSecret)?
                .expose(),
        };
        command = command.env(name.as_str(), value);
    }
    for name in PARENT_SESSION_VARIABLES {
        command = command.env_remove(name);
    }
    Ok(ProviderLaunch {
        command,
        origin,
        denied: settings.disallowed_tools.clone(),
    })
}

fn parent_marker(name: &EnvName) -> bool {
    PARENT_SESSION_VARIABLES
        .iter()
        .any(|parent| parent.eq_ignore_ascii_case(name.as_str()))
}

#[cfg(test)]
mod tests;
