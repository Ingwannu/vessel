//! The event maps against Paseo's, and the installer in a scratch home only:
//! every write goes through `files`, whose test build refuses the real home.

use std::path::{Path, PathBuf};

use vessel_harness::HarnessEnv;
use vessel_proto::notifications::TerminalActivitySignal::{Idle, NeedsInput, Working};

use super::files::{backup_of, refuse_real, refuse_real_home};
use super::*;

#[test]
fn every_event_maps_as_paseo_maps_it() {
    let table = [
        ("claude", "UserPromptSubmit", Some(Working)),
        ("claude", "Stop", Some(Idle)),
        ("claude", "StopFailure", Some(Idle)),
        ("claude", "SessionEnd", Some(Idle)),
        ("codex", "UserPromptSubmit", Some(Working)),
        ("codex", "PreToolUse", Some(Working)),
        ("codex", "PostToolUse", Some(Working)),
        ("codex", "PermissionRequest", Some(NeedsInput)),
        ("codex", "Stop", Some(Idle)),
        ("opencode", "session.status.busy", Some(Working)),
        ("opencode", "session.status.retry", Some(Working)),
        ("opencode", "session.status.idle", Some(Idle)),
        ("opencode", "permission.asked", Some(NeedsInput)),
        ("opencode", "permission.replied", Some(Working)),
    ];
    for (agent, event, signal) in table {
        let parsed = Event::parse(agent, event).unwrap();
        assert!(!parsed.reads_input(), "{agent} {event}");
        assert_eq!(parsed.signal(None), signal, "{agent} {event}");
    }
    assert_eq!(
        Event::parse("CLAUDE", "Stop").unwrap().agent(),
        Agent::Claude
    );
    assert!(
        Event::parse("claude", "stop").is_none(),
        "events match exactly"
    );
    assert!(Event::parse("gemini", "Stop").is_none());
    assert!(Event::parse("codex", "Notification").is_none());
}

