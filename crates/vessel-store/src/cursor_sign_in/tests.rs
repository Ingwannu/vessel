use super::*;
use std::error::Error;

const MARKER: &str = "PRIVATE-CURSOR-LOGIN-CANARY";

pub(super) fn fixture(value: impl rusqlite::ToSql) -> (tempfile::TempDir, std::path::PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state.vscdb");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("CREATE TABLE ItemTable(key TEXT PRIMARY KEY,value BLOB)")
        .unwrap();
    let _inserted = connection
        .execute(
            "INSERT INTO ItemTable VALUES (?1,?2)",
            rusqlite::params!["cursorAuthStatus", value],
        )
        .unwrap();
    drop(connection);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    (root, path)
}

fn safe_chain(error: &(dyn Error + 'static)) {
    let mut current = Some(error);
    while let Some(error) = current {
        assert!(!error.to_string().contains(MARKER));
        assert!(!format!("{error:?}").contains(MARKER));
        assert!(!format!("{error:#?}").contains(MARKER));
        assert!(error.downcast_ref::<serde_json::Error>().is_none());
        current = error.source();
    }
}

fn assert_private_read(path: &Path, body: &[u8]) {
    let before = std::fs::read(path).unwrap();
    let metadata = std::fs::metadata(path).unwrap();
    let error = read_cursor_usage_sign_in(path).unwrap_err();
    safe_chain(&error);
    let StoreError::Io { source, .. } = error else {
        panic!("expected a legacy decode refusal");
    };
    assert_eq!(source.kind(), io::ErrorKind::Other);
    safe_chain(source.get_ref().unwrap());
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert!(before.windows(body.len()).any(|window| window == body));
    assert_eq!(
        std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
        1
    );
    let after = std::fs::metadata(path).unwrap();
    assert_eq!(metadata.permissions(), after.permissions());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        assert_eq!(after.mode() & 0o777, 0o600);
        assert_eq!((metadata.uid(), metadata.gid()), (after.uid(), after.gid()));
    }
}

#[test]
fn actual_text_and_blob_credential_errors_are_safe_without_writing_the_store() {
    for body in [
        format!("\"{MARKER}\""),
        format!(r#"{{"accessToken":{{"{MARKER}":true}}}}"#),
        format!(r#"{{"accessToken":["{MARKER}"]}}"#),
        format!(r#"{{"accessToken":"{MARKER}"}} trailing"#),
        format!(r#"{{"accessToken":"{MARKER}"#),
    ] {
        let (_root, text) = fixture(body.as_str());
        assert_private_read(&text, body.as_bytes());
        let (_root, blob) = fixture(body.as_bytes());
        assert_private_read(&blob, body.as_bytes());
    }
}

#[test]
fn direct_key_precedence_and_empty_legacy_sign_ins_are_unchanged() {
    let (_root, path) = fixture(format!("\"{MARKER}\""));
    let connection = Connection::open(&path).unwrap();
    let _inserted = connection
        .execute(
            "INSERT INTO ItemTable VALUES (?1,?2)",
            rusqlite::params!["cursorAuth/accessToken", "fixture-direct"],
        )
        .unwrap();
    drop(connection);
    assert_eq!(
        read_cursor_usage_sign_in(&path).unwrap().unwrap().as_str(),
        "fixture-direct"
    );
    for body in ["{}", r#"{"accessToken":null}"#, r#"{"accessToken":"  "}"#] {
        let (_root, path) = fixture(body);
        assert!(read_cursor_usage_sign_in(&path).unwrap().is_none());
    }
}

#[test]
fn fixed_query_is_read_only_handles_legacy_and_refuses_active_wal() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state.vscdb");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("CREATE TABLE ItemTable(key TEXT PRIMARY KEY,value BLOB)")
        .unwrap();
    let _inserted = connection
        .execute(
            "INSERT INTO ItemTable VALUES (?1,?2)",
            rusqlite::params!["cursorAuthStatus", r#"{"accessToken":"fixture-legacy"}"#],
        )
        .unwrap();
    drop(connection);
    let before = std::fs::read(&path).unwrap();
    assert_eq!(
        read_cursor_usage_sign_in(&path).unwrap().unwrap().as_str(),
        "fixture-legacy"
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    std::fs::write(root.path().join("state.vscdb-wal"), b"active fixture WAL").unwrap();
    assert!(read_cursor_usage_sign_in(&path).is_err());
    let absent = root.path().join("absent.db");
    assert!(read_cursor_usage_sign_in(&absent).is_err());
    assert!(!absent.exists());
}
