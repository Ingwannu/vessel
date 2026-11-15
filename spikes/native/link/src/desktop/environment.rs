//! Resolve login environment into child command data, never mutate process globals.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use tokio::io::AsyncReadExt as _;
use vessel_proc::{CommandSpec, EnvValue};
use vessel_proto::SessionId;

use super::DesktopError;

/// The reference's overall shell-resolution deadline.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_ENV_BYTES: u64 = 1024 * 1024;

/// Values stay redacted through the process crate's existing environment type.
#[derive(Debug, Clone, Default)]
pub struct ShellEnvironment {
    overrides: Vec<(OsString, EnvValue)>,
}

impl ShellEnvironment {
    /// The inherited OS environment is the Windows path and the failure fallback.
    #[must_use]
    pub const fn inherited() -> Self {
        Self {
            overrides: Vec::new(),
        }
    }

    /// App-only restart markers must not turn later child launches into relaunches.
    #[must_use]
    pub fn without(mut self, name: impl Into<OsString>) -> Self {
        self.overrides.push((name.into(), EnvValue::Remove));
        self
    }

    /// Pass captured values only to an explicitly owned child, without unsafe `set_var`.
    #[must_use]
    pub fn apply(&self, mut command: CommandSpec) -> CommandSpec {
        command.env.extend(self.overrides.iter().cloned());
        command
    }
}

/// A validated login shell path has no embedded argument syntax.
#[derive(Debug, Clone)]
pub struct LoginShell(PathBuf);

impl LoginShell {
    /// Resolve the shell only at the binary edge; the library never reads HOME/SHELL.
    ///
    /// # Errors
    /// Refuses relative paths and shells whose invocation semantics are unknown.
    pub fn new(path: PathBuf) -> Result<Self, DesktopError> {
        if !path.is_absolute() {
            return Err(DesktopError::RelativePath);
        }
        match path.file_name().and_then(|name| name.to_str()) {
            Some("bash" | "zsh" | "sh" | "dash" | "fish") => Ok(Self(path)),
            Some(_) | None => Err(DesktopError::ShellEnvironment),
        }
    }
}

/// Capture both shell attempts inside one deadline; all pipes are bounded/discarded.
///
/// # Errors
/// Refuses timeout, output overflow, nonzero exit, malformed values and process errors.
pub async fn resolve(
    shell: LoginShell,
    timeout: Duration,
) -> Result<ShellEnvironment, DesktopError> {
    let mark = format!("vessel-env-{}", SessionId::random());
    let command = format!("printf '\\0{mark}\\0'; /usr/bin/env -0; printf '\\0{mark}\\0'");
    let spec = CommandSpec::new(shell.0)
        .env_remove("PASEO_NODE_ENV")
        .env_remove("PASEO_DESKTOP_MANAGED")
        .env_remove("PASEO_SUPERVISED");
    let first = timeout
        .checked_div(2)
        .ok_or(DesktopError::ShellEnvironment)?;
    let work = async {
        match tokio::time::timeout(
            first,
            attempt(spec.clone().args(["-i", "-l", "-c", &command]), &mark),
        )
        .await
        {
            Ok(Ok(environment)) => Ok(environment),
            Ok(Err(_)) | Err(_) => attempt(spec.args(["-l", "-c", &command]), &mark).await,
        }
    };
    tokio::time::timeout(timeout, work)
        .await
        .map_err(|_expired| DesktopError::ShellEnvironment)?
}

async fn attempt(spec: CommandSpec, mark: &str) -> Result<ShellEnvironment, DesktopError> {
    let vessel_proc::Spawned {
        mut child,
        stdin,
        stdout,
        stderr,
    } = vessel_proc::spawn(&spec)?;
    drop(stdin);
    let mut output = Vec::new();
    let stdout = async {
        let _read = stdout
            .take(MAX_ENV_BYTES.saturating_add(1))
            .read_to_end(&mut output)
            .await?;
        if u64::try_from(output.len()).map_err(|_overflow| DesktopError::ShellEnvironment)?
            > MAX_ENV_BYTES
        {
            return Err(DesktopError::ShellEnvironment);
        }
        Ok::<(), DesktopError>(())
    };
    let stderr = async {
        let _discarded =
            tokio::io::copy(&mut stderr.take(MAX_ENV_BYTES), &mut tokio::io::sink()).await?;
        Ok::<(), DesktopError>(())
    };
    let ((), (), exit) = tokio::try_join!(stdout, stderr, async {
        child.wait().await.map_err(DesktopError::from)
    })?;
    if exit != Some(0) {
        return Err(DesktopError::ShellEnvironment);
    }
    decode(&output, mark)
}

fn decode(bytes: &[u8], mark: &str) -> Result<ShellEnvironment, DesktopError> {
    let marker = format!("\0{mark}\0");
    let text = std::str::from_utf8(bytes).map_err(|_invalid| DesktopError::ShellEnvironment)?;
    let (_preamble, rest) = text
        .split_once(&marker)
        .ok_or(DesktopError::ShellEnvironment)?;
    let (body, _epilogue) = rest
        .split_once(&marker)
        .ok_or(DesktopError::ShellEnvironment)?;
    let mut overrides = Vec::new();
    for pair in body.split('\0').filter(|pair| !pair.is_empty()) {
        let (name, value) = pair
            .split_once('=')
            .filter(|(name, _)| !name.is_empty())
            .ok_or(DesktopError::ShellEnvironment)?;
        // Session/display routing belongs to the actual GUI process, even when a
        // login profile exports a stale value from a previous desktop session.
        if name == "XDG_RUNTIME_DIR" {
            continue;
        }
        overrides.push((name.into(), EnvValue::Set(value.into())));
    }
    Ok(ShellEnvironment { overrides })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_noise_is_excluded_and_values_are_never_in_debug() {
        let environment=decode(b"profile noise\0mark\0PATH=/custom/bin\0TOKEN=hidden-value\0XDG_RUNTIME_DIR=/stale\0\0mark\0bye","mark").unwrap();
        let spec = environment.apply(CommandSpec::new("/unused"));
        assert_eq!(
            spec.env_change("PATH"),
            Some(&EnvValue::Set("/custom/bin".into()))
        );
        assert_eq!(spec.env_change("XDG_RUNTIME_DIR"), None);
        assert!(!format!("{environment:?}").contains("hidden-value"));
        assert!(decode(b"missing markers", "mark").is_err());
        assert!(decode(b"\0mark\0BROKEN\0\0mark\0", "mark").is_err());
    }

    #[test]
    fn a_removed_app_marker_wins_over_inherited_and_captured_values() {
        for environment in [
            ShellEnvironment::inherited(),
            decode(b"\0m\0APP_MARKER=old\0\0m\0", "m").unwrap(),
        ] {
            let child = environment
                .without("APP_MARKER")
                .apply(CommandSpec::new("/unused").env("APP_MARKER", "new"));
            assert_eq!(child.env_change("APP_MARKER"), Some(&EnvValue::Remove));
        }
    }
}
