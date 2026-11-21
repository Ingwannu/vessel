//! What a terminal's process gets in its environment, and which shell runs
//! (ADR 0031). Pure, so both are tested without a process environment.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use vessel_proc::CommandSpec;

/// The variables a model route sets for a routed harness (ADR 0015):
/// removed only from a node that runs inside a session's process tree,
/// where they are that session's.
const ROUTE_VARIABLES: [&str; 2] = ["ANTHROPIC_BASE_URL", "ANTHROPIC_AUTH_TOKEN"];

/// The two that name an agent's connection to its node (ADR 0013).
const AGENT_VARIABLES: [&str; 2] = ["VESSEL_AGENT_SESSION", "VESSEL_NODE_SOCKET"];

/// The three a terminal's activity capability sets (ADR 0089). They are
/// removed like credentials, so another terminal never reports as this one.
/// They say that the node runs inside a terminal, not a session's process
/// tree, so they never take a model route's variables with them: those are
/// the user's own there (owed item 3 of the node review of TRACKER 815).
const TERMINAL_ACTIVITY_VARIABLES: [&str; 3] = [
    vessel_proto::notifications::TERMINAL_ACTIVITY_ID_ENV,
    vessel_proto::notifications::TERMINAL_ACTIVITY_SECRET_ENV,
    vessel_proto::notifications::TERMINAL_ACTIVITY_SOCKET_ENV,
];

/// Every credential Vessel itself defines, by name: what a node in a test
/// (whose environment is not read) removes.
pub const KNOWN_CREDENTIALS: [&str; 8] = [
    "VESSEL_TOKEN",
    "VESSEL_AGENT_SECRET",
    "VESSEL_MODEL_SECRET",
    "VESSEL_AGENT_SESSION",
    "VESSEL_NODE_SOCKET",
    vessel_proto::notifications::TERMINAL_ACTIVITY_ID_ENV,
    vessel_proto::notifications::TERMINAL_ACTIVITY_SECRET_ENV,
    vessel_proto::notifications::TERMINAL_ACTIVITY_SOCKET_ENV,
];

/// Whether `name` is one of Vessel's credentials: a `VESSEL_*` name ending
/// in `TOKEN` or `SECRET` (the node's token, an agent's or an MCP server's
/// secret, a model route's), one that names an agent's connection, or a
/// terminal's activity capability.
fn is_credential(name: &str) -> bool {
    (name.starts_with("VESSEL_") && (name.ends_with("TOKEN") || name.ends_with("SECRET")))
        || AGENT_VARIABLES.contains(&name)
        || TERMINAL_ACTIVITY_VARIABLES.contains(&name)
}

/// The names to remove from every terminal's environment, given the names
/// in the node's own: Vessel's credentials, and, when one of them other
/// than a terminal's activity capability is there (the node runs inside a
/// session's process tree), a model route's variables too. The binaries
/// call this with their own environment's names at start; library code
/// reads no process environment (AGENTS.md section 8).
pub fn credentials_in(names: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut found: Vec<OsString> = names
        .into_iter()
        .filter(|name| name.to_str().is_some_and(is_credential))
        .collect();
    let in_a_session = found.iter().any(|name| {
        name.to_str()
            .is_some_and(|name| !TERMINAL_ACTIVITY_VARIABLES.contains(&name))
    });
    if in_a_session {
        found.extend(ROUTE_VARIABLES.iter().map(OsString::from));
    }
    found.sort();
    found.dedup();
    found
}

/// The shell a terminal runs, found once by a binary at start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shell {
    /// The program.
    pub program: PathBuf,
    /// Its arguments (`-l` on macOS, `-NoLogo` for `PowerShell`).
    pub args: Vec<String>,
}

impl Shell {
    /// The user's shell (ADR 0031): on Unix `$SHELL` when it names an
    /// executable file, else the account's (`account`), else `/bin/sh`,
    /// with `-l` on macOS only (its Terminal starts every shell that way,
    /// so `~/.zprofile` applies). On Windows `pwsh.exe`, then
    /// `powershell.exe` (both `-NoLogo`), found on the search path, then
    /// `%ComSpec%`, then `cmd.exe` (agent-orchestrator's order).
    ///
    /// `var` reads the binary's environment, `account` the password
    /// database's shell, `on_path` finds a program; all given, so the
    /// choice is tested on every platform.
    #[must_use]
    pub fn detect(
        windows: bool,
        macos: bool,
        var: impl Fn(&str) -> Option<OsString>,
        account: impl Fn() -> Option<PathBuf>,
        on_path: impl Fn(&str) -> Option<PathBuf>,
        executable: impl Fn(&Path) -> bool,
    ) -> Self {
        if windows {
            for name in ["pwsh.exe", "powershell.exe"] {
                if let Some(program) = on_path(name) {
                    return Self {
                        program,
                        args: vec!["-NoLogo".to_owned()],
                    };
                }
            }
            let program = var("ComSpec")
                .map(PathBuf::from)
                .filter(|path| executable(path))
                .unwrap_or_else(|| PathBuf::from("cmd.exe"));
            return Self {
                program,
                args: Vec::new(),
            };
        }
        let program = var("SHELL")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && executable(path))
            .or_else(|| account().filter(|path| path.is_absolute() && executable(path)))
            .unwrap_or_else(|| PathBuf::from("/bin/sh"));
        let args = if macos {
            vec!["-l".to_owned()]
        } else {
            Vec::new()
        };
        Self { program, args }
    }

    /// `program` with no arguments (a test's `/bin/sh`).
    #[must_use]
    pub fn plain(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
        }
    }
}

