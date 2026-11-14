//! The kept lists' file: owner-only, versioned, bounded, keyed by node.

use std::path::PathBuf;

use chrono::{Duration, TimeZone as _, Utc};
use vessel_proto::{
    HarnessKind, HarnessRef, NodeId, OptionCategory, OptionId, OptionKind, OptionValue,
    OptionValueId, ProfileName, SessionOption,
};

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vessel-app-offers-{}-{name}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    dir.join("state").join(FILE)
}

fn models(ids: &[&str]) -> Vec<SessionOption> {
    vec![SessionOption {
        id: OptionId::new("model"),
        name: "Model".to_owned(),
        description: None,
        category: OptionCategory::Model,
        kind: OptionKind::Select {
            current: OptionValueId::new(ids.first().copied().unwrap_or("default")),
            values: ids
                .iter()
                .map(|id| OptionValue {
                    id: OptionValueId::new(*id),
                    name: (*id).to_owned(),
                    description: Some(format!("{id}, as the agent describes it")),
                    group: None,
                })
                .collect(),
        },
    }]
}

fn claude() -> HarnessRef {
    HarnessRef::Known(HarnessKind::ClaudeCode)
}

fn at(minute: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 14, 4, 0, 0)
        .single()
        .unwrap()
        .checked_add_signed(Duration::minutes(minute))
        .unwrap()
}

#[test]
fn a_list_written_reads_back_for_its_node_only() {
    let path = scratch("roundtrip");
    let laptop = NodeId::random();
    let desk = NodeId::random();
    let mut store = Store::load(Some(path.clone()));
    assert!(store.keep(
        laptop,
        claude(),
        None,
        Said::Options(models(&["a", "b"])),
        at(0)
    ));
    assert!(store.keep(desk, claude(), None, Said::Options(models(&["c"])), at(1)));
    store.save().unwrap();
    let read = Store::load(Some(path));
    let on_laptop = read.for_node(&laptop);
    assert_eq!(on_laptop.len(), 1);
    assert_eq!(on_laptop[0].options, Some(models(&["a", "b"])));
    assert_eq!(read.for_node(&desk)[0].options, Some(models(&["c"])));
    assert!(read.for_node(&NodeId::random()).is_empty());
}

#[cfg(unix)]
#[test]
fn the_file_is_owner_only_in_an_owner_only_folder() {
    use std::os::unix::fs::PermissionsExt as _;

    let path = scratch("modes");
    let mut store = Store::load(Some(path.clone()));
    let _changed = store.keep(
        NodeId::random(),
        claude(),
        None,
        Said::Options(models(&["a"])),
        at(0),
    );
    store.save().unwrap();
    let mode =
        |path: &std::path::Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&path), 0o600, "the file");
    assert_eq!(mode(path.parent().unwrap()), 0o700, "its folder");
    // Written again (temporary file, then rename): still owner-only.
    let _changed = store.keep(
        NodeId::random(),
        claude(),
        None,
        Said::Options(models(&["b"])),
        at(1),
    );
    store.save().unwrap();
    assert_eq!(mode(&path), 0o600, "the file, rewritten");
    assert!(
        !path.with_extension("json.tmp").exists(),
        "no temporary left"
    );
}

#[test]
fn the_same_list_is_not_a_change_and_a_different_one_is() {
    let node = NodeId::random();
    let mut store = Store::load(None);
    assert!(store.keep(node, claude(), None, Said::Options(models(&["a"])), at(0)));
    assert!(!store.keep(node, claude(), None, Said::Options(models(&["a"])), at(5)));
    assert_eq!(
        store.for_node(&node)[0].at,
        at(0),
        "the same list keeps its time"
    );
    assert!(store.keep(
        node,
        claude(),
        None,
        Said::Options(models(&["a", "b"])),
        at(6)
    ));
    assert_eq!(store.for_node(&node).len(), 1, "replaced, not added");
    assert_eq!(store.for_node(&node)[0].at, at(6));
}

#[test]
fn agents_and_profiles_are_kept_apart() {
    let node = NodeId::random();
    let work: ProfileName = "work".parse().unwrap();
    let mut store = Store::load(None);
    assert!(store.keep(node, claude(), None, Said::Options(models(&["a"])), at(0)));
    assert!(store.keep(
        node,
        claude(),
        Some(work.clone()),
        Said::Options(models(&["w"])),
        at(1)
    ));
    assert!(store.keep(
        node,
        HarnessRef::Known(HarnessKind::Codex),
        None,
        Said::Options(models(&["x"])),
        at(2)
    ));
    let kept = store.for_node(&node);
    assert_eq!(kept.len(), 3);
    assert_eq!(
        kept.iter()
            .find(|kept| kept.profile.as_ref() == Some(&work))
            .and_then(|kept| kept.options.clone()),
        Some(models(&["w"]))
    );
}

