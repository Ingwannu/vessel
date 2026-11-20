//! Settings documents against the testkit's `fake-plugin` helper (ADR 0115,
//! "Settings"): defaults, a save kept owner-only, a stale revision, a
//! refusal cut to its bound, a migration saved once, a newer version kept,
//! reset, and the cases with no helper to ask. Nothing is fetched.

use vessel_proto::RawJson;
use vessel_proto::plugins::{
    PluginGeneration, PluginSettingsRead, PluginSettingsWrite, PluginStatus, ResetPluginSettings,
    SettingsId, SettingsRevision, WritePluginSettings,
};

use super::super::tests::{Fixture, client_only, helper_plugin, id, until};
use super::super::{PluginError, Plugins};

fn display() -> SettingsId {
    "display".parse().unwrap()
}

fn json(text: &str) -> RawJson {
    RawJson::from_text(text).unwrap()
}

/// `name` running `fake-plugin` through a one-line `sh` wrapper (the
/// manifest's command is inside the plugin's folder).
fn fake_plugin(fixture: &Fixture, name: &str) {
    let fake = vessel_testkit::Fake::Plugin.path().unwrap();
    let script = format!("#!/bin/sh\nexec '{}'\n", fake.display());
    helper_plugin(fixture, name, script.as_bytes());
}

async fn running(fixture: &Fixture, name: &str) -> Plugins {
    let plugins = fixture.host(true).await;
    fake_plugin(fixture, name);
    let _item = plugins
        .install(&format!("~/{name}"), None, None)
        .await
        .unwrap();
    until(&plugins, name, PluginStatus::Running).await;
    plugins
}

fn write(plugin: &str, revision: SettingsRevision, values: &str) -> WritePluginSettings {
    WritePluginSettings {
        plugin: id(plugin),
        settings: display(),
        revision,
        values: json(values),
    }
}