/// Whether `path` is a file this account may run: on Unix an execute bit
/// (the kernel checks the rest at `exec`), on Windows a file.
pub(crate) fn executable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        metadata.is_file()
    }
}

/// The variables every terminal's process gets on top of the node's.
const TERMINAL_VARIABLES: [(&str, &str); 2] =
    [("TERM", "xterm-256color"), ("COLORTERM", "truecolor")];

/// `spec` with the terminal's variables set and `removed` taken out.
pub(crate) fn scrubbed(mut spec: CommandSpec, removed: &[OsString]) -> CommandSpec {
    for name in removed {
        spec = spec.env_remove(name);
    }
    for (name, value) in TERMINAL_VARIABLES {
        spec = spec.env(name, value);
    }
    spec
}

/// Where a terminal profile's program is (ADR 0117; Paseo's
/// `assertTerminalCommandCanStart`, `server/src/terminal/terminal.ts`
/// 706-747), found before anything starts so the person reads
/// `<program>: command not found` rather than a terminal that ends at once:
/// a program with a folder in it from `folder`, a bare name on `search`
/// (the node's own `PATH`, which its terminals inherit), skipping empty
/// entries. `None`: not there. Without a search path, or on Windows (whose
/// `.cmd` and `.bat` lookup Paseo leaves to the system too), the name is
/// taken as given.
pub(crate) fn find_program(
    program: &str,
    folder: &Path,
    search: Option<&OsStr>,
    executable: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let path = Path::new(program);
    if cfg!(windows) {
        return Some(path.to_path_buf());
    }
    if path.is_absolute() || path.components().nth(1).is_some() {
        let found = folder.join(path);
        return executable(&found).then_some(found);
    }
    let Some(search) = search else {
        return Some(path.to_path_buf());
    };
    std::env::split_paths(search)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| dir.join(path))
        .find(|candidate| executable(candidate))
}

/// The shell's command in `folder`.
pub(crate) fn shell_spec(shell: &Shell, folder: &Path, removed: &[OsString]) -> CommandSpec {
    let spec = CommandSpec::new(&shell.program)
        .args(shell.args.iter().map(OsStr::new))
        .cwd(folder);
    scrubbed(spec, removed)
}

#[cfg(test)]
mod tests {
    use vessel_proc::EnvValue;

    use super::*;

    fn names(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_profiles_program_is_found_on_the_search_path_or_from_the_folder() {
        if cfg!(windows) {
            return;
        }
        let here = Path::new("/work/site");
        let on = |found: &'static [&'static str]| move |path: &Path| found.iter().any(|f| path == Path::new(f));
        let search = OsString::from("/opt/a::/usr/bin");
        let found = on(&["/usr/bin/claude", "/work/site/bin/tool", "/opt/tools/x"]);
        assert_eq!(
            find_program("claude", here, Some(&search), &found),
            Some(PathBuf::from("/usr/bin/claude"))
        );
        assert_eq!(find_program("codex", here, Some(&search), &found), None);
        assert_eq!(
            find_program("bin/tool", here, Some(&search), &found),
            Some(PathBuf::from("/work/site/bin/tool"))
        );
        assert_eq!(
            find_program("/opt/tools/x", here, Some(&search), &found),
            Some(PathBuf::from("/opt/tools/x"))
        );
        assert_eq!(find_program("./missing", here, Some(&search), &found), None);
        // An empty entry is not the node's own folder.
        let only_empty = OsString::from(":");
        let local = on(&["claude"]);
        assert_eq!(find_program("claude", here, Some(&only_empty), &local), None);
        assert_eq!(
            find_program("claude", here, None, &found),
            Some(PathBuf::from("claude"))
        );
    }

