//! `settings.json` set aside instead of lost (ADR 0038, 2026-11-14): an
//! unusable file is moved aside at start instead of being replaced by the
//! next change, and one damaged again before a later start does not
//! replace the first copy; a later build's file is copied before the first
//! change this build writes, and a file this build fully knows is not.

use std::path::PathBuf;

use vessel_proto::parking::ParkAfter;
use vessel_proto::settings::{GetNodeSettings, SetNodeSettings};

use super::{flag, idle};
use crate::support::{Fixture, within};

/// The copies of `kind` (`unreadable`, `later`) set aside in the data
/// directory, each with its bytes, by name.
fn set_aside(fixture: &Fixture, kind: &str) -> Vec<(PathBuf, Vec<u8>)> {
    let prefix = format!("settings.json.{kind}-");
    let mut copies: Vec<_> = std::fs::read_dir(fixture.data_dir().root())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
        })
        .map(|path| {
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    copies.sort();
    copies
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_damaged_file_is_set_aside_and_a_change_does_not_replace_it() {
    within(async {
        let fixture = Fixture::new();
        let settings = fixture.data_dir().settings_path();
        std::fs::write(&settings, b"{ damaged").unwrap();
        let node = fixture.start_with(flag).await;
        let owner = fixture.connect(&node).await;
        assert_eq!(
            owner
                .call(GetNodeSettings::default())
                .await
                .unwrap()
                .park_after,
            idle(30),
            "the flag's value"
        );
        assert!(!settings.exists(), "moved, not left to be replaced");
        let first = set_aside(&fixture, "unreadable");
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].1, b"{ damaged");

        // A change writes a new file; the copy stays as it was.
        let changed = owner
            .call(SetNodeSettings {
                park_after: Some(ParkAfter::Never),
                drivers: None,
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        assert_eq!(changed.park_after, ParkAfter::Never);
        assert!(settings.exists());
        assert_eq!(set_aside(&fixture, "unreadable"), first);
        node.stop().await.unwrap();

        // Damaged again before the next start: a second copy, the first kept.
        std::fs::write(&settings, b"{ damaged again").unwrap();
        let node = fixture.start_with(flag).await;
        let copies = set_aside(&fixture, "unreadable");
        assert_eq!(copies.len(), 2);
        assert!(copies.contains(&first[0]));
        assert!(copies.iter().any(|(_, bytes)| bytes == b"{ damaged again"));
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_later_builds_file_is_copied_before_the_first_change() {
    within(async {
        let fixture = Fixture::new();
        let settings = fixture.data_dir().settings_path();
        let later = br#"{"park_after":{"kind":"never"},"drivers":{"codex":"acp","gemini":"direct"},"dictation":{"on":true}}"#;
        std::fs::write(&settings, later).unwrap();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        assert!(set_aside(&fixture, "later").is_empty(), "only a change copies");
        let _in_force = owner
            .call(SetNodeSettings {
                park_after: Some(idle(5)),
                drivers: None,
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        let copies = set_aside(&fixture, "later");
        assert_eq!(copies.len(), 1);
        assert_eq!(copies[0].1, later);
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&settings).unwrap()).unwrap();
        assert_eq!(
            written,
            serde_json::json!({"park_after":{"kind":"idle","minutes":5},"drivers":{"codex":"acp"}})
        );
        // Copied once: a second change writes without another copy.
        let _in_force = owner
            .call(SetNodeSettings {
                park_after: Some(ParkAfter::Never),
                drivers: None,
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        assert_eq!(set_aside(&fixture, "later").len(), 1);
        node.stop().await.unwrap();

        // A file this build fully knows is never copied.
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let _in_force = owner
            .call(SetNodeSettings {
                park_after: Some(idle(7)),
                drivers: None,
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        assert_eq!(set_aside(&fixture, "later").len(), 1);
        node.stop().await.unwrap();
    })
    .await;
}
