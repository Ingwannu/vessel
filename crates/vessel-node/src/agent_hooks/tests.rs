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
        r#"if [ -n "$VESSEL_TERMINAL_ACTIVITY_ID" ]; then "${VESSEL_HOOK_CLI:-vessel}" hooks "claude" "Stop"; fi"#
    );
    assert_eq!(
        windows_command(Agent::Codex, "Stop"),
        r#"if defined VESSEL_TERMINAL_ACTIVITY_ID (if defined VESSEL_HOOK_CLI ("%VESSEL_HOOK_CLI%" hooks "codex" "Stop") else (vessel hooks "codex" "Stop")) else (exit /b 0)"#
    );
    for agent in [Agent::Claude, Agent::Codex] {
        for event in agent.event_names() {
            for command in [shell_command(agent, event), windows_command(agent, event)] {
                assert!(marked(&command, agent), "{command}");
                // Paseo 0.15.4 drops every command containing its marker.
                for paseo_marker in ["hooks claude", "hooks codex"] {
                    assert!(!command.contains(paseo_marker), "{command}");
                }
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

/// A scratch home under `root` and its agents' homes.
fn scratch(root: &Path) -> (PathBuf, Targets) {
    let home = root.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let targets = Targets::of(&HarnessEnv::new(home.clone()));
    refuse_real_home(&targets);
    (home, targets)
}

fn read(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn commands(config: &serde_json::Value, event: &str) -> Vec<String> {
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
    // Again: nothing changes.
    let before = std::fs::read(&settings).unwrap();
    targets.apply(true);
    assert_eq!(std::fs::read(&settings).unwrap(), before);
    assert_eq!(commands(&read(&settings), "Stop").len(), 1);
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

/// Paseo 0.15.4's installer output over a person's file
/// (`tests/fixtures/agent_hooks/generate.mjs` runs Paseo's own installer):
/// `.json` as Paseo installs it, `.paseo-after-vessel.json` after Vessel's
/// entries were added and Paseo installed again over them, and
/// `.paseo-after-vessel.vessel-off.json` that file without Vessel's entries.
macro_rules! paseo_fixture {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/agent_hooks/paseo-0.15.4-",
            $name,
            ".json"
        ))
    };
}

/// Every value in `text` that is not Vessel's, in its order, as its text in
/// the file: the top-level keys but `hooks`, and each event's entries.
fn not_vessels(agent: Agent, text: &str) -> Vec<(String, String)> {
    let config: config_file::Object = serde_json::from_str(text).unwrap();
    let mut kept = Vec::new();
    for (key, value) in config.0 {
        if key != "hooks" {
            kept.push((key, value.get().to_owned()));
            continue;
        }
        let events: config_file::Object = serde_json::from_str(value.get()).unwrap();
        for (event, list) in events.0 {
            let entries: Vec<Box<serde_json::value::RawValue>> =
                serde_json::from_str(list.get()).unwrap();
            for entry in entries {
                let parsed: serde_json::Value = serde_json::from_str(entry.get()).unwrap();
                let vessels = parsed["hooks"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|hook| {
                        hook["command"]
                            .as_str()
                            .is_some_and(|command| marked(command, agent))
                    });
                if !vessels {
                    kept.push((format!("hooks.{event}"), entry.get().to_owned()));
                }
            }
        }
    }
    kept
}

/// The operator's machines run Paseo and Vessel side by side, and both
/// install hooks into the same files (the coordinator, 2026-11-20).
#[test]
fn paseo_entries_stay_byte_identical_and_in_place_through_install_reinstall_and_off() {
    let cases = [
        (
            Agent::Claude,
            [".claude", "settings.json"],
            paseo_fixture!("claude"),
            paseo_fixture!("claude.paseo-after-vessel"),
            paseo_fixture!("claude.paseo-after-vessel.vessel-off"),
        ),
        (
            Agent::Codex,
            [".codex", "hooks.json"],
            paseo_fixture!("codex"),
            paseo_fixture!("codex.paseo-after-vessel"),
            paseo_fixture!("codex.paseo-after-vessel.vessel-off"),
        ),
    ];
    for (agent, [folder, name], paseo, after_vessel, vessel_off) in cases {
        let temp = tempfile::tempdir().unwrap();
        let (home, targets) = scratch(temp.path());
        let path = home.join(folder).join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        assert!(paseo.contains("PASEO_HOOK_CLI") && !paseo.contains("VESSEL"));
        let others = not_vessels(agent, paseo);
        assert!(others.len() > 5, "{agent:?}: {others:?}");

        // Install: every value that is not Vessel's keeps its bytes and its
        // place; Vessel's entry is last in each of its events.
        std::fs::write(&path, paseo).unwrap();
        targets.apply(true);
        let installed = std::fs::read_to_string(&path).unwrap();
        assert_eq!(not_vessels(agent, &installed), others, "{agent:?}");
        let config = read(&path);
        for event in agent.event_names() {
            assert_eq!(
                commands(&config, event).last(),
                Some(&shell_command(agent, event)),
                "{agent:?} {event}"
            );
        }
        assert_eq!(std::fs::read_to_string(backup_of(&path)).unwrap(), paseo);

        // Reinstall writes nothing; off gives Paseo's file back byte for byte.
        targets.apply(true);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), installed);
        targets.apply(false);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), paseo, "{agent:?}");

        // Paseo installed again after Vessel: its entries now follow
        // Vessel's. Vessel's install moves nothing, and off takes out only
        // Vessel's entries.
        std::fs::write(&path, after_vessel).unwrap();
        targets.apply(true);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), after_vessel);
        targets.apply(false);
        let off = std::fs::read_to_string(&path).unwrap();
        assert_eq!(off, vessel_off, "{agent:?}");
        assert_eq!(not_vessels(agent, &off), not_vessels(agent, after_vessel));
    }
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
