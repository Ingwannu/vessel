//! Privacy and independent-read rules; existing page parity cases stay in sync.rs.
use super::member;
use crate::devices::{MachineKey, Target};
use crate::sync::*;
use std::collections::BTreeSet;
use vessel_proto::{
    BlobHash, BundleFile, ErrorCode, FleetNode, Generation, Manifest, McpServerName, NodeId,
    ProtocolVersion, SecretName,
};

fn node() -> NodeId {
    "00000000-0000-4000-8000-000000000099".parse().unwrap()
}
fn connection() -> FleetNode {
    FleetNode {
        node: node(),
        name: "home".into(),
        protocol: ProtocolVersion::new(6),
        reachable: true,
    }
}
pub(super) fn manifest() -> Manifest {
    let mut manifest: Manifest=serde_json::from_value(serde_json::json!({
        "generation":5,"mcp": {
            "search":{"type":"stdio","command":"PRIVATE-VALUE-MARKER","args":["PRIVATE-VALUE-MARKER"],
                "env":{"KEY":{"secret":"SEARCH_KEY"},"MODE":"PRIVATE-VALUE-MARKER"}},
            "docs":{"type":"http","url":"https://example.invalid/PRIVATE-VALUE-MARKER",
                "headers":{"Authorization":{"secret":"DOCS_TOKEN"},"X-Private":"PRIVATE-VALUE-MARKER"}}
        }
    })).unwrap();
    for (path, size, executable) in [
        ("AGENTS.md", 2048, false),
        ("skills/한글/run.sh", 3_145_728, true),
    ] {
        let _previous = manifest.files.insert(
            path.parse().unwrap(),
            BundleFile {
                hash: BlobHash::from_bytes([7; 32]),
                size,
                executable,
            },
        );
    }
    manifest
}
fn reads() -> Reads {
    Reads {
        published: Ok(Published::project(manifest())),
        members: Ok(vec![member()]),
        nodes: Ok(vec![connection()]),
        peers: Ok(vec![connection()]),
        secrets: Ok(vec![
            "DOCS_TOKEN".parse().unwrap(),
            "LOCAL_ONLY".parse().unwrap(),
        ]),
    }
}
fn snapshot(reads: Reads) -> Snapshot {
    Snapshot::new(node(), "home".into(), reads)
}

#[test]
fn projection_keeps_every_file_name_size_mode_server_name_and_secret_reference_only() {
    let published = Published::project(manifest());
    assert_eq!(published.generation, Generation::new(5));
    assert_eq!(
        published
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.size.word(), f.mode))
            .collect::<Vec<_>>(),
        [
            ("AGENTS.md", "2.0 KB".into(), FileMode::Data),
            ("skills/한글/run.sh", "3.0 MB".into(), FileMode::Executable)
        ]
    );
    assert_eq!(
        published
            .servers
            .iter()
            .map(McpServerName::as_str)
            .collect::<Vec<_>>(),
        ["docs", "search"]
    );
    assert_eq!(
        published
            .required
            .iter()
            .map(SecretName::as_str)
            .collect::<Vec<_>>(),
        ["DOCS_TOKEN", "SEARCH_KEY"]
    );
    // Security assertion, not a prefix-based wording test: no discarded
    // command, argument, URL, header or environment literal can reach Debug.
    assert!(!format!("{published:?}").contains("PRIVATE-VALUE-MARKER"));
    assert_eq!(
        FileKind::of(&"instructions.md".parse().unwrap()),
        FileKind::Instructions
    );
    assert_eq!(
        FileKind::of(&"AGENTS.md".parse().unwrap()),
        FileKind::Instructions
    );
    assert_eq!(
        FileKind::of(&"skills/한글/run.sh".parse().unwrap()),
        FileKind::Skill
    );
    assert_eq!(FileKind::of(&"other.txt".parse().unwrap()), FileKind::Other);
}

#[test]
fn present_missing_and_local_only_are_different_and_sorted_without_secret_values() {
    let snapshot = snapshot(reads());
    assert_eq!(
        snapshot
            .secret_rows()
            .iter()
            .map(|s| (s.name.as_str(), s.presence, s.usage))
            .collect::<Vec<_>>(),
        [
            ("DOCS_TOKEN", Presence::Present, Usage::Shared),
            ("LOCAL_ONLY", Presence::Present, Usage::LocalOnly),
            ("SEARCH_KEY", Presence::Missing, Usage::Shared),
        ]
    );
    assert_eq!(
        snapshot.member_state(&member()),
        Some(MemberState::Behind(2))
    );
    assert!(!format!("{snapshot:?}").contains("PRIVATE-VALUE-MARKER"));
}

#[test]
fn failed_reads_do_not_turn_unknown_presence_or_usage_into_absence() {
    let mut partial = reads();
    partial.secrets = Err(Problem::Refused(ErrorCode::Forbidden));
    let first = snapshot(partial);
    assert_eq!(
        first
            .secret_rows()
            .iter()
            .map(|s| (s.name.as_str(), s.presence, s.usage))
            .collect::<Vec<_>>(),
        [
            ("DOCS_TOKEN", Presence::Unknown, Usage::Shared),
            ("SEARCH_KEY", Presence::Unknown, Usage::Shared)
        ]
    );
    let mut partial = reads();
    partial.published = Err(Problem::Refused(ErrorCode::Unavailable));
    let second = snapshot(partial);
    assert_eq!(
        second
            .secret_rows()
            .iter()
            .map(|s| (s.name.as_str(), s.presence, s.usage))
            .collect::<Vec<_>>(),
        [
            ("DOCS_TOKEN", Presence::Present, Usage::Unreported),
            ("LOCAL_ONLY", Presence::Present, Usage::Unreported)
        ]
    );
    assert_eq!(second.member_state(&member()), None);
    assert!(second.members.is_ok());
    assert!(second.connections.is_ok());
}

