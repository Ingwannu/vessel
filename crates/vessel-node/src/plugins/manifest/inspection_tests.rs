use super::*;
use std::fs;

const MANIFEST: &str = r#"{"format":"vessel.plugin/v1","id":"sample","requirements":{"node":23,"client":23},"client":"client.json"}"#;
const DOCUMENT: &str = r#"{"methods":[],"contributions":[],"themes":[]}"#;
const CURRENT: ProtocolVersion = ProtocolVersion::new(23);

#[tokio::test]
async fn static_declarations_cannot_advertise_callbacks_without_a_helper() {
    let (directory, root) = package();
    let callbacks = DOCUMENT.replace("\"methods\":[]", "\"methods\":[\"callback\"]");
    fs::write(directory.path().join("client.json"), &callbacks).expect("callback declarations");
    assert!(matches!(
        Inspection::read(&root, CURRENT, CURRENT).await,
        Err(InspectError::CallbackWithoutHelper)
    ));
    let helper = MANIFEST.replace("\"client\":\"client.json\"", "\"client\":\"client.json\",\"helpers\":[{\"target\":\"x86_64-unknown-linux-gnu\",\"protocol\":1,\"program\":\"helper\"}]");
    fs::write(directory.path().join(MANIFEST_FILE), helper).expect("declared helper");
    let inspected = Inspection::read(&root, CURRENT, CURRENT)
        .await
        .expect("inspection only");
    assert_eq!(inspected.document.expect("document").methods().len(), 1);
    // No program exists: successful declaration inspection grants no launch or
    // actual callback-registration capability. Runtime Ready owns that later.
    assert!(!directory.path().join("helper").exists());
}

fn package() -> (tempfile::TempDir, Root) {
    let directory = tempfile::tempdir().expect("isolated package");
    fs::write(directory.path().join(MANIFEST_FILE), MANIFEST).expect("manifest");
    fs::write(directory.path().join("client.json"), DOCUMENT).expect("declarations");
    let root = Root::open(directory.path()).expect("root");
    (directory, root)
}

#[tokio::test]
async fn disk_inspection_decodes_whole_documents_and_rechecks_exact_bytes() {
    let (directory, root) = package();
    let inspected = Inspection::read(&root, CURRENT, CURRENT)
        .await
        .expect("inspect");
    assert_eq!(inspected.manifest.0.id.as_str(), "sample");
    assert!(
        inspected
            .document
            .as_ref()
            .expect("client")
            .themes()
            .is_empty()
    );
    assert!(inspected.identity.client.is_some());
    inspected
        .recheck(&root, CURRENT, CURRENT)
        .await
        .expect("unchanged");
    // Valid equivalent JSON still differs: recheck binds bytes, not mtime or a
    // lossy parsed representation. No helper/build is run by either read.
    fs::write(directory.path().join("client.json"), format!(" {DOCUMENT}")).expect("edit");
    assert!(matches!(
        inspected.recheck(&root, CURRENT, CURRENT).await,
        Err(InspectError::Changed)
    ));
}

#[tokio::test]
async fn manifest_edits_and_incompatible_requirements_never_reuse_inspection() {
    let (directory, root) = package();
    let inspected = Inspection::read(&root, CURRENT, CURRENT)
        .await
        .expect("inspect");
    fs::write(
        directory.path().join(MANIFEST_FILE),
        MANIFEST.replace("sample", "changed"),
    )
    .expect("manifest edit");
    assert!(matches!(
        inspected.recheck(&root, CURRENT, CURRENT).await,
        Err(InspectError::Changed)
    ));
    assert!(matches!(
        Inspection::read(&root, ProtocolVersion::new(22), CURRENT).await,
        Err(InspectError::Manifest(ManifestError::Incompatible))
    ));
}

#[tokio::test]
async fn incomplete_binary_oversized_and_invalid_graph_files_refuse_atomically() {
    let (directory, root) = package();
    for bytes in [
        vec![0, 1],
        vec![0xff],
        b"{\"contributions\":false}".to_vec(),
    ] {
        fs::write(directory.path().join("client.json"), bytes).expect("bad client");
        assert!(Inspection::read(&root, CURRENT, CURRENT).await.is_err());
    }
    fs::write(
        directory.path().join("client.json"),
        vec![b' '; MAX_DOCUMENT_BYTES.saturating_add(1)],
    )
    .expect("oversized client");
    assert!(matches!(
        Inspection::read(&root, CURRENT, CURRENT).await,
        Err(InspectError::Capacity)
    ));
    fs::write(
        directory.path().join(MANIFEST_FILE),
        vec![b' '; MAX_MANIFEST_BYTES.saturating_add(1)],
    )
    .expect("oversized manifest");
    assert!(matches!(
        Inspection::read(&root, CURRENT, CURRENT).await,
        Err(InspectError::Capacity)
    ));
}

#[tokio::test]
async fn private_file_and_parser_causes_do_not_reopen_path_or_payload_text() {
    let (directory, root) = package();
    fs::write(
        directory.path().join(MANIFEST_FILE),
        MANIFEST.replace("client.json", "PRIVATE-INSPECT-CANARY"),
    )
    .expect("private missing path");
    let error = Inspection::read(&root, CURRENT, CURRENT)
        .await
        .expect_err("missing");
    safe_chain(&error);
    fs::write(directory.path().join(MANIFEST_FILE), MANIFEST).expect("manifest");
    fs::write(
        directory.path().join("client.json"),
        DOCUMENT.replace("methods", "PRIVATE-INSPECT-CANARY"),
    )
    .expect("bad graph");
    safe_chain(
        &Inspection::read(&root, CURRENT, CURRENT)
            .await
            .expect_err("graph"),
    );
}

fn safe_chain(error: &dyn Error) {
    let mut current = error;
    loop {
        assert!(!format!("{current:?} {current}").contains("PRIVATE-INSPECT-CANARY"));
        match current.source() {
            Some(next) => current = next,
            None => break,
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn external_symlinks_and_nonregular_declarations_use_existing_file_refusals() {
    use std::os::unix::fs::symlink;
    let (directory, root) = package();
    let outside = tempfile::tempdir().expect("outside");
    let external = outside.path().join("client.json");
    fs::write(&external, DOCUMENT).expect("external document");
    let client = directory.path().join("client.json");
    fs::remove_file(&client).expect("remove");
    symlink(&external, &client).expect("escaping link");
    assert!(matches!(
        Inspection::read(&root, CURRENT, CURRENT).await,
        Err(InspectError::Read(_))
    ));
    fs::remove_file(&client).expect("remove link");
    fs::create_dir(&client).expect("directory");
    assert!(matches!(
        Inspection::read(&root, CURRENT, CURRENT).await,
        Err(InspectError::Read(_))
    ));
}
