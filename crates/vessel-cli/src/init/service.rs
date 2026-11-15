//! `--service`: the node as a per-user service (plan decision 10): a
//! systemd user unit on Linux, a launchd agent on macOS; Windows later.
//!
//! [Decision Log]
//! - The service runs this executable's `node start` with this data
//!   directory, and with the `PATH` `vessel init` ran with: a service
//!   manager starts programs with a minimal `PATH`, where the person's
//!   harness installs (`~/.local/bin`, `nvm`, Homebrew) are missing, and a
//!   node finds harnesses on its `PATH`. The file says so, for editing.
//! - An existing file with the same content is left as it is (and enabled
//!   if it is not); one that differs is left alone and reported, since it
//!   may hold the person's edits. How to undo is printed, and written into
//!   the file.
//! - A successful node shutdown must stay stopped. launchd therefore uses
//!   `SuccessfulExit=false`, matching systemd `Restart=on-failure`; the
//!   existing-file check preserves older agents and personal edits (ADR 0064).
//! - The service managers are found on the search path, as harnesses are;
//!   tests put stand-ins there and never reach the real ones.

pub(super) mod matching;
pub(super) mod paths;

use std::path::{Path, PathBuf};
use std::process::Stdio;

use vessel_remote::RemoteCommand;

use super::{Init, Outcome, shown};
use crate::error::CliError;

/// What runs as the service: this executable, `--data-dir`, `node start`.
struct Node {
    exe: PathBuf,
    data_dir: PathBuf,
    path: String,
}

pub(super) async fn run(init: &mut Init) -> Result<(), CliError> {
    let Some(manager) = paths::Manager::current() else {
        init.report(
            Outcome::Skipped,
            "service",
            "not available on this system yet; start the node with `vessel node start` \
             (for example from Task Scheduler at logon)",
        )
        .await?;
        return Ok(());
    };
    let node = Node {
        exe: std::env::current_exe()?,
        data_dir: std::path::absolute(init.data_dir.root())?,
        path: search_path()?,
    };
    paths::validate_paths(&node.exe, &node.data_dir)?;
    match manager {
        paths::Manager::Systemd => systemd(init, &node).await,
        paths::Manager::Launchd => launchd(init, &node).await,
    }
}

/// Capture environment only at the binary edge, beside executable and directory.
/// The raw text is used only as a unit's Environment value after validation.
fn search_path() -> Result<String, paths::ServiceError> {
    let value = std::env::var_os("PATH")
        .unwrap_or_default()
        .into_string()
        .map_err(|_value| paths::ServiceError::Text(paths::Field::Search))?;
    paths::validate_search(&value)?;
    Ok(value)
}

/// The unit's name.
const UNIT: &str = "vessel.service";

async fn systemd(init: &mut Init, node: &Node) -> Result<(), CliError> {
    let config = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
    let file = paths::Manager::Systemd.file(&init.options.home, config.as_deref())?;
    let undo = paths::Manager::Systemd.undo(&file)?;
    let content = systemd_unit(node, undo.as_str());
    let Some(systemctl) = init.lookup.find("systemctl") else {
        init.report(Outcome::Skipped, "service", "no systemctl on this machine")
            .await?;
        return Ok(());
    };
    let wrote = match write_service_file(&file, content).await? {
        Written::Differs => {
            let detail = format!(
                "{} exists and differs; left as it is (remove it to have it written again)",
                shown(init, &file)
            );
            init.report(Outcome::Skipped, "service", &detail).await?;
            return Ok(());
        }
        Written::Same => false,
        Written::New => true,
    };
    let enabled = succeeds(&systemctl, &["--user", "is-enabled", "--quiet", UNIT]).await;
    if !wrote && enabled {
        init.report(Outcome::Already, "service", &format!("{UNIT}, enabled"))
            .await?;
        return Ok(());
    }
    let reloaded = succeeds(&systemctl, &["--user", "daemon-reload"]).await;
    if !reloaded || !succeeds(&systemctl, &["--user", "enable", "--now", UNIT]).await {
        let detail = format!(
            "wrote {}, but `systemctl --user enable --now {UNIT}` failed",
            shown(init, &file)
        );
        init.report(Outcome::Skipped, "service", &detail).await?;
        init.todo.push(format!(
            "enable the service: `systemctl --user enable --now {UNIT}`"
        ));
        return Ok(());
    }
    let detail = format!(
        "{} written, enabled and started; to undo: `{undo}`",
        shown(init, &file)
    );
    init.report(Outcome::Done, "service", &detail).await?;
    init.todo
        .push("to keep the node running after you log out: `loginctl enable-linger`".to_owned());
    Ok(())
}

