//! Starting one installation (ADR 0115): its client document, and its
//! helper under `vessel-plugin`'s supervisor until the helper says ready.
//!
//! [Decision Log]
//! - Purpose: Paseo starts a plugin by compiling its entries and forking a
//!   child that answers `initialize` with its registrations
//!   (`packages/server/src/server/plugins/runtime.ts`); a start that does
//!   not reach ready leaves the plugin failed with the reason.
//! - Choice: the reviewed supervisor does the process work (group, frames,
//!   the 30 s ready deadline, stderr into the log tail). This module only
//!   prepares the launch from the manifest: the program resolved inside
//!   the directory or on the node's `PATH`, the directory as working
//!   directory, the environment cleared to the platform variables the
//!   supervisor admits (`PATH`, `HOME`, `LANG`, `LC_ALL`, `LC_CTYPE`, and
//!   the Windows temp/profile ones). No token, pairing link, harness home
//!   or secret reaches the helper; its identity comes in `initialize`.
//! - A plugin with only a client document has no process: it runs when its
//!   document validates.
//! - Trust: the helper is unsandboxed code with the node account's access
//!   (ADR 0079); this is a fault boundary, not a security one.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;
use vessel_plugin::{PreparedCommand, RuntimeState, SharedLogs, Supervisor};
use vessel_proc::CommandSpec;
use vessel_proto::plugins::{ClientDocument, MAX_DOCUMENT_BYTES, PluginAddress};

use super::manifest::Manifest;

/// The variables a helper gets from the node's own environment; every other
/// one is cleared. The supervisor refuses any name outside its list.
const PASSED: &[&str] = &[
    "PATH",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "TEMP",
    "TMP",
    "SYSTEMROOT",
    "WINDIR",
    "USERPROFILE",
];

/// The environment every helper starts with.
#[derive(Debug, Clone)]
pub(crate) struct LaunchEnv {
    passed: Vec<(&'static str, OsString)>,
    home: PathBuf,
}

impl LaunchEnv {
    /// From the node's process environment and the node user's home.
    pub(crate) fn from_node(home: &Path) -> Self {
        Self::from_vars(home, std::env::vars_os())
    }

    /// Keep, of `vars`, only the [`PASSED`] names (compared without case,
    /// as Windows compares them); everything else, a token or an API key
    /// the node itself was given included, stays with the node.
    pub(crate) fn from_vars(
        home: &Path,
        vars: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Self {
        let passed = vars
            .into_iter()
            .filter_map(|(name, value)| {
                let name = name.to_str()?;
                PASSED
                    .iter()
                    .find(|passed| passed.eq_ignore_ascii_case(name))
                    .map(|passed| (*passed, value))
            })
            .collect();
        Self {
            passed,
            home: home.to_path_buf(),
        }
    }

    fn path(&self) -> Option<&OsString> {
        self.passed
            .iter()
            .find(|(name, _)| *name == "PATH")
            .map(|(_, value)| value)
    }
}

/// Why an installation did not start, in the node's words (shown to an
/// administrator as the row's error, as Paseo's `errors` map).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum StartError {
    /// The manifest could not be read again or changed under the node.
    #[error("{0}")]
    Manifest(String),
    /// The client document could not be read or did not validate.
    #[error("client document {path}: {reason}")]
    Client {
        /// Its path.
        path: String,
        /// Why.
        reason: String,
    },
    /// The program could not be found or is outside the directory.
    #[error("server command: {0}")]
    Program(String),
    /// The supervisor refused the launch or the helper ended before ready.
    #[error("{0}")]
    Helper(String),
    /// The node is stopping.
    #[error("the node is stopping")]
    Stopping,
}

/// What runs for a started installation.
#[derive(Debug)]
pub(crate) enum Started {
    /// A client document that validated, and no process. The catalog
    /// (ADR 0115, P3) keeps the document; this packet only checks it.
    Client,
    /// A helper that answered ready.
    Helper(Supervisor),
}

/// Start an installation from `directory` with `manifest`.
pub(crate) async fn start(
    manifest: &Manifest,
    directory: &Path,
    address: PluginAddress,
    logs: &SharedLogs,
    env: &LaunchEnv,
    parent: &CancellationToken,
) -> Result<Started, StartError> {
    let client = match &manifest.client {
        Some(path) => Some(read_client(path).await?),
        None => None,
    };
    let Some(command) = &manifest.server else {
        let _validated: ClientDocument = client.ok_or_else(|| {
            StartError::Manifest(
                "it names neither a server command nor a client document".to_owned(),
            )
        })?;
        return Ok(Started::Client);
    };
    let launch = prepare(command, directory, env)?;
    let supervisor = Supervisor::spawn(launch, address, logs.clone(), parent)
        .map_err(|error| StartError::Helper(error.to_string()))?;
    let mut state = supervisor.handle().subscribe();
    let ready = state
        .wait_for(|state| !matches!(state, RuntimeState::Starting))
        .await
        .map(|state| matches!(*state, RuntimeState::Running { .. }));
    if matches!(ready, Ok(true)) {
        return Ok(Started::Helper(supervisor));
    }
    // It stopped before ready: the supervisor's result says why.
    match supervisor.shutdown().await {
        Err(error) => Err(StartError::Helper(error.to_string())),
        Ok(_receipt) if parent.is_cancelled() => Err(StartError::Stopping),
        Ok(_receipt) => Err(StartError::Helper(
            "plugin helper stopped before it was ready".to_owned(),
        )),
    }
}

/// Read and validate a client document, bounded before it is parsed.
async fn read_client(path: &Path) -> Result<ClientDocument, StartError> {
    let failed = |reason: String| StartError::Client {
        path: path.display().to_string(),
        reason,
    };
    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|error| failed(error.to_string()))?;
    if metadata.len() > u64::try_from(MAX_DOCUMENT_BYTES).unwrap_or(u64::MAX) {
        return Err(failed(format!("larger than {MAX_DOCUMENT_BYTES} bytes")));
    }
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|error| failed(error.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|error| failed(error.to_string()))
}