#[test]
fn claude_notification_needs_input_only_for_its_idle_prompt() {
    let notification = Event::parse("claude", "Notification").unwrap();
    assert!(notification.reads_input());
    assert_eq!(
        notification.signal(Some(
            r#"{"notification_type":"idle_prompt","message":"waiting"}"#
        )),
        Some(NeedsInput)
    );
    for other in [
        Some(r#"{"notification_type":"permission_prompt"}"#),
        Some("not json"),
        Some("[]"),
        None,
    ] {
        assert_eq!(notification.signal(other), None, "{other:?}");
    }
}

#[test]
fn commands_run_only_in_a_vessel_terminal_and_are_recognised_exactly() {
    assert_eq!(
        shell_command(Agent::Claude, "Stop"),
        r#"if [ -n "$VESSEL_TERMINAL_ACTIVITY_ID" ]; then "${VESSEL_HOOK_CLI:-vessel}" hooks "claude" "Stop" 2>/dev/null || true; fi"#
    );
    assert_eq!(
        windows_command(Agent::Codex, "Stop"),
        r#"if defined VESSEL_TERMINAL_ACTIVITY_ID (if defined VESSEL_HOOK_CLI ("%VESSEL_HOOK_CLI%" hooks "codex" "Stop" 2>NUL & exit /b 0) else (vessel hooks "codex" "Stop" 2>NUL & exit /b 0)) else (exit /b 0)"#
    );
    for agent in [Agent::Claude, Agent::Codex] {
        for event in agent.event_names() {
            for command in [shell_command(agent, event), windows_command(agent, event)] {
                assert!(marked(&command, agent), "{command}");
            }
        }
    }
    assert!(!marked(&shell_command(Agent::Claude, "Stop"), Agent::Codex));
    // Paseo's own entries in the same file are not Vessel's.
    let paseo =
        r#"if [ -n "$PASEO_TERMINAL_ID" ]; then "${PASEO_HOOK_CLI:-paseo}" hooks claude Stop; fi"#;
    assert!(!marked(paseo, Agent::Claude));
    // Nor is a person's command that only resembles Vessel's.
    for near in [
        format!("{}; notify-send done", shell_command(Agent::Claude, "Stop")),
        shell_command(Agent::Claude, "Stop").replace("\"Stop\"", "\"PreCompact\""),
        r#""${VESSEL_HOOK_CLI:-vessel}" hooks "claude" "Stop""#.to_owned(),
    ] {
        assert!(!marked(&near, Agent::Claude), "{near}");
    }
}

/// A hook never fails the agent: whatever `vessel` does, the command
/// returns 0 and keeps `vessel`'s standard error out of the agent's view
/// (node review R4). The `cmd.exe` form is checked on Windows by hand.
#[cfg(unix)]
#[test]
fn a_missing_or_older_vessel_still_returns_zero() {
    let temp = tempfile::tempdir().unwrap();
    let older = temp.path().join("vessel-older");
    vessel_testkit::write_executable(
        &older,
        "#!/bin/sh\necho 'unrecognized subcommand' >&2\nexit 2\n",
    )
    .unwrap();
    let absent = temp.path().join("absent");
    let command = shell_command(Agent::Claude, "Stop");
    for cli in [older.as_os_str(), absent.as_os_str()] {
        let ran = std::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .env(vessel_proto::notifications::TERMINAL_ACTIVITY_ID_ENV, "t")
            .env(HOOK_CLI_ENV, cli)
            .output()
            .unwrap();
        assert!(ran.status.success(), "{cli:?}: {ran:?}");
        assert!(ran.stderr.is_empty(), "{cli:?}: {ran:?}");
    }
    // Outside a Vessel terminal nothing runs at all.
    let outside = std::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .env_remove(vessel_proto::notifications::TERMINAL_ACTIVITY_ID_ENV)
        .env(HOOK_CLI_ENV, older.as_os_str())
        .output()
        .unwrap();
    assert!(outside.status.success());
}

/// A scratch home under `root` and its agents' homes.
pub(super) fn scratch(root: &Path) -> (PathBuf, Targets) {
    let home = root.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let targets = Targets::of(&HarnessEnv::new(home.clone()));
    refuse_real_home(&targets);
    (home, targets)
}

pub(super) fn read(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

pub(super) fn commands(config: &serde_json::Value, event: &str) -> Vec<String> {
    config["hooks"][event]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|entry| entry["hooks"].as_array().cloned().unwrap_or_default())
        .filter_map(|hook| hook["command"].as_str().map(ToOwned::to_owned))
        .collect()
}

#[test]
fn targets_are_the_agents_homes_as_vessel_runs_them() {
    let temp = tempfile::tempdir().unwrap();
    let (home, targets) = scratch(temp.path());
    assert_eq!(targets.home(Agent::Claude), Some(&home.join(".claude")));
    assert_eq!(targets.home(Agent::Codex), Some(&home.join(".codex")));
    assert_eq!(
        targets.home(Agent::OpenCode),
        Some(&home.join(".config").join("opencode"))
    );
}

#[test]
fn installing_creates_each_agents_hooks_once() {
    let temp = tempfile::tempdir().unwrap();
    let (home, targets) = scratch(temp.path());
    targets.apply(true);
    let settings = home.join(".claude").join("settings.json");
    let config = read(&settings);
    for event in Agent::Claude.event_names() {
        assert_eq!(
            commands(&config, event),
            vec![shell_command(Agent::Claude, event)]
        );
    }
    assert_eq!(config["hooks"]["Stop"][0]["hooks"][0]["timeout"], 10);
    assert_eq!(config["hooks"]["Stop"][0]["matcher"], "");
    let codex = read(&home.join(".codex").join("hooks.json"));
    assert_eq!(
        codex["hooks"]["PermissionRequest"][0]["hooks"][0]["commandWindows"],
        windows_command(Agent::Codex, "PermissionRequest")
    );
    let plugin = home
        .join(".config")
        .join("opencode")
        .join("plugins")
        .join("vessel-terminal-activity.js");
    assert_eq!(
        std::fs::read_to_string(&plugin).unwrap(),
        plugin_file::source()
    );
    assert!(
        !backup_of(&settings).exists(),
        "nothing existed, nothing to keep"
    );
    // Again: nothing changes, and still nothing is kept.
    let before = std::fs::read(&settings).unwrap();
    targets.apply(true);
    assert_eq!(std::fs::read(&settings).unwrap(), before);
    assert_eq!(commands(&read(&settings), "Stop").len(), 1);
    assert!(!backup_of(&settings).exists());
}

#[test]
fn a_persons_file_keeps_its_order_its_entries_and_a_first_backup() {
    let temp = tempfile::tempdir().unwrap();
    let (home, targets) = scratch(temp.path());
    let settings = home.join(".claude").join("settings.json");
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    let paseo = r#"if [ -n \"$PASEO_TERMINAL_ID\" ]; then \"${PASEO_HOOK_CLI:-paseo}\" hooks claude Stop; fi"#;
    let original = format!(
        r#"{{
  "model": "opus",
  "hooks": {{
    "Stop": [
      {{ "matcher": "", "hooks": [{{ "type": "command", "command": "notify-send done" }}] }},
      {{ "matcher": "", "hooks": [{{ "type": "command", "command": "{paseo}" }}] }}
    ],
    "PreCompact": [{{ "matcher": "", "hooks": [{{ "type": "command", "command": "mine" }}] }}]
  }},
  "permissions": {{ "allow": ["Bash(ls)"] }}
}}
"#
    );
    std::fs::write(&settings, &original).unwrap();
    targets.apply(true);
    assert_eq!(
        std::fs::read_to_string(backup_of(&settings)).unwrap(),
        original
    );
    let installed = std::fs::read_to_string(&settings).unwrap();
    let order: Vec<usize> = ["\"model\"", "\"hooks\"", "\"permissions\""]
        .iter()
        .map(|key| installed.find(key).unwrap())
        .collect();
    assert!(order.is_sorted(), "the file's own key order: {installed}");
    let config = read(&settings);
    assert_eq!(config["model"], "opus");
    assert_eq!(config["permissions"]["allow"][0], "Bash(ls)");
    assert_eq!(commands(&config, "PreCompact"), vec!["mine".to_owned()]);
    let stop = commands(&config, "Stop");
    assert_eq!(stop.len(), 3, "{stop:?}");
    assert_eq!(stop[0], "notify-send done");
    assert!(stop[1].contains("PASEO_HOOK_CLI"), "Paseo's entry stays");
    assert_eq!(stop[2], shell_command(Agent::Claude, "Stop"));

    // Off: exactly the marked entries go; the first backup is kept.
    targets.apply(false);
    let config = read(&settings);
    assert_eq!(commands(&config, "Stop").len(), 2);
    assert!(config["hooks"].get("UserPromptSubmit").is_none());
    assert_eq!(commands(&config, "PreCompact"), vec!["mine".to_owned()]);
    assert_eq!(
        std::fs::read_to_string(backup_of(&settings)).unwrap(),
        original
    );
    assert_eq!(
        read(&settings),
        serde_json::from_str::<serde_json::Value>(&original).unwrap(),
        "on then off gives the person's file back"
    );
}

