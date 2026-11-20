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

pub(super) mod paths;

use std::path::{Path, PathBuf};
use std::process::Stdio;

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
        path: paths::search_path()?,
    };
    paths::validate_paths(&node.exe, &node.data_dir)?;
    match manager {
        paths::Manager::Systemd => systemd(init, &node).await,
        paths::Manager::Launchd => launchd(init, &node).await,
    }
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
    let wrote = match write_once(&file, &content)? {
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
    let wrote = match write_once(&file, &content)? {
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
        init.todo
            .push(format!("load the service: `launchctl load -w '{path}'`"));
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

/// Write `content` to `file` unless it exists; an existing one is compared.
fn write_once(file: &Path, content: &str) -> std::io::Result<Written> {
    match std::fs::read_to_string(file) {
        Ok(existing) if existing == content => return Ok(Written::Same),
        Ok(_) => return Ok(Written::Differs),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(file, content)?;
    Ok(Written::New)
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
        // A comment expands no entities and may not hold `--`.
        undo.replace("--", "- -"),
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
}
