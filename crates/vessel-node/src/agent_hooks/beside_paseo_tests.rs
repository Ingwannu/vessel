//! Beside Paseo: Paseo 0.15.4 and Vessel install hooks into the same files
//! on the operator's machines (the coordinator, 2026-11-20). Vessel leaves
//! every Paseo entry as it is, and Paseo never takes Vessel's for its own.

use super::files::backup_of;
use super::tests::{commands, read, scratch};
use super::*;

/// Paseo 0.15.4 drops every hook whose `command` (or, for Codex,
/// `commandWindows`) includes its marker, on each install and uninstall:
/// `hooks claude` (`claude/claude.ts:25`, `claude-settings.ts:111-113`) and
/// `hooks codex` (`codex/codex.ts:26`, `codex-settings.ts:124-139`). No
/// command Vessel writes may include either (node review R1).
#[test]
fn paseos_markers_match_no_command_vessel_writes() {
    let paseo_includes = |command: &str, marker: &str| command.contains(marker);
    for agent in [Agent::Claude, Agent::Codex] {
        for event in agent.event_names() {
            for command in [shell_command(agent, event), windows_command(agent, event)] {
                for marker in ["hooks claude", "hooks codex"] {
                    assert!(!paseo_includes(&command, marker), "{marker} in {command}");
                }
            }
        }
    }
    // The form before ff18ac3b6 did include it: the check is not vacuous.
    let unquoted = shell_command(Agent::Claude, "Stop").replace('"', "");
    assert!(paseo_includes(&unquoted, "hooks claude"));
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