    #[test]
    fn credentials_are_found_by_name() {
        let found = credentials_in(names(&[
            "PATH",
            "VESSEL_TOKEN",
            "VESSEL_MCP_1_0_SECRET",
            "VESSEL_DATA_DIR",
            "MY_TOKEN",
            "VESSEL_AGENT_SESSION",
        ]));
        assert_eq!(
            found,
            names(&[
                "ANTHROPIC_AUTH_TOKEN",
                "ANTHROPIC_BASE_URL",
                "VESSEL_AGENT_SESSION",
                "VESSEL_MCP_1_0_SECRET",
                "VESSEL_TOKEN",
            ]),
            "the user's own MY_TOKEN and the data folder stay; a route's \
             variables go only with a session's"
        );
    }

    #[test]
    fn a_node_started_in_a_terminal_keeps_the_users_route() {
        let activity = names(&[
            "PATH",
            vessel_proto::notifications::TERMINAL_ACTIVITY_ID_ENV,
            vessel_proto::notifications::TERMINAL_ACTIVITY_SECRET_ENV,
            vessel_proto::notifications::TERMINAL_ACTIVITY_SOCKET_ENV,
        ]);
        let mut expected = names(&TERMINAL_ACTIVITY_VARIABLES);
        expected.sort();
        assert_eq!(
            credentials_in(activity.clone()),
            expected,
            "the capability goes, the user's ANTHROPIC_BASE_URL and \
             ANTHROPIC_AUTH_TOKEN stay"
        );
        // Inside a session's process tree as well, the route goes too.
        let mut in_a_session = activity;
        in_a_session.push(OsString::from("VESSEL_AGENT_SESSION"));
        let found = credentials_in(in_a_session);
        for route in ROUTE_VARIABLES {
            assert!(found.contains(&OsString::from(route)), "{route}");
        }
    }

    #[test]
    fn the_forges_sign_in_never_reaches_a_terminal() {
        // Whatever the binary found, and by default (a test's node): the
        // variables the node's forge token is read from (ADR 0029) are
        // removed with Vessel's own credentials.
        let detected =
            crate::TerminalsConfig::detect(|_| None, names(&["PATH", "GH_TOKEN"]), |_| None);
        for config in [detected, crate::TerminalsConfig::default()] {
            for name in vessel_forge::SIGN_IN_VARIABLES
                .iter()
                .chain(KNOWN_CREDENTIALS.iter())
            {
                assert!(
                    config.removed.contains(&OsString::from(name)),
                    "{name} stays in {:?}",
                    config.removed
                );
            }
            assert!(!config.removed.contains(&OsString::from("PATH")));
        }
    }

    #[test]
    fn a_normal_nodes_environment_keeps_the_users_keys() {
        assert!(credentials_in(names(&["ANTHROPIC_AUTH_TOKEN", "HOME"])).is_empty());
    }

    #[test]
    fn the_scrubbed_spec_removes_and_sets() {
        let spec = shell_spec(
            &Shell::plain("/bin/sh"),
            Path::new("/tmp"),
            &names(&["VESSEL_TOKEN"]),
        );
        assert_eq!(spec.env_change("VESSEL_TOKEN"), Some(&EnvValue::Remove));
        assert_eq!(
            spec.env_change("TERM"),
            Some(&EnvValue::Set(OsString::from("xterm-256color")))
        );
    }

    #[test]
    fn the_shell_is_chosen_as_each_platform_starts_one() {
        let none = |_: &str| None::<OsString>;
        let nowhere = |_: &str| None::<PathBuf>;
        let yes = |_: &Path| true;
        let zsh = |name: &str| (name == "SHELL").then(|| OsString::from("/bin/zsh"));
        let mac = Shell::detect(false, true, zsh, || None, nowhere, yes);
        assert_eq!(mac.program, PathBuf::from("/bin/zsh"));
        assert_eq!(mac.args, ["-l"]);
        let linux = Shell::detect(false, false, zsh, || None, nowhere, yes);
        assert!(linux.args.is_empty());

        let relative = |name: &str| (name == "SHELL").then(|| OsString::from("zsh"));
        let account = || Some(PathBuf::from("/usr/bin/fish"));
        assert_eq!(
            Shell::detect(false, false, relative, account, nowhere, yes).program,
            PathBuf::from("/usr/bin/fish")
        );
        assert_eq!(
            Shell::detect(false, false, none, || None, nowhere, yes).program,
            PathBuf::from("/bin/sh")
        );

        let powershell =
            |name: &str| (name == "powershell.exe").then(|| PathBuf::from("C:/ps/powershell.exe"));
        let windows = Shell::detect(true, false, none, || None, powershell, yes);
        assert_eq!(windows.program, PathBuf::from("C:/ps/powershell.exe"));
        assert_eq!(windows.args, ["-NoLogo"]);
        let comspec = |name: &str| (name == "ComSpec").then(|| OsString::from("C:/cmd.exe"));
        assert_eq!(
            Shell::detect(true, false, comspec, || None, nowhere, yes).program,
            PathBuf::from("C:/cmd.exe")
        );
    }
}