/// The launchd agent's label.
const LABEL: &str = "vessel.node";

async fn launchd(init: &mut Init, node: &Node) -> Result<(), CliError> {
    let file = paths::Manager::Launchd.file(&init.options.home, None)?;
    let undo = paths::Manager::Launchd.undo(&file)?;
    let content = launchd_plist(node, undo.as_str());
    let Some(launchctl) = init.lookup.find("launchctl") else {
        init.report(Outcome::Skipped, "service", "no launchctl on this machine")
            .await?;
        return Ok(());
    };
    let wrote = match write_service_file(&file, content).await? {
        Written::Differs => {
            let detail = format!(
                "{} exists and differs; left as it is (remove it to have it written again)",
                shown(init, &file)
            );
            init.report(Outcome::Skipped, "service", &detail).await?;
            return Ok(());
        }
        Written::Same => false,
        Written::New => true,
    };
    let loaded = succeeds(&launchctl, &["list", LABEL]).await;
    if !wrote && loaded {
        init.report(Outcome::Already, "service", &format!("{LABEL}, loaded"))
            .await?;
        return Ok(());
    }
    let path = file.display().to_string();
    if !succeeds(&launchctl, &["load", "-w", &path]).await {
        let detail = format!(
            "wrote {}, but `launchctl load -w` failed",
            shown(init, &file)
        );
        init.report(Outcome::Skipped, "service", &detail).await?;
        let advice = RemoteCommand::program("launchctl")
            .arg("load")
            .arg("-w")
            .value(&path);
        init.todo.push(format!("load the service: `{advice}`"));
        return Ok(());
    }
    let detail = format!(
        "{} written and loaded; to undo: `{undo}`",
        shown(init, &file)
    );
    init.report(Outcome::Done, "service", &detail).await?;
    Ok(())
}

enum Written {
    New,
    Same,
    Differs,
}

/// The installer validates paths before handing its owned buffer to blocking I/O.
async fn write_service_file(file: &Path, content: String) -> Result<Written, CliError> {
    let writing = file.to_owned();
    Ok(tokio::task::spawn_blocking(move || write_once(&writing, &content)).await??)
}

/// Write `content` to `file` unless it exists; an existing one is compared.
fn write_once(file: &Path, content: &str) -> std::io::Result<Written> {
    use std::io::Write as _;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Exclusive creation closes the read-then-write race: another initializer's
    // new file or a person's edit must never be truncated by this invocation.
    match std::fs::File::options()
        .write(true)
        .create_new(true)
        .open(file)
    {
        Ok(mut output) => {
            output.write_all(content.as_bytes())?;
            Ok(Written::New)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = std::fs::File::open(file)?;
            Ok(if same_content(existing, content)? {
                Written::Same
            } else {
                Written::Differs
            })
        }
        Err(error) => Err(error),
    }
}

/// Byte equality needs only the expected prefix and EOF, with bounded buffering.
/// A large or non-Unicode personal file is different; it is never decoded or echoed.
fn same_content(existing: impl std::io::Read, expected: &str) -> std::io::Result<bool> {
    use std::io::Read as _;
    let mut bytes = std::io::BufReader::new(existing).bytes();
    for expected in expected.bytes() {
        if bytes.next().transpose()? != Some(expected) {
            return Ok(false);
        }
    }
    Ok(bytes.next().transpose()?.is_none())
}