#[test]
fn an_entry_shared_with_a_persons_command_keeps_that_command() {
    let temp = tempfile::tempdir().unwrap();
    let (home, targets) = scratch(temp.path());
    let settings = home.join(".claude").join("settings.json");
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    let ours = shell_command(Agent::Claude, "Stop").replace('"', "\\\"");
    std::fs::write(
        &settings,
        format!(
            r#"{{"hooks":{{"Stop":[{{"matcher":"","hooks":[{{"type":"command","command":"mine"}},{{"type":"command","command":"{ours}"}}]}}]}}}}"#
        ),
    )
    .unwrap();
    targets.apply(false);
    assert_eq!(commands(&read(&settings), "Stop"), vec!["mine".to_owned()]);
}

#[test]
fn a_file_vessel_cannot_read_is_left_as_it_is() {
    let temp = tempfile::tempdir().unwrap();
    let (home, targets) = scratch(temp.path());
    let settings = home.join(".claude").join("settings.json");
    let codex = home.join(".codex").join("hooks.json");
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    std::fs::create_dir_all(codex.parent().unwrap()).unwrap();
    let cases = [
        (&settings, "{ not json"),
        (&codex, r#"{"hooks": ["a list, not an object"]}"#),
    ];
    for (path, text) in cases {
        std::fs::write(path, text).unwrap();
    }
    for on in [true, false] {
        targets.apply(on);
        for (path, text) in cases {
            assert_eq!(std::fs::read_to_string(path).unwrap(), text);
            assert!(!backup_of(path).exists());
        }
    }
}

#[test]
fn only_vessels_own_plugin_file_is_installed_over_or_removed() {
    let temp = tempfile::tempdir().unwrap();
    let (home, targets) = scratch(temp.path());
    let plugin = home
        .join(".config")
        .join("opencode")
        .join("plugins")
        .join("vessel-terminal-activity.js");
    targets.apply(true);
    assert!(plugin.exists());
    targets.apply(false);
    assert!(!plugin.exists());
    assert!(
        !backup_of(&plugin).exists(),
        "Vessel's own file leaves no copy behind"
    );
    // A file of someone else's at the same path stays, on and off.
    std::fs::write(&plugin, "export default {};\n").unwrap();
    for on in [true, false] {
        targets.apply(on);
        assert_eq!(
            std::fs::read_to_string(&plugin).unwrap(),
            "export default {};\n"
        );
        assert!(!backup_of(&plugin).exists());
    }
    // Nothing installed and off: no file is created.
    let other = tempfile::tempdir().unwrap();
    let (fresh, untouched) = scratch(other.path());
    untouched.apply(false);
    assert!(!fresh.join(".claude").join("settings.json").exists());
    assert!(!fresh.join(".codex").join("hooks.json").exists());
    assert!(!fresh.join(".config").join("opencode").exists());
}

/// The guard every write goes through refuses the real home's agent
/// configuration (the coordinator's condition, 2026-11-20).
#[test]
fn the_guard_refuses_the_real_agent_configuration() {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    for real in [
        home.join(".claude").join("settings.json"),
        home.join(".claude-alt").join("settings.json"),
        home.join(".codex").join("hooks.json"),
        home.join(".config")
            .join("opencode")
            .join("plugins")
            .join("x.js"),
    ] {
        let refused = std::panic::catch_unwind(|| refuse_real(&real));
        assert!(refused.is_err(), "{} was not refused", real.display());
    }
    let temp = tempfile::tempdir().unwrap();
    refuse_real(&temp.path().join(".claude").join("settings.json"));
}
