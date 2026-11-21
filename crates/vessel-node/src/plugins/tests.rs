//! The plugin host against real directories: client-only plugins, and on
//! Unix small `sh` helpers that speak the private frames (a length, then
//! JSON). Nothing is fetched; every plugin is a temporary directory.

use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::NodeId;
use vessel_proto::plugins::{LogStream, PluginGeneration, PluginId, PluginStatus};
use vessel_store::DataDir;

use super::{Opening, PluginError, Plugins, WATCH_LIMIT};

const EMPTY_CLIENT: &str = r#"{"methods":[],"contributions":[],"themes":[]}"#;

pub(super) struct Fixture {
    root: tempfile::TempDir,
    tasks: TaskTracker,
    stop: CancellationToken,
}

impl Fixture {
    pub(super) fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
            tasks: TaskTracker::new(),
            stop: CancellationToken::new(),
        }
    }

    pub(super) fn data(&self) -> DataDir {
        DataDir::new(self.root.path().join("data"))
    }

    pub(super) fn home(&self) -> PathBuf {
        self.root.path().join("home")
    }

    pub(super) async fn host(&self, switch: bool) -> Plugins {
        std::fs::create_dir_all(self.home()).unwrap();
        Plugins::start(Opening {
            node: NodeId::random(),
            data: self.data(),
            home: self.home(),
            switch,
            tasks: self.tasks.clone(),
            stop: self.stop.clone(),
        })
        .await
    }

    /// A plugin directory under the home with `manifest` and `files`.
    pub(super) fn plugin(&self, name: &str, manifest: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let directory = self.home().join(name);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("vessel-plugin.json"), manifest).unwrap();
        for (file, contents) in files {
            write_file(&directory.join(file), contents);
        }
        directory
    }

    pub(super) async fn finish(self) {
        self.stop.cancel();
        let _closed = self.tasks.close();
        self.tasks.wait().await;
    }
}