/// Run a service manager command quietly; whether it succeeded.
async fn succeeds(program: &Path, args: &[&str]) -> bool {
    tokio::process::Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .is_ok_and(|status| status.success())
}

/// A systemd user unit for `node`.
fn systemd_unit(node: &Node, undo: &str) -> String {
    format!(
        "# Written by `vessel init --service`. To undo:\n\
         #   {undo}\n\
         # PATH is the one `vessel init` ran with, so the node finds the same\n\
         # harnesses; edit it here if they move.\n\
         [Unit]\n\
         Description=Vessel node\n\
         \n\
         [Service]\n\
         ExecStart={} --data-dir {} node start\n\
         Environment={}\n\
         Restart=on-failure\n\
         RestartSec=5\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        systemd_quoted(&node.exe.display().to_string()),
        systemd_argument(&node.data_dir.display().to_string()),
        systemd_quoted(&format!("PATH={}", node.path)),
    )
}

/// Exec arguments expand dollars even inside quotes; Environment= does not.
/// The executable path is resolved separately by systemd, without this expansion.
fn systemd_argument(text: &str) -> String {
    systemd_quoted(&text.replace('$', "$$"))
}

/// A double-quoted systemd word: `\` and `"` escaped, `%` doubled (it
/// starts a specifier).
fn systemd_quoted(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%");
    format!("\"{escaped}\"")
}

/// A launchd agent for `node`, logging to the data directory.
fn launchd_plist(node: &Node, undo: &str) -> String {
    let log = node.data_dir.join("logs").join("vessel-service.log");
    let string = |text: &str| format!("<string>{}</string>", xml_escaped(text));
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <!-- Written by `vessel init` (its service option). To undo: {} -->\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \x20 <key>Label</key>{}\n\
         \x20 <key>ProgramArguments</key>\n\
         \x20 <array>{}{}{}{}{}</array>\n\
         \x20 <key>EnvironmentVariables</key>\n\
         \x20 <dict><key>PATH</key>{}</dict>\n\
         \x20 <key>RunAtLoad</key><true/>\n\
         \x20 <key>KeepAlive</key>\n\
         \x20 <dict><key>SuccessfulExit</key><false/></dict>\n\
         \x20 <key>StandardOutPath</key>{}\n\
         \x20 <key>StandardErrorPath</key>{}\n\
         </dict>\n\
         </plist>\n",
        xml_comment(undo),
        string(LABEL),
        string(&node.exe.display().to_string()),
        string("--data-dir"),
        string(&node.data_dir.display().to_string()),
        string("node"),
        string("start"),
        string(&node.path),
        string(&log.display().to_string()),
        string(&log.display().to_string()),
    )
}

/// Comment syntax differs from XML text: overlapping runs can survive a
/// pair replacement and close the comment. Separate adjacent hyphens as
/// they are emitted, and keep the result valid even without a closing space.
fn xml_comment(text: &str) -> String {
    let mut escaped = String::new();
    for ch in text.chars() {
        if ch == '-' && escaped.ends_with('-') {
            escaped.push(' ');
        }
        escaped.push(ch);
    }
    if escaped.ends_with('-') {
        escaped.push(' ');
    }
    escaped
}