/// The helper's `changed` notices so far.
fn notices(fixture: &Fixture, name: &str) -> Vec<serde_json::Value> {
    std::fs::read_to_string(fixture.home().join(name).join("notices.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[cfg(unix)]
#[tokio::test]
async fn a_missing_document_reads_its_defaults_and_a_save_is_kept_owner_only() {
    use std::os::unix::fs::PermissionsExt as _;

    let fixture = Fixture::new();
    let plugins = running(&fixture, "prefs").await;
    let read = plugins
        .read_settings(&id("prefs"), &display())
        .await
        .unwrap();
    assert_eq!(
        read,
        PluginSettingsRead::Ready {
            revision: SettingsRevision::missing(),
            values: json(r#"{"color":"blue","size":1}"#),
        }
    );
    let saved = plugins
        .write_settings(write("prefs", SettingsRevision::missing(), r#"{"size":3}"#))
        .await
        .unwrap();
    let PluginSettingsWrite::Saved { revision, values } = saved else {
        panic!("saved: {saved:?}");
    };
    assert_eq!(values, json(r#"{"color":"blue","size":3}"#));
    assert_ne!(revision, SettingsRevision::missing());
    let data = fixture.data();
    let file = std::fs::metadata(data.plugin_settings_path(&id("prefs"), &display())).unwrap();
    assert_eq!(file.permissions().mode() & 0o777, 0o600);
    let folder = std::fs::metadata(data.plugin_settings_dir(&id("prefs"))).unwrap();
    assert_eq!(folder.permissions().mode() & 0o777, 0o700);
    let stored: serde_json::Value = serde_json::from_slice(
        &data
            .read_plugin_settings(&id("prefs"), &display())
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        stored,
        serde_json::json!({"version": 2, "values": {"color": "blue", "size": 3}})
    );
    // The helper heard of the save before the reply (its `subscribe()`).
    let told = notices(&fixture, "prefs");
    assert!(
        told.iter().any(|notice| notice["state"] == "ready"
            && notice["revision"] == revision.as_str()
            && notice["values"]["size"] == 3),
        "{told:?}"
    );
    assert_eq!(
        plugins
            .read_settings(&id("prefs"), &display())
            .await
            .unwrap(),
        PluginSettingsRead::Ready { revision, values }
    );
    fixture.finish().await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_stale_revision_conflicts_and_a_refusal_reaches_the_caller_bounded() {
    let fixture = Fixture::new();
    let plugins = running(&fixture, "prefs").await;
    let saved = plugins
        .write_settings(write("prefs", SettingsRevision::missing(), "{}"))
        .await
        .unwrap();
    let PluginSettingsWrite::Saved { revision, .. } = saved else {
        panic!("saved: {saved:?}");
    };
    let stale = plugins
        .write_settings(write("prefs", SettingsRevision::missing(), "{}"))
        .await
        .unwrap();
    let PluginSettingsWrite::Conflict { error } = stale else {
        panic!("conflict: {stale:?}");
    };
    assert_eq!(
        error.as_str(),
        "Settings changed on another client. Reload before saving again."
    );
    let wrong = plugins
        .write_settings(write("prefs", revision.clone(), r#"{"size":"big"}"#))
        .await
        .unwrap();
    let PluginSettingsWrite::Invalid { error } = wrong else {
        panic!("invalid: {wrong:?}");
    };
    assert_eq!(error.as_str(), "size has the wrong type");
    let boom = plugins
        .write_settings(write("prefs", revision.clone(), r#"{"boom":true}"#))
        .await
        .unwrap();
    let PluginSettingsWrite::Invalid { error } = boom else {
        panic!("invalid: {boom:?}");
    };
    assert!(error.as_str().starts_with("boom  x"), "{error:?}");
    assert!(error.as_str().len() <= 1024);
    assert!(!error.as_str().chars().any(char::is_control));
    // Refused values were not kept.
    assert!(matches!(
        plugins.read_settings(&id("prefs"), &display()).await.unwrap(),
        PluginSettingsRead::Ready { revision: now, .. } if now == revision
    ));
    fixture.finish().await;
}

#[cfg(unix)]
#[tokio::test]
async fn an_older_document_migrates_once_and_a_newer_one_is_kept_until_a_reset() {
    let fixture = Fixture::new();
    let data = fixture.data();
    data.write_plugin_settings(
        &id("prefs"),
        &display(),
        br#"{"version":1,"values":{"colour":"red"}}"#,
    )
    .unwrap();
    let plugins = fixture.host(true).await;
    fake_plugin(&fixture, "prefs");
    let _item = plugins.install("~/prefs", None, None).await.unwrap();
    // The run's first delivery migrates the document and moves the watch.
    let mut seen = PluginGeneration::default();
    for _ in 0..40 {
        let stored = data
            .read_plugin_settings(&id("prefs"), &display())
            .unwrap()
            .unwrap();
        if stored.starts_with(br#"{"version":2"#) {
            break;
        }
        seen = plugins.watch(seen).await;
    }
    let read = plugins
        .read_settings(&id("prefs"), &display())
        .await
        .unwrap();
    let PluginSettingsRead::Ready { revision, values } = read else {
        panic!("ready: {read:?}");
    };
    assert_eq!(values, json(r#"{"color":"red","size":1}"#));
    // Delivered once, as migrated; the read after it did not migrate again.
    let told = notices(&fixture, "prefs");
    assert_eq!(told.len(), 1, "{told:?}");
    assert_eq!(told[0]["revision"], revision.as_str());

    let newer = br#"{"version":3,"values":{}}"#;
    data.write_plugin_settings(&id("prefs"), &display(), newer)
        .unwrap();
    let read = plugins
        .read_settings(&id("prefs"), &display())
        .await
        .unwrap();
    let PluginSettingsRead::Invalid { revision, error } = read else {
        panic!("invalid: {read:?}");
    };
    assert_eq!(
        error.as_str(),
        "Settings were saved by a newer plugin version"
    );
    let refused = plugins
        .write_settings(write("prefs", revision.clone(), "{}"))
        .await
        .unwrap();
    assert!(
        matches!(refused, PluginSettingsWrite::Invalid { .. }),
        "{refused:?}"
    );
    assert_eq!(
        data.read_plugin_settings(&id("prefs"), &display())
            .unwrap()
            .unwrap(),
        newer.to_vec()
    );
    let reset = plugins
        .reset_settings(ResetPluginSettings {
            plugin: id("prefs"),
            settings: display(),
            revision,
        })
        .await
        .unwrap();
    let PluginSettingsWrite::Saved { values, .. } = reset else {
        panic!("reset: {reset:?}");
    };
    assert_eq!(values, json(r#"{"color":"blue","size":1}"#));
    fixture.finish().await;
}

#[cfg(unix)]
#[tokio::test]
async fn settings_need_a_running_helper_that_declares_them_and_go_with_the_plugin() {
    let fixture = Fixture::new();
    let plugins = running(&fixture, "prefs").await;
    let other: SettingsId = "other".parse().unwrap();
    let undeclared = plugins.read_settings(&id("prefs"), &other).await;
    assert!(
        matches!(undeclared, Err(PluginError::NoSettings { .. })),
        "{undeclared:?}"
    );
    let missing = plugins.read_settings(&id("nothing"), &display()).await;
    assert!(
        matches!(missing, Err(PluginError::NotInstalled(_))),
        "{missing:?}"
    );
    let client_json = r#"{"methods":[],"contributions":[],"themes":[]}"#;
    let _directory = fixture.plugin(
        "night",
        &client_only("night"),
        &[("client.json", client_json.as_bytes())],
    );
    let _item = plugins.install("~/night", None, None).await.unwrap();
    let client = plugins.read_settings(&id("night"), &display()).await;
    assert!(
        matches!(client, Err(PluginError::NoSettings { .. })),
        "{client:?}"
    );
    let _saved = plugins
        .write_settings(write("prefs", SettingsRevision::missing(), "{}"))
        .await
        .unwrap();
    let _disabled = plugins.disable(&id("prefs")).await.unwrap();
    let stopped = plugins.read_settings(&id("prefs"), &display()).await;
    assert!(
        matches!(stopped, Err(PluginError::NotRunning(_))),
        "{stopped:?}"
    );
    // Kept while disabled; gone with the installation.
    let folder = fixture.data().plugin_settings_dir(&id("prefs"));
    assert!(folder.exists());
    plugins.remove(&id("prefs")).await.unwrap();
    assert!(!folder.exists());
    fixture.finish().await;
}
