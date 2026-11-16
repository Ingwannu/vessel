//! Launch policy for trusted unsandboxed helpers; no ambient secret forwarding.

use std::fmt;

use vessel_proc::{CommandSpec, EnvValue, Environment};

/// Manifest argv cannot make process bookkeeping allocate without limit.
pub const MAX_HELPER_ARGUMENTS: usize = 64;
/// One argv item, measured in the platform encoding's bytes.
pub const MAX_HELPER_ARGUMENT_BYTES: usize = 16 * 1024;
/// All explicitly supplied platform environment values together.
pub const MAX_HELPER_ENVIRONMENT_BYTES: usize = 64 * 1024;

/// A command checked before any process is spawned. This is an invocation
/// policy, not an OS sandbox: trusted code still runs with the node account's
/// filesystem/network permissions. Node owns manifest path confinement and
/// candidate/revision review before it constructs this value.
pub struct PreparedCommand(CommandSpec);

impl fmt::Debug for PreparedCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PreparedCommand(<private node-prepared launch>)")
    }
}

/// Fixed refusals never include environment values, arguments or private paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LaunchRefusal {
    /// Program and cwd are resolved on the owning node before review.
    #[error("plugin helper program and working directory must be absolute")]
    Relative,
    /// No implicit environment inheritance or unspecified working directory.
    #[error("plugin helper requires an explicit working directory and cleared environment")]
    Ambient,
    /// Includes loader/runtime injection names, tokens and arbitrary settings.
    #[error("plugin helper environment contains a non-platform variable")]
    Environment,
    /// Refuse rather than truncating invocation semantics.
    #[error("plugin helper launch exceeds its argument/environment budget")]
    Capacity,
}

impl PreparedCommand {
    /// Validate a minimal environment and fully resolved launch description.
    ///
    /// # Errors
    /// Refuses ambient/relative launch or excessive/unsupported environment.
    pub fn new(spec: CommandSpec) -> Result<Self, LaunchRefusal> {
        if spec.environment != Environment::Clear || spec.cwd.is_none() {
            return Err(LaunchRefusal::Ambient);
        }
        if !spec.program.is_absolute() || spec.cwd.as_ref().is_some_and(|cwd| !cwd.is_absolute()) {
            return Err(LaunchRefusal::Relative);
        }
        if spec.args.len() > MAX_HELPER_ARGUMENTS
            || spec
                .args
                .iter()
                .any(|arg| arg.as_encoded_bytes().len() > MAX_HELPER_ARGUMENT_BYTES)
        {
            return Err(LaunchRefusal::Capacity);
        }
        let mut bytes = 0_usize;
        for (name, value) in &spec.env {
            let text = name.to_str().ok_or(LaunchRefusal::Environment)?;
            if !platform_variable(text) {
                return Err(LaunchRefusal::Environment);
            }
            let length = match value {
                EnvValue::Set(value) => value.as_encoded_bytes().len(),
                EnvValue::Remove => 0,
            };
            bytes = bytes
                .saturating_add(length)
                .saturating_add(name.as_encoded_bytes().len());
            if bytes > MAX_HELPER_ENVIRONMENT_BYTES {
                return Err(LaunchRefusal::Capacity);
            }
        }
        Ok(Self(spec))
    }

    pub(crate) fn into_spec(self) -> CommandSpec {
        self.0
    }
}

fn platform_variable(name: &str) -> bool {
    matches!(
        name.to_ascii_uppercase().as_str(),
        "PATH"
            | "SYSTEMROOT"
            | "WINDIR"
            | "TEMP"
            | "TMP"
            | "HOME"
            | "USERPROFILE"
            | "LANG"
            | "LC_ALL"
            | "LC_CTYPE"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command() -> CommandSpec {
        let executable = std::env::current_exe().expect("test executable");
        CommandSpec::new(executable.clone())
            .env_clear()
            .cwd(executable.parent().expect("parent"))
    }

    #[test]
    fn ambient_environment_loader_injection_and_relative_launch_refuse() {
        let mut inherited = command();
        inherited.environment = Environment::Inherit;
        assert!(matches!(
            PreparedCommand::new(inherited),
            Err(LaunchRefusal::Ambient)
        ));
        for name in [
            "SECRET_CANARY",
            "NODE_OPTIONS",
            "LD_PRELOAD",
            "DYLD_INSERT_LIBRARIES",
            "RUST_LOG",
        ] {
            let error = PreparedCommand::new(command().env(name, "PRIVATE-LAUNCH-CANARY"))
                .expect_err("refuse");
            assert_eq!(error, LaunchRefusal::Environment);
            assert!(!format!("{error:?} {error}").contains("PRIVATE-LAUNCH-CANARY"));
        }
        let mut relative = command();
        relative.program = "helper".into();
        assert!(matches!(
            PreparedCommand::new(relative),
            Err(LaunchRefusal::Relative)
        ));
    }

    #[test]
    fn platform_values_are_explicit_and_launch_debug_is_private() {
        let prepared = PreparedCommand::new(
            command()
                .env("HOME", "PRIVATE-LAUNCH-CANARY")
                .arg("PRIVATE-LAUNCH-CANARY"),
        )
        .expect("explicit");
        assert!(!format!("{prepared:?}").contains("PRIVATE-LAUNCH-CANARY"));
        assert_eq!(prepared.0.environment, Environment::Clear);
    }
}

#[cfg(test)]
mod runtime_entry_tests {
    use super::*;
    use crate::{RuntimeError, SharedLogs, Supervisor};
    use tokio_util::sync::CancellationToken;
    use vessel_proto::{
        NodeId,
        plugins::{PluginAddress, RuntimeGeneration},
    };

    #[test]
    fn launch_without_a_reactor_refuses_before_process_spawn() {
        let executable = std::env::current_exe().expect("test executable");
        let spec = CommandSpec::new(executable.clone())
            .env_clear()
            .cwd(executable.parent().expect("parent"));
        let address = PluginAddress {
            node: NodeId::random(),
            plugin: "test".parse().expect("id"),
            generation: RuntimeGeneration::random(),
        };
        assert!(matches!(
            Supervisor::spawn(
                PreparedCommand::new(spec).expect("prepared"),
                address,
                SharedLogs::default(),
                &CancellationToken::new()
            ),
            Err(RuntimeError::Runtime(_))
        ));
    }
}