fn xml_escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node() -> Node {
        Node {
            exe: PathBuf::from("/opt/my tools/vessel"),
            data_dir: PathBuf::from("/home/someone/.vessel"),
            path: "/usr/bin:/home/someone/100%/bin".to_owned(),
        }
    }

    #[test]
    fn the_unit_runs_this_node_with_its_path_quoted_for_systemd() {
        let unit = systemd_unit(&node(), "systemctl --user disable --now vessel.service");
        assert!(
            unit.contains(
                "ExecStart=\"/opt/my tools/vessel\" --data-dir \"/home/someone/.vessel\" node start\n"
            ),
            "{unit}"
        );
        assert!(
            unit.contains("Environment=\"PATH=/usr/bin:/home/someone/100%%/bin\"\n"),
            "{unit}"
        );
        assert!(unit.contains("#   systemctl --user disable"), "{unit}");
        assert!(unit.contains("WantedBy=default.target"), "{unit}");
    }

    #[test]
    fn the_agent_runs_this_node_and_escapes_xml() {
        let mut node = node();
        node.path = "/a&b<c".to_owned();
        let plist = launchd_plist(&node, "launchctl unload -w 'x' && rm 'x'");
        assert!(
            plist.contains("<string>/opt/my tools/vessel</string><string>--data-dir</string>"),
            "{plist}"
        );
        assert!(plist.contains("<string>/a&amp;b&lt;c</string>"), "{plist}");
        assert!(
            plist.contains("To undo: launchctl unload -w 'x' && rm 'x' -->"),
            "{plist}"
        );
        // An XML comment may not hold `--`.
        let comment = plist.lines().find(|line| line.starts_with("<!--")).unwrap();
        assert!(
            !comment
                .trim_start_matches("<!--")
                .trim_end_matches("-->")
                .contains("--"),
            "{comment}"
        );
    }

    #[test]
    fn launchd_restarts_only_failed_exits() {
        let node = Node {
            exe: PathBuf::from("/opt/vessel"),
            data_dir: PathBuf::from("/home/test/.vessel"),
            path: "/usr/bin".to_owned(),
        };
        let plist = launchd_plist(&node, "remove the agent");
        assert!(
            plist
                .contains("<key>KeepAlive</key>\n  <dict><key>SuccessfulExit</key><false/></dict>")
        );
        assert!(!plist.contains("<key>KeepAlive</key><true/>"));
        assert!(plist.contains("<key>RunAtLoad</key><true/>"));
        assert!(systemd_unit(&node, "remove the unit").contains("Restart=on-failure"));
    }

    #[test]
    fn an_older_launchd_agent_is_preserved_instead_of_migrated() {
        let node = Node {
            exe: PathBuf::from("/opt/vessel"),
            data_dir: PathBuf::from("/home/test/.vessel"),
            path: "/usr/bin".to_owned(),
        };
        let current = launchd_plist(&node, "remove the agent");
        let older = current.replace(
            "<key>KeepAlive</key>\n  <dict><key>SuccessfulExit</key><false/></dict>",
            "<key>KeepAlive</key><true/>",
        );
        assert_ne!(older, current);
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("vessel.node.plist");
        std::fs::write(&file, &older).unwrap();
        assert!(matches!(
            write_once(&file, &current).unwrap(),
            Written::Differs
        ));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), older);
    }

    #[test]
    fn an_existing_file_is_compared_not_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a").join("unit");
        assert!(matches!(write_once(&file, "one").unwrap(), Written::New));
        assert!(matches!(write_once(&file, "one").unwrap(), Written::Same));
        assert!(matches!(
            write_once(&file, "two").unwrap(),
            Written::Differs
        ));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "one");
    }

    #[test]
    fn systemd_preserves_dollars_in_data_arguments_without_changing_the_environment() {
        let node = Node {
            exe: "/example/$literal/vessel".into(),
            data_dir: "/example/${HOME}/node".into(),
            path: "/example/$literal/bin".to_owned(),
        };
        let unit = systemd_unit(&node, "remove the unit");
        assert!(unit.contains("ExecStart=\"/example/$literal/vessel\" --data-dir \"/example/$${HOME}/node\" node start"));
        assert!(unit.contains("Environment=\"PATH=/example/$literal/bin\""));
    }

    #[test]
    fn concurrent_creators_preserve_the_first_written_service_file() {
        const WRITERS: usize = 2;
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("unit");
        let ready = std::sync::Barrier::new(WRITERS.saturating_add(1));
        let (first, second) = std::thread::scope(|scope| {
            let first = scope.spawn(|| {
                let _leader = ready.wait();
                write_once(&file, "first").unwrap()
            });
            let second = scope.spawn(|| {
                let _leader = ready.wait();
                write_once(&file, "second").unwrap()
            });
            let _leader = ready.wait();
            (first.join().unwrap(), second.join().unwrap())
        });
        let expected = if matches!(first, Written::New) {
            assert!(matches!(second, Written::Differs));
            "first"
        } else {
            assert!(matches!(first, Written::Differs));
            assert!(matches!(second, Written::New));
            "second"
        };
        assert_eq!(std::fs::read_to_string(file).unwrap(), expected);
    }

    #[test]
    fn a_non_unicode_existing_file_is_preserved_and_an_endless_reader_is_different() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("unit");
        let original = b"personal file: \xff";
        std::fs::write(&file, original).unwrap();
        assert!(matches!(
            write_once(&file, "new template").unwrap(),
            Written::Differs
        ));
        assert_eq!(std::fs::read(file).unwrap(), original);
        assert!(!same_content(std::io::repeat(b'x'), "x").unwrap());
    }

    #[cfg(unix)]
    mod plist_comment {

        use super::*;
        use tokio::io::AsyncWriteExt as _;

        const HYPHEN_RUNS: std::ops::RangeInclusive<usize> = 2..=6;
        const PLIST_TEST_BUDGET: std::time::Duration = std::time::Duration::from_secs(60);

        #[derive(serde::Serialize)]
        struct Case<'a> {
            plist: &'a str,
            exe: &'a Path,
            directory: &'a Path,
            search: &'a str,
            log: PathBuf,
        }

        async fn parse_and_check(plist: &str, node: &Node) {
            // Isolated Python imports standard library only and starts no child;
            // kill_on_drop bounds this direct parser to the regression's deadline.
            let mut child = tokio::process::Command::new("python3")
                .args([
                    "-I",
                    "-S",
                    "-c",
                    include_str!("../../tests/fixtures/service_plist.py"),
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .unwrap();
            let payload = serde_json::to_vec(&Case {
                plist,
                exe: &node.exe,
                directory: &node.data_dir,
                search: &node.path,
                log: node.data_dir.join("logs/vessel-service.log"),
            })
            .unwrap();
            let mut input = child.stdin.take().unwrap();
            input.write_all(&payload).await.unwrap();
            drop(input);
            let output = child.wait_with_output().await.unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        #[tokio::test]
        async fn comment_delimiters_in_home_paths_cannot_add_or_change_plist_keys() {
            vessel_testkit::within(PLIST_TEST_BUDGET, async {
                let temp = tempfile::tempdir().unwrap();
                for count in HYPHEN_RUNS {
                    let run = "-".repeat(count);
                    for name in [
                        format!("a{run}b"),
                        format!("x{run}><key>y"),
                        format!("x{run}!><key>y"),
                    ] {
                        let home = temp.path().join(name);
                        let file = paths::Manager::Launchd.file(&home, None).unwrap();
                        let undo = paths::Manager::Launchd.undo(&file).unwrap();
                        let node = Node {
                            exe: home.join("vessel"),
                            data_dir: home.join("data"),
                            path: home.join("bin").to_str().unwrap().to_owned(),
                        };
                        paths::validate_paths(&node.exe, &node.data_dir).unwrap();
                        paths::validate_search(&node.path).unwrap();
                        let plist = launchd_plist(&node, undo.as_str());
                        let comment = plist
                            .split_once("<!--")
                            .unwrap()
                            .1
                            .split_once("-->")
                            .unwrap()
                            .0;
                        assert!(!comment.contains("--"), "{comment}");
                        assert!(!comment.ends_with('-'), "{comment}");
                        parse_and_check(&plist, &node).await;
                    }
                    let terminal = xml_comment(&format!("x{run}"));
                    assert!(!terminal.contains("--"));
                    assert!(!terminal.ends_with('-'));
                }
            })
            .await
            .unwrap_or_else(|error| {
                panic!("plist regression exceeded {PLIST_TEST_BUDGET:?}: {error}")
            });
        }
    }
}
