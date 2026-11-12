//! The processes under load: one `vesseld` with the fake harnesses and
//! one `vessel ui` whose book names it, each its own process so each one's
//! memory is its own. Everything lives in one temporary directory; no
//! process sees the machine's own account or harnesses.

use std::io::Write as _;
use std::net::{Ipv4Addr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt as _, BufReader};
use tokio::process::{Child, Command};
use vessel_harness::fakes::{Role, install};
use vessel_proto::HarnessKind;
use vessel_testkit::Fake;

/// The node and the UI, running.
pub(crate) struct Fleet {
    _root: TempDir,
    pub(crate) node: Child,
    pub(crate) ui: Child,
    /// The UI's launch: its host (`vessel-<id>.localhost:<port>`), port and
    /// one-use bootstrap secret.
    pub(crate) launch: Launch,
}

/// Where the UI listens and the secret its page would be handed.
pub(crate) struct Launch {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) bootstrap: String,
}

impl Fleet {
    /// Start the node and the UI. `vesseld` comes from `vessel_testkit`'s
    /// nested build in this test's profile, so `--release` measures
    /// release binaries.
    pub(crate) async fn start() -> Self {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let tui = Fake::Tui.path().unwrap();
        drop(install(&bin, HarnessKind::ClaudeCode, Role::Terminal, &tui).unwrap());
        let acp = Fake::Acp.path().unwrap();
        drop(install(&bin, HarnessKind::ClaudeCode, Role::Structured, &acp).unwrap());
        let vesseld = vessel_testkit::vesseld().unwrap();
        let data = root.path().join("node");
        let home = root.path().join("node-home");
        std::fs::create_dir_all(&home).unwrap();
        let port = free_port();
        let mut node = Command::new(vesseld)
            .arg("--data-dir")
            .arg(&data)
            .args(["--bind", &format!("127.0.0.1:{port}"), "--name", "load"])
            .env_clear()
            .env("PATH", path(&bin))
            .env("HOME", &home)
            // Warnings, and the one line that says it is listening.
            .env("RUST_LOG", "warn,vessel_node::node=info")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        listening(&mut node).await;
        let token = data.join("token");
        let ui_home = root.path().join("ui-home");
        let vessel = |args: &[&str]| {
            let mut command = Command::new(env!("CARGO_BIN_EXE_vessel"));
            let _command = command
                .args(args)
                .env_clear()
                .env("PATH", path(&bin))
                .env("HOME", &ui_home)
                .env("VESSEL_CONFIG_DIR", ui_home.join("config"))
                .env("VESSEL_DATA_DIR", ui_home.join("data"))
                .env("RUST_LOG", "warn")
                .stdin(Stdio::null())
                .kill_on_drop(true);
            command
        };
        let added = vessel(&[
            "nodes",
            "add",
            "load",
            &format!("ws://127.0.0.1:{port}"),
            "--token-file",
            &token.display().to_string(),
            "--default",
        ])
        .output()
        .await
        .unwrap();
        assert!(added.status.success(), "{added:?}");
        let mut ui = vessel(&["ui", "--no-open", "--port", "0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let launch = read_launch(&mut ui).await;
        Self {
            _root: root,
            node,
            ui,
            launch,
        }
    }

    /// The node's and the UI's process ids.
    pub(crate) fn pids(&self) -> (u32, u32) {
        (self.node.id().unwrap(), self.ui.id().unwrap())
    }
}

/// The fakes first, then the system's directories (for `/bin/sh` and the
/// like), never the user's own.
fn path(bin: &Path) -> String {
    format!("{}:/usr/local/bin:/usr/bin:/bin", bin.display())
}

/// A port nothing listens on right now. Another process could take it
/// before the node binds; on a machine running this test by hand that is
/// not worth a retry loop.
fn free_port() -> u16 {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.local_addr().unwrap().port()
}

/// Wait for the node's `node listening` line (its token is written by
/// then), then keep its log flowing to this test's stderr.
async fn listening(node: &mut Child) {
    let mut lines = BufReader::new(node.stderr.take().unwrap()).lines();
    loop {
        let line = lines.next_line().await.unwrap().expect("vesseld ended");
        if line.contains("node listening") {
            break;
        }
        // An earlier line is a warning worth seeing.
        let _shown = writeln!(std::io::stderr().lock(), "vesseld: {line}");
    }
    drop(tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            let _shown = writeln!(std::io::stderr().lock(), "vesseld: {line}");
        }
    }));
}

/// The launch from `vessel ui`'s `Vessel UI: http://<host>/#bootstrap=…`.
async fn read_launch(ui: &mut Child) -> Launch {
    let mut lines = BufReader::new(ui.stdout.take().unwrap()).lines();
    loop {
        let line = lines.next_line().await.unwrap().expect("vessel ui ended");
        let Some(url) = line.strip_prefix("Vessel UI: http://") else {
            continue;
        };
        let (host, bootstrap) = url.split_once("/#bootstrap=").unwrap();
        let port = host.rsplit_once(':').unwrap().1.parse().unwrap();
        // The rest of its output is not read; keep draining it so the UI
        // never blocks on a full pipe.
        drop(tokio::spawn(async move {
            while let Ok(Some(_line)) = lines.next_line().await {}
        }));
        return Launch {
            host: host.to_owned(),
            port,
            bootstrap: bootstrap.to_owned(),
        };
    }
}

/// Resident memory of `pid` now and at its peak, in KiB (Linux:
/// `/proc/<pid>/status`; `None` elsewhere).
pub(crate) fn memory(pid: u32) -> Option<(u64, u64)> {
    let status = std::fs::read_to_string(PathBuf::from(format!("/proc/{pid}/status"))).ok()?;
    let field = |name: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .and_then(|rest| {
                rest.trim()
                    .trim_end_matches("kB")
                    .trim()
                    .parse::<u64>()
                    .ok()
            })
    };
    Some((field("VmRSS:")?, field("VmHWM:")?))
}