/// The launch for `command` run in `directory`.
fn prepare(
    command: &[String],
    directory: &Path,
    env: &LaunchEnv,
) -> Result<PreparedCommand, StartError> {
    PreparedCommand::new(spec(command, directory, env)?)
        .map_err(|refusal| StartError::Program(refusal.to_string()))
}

/// The command, its directory and its cleared environment.
fn spec(command: &[String], directory: &Path, env: &LaunchEnv) -> Result<CommandSpec, StartError> {
    let (program, arguments) = command
        .split_first()
        .ok_or_else(|| StartError::Program("the program is missing".to_owned()))?;
    let directory = std::fs::canonicalize(directory)
        .map_err(|error| StartError::Program(format!("{}: {error}", directory.display())))?;
    let program = resolve(program, &directory, env)?;
    let mut spec = CommandSpec::new(program)
        .env_clear()
        .cwd(&directory)
        .args(arguments)
        .env("HOME", &env.home);
    for (name, value) in &env.passed {
        spec = spec.env(*name, value);
    }
    Ok(spec)
}

/// The program: a path with a separator is inside `directory` (after links
/// are resolved), an absolute path is used as it is, a bare name is found
/// on the node's `PATH`.
fn resolve(program: &str, directory: &Path, env: &LaunchEnv) -> Result<PathBuf, StartError> {
    let path = Path::new(program);
    if path.is_absolute() {
        return path
            .is_file()
            .then(|| path.to_path_buf())
            .ok_or_else(|| StartError::Program(format!("{program} is not a file")));
    }
    if path.components().count() > 1 || program.starts_with('.') {
        let resolved = std::fs::canonicalize(directory.join(path))
            .map_err(|error| StartError::Program(format!("{program}: {error}")))?;
        if !resolved.starts_with(directory) {
            return Err(StartError::Program(format!(
                "{program} is outside the plugin directory"
            )));
        }
        return Ok(resolved);
    }
    which::which_in(program, env.path(), directory)
        .map_err(|_not_found| StartError::Program(format!("{program} was not found on PATH")))
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    #[test]
    fn programs_resolve_inside_the_directory_or_on_path() {
        let root = tempfile::tempdir().unwrap();
        let directory = std::fs::canonicalize(root.path()).unwrap();
        std::fs::create_dir(directory.join("bin")).unwrap();
        std::fs::write(directory.join("bin/helper"), b"").unwrap();
        let env = LaunchEnv {
            passed: vec![("PATH", directory.join("bin").into_os_string())],
            home: directory.clone(),
        };
        assert_eq!(
            resolve("./bin/helper", &directory, &env).unwrap(),
            directory.join("bin/helper")
        );
        assert_eq!(
            resolve("bin/helper", &directory, &env).unwrap(),
            directory.join("bin/helper")
        );
        assert!(matches!(
            resolve("../outside", &directory, &env),
            Err(StartError::Program(_))
        ));
        assert!(matches!(
            resolve("missing-program-name", &directory, &env),
            Err(StartError::Program(_))
        ));
    }

    /// Names a credential is kept under: none may reach a helper, even
    /// when the node has it set (ADR 0115, coordinator's check).
    pub(in crate::plugins) const CREDENTIALS: &[(&str, &str)] = &[
        ("ANTHROPIC_API_KEY", "canary-anthropic"),
        ("OPENAI_API_KEY", "canary-openai"),
        ("GH_TOKEN", "canary-gh"),
        ("GITHUB_TOKEN", "canary-github"),
        ("VESSEL_TOKEN", "canary-vessel"),
        ("AWS_SECRET_ACCESS_KEY", "canary-aws"),
        ("DATABASE_PASSWORD", "canary-password"),
        ("SSH_AUTH_SOCK", "canary-auth"),
        ("NPM_CONFIG__AUTH", "canary-npm"),
        ("Path_Secret_Key", "canary-mixed-case"),
    ];

    /// Whether `name` looks like it holds a credential.
    pub(in crate::plugins) fn credential_shaped(name: &str) -> bool {
        let upper = name.to_ascii_uppercase();
        ["TOKEN", "KEY", "SECRET", "PASSWORD", "AUTH"]
            .iter()
            .any(|part| upper.contains(part))
    }

    /// A node environment with every platform variable and every
    /// credential above.
    pub(in crate::plugins) fn node_vars(path: &str) -> Vec<(OsString, OsString)> {
        let mut vars: Vec<(OsString, OsString)> = CREDENTIALS
            .iter()
            .map(|(name, value)| ((*name).into(), (*value).into()))
            .collect();
        vars.extend(
            [
                ("Path", path),
                ("LANG", "C.UTF-8"),
                ("LC_ALL", "C.UTF-8"),
                ("HOME", "/node-home-is-not-passed"),
                ("TMPDIR", "/tmp/node"),
                ("RUST_LOG", "debug"),
            ]
            .iter()
            .map(|(name, value)| ((*name).into(), (*value).into())),
        );
        vars
    }

    #[test]
    fn no_credential_shaped_variable_reaches_a_helper() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("helper"), b"").unwrap();
        let env = LaunchEnv::from_vars(root.path(), node_vars("/usr/bin"));
        let spec = spec(&["./helper".to_owned()], root.path(), &env).unwrap();
        assert_eq!(spec.environment, vessel_proc::Environment::Clear);
        let names: Vec<String> = spec
            .env
            .iter()
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect();
        for name in &names {
            assert!(
                PASSED.contains(&name.as_str()) || name == "HOME",
                "{name} is not a platform variable"
            );
            assert!(!credential_shaped(name), "{name} reached the helper");
        }
        assert!(names.contains(&"PATH".to_owned()), "{names:?}");
        for (name, value) in &spec.env {
            let vessel_proc::EnvValue::Set(value) = value else {
                panic!("{name:?} is removed rather than set");
            };
            let value = value.to_string_lossy();
            assert!(!value.contains("canary"), "{name:?} carries a credential");
            assert!(
                !value.contains("node-home-is-not-passed"),
                "HOME is the node user's home"
            );
        }
        assert!(credential_shaped("GH_TOKEN") && credential_shaped("openai_api_key"));
        assert!(!credential_shaped("PATH") && !credential_shaped("LANG"));
    }

    #[test]
    fn the_launch_has_only_platform_variables() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("helper"), b"").unwrap();
        let env = LaunchEnv {
            passed: vec![("PATH", OsString::from("/usr/bin"))],
            home: root.path().to_path_buf(),
        };
        assert!(prepare(&["./helper".to_owned()], root.path(), &env).is_ok());
    }
}