#[test]
fn only_home_only_success_proves_a_home_never_the_first_connection_or_its_name() {
    assert!(
        matches!(snapshot(reads()).home,Home::Here{node:id,name} if id==node() && name=="home")
    );
    let unavailable = Problem::Refused(ErrorCode::Unavailable);
    let mut partial = reads();
    partial.published = Err(unavailable);
    assert!(matches!(snapshot(partial).home, Home::Here { .. }));
    let mut partial = reads();
    partial.peers = Err(unavailable);
    assert!(matches!(snapshot(partial).home, Home::Here { .. }));
    let mut partial = reads();
    partial.peers = Err(unavailable);
    partial.published = Err(unavailable);
    let snapshot = snapshot(partial);
    assert!(matches!(snapshot.home,Home::Unreported(why) if why==unavailable));
    assert_eq!(snapshot.connections.unwrap(), [connection()]);
}

#[test]
fn a_failed_nodes_read_can_use_peers_without_discarding_other_successful_sections() {
    let mut partial = reads();
    partial.nodes = Err(Problem::Offline);
    partial.members = Err(Problem::Refused(ErrorCode::Forbidden));
    let snapshot = snapshot(partial);
    assert_eq!(snapshot.connections.as_ref().unwrap(), &[connection()]);
    assert_eq!(
        snapshot.members,
        Err(Problem::Refused(ErrorCode::Forbidden))
    );
    assert!(snapshot.published.is_ok());
    assert!(snapshot.secrets.is_ok());
}

#[test]
fn navigation_and_repeated_refreshes_cannot_accept_a_stale_sync_reply() {
    let mut page = Page::default();
    let target = Target::Online(MachineKey::from_book("home"));
    page.target(target.clone());
    let old = page.tickets.next_ticket().unwrap();
    page.reading = Reading::Loading(old);
    page.target(Target::Hidden);
    page.target(target);
    let new = page.tickets.next_ticket().unwrap();
    assert_ne!(old, new);
    page.reading = Reading::Loading(new);
    page.reading.answer(old, Ok(Box::new(snapshot(reads()))));
    assert!(matches!(page.reading, Reading::Loading(_)));
    page.reading.answer(new, Ok(Box::new(snapshot(reads()))));
    assert!(matches!(page.reading, Reading::Ready(_)));
    page.target(Target::Offline(MachineKey::from_book("home")));
    assert!(matches!(page.reading, Reading::Empty));
    page.reading.answer(new, Err(Problem::Offline));
    assert!(matches!(page.reading, Reading::Empty));
}

#[test]
fn shared_and_stored_duplicates_are_each_shown_once_and_an_empty_read_is_truly_empty() {
    let mut partial = reads();
    partial.secrets = Ok(vec!["DOCS_TOKEN".parse().unwrap(); 3]);
    assert_eq!(snapshot(partial).secret_rows().len(), 2);
    let mut partial = reads();
    partial.published = Ok(Published {
        generation: Generation::new(0),
        files: Vec::new(),
        servers: Vec::new(),
        required: BTreeSet::new(),
    });
    partial.secrets = Ok(Vec::new());
    assert!(snapshot(partial).secret_rows().is_empty());
}

#[test]
fn commands_are_existing_local_cli_commands_and_never_a_remote_sync_action() {
    assert_eq!(
        LocalCommand::ALL.map(LocalCommand::text),
        [
            "vessel fleet init",
            "vessel fleet invite",
            "vessel fleet join",
            "vessel fleet leave",
            "vessel fleet status"
        ]
    );
    let mut connection = connection();
    assert_eq!(
        connection_variant(&connection),
        crate::badge::Variant::Success
    );
    connection.reachable = false;
    assert_eq!(
        connection_variant(&connection),
        crate::badge::Variant::Error
    );
}

#[test]
fn empty_secret_lists_distinguish_successful_absence_from_unread_references() {
    let mut partial = reads();
    partial.secrets = Ok(Vec::new());
    assert_eq!(snapshot(partial).secret_empty(), None);
    let mut partial = reads();
    partial.secrets = Ok(Vec::new());
    partial.published = Ok(Published::project(Manifest::default()));
    assert_eq!(
        snapshot(partial).secret_empty(),
        Some(SecretEmpty::NoneReported)
    );
    let mut partial = reads();
    partial.secrets = Ok(Vec::new());
    partial.published = Err(Problem::Offline);
    assert_eq!(
        snapshot(partial).secret_empty(),
        Some(SecretEmpty::StoredOnly)
    );
    let mut partial = reads();
    partial.secrets = Err(Problem::Offline);
    partial.published = Ok(Published::project(Manifest::default()));
    assert_eq!(snapshot(partial).secret_empty(), None);
    assert_eq!(
        crate::Lang::En.sync().secret_empty(SecretEmpty::StoredOnly),
        "This machine stores no secret names. Shared references could not be checked."
    );
    assert_eq!(
        crate::Lang::Ko.sync().secret_empty(SecretEmpty::StoredOnly),
        "이 머신에 저장된 비밀값 이름이 없습니다. 공유 설정에서 사용하는 이름은 확인하지 못했습니다."
    );
}