fn write_file(path: &Path, contents: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

pub(super) fn id(text: &str) -> PluginId {
    text.parse().unwrap()
}

pub(super) fn client_only(id: &str) -> String {
    format!(r#"{{"id":"{id}","name":"Night","description":"A theme","client":"client.json"}}"#)
}

async fn status(plugins: &Plugins, plugin: &str) -> Option<PluginStatus> {
    plugins
        .list(true)
        .await
        .plugins
        .into_iter()
        .find(|item| item.id.as_str() == plugin)
        .map(|item| item.status)
}

/// Wait until `plugin` has `wanted` (a switch applies in the background).
pub(super) async fn until(plugins: &Plugins, plugin: &str, wanted: PluginStatus) {
    let mut generation = PluginGeneration::default();
    for _ in 0..40 {
        if status(plugins, plugin).await == Some(wanted) {
            return;
        }
        generation = tokio::time::timeout(WATCH_LIMIT, plugins.watch(generation))
            .await
            .unwrap();
    }
    panic!("{plugin} never became {wanted:?}");
}

#[tokio::test]
async fn a_client_only_plugin_installs_disabled_while_off_and_runs_when_switched_on() {
    let fixture = Fixture::new();
    let plugins = fixture.host(false).await;
    let _directory = fixture.plugin(
        "night",
        &client_only("night"),
        &[("client.json", EMPTY_CLIENT.as_bytes())],
    );
    let installed = plugins.install("~/night", None, None).await.unwrap();
    assert_eq!(installed.id.as_str(), "night");
    assert_eq!(installed.status, PluginStatus::Disabled);
    assert!(installed.enabled);
    assert_eq!(installed.name.unwrap().as_str(), "Night");
    // Paths and errors are an administrator's.
    let scoped = plugins.list(false).await;
    assert_eq!(scoped.plugins.first().unwrap().path, None);
    let admin = plugins.list(true).await;
    assert_eq!(
        admin.plugins.first().unwrap().path.as_deref(),
        Some(fixture.home().join("night").to_str().unwrap())
    );
    plugins.set_switch(true);
    until(&plugins, "night", PluginStatus::Running).await;
    let logs = plugins.logs(&id("night")).unwrap();
    let lines: Vec<_> = logs
        .entries
        .iter()
        .map(|entry| (entry.stream, entry.message.as_str().to_owned()))
        .collect();
    assert!(lines.contains(&(LogStream::Host, "[vessel] loading".to_owned())));
    assert!(lines.contains(&(LogStream::Host, "[vessel] ready".to_owned())));
    plugins.set_switch(false);
    until(&plugins, "night", PluginStatus::Disabled).await;
    fixture.finish().await;
}

#[tokio::test]
async fn enable_disable_reload_and_remove_follow_paseo() {
    let fixture = Fixture::new();
    let plugins = fixture.host(true).await;
    let directory = fixture.plugin(
        "night",
        &client_only("night"),
        &[("client.json", EMPTY_CLIENT.as_bytes())],
    );
    let night = id("night");
    let other = plugins
        .install("~/night", Some(id("second")), None)
        .await
        .unwrap();
    assert_eq!(other.status, PluginStatus::Running);
    assert_eq!(
        plugins.install("~/night", None, None).await.unwrap().status,
        PluginStatus::Running
    );
    assert!(matches!(
        plugins.install("~/night", None, None).await,
        Err(PluginError::Exists(_))
    ));
    let disabled = plugins.disable(&night).await.unwrap();
    assert_eq!(
        (disabled.enabled, disabled.status),
        (false, PluginStatus::Disabled)
    );
    assert!(matches!(
        plugins.reload(&night).await,
        Err(PluginError::Disabled(_))
    ));
    let enabled = plugins.enable(&night).await.unwrap();
    assert_eq!(enabled.status, PluginStatus::Running);
    assert_eq!(
        plugins.reload(&night).await.unwrap().status,
        PluginStatus::Running
    );
    // A plugin's settings go with it; its directory stays.
    let settings = fixture.data().plugin_settings_dir(&night);
    write_file(&settings.join("main.json"), b"{}");
    plugins.remove(&night).await.unwrap();
    assert!(!settings.exists());
    assert!(directory.exists());
    assert!(matches!(
        plugins.logs(&night),
        Err(PluginError::NotInstalled(_))
    ));
    assert!(matches!(
        plugins.remove(&night).await,
        Err(PluginError::NotInstalled(_))
    ));
    let listed: Vec<_> = plugins
        .list(true)
        .await
        .plugins
        .into_iter()
        .map(|item| item.id)
        .collect();
    assert_eq!(listed, [id("second")]);
    fixture.finish().await;
}

#[tokio::test]
async fn installations_are_kept_owner_only_and_start_again_with_the_node() {
    let fixture = Fixture::new();
    let _directory = fixture.plugin(
        "night",
        &client_only("night"),
        &[("client.json", EMPTY_CLIENT.as_bytes())],
    );
    {
        let plugins = fixture.host(true).await;
        let _installed = plugins.install("~/night", None, None).await.unwrap();
        let _disabled = plugins.disable(&id("night")).await.unwrap();
        let _enabled = plugins.enable(&id("night")).await.unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let file = std::fs::metadata(fixture.data().plugin_installations_path()).unwrap();
        assert_eq!(file.permissions().mode() & 0o777, 0o600);
    }
    let again = fixture.host(true).await;
    // The first list waits for the start at node start.
    assert_eq!(status(&again, "night").await, Some(PluginStatus::Running));
    fixture.finish().await;
}

#[tokio::test]
async fn refusals_say_why() {
    let fixture = Fixture::new();
    let plugins = fixture.host(true).await;
    let paseo = fixture.home().join("paseo");
    write_file(&paseo.join("paseo-plugin.json"), br#"{"id":"paseo"}"#);
    let refused = plugins.install("~/paseo", None, None).await.unwrap_err();
    assert!(matches!(
        refused,
        PluginError::Manifest(super::ManifestError::Paseo)
    ));
    assert!(matches!(
        plugins.install("github:owner/repo", None, None).await,
        Err(PluginError::Source(super::SourceError::Managed))
    ));
    assert!(matches!(
        plugins.inspect("~/paseo").await,
        Err(PluginError::Manifest(super::ManifestError::Paseo))
    ));
    let _directory = fixture.plugin(
        "night",
        &client_only("night"),
        &[("client.json", EMPTY_CLIENT.as_bytes())],
    );
    assert_eq!(
        plugins.inspect("~/night").await.unwrap().id.as_str(),
        "night"
    );
    // A client document that does not validate leaves the row failed.
    let _directory = fixture.plugin(
        "broken",
        &client_only("broken"),
        &[("client.json", b"{\"methods\":7}")],
    );
    let failed = plugins.install("~/broken", None, None).await.unwrap_err();
    assert!(matches!(failed, PluginError::Failed { .. }), "{failed}");
    let row = plugins
        .list(true)
        .await
        .plugins
        .into_iter()
        .find(|item| item.id.as_str() == "broken")
        .unwrap();
    assert_eq!(row.status, PluginStatus::Failed);
    assert!(row.error.unwrap().contains("client document"));
    // While plugins are off, reload says so.
    plugins.set_switch(false);
    until(&plugins, "broken", PluginStatus::Disabled).await;
    assert!(matches!(
        plugins.reload(&id("broken")).await,
        Err(PluginError::Off)
    ));
    fixture.finish().await;
}

#[tokio::test]
async fn watch_answers_a_newer_generation_after_a_change() {
    let fixture = Fixture::new();
    let plugins = fixture.host(false).await;
    let before = plugins.watch(PluginGeneration::new(u64::MAX)).await;
    let waiting = {
        let plugins = plugins.clone();
        tokio::spawn(async move { plugins.watch(before).await })
    };
    let _directory = fixture.plugin(
        "night",
        &client_only("night"),
        &[("client.json", EMPTY_CLIENT.as_bytes())],
    );
    let _installed = plugins.install("~/night", None, None).await.unwrap();
    let after = waiting.await.unwrap();
    assert!(after > before);
    fixture.finish().await;
}

/// A POSIX `sh` helper: it writes `ready` (a frame), then reads frames and
/// exits after `frames` of them (the first is `initialize`; the second, in
/// these tests, `shutdown`). `exit_now` exits before writing anything.
#[cfg(unix)]
fn helper_script(frames: u32, exit_now: bool) -> Vec<u8> {
    use std::fmt::Write as _;

    use vessel_proto::plugins::{ClientDocument, HelperProtocol, HelperToNode, HookSubscriptions};

    if exit_now {
        return b"#!/bin/sh\nexit 3\n".to_vec();
    }
    let ready = HelperToNode::Ready {
        protocol: HelperProtocol::V1,
        client: ClientDocument::new(vec![], vec![], vec![]).unwrap(),
        hooks: HookSubscriptions::default(),
        settings: vessel_proto::plugins::SettingsDeclarations::default(),
    };
    let body = serde_json::to_vec(&ready).unwrap();
    let mut frame = u32::try_from(body.len()).unwrap().to_be_bytes().to_vec();
    frame.extend_from_slice(&body);
    let mut escaped = String::new();
    for byte in frame {
        write!(escaped, "\\{byte:03o}").unwrap();
    }
    format!(
        "#!/bin/sh\nprintf '{escaped}'\nframes=0\nwhile :; do\n  set -- $(dd bs=1 count=4 2>/dev/null | od -An -tu1)\n  [ \"$#\" -eq 4 ] || exit 0\n  length=$(( ($1 << 24) | ($2 << 16) | ($3 << 8) | $4 ))\n  dd bs=1 count=\"$length\" of=/dev/null 2>/dev/null\n  frames=$((frames + 1))\n  [ \"$frames\" -ge {frames} ] && exit 0\ndone\n"
    )
    .into_bytes()
}

#[cfg(unix)]
pub(super) fn helper_plugin(fixture: &Fixture, name: &str, script: &[u8]) {
    use std::os::unix::fs::PermissionsExt as _;

    let manifest = format!(r#"{{"id":"{name}","server":{{"command":["./helper.sh"]}}}}"#);
    let directory = fixture.plugin(name, &manifest, &[]);
    let helper = directory.join("helper.sh");
    // A script this test runs is never written by this process: another
    // test's fork would hold the open descriptor, and running it would then
    // fail with ETXTBSY (`vessel_testkit::write_executable`, AGENTS.md 8).
    vessel_testkit::write_executable(&helper, std::str::from_utf8(script).unwrap()).unwrap();
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn a_helper_runs_until_disabled_and_a_failing_one_is_failed_with_its_reason() {
    let fixture = Fixture::new();
    let plugins = fixture.host(true).await;
    helper_plugin(&fixture, "steady", &helper_script(2, false));
    let running = plugins.install("~/steady", None, None).await.unwrap();
    assert_eq!(running.status, PluginStatus::Running);
    assert_eq!(
        plugins.reload(&id("steady")).await.unwrap().status,
        PluginStatus::Running
    );
    let disabled = plugins.disable(&id("steady")).await.unwrap();
    assert_eq!(disabled.status, PluginStatus::Disabled);
    let lines: Vec<String> = plugins
        .logs(&id("steady"))
        .unwrap()
        .entries
        .iter()
        .map(|entry| entry.message.as_str().to_owned())
        .collect();
    assert!(lines.contains(&"[vessel] stopping".to_owned()), "{lines:?}");
    assert!(lines.contains(&"[vessel] stopped".to_owned()), "{lines:?}");

    helper_plugin(&fixture, "crashing", &helper_script(0, true));
    let failed = plugins.install("~/crashing", None, None).await.unwrap_err();
    assert!(matches!(failed, PluginError::Failed { .. }), "{failed}");
    assert_eq!(
        status(&plugins, "crashing").await,
        Some(PluginStatus::Failed)
    );

    // Ready, then gone after `initialize`: the row ends failed.
    helper_plugin(&fixture, "leaving", &helper_script(1, false));
    let _outcome = plugins.install("~/leaving", None, None).await;
    until(&plugins, "leaving", PluginStatus::Failed).await;
    let error = plugins
        .list(true)
        .await
        .plugins
        .into_iter()
        .find(|item| item.id.as_str() == "leaving")
        .and_then(|item| item.error);
    assert!(error.is_some());
    fixture.finish().await;
}

/// The coordinator's check on ADR 0115: a real helper, started while the
/// node's own environment holds API keys and tokens, sees none of them;
/// it prints its whole environment to stderr, which the log tail keeps.
#[cfg(unix)]
#[tokio::test]
async fn a_real_helper_sees_no_credential_the_node_has() {
    use vessel_proto::plugins::{PluginAddress, RuntimeGeneration};

    use super::host::tests::{credential_shaped, node_vars};

    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.home()).unwrap();
    let script = String::from_utf8(helper_script(2, false))
        .unwrap()
        .replacen("#!/bin/sh\n", "#!/bin/sh\nenv >&2\n", 1);
    helper_plugin(&fixture, "envcheck", script.as_bytes());
    let directory = fixture.home().join("envcheck");
    let manifest = super::manifest::read(&directory, &semver::Version::new(0, 0, 0)).unwrap();
    let env = super::host::LaunchEnv::from_vars(&fixture.home(), node_vars("/usr/bin:/bin"));
    let logs = vessel_plugin::SharedLogs::default();
    let address = PluginAddress {
        node: NodeId::random(),
        plugin: id("envcheck"),
        generation: RuntimeGeneration::random(),
    };
    let started = super::host::start(
        &manifest,
        &directory,
        address,
        &logs,
        &env,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    let super::host::Started::Helper(helper) = started else {
        panic!("a server plugin starts a helper");
    };
    // Shutdown drains the helper's stderr into the tail before it returns.
    let _exit = helper.shutdown().await;
    let lines: Vec<String> = logs
        .snapshot()
        .unwrap()
        .iter()
        .filter(|entry| entry.stream == LogStream::Stderr)
        .map(|entry| entry.message.as_str().to_owned())
        .collect();
    assert!(
        lines.iter().any(|line| line == "PATH=/usr/bin:/bin"),
        "{lines:?}"
    );
    let home = format!("HOME={}", fixture.home().display());
    assert!(lines.contains(&home), "{lines:?}");
    for line in &lines {
        let name = line.split('=').next().unwrap_or_default();
        assert!(!credential_shaped(name), "{name} reached the helper");
        assert!(
            !line.contains("canary"),
            "a credential's value reached the helper"
        );
    }
}
