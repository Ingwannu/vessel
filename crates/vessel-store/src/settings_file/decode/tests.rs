use std::collections::BTreeMap;
use std::error::Error as _;

use super::*;
use crate::{DataDir, SettingsFileError};

const MARKER: &str = "settings-private-marker-73e0199c";

#[derive(Debug, serde::Deserialize)]
struct Tree(BTreeMap<String, Tree>);

fn safe_chain(error: &(dyn std::error::Error + 'static)) {
    let mut current = Some(error);
    while let Some(error) = current {
        assert!(!error.to_string().contains(MARKER));
        assert!(!format!("{error:?}").contains(MARKER));
        current = error.source();
    }
}

#[test]
fn invalid_block_keeps_the_cause_private_across_all_public_diagnostics() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    let bytes = format!(r#"{{"drivers":"{MARKER}"}}"#);
    std::fs::write(data.settings_path(), &bytes).unwrap();
    let error = data.read_settings().unwrap_err();
    safe_chain(&error);
    assert!(error.to_string().contains("invalid field at $.<field>"));
    assert!(error.to_string().contains("settings.json"));
    let SettingsFileError::Parse { source, .. } = error else {
        panic!("expected a parse refusal");
    };
    assert!(source.cause.to_string().contains(MARKER));
    assert!(source.source().is_none());
    assert_eq!(
        std::fs::read(data.settings_path()).unwrap(),
        bytes.as_bytes()
    );
}

#[test]
fn input_keys_are_masked_and_list_positions_remain_useful() {
    let bytes = format!(r#"{{"{MARKER}":["{MARKER}"]}}"#);
    let error = settings::<BTreeMap<String, Vec<u32>>>(bytes.as_bytes()).unwrap_err();
    safe_chain(&error);
    assert_eq!(error.to_string(), "invalid field at $.<field>[0]");
    assert!(error.cause.to_string().contains(MARKER));
}

#[test]
fn one_complete_json_value_is_still_required() {
    for bytes in [b"{".as_slice(), b"{} trailing", b"{} {}", b"[}"] {
        let error = settings::<BTreeMap<String, u32>>(bytes).unwrap_err();
        safe_chain(&error);
    }
    assert!(
        settings::<BTreeMap<String, u32>>(b"{} \n\t")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn deeply_nested_paths_are_bounded_without_printing_map_keys() {
    let depth = MAX_PATH_SEGMENTS.saturating_add(1);
    let prefix = format!(r#"{{"{MARKER}":"#).repeat(depth);
    let input = format!("{prefix}true{}", "}".repeat(depth));
    let error = settings::<Tree>(input.as_bytes()).unwrap_err();
    safe_chain(&error);
    assert_eq!(error.path.0.matches(".<field>").count(), MAX_PATH_SEGMENTS);
    assert!(error.path.0.ends_with(".…"));
    assert!(settings::<Tree>(b"{}").unwrap().0.is_empty());
}
