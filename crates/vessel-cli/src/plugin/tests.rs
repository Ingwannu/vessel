use super::*;
use crate::args::{Cli, Command, plugin::PluginCommand};
use clap::Parser as _;
use std::fs;
use vessel_proto::plugins::ClientDocument;

#[test]
fn scaffold_is_valid_static_data_and_cannot_replace_an_existing_project() {
    let root = tempfile::tempdir().expect("isolated root");
    let directory = root.path().join("sample");
    let created = scaffold::create(&directory, None).expect("create scaffold");
    let client = fs::read(directory.join("client.json")).expect("client");
    let document = ClientDocument::decode(&client).expect("canonical decoder");
    assert!(document.methods().is_empty());
    assert_eq!(document.contributions().len(), 1);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("vessel-plugin.json")).expect("manifest"))
            .expect("manifest JSON");
    assert_eq!(manifest["id"], "sample");
    assert_eq!(manifest["format"], "vessel.plugin/v1");
    assert!(manifest.get("helpers").is_none());
    assert!(matches!(
        scaffold::create(&directory, None),
        Err(InitError::NotEmpty)
    ));
    assert_eq!(
        fs::read(directory.join("client.json")).expect("retained"),
        client
    );
    let row =
        crate::parity::output::render_list(crate::args::parity::Presentation::Quiet, &[created])
            .expect("shared formatter");
    assert_eq!(row, b"sample\n");
}

#[test]
fn invalid_default_identity_has_no_effect_and_an_explicit_id_accepts_an_empty_directory() {
    let root = tempfile::tempdir().expect("root");
    let directory = root.path().join("INVALID-PRIVATE-CANARY");
    let error = scaffold::create(&directory, None).expect_err("invalid basename");
    assert!(!format!("{error:?} {error}").contains("PRIVATE-CANARY"));
    assert!(!directory.exists());
    fs::create_dir(&directory).expect("existing empty");
    scaffold::create(&directory, Some("sample".parse().expect("id")))
        .expect("explicit id and empty target");
}

#[cfg(unix)]
#[test]
fn private_modes_and_owner_are_retained_and_symlink_targets_are_not_followed() {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _, symlink};
    let root = tempfile::tempdir().expect("root");
    let directory = root.path().join("sample");
    scaffold::create(&directory, None).expect("private directory");
    let parent = fs::metadata(root.path()).expect("owner");
    let metadata = fs::metadata(&directory).expect("directory metadata");
    assert_eq!(metadata.mode() & 0o777, 0o700);
    assert_eq!(metadata.uid(), parent.uid());
    for name in [
        "vessel-plugin.json",
        "client.json",
        "README.md",
        "HELPER.md",
    ] {
        let metadata = fs::metadata(directory.join(name)).expect("file metadata");
        assert_eq!(metadata.mode() & 0o777, 0o600);
        assert_eq!(metadata.uid(), parent.uid());
    }
    let existing = root.path().join("existing");
    fs::create_dir(&existing).expect("empty directory");
    fs::set_permissions(&existing, fs::Permissions::from_mode(0o750)).expect("existing mode");
    scaffold::create(&existing, None).expect("preserve directory mode");
    assert_eq!(
        fs::metadata(&existing).expect("metadata").mode() & 0o777,
        0o750
    );
    let outside = tempfile::tempdir().expect("unrelated empty target");
    let link = root.path().join("linked");
    symlink(outside.path(), &link).expect("symlink fixture");
    assert!(matches!(
        scaffold::create(&link, None),
        Err(InitError::NotEmpty)
    ));
    assert_eq!(
        fs::read_dir(outside.path())
            .expect("outside unchanged")
            .count(),
        0
    );
}

#[test]
fn root_parses_only_the_implemented_local_leaf_and_admits_its_output_modes() {
    for output in [
        vec![],
        vec!["--json"],
        vec!["--quiet"],
        vec!["--no-headers"],
        vec!["-o", "yaml"],
    ] {
        let mut args = vec!["vessel", "--node", "ws://[", "plugin", "init", "sample"];
        args.extend(output);
        let cli = Cli::try_parse_from(args).expect("root parser");
        assert!(crate::presentation::check(&cli).is_ok());
        assert!(matches!(
            cli.command,
            Command::Plugin {
                command: PluginCommand::Init { .. }
            }
        ));
    }
    for leaf in ["install", "logs", "enable", "remove"] {
        assert!(Cli::try_parse_from(["vessel", "plugin", leaf, "sample", "--json"]).is_err());
    }
}