#[test]
fn a_file_of_another_version_or_that_does_not_read_is_dropped() {
    let path = scratch("version");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let node = NodeId::random();
    let newer = serde_json::json!({
        "version": VERSION.checked_add(1).unwrap(),
        "entries": [{
            "node": node.to_string(),
            "agent": "claude-code",
            "at": "2026-11-14T04:00:00Z",
            "options": []
        }]
    });
    std::fs::write(&path, newer.to_string()).unwrap();
    assert!(Store::load(Some(path.clone())).for_node(&node).is_empty());
    std::fs::write(&path, b"{ not json").unwrap();
    assert!(Store::load(Some(path.clone())).for_node(&node).is_empty());
    // The next write replaces it with this version's.
    let mut store = Store::load(Some(path.clone()));
    let _changed = store.keep(node, claude(), None, Said::Options(models(&["a"])), at(0));
    store.save().unwrap();
    assert_eq!(Store::load(Some(path)).for_node(&node).len(), 1);
}

#[test]
fn at_most_max_entries_are_kept_the_oldest_going_first() {
    let mut store = Store::load(None);
    let nodes: Vec<NodeId> = (0..=MAX_ENTRIES).map(|_| NodeId::random()).collect();
    for (minute, node) in (0_i64..).zip(&nodes) {
        let _changed = store.keep(
            *node,
            claude(),
            None,
            Said::Options(models(&["a"])),
            at(minute),
        );
    }
    assert_eq!(store.entries.len(), MAX_ENTRIES);
    assert!(store.for_node(&nodes[0]).is_empty(), "the oldest went");
    assert_eq!(
        store.for_node(&nodes[MAX_ENTRIES]).len(),
        1,
        "the newest stayed"
    );
}

#[test]
fn the_file_stays_within_max_bytes() {
    let mut store = Store::load(None);
    // About 190 KiB a list (2,000 values of about 95 bytes): six of them
    // are over the bound, eleven far over it.
    let many: Vec<String> = (0..2_000)
        .map(|index| format!("model-{index:04}"))
        .collect();
    let ids: Vec<&str> = many.iter().map(String::as_str).collect();
    let nodes: Vec<NodeId> = (0..11).map(|_| NodeId::random()).collect();
    for (minute, node) in (0_i64..).zip(&nodes) {
        let _changed = store.keep(
            *node,
            claude(),
            None,
            Said::Options(models(&ids)),
            at(minute),
        );
    }
    let bytes = store.encoded().unwrap().len();
    assert!(bytes <= MAX_BYTES, "{bytes} bytes");
    assert!(store.entries.len() < nodes.len(), "some went");
    assert!(store.for_node(&nodes[0]).is_empty(), "the oldest went");
    assert_eq!(store.for_node(&nodes[10]).len(), 1, "the newest stayed");
}

#[test]
fn the_file_holds_only_what_the_agents_said_and_whose_it_was() {
    let path = scratch("fields");
    let mut store = Store::load(Some(path.clone()));
    let _changed = store.keep(
        NodeId::random(),
        claude(),
        None,
        Said::Options(models(&["a"])),
        at(0),
    );
    store.save().unwrap();
    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let keys = |value: &serde_json::Value| {
        let mut keys: Vec<String> = value
            .as_object()
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default();
        keys.sort();
        keys
    };
    assert_eq!(keys(&written), ["entries", "version"]);
    assert_eq!(
        keys(&written["entries"][0]),
        ["agent", "at", "node", "options"]
    );
    assert_eq!(store.path(), Some(path.as_path()));
}

#[test]
fn without_a_state_folder_nothing_is_written() {
    let mut store = Store::load(None);
    let _changed = store.keep(
        NodeId::random(),
        claude(),
        None,
        Said::Options(models(&["a"])),
        at(0),
    );
    assert!(store.save().is_ok());
}

#[test]
fn commands_are_kept_beside_options_and_neither_undoes_the_other() {
    let path = scratch("commands");
    let node = NodeId::random();
    let mut store = Store::load(Some(path.clone()));
    let commands = vec![AgentCommand {
        name: "review".parse().unwrap(),
        description: "Review the change".to_owned(),
        hint: None,
    }];
    // Commands first (a session reported them before any options).
    assert!(store.keep(
        node,
        claude(),
        None,
        Said::Commands(commands.clone()),
        at(0)
    ));
    assert_eq!(
        store.for_node(&node)[0].options,
        None,
        "options not known yet"
    );
    assert!(store.keep(node, claude(), None, Said::Options(models(&["a"])), at(1)));
    assert!(!store.keep(
        node,
        claude(),
        None,
        Said::Commands(commands.clone()),
        at(2)
    ));
    store.save().unwrap();
    let read = Store::load(Some(path));
    let kept = read.for_node(&node);
    assert_eq!(kept.len(), 1, "one entry for the agent");
    assert_eq!(kept[0].options, Some(models(&["a"])));
    assert_eq!(kept[0].commands, Some(commands));
}
