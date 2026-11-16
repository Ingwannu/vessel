//! Page invariants exercise values, not a retained runtime snapshot archive.

use super::*;
use crate::{
    scripts::{ScriptEntry, ScriptKind},
    workspace_services::WorkspaceScriptOrigin,
};

fn row(name: &str) -> WorkspaceScriptRow {
    let entry = ScriptEntry::new(
        name.parse().unwrap(),
        ScriptKind::Script,
        "synthetic-command".parse().unwrap(),
        None,
        None,
    )
    .unwrap();
    WorkspaceScriptRow::new(entry, WorkspaceScriptOrigin::Configured, None, None).unwrap()
}

fn page(
    rows: Vec<WorkspaceScriptRow>,
    next: Option<ServiceCursor>,
    snapshot: ServiceSnapshotId,
) -> Result<ServicePage, InvalidServicePage> {
    ServicePage::new(
        WorkspaceId::random(),
        snapshot,
        ServiceConfiguration::None,
        rows,
        next,
    )
}

#[test]
fn shared_count_unique_order_and_cursor_are_enforced_without_repair() {
    let snapshot = ServiceSnapshotId::random();
    let mut rows: Vec<_> = (0..MAX_PROJECT_COMMANDS)
        .map(|index| row(&format!("web-{index}")))
        .collect();
    rows.sort_by(|left, right| {
        left.view()
            .definition
            .name
            .cmp(&right.view().definition.name)
    });
    let valid = page(rows.clone(), None, snapshot).unwrap();
    let json = serde_json::to_string(&valid).unwrap();
    assert_eq!(serde_json::from_str::<ServicePage>(&json).unwrap(), valid);
    rows.push(row("z-extra"));
    assert_eq!(page(rows, None, snapshot), Err(InvalidServicePage::TooMany));
    for rows in [vec![row("a"), row("a")], vec![row("b"), row("a")]] {
        assert_eq!(page(rows, None, snapshot), Err(InvalidServicePage::Order));
    }
    let next = ServiceCursor {
        snapshot,
        after: "a".parse().unwrap(),
    };
    assert!(page(vec![row("a")], Some(next.clone()), snapshot).is_ok());
    assert_eq!(
        page(vec![], Some(next.clone()), snapshot),
        Err(InvalidServicePage::Cursor)
    );
    assert_eq!(
        page(vec![row("b")], Some(next.clone()), snapshot),
        Err(InvalidServicePage::Cursor)
    );
    assert_eq!(
        page(vec![row("a")], Some(next), ServiceSnapshotId::random()),
        Err(InvalidServicePage::Cursor)
    );
}

#[test]
fn wire_pages_cannot_bypass_count_order_or_continuation() {
    let snapshot = ServiceSnapshotId::random();
    let workspace = WorkspaceId::random();
    let oversized: Vec<_> = (0..MAX_PROJECT_COMMANDS.checked_add(1).unwrap())
        .map(|index| row(&format!("web-{index}")))
        .collect();
    for rows in [oversized, vec![row("a"), row("a")]] {
        let json = format!(
            r#"{{"workspace":"{workspace}","snapshot":"{snapshot}","configuration":{{"kind":"none"}},"rows":{}}}"#,
            serde_json::to_string(&rows).unwrap()
        );
        assert!(serde_json::from_str::<ServicePage>(&json).is_err());
    }
    let next = ServiceCursor {
        snapshot: ServiceSnapshotId::random(),
        after: "a".parse().unwrap(),
    };
    let json = format!(
        r#"{{"workspace":"{workspace}","snapshot":"{snapshot}","configuration":{{"kind":"none"}},"rows":[{}],"next":{}}}"#,
        serde_json::to_string(&row("a")).unwrap(),
        serde_json::to_string(&next).unwrap()
    );
    assert!(serde_json::from_str::<ServicePage>(&json).is_err());
}

#[test]
fn summary_digest_stays_hidden_until_the_existing_display_gate_is_assigned() {
    let digest = CommandsDigest::from_sha256(Default::default());
    let snapshot_page = ServicePage::new(
        WorkspaceId::random(),
        ServiceSnapshotId::random(),
        ServiceConfiguration::Current {
            digest: digest.clone(),
            approval: ServiceApproval::Approved,
        },
        vec![row("web")],
        None,
    )
    .unwrap();
    let result = WorkspaceScriptQueryResult::Snapshot {
        page: snapshot_page,
    };
    for protocol in [crate::MIN_PROTOCOL_VERSION, crate::PROTOCOL_VERSION] {
        let projected = result.for_protocol(protocol).unwrap();
        let json = serde_json::to_string(&projected).unwrap();
        assert!(!json.contains(digest.as_str()));
        assert!(!json.contains("\"approved\""));
        assert!(json.contains("approval_required"));
    }
    let unknown =
        serde_json::from_str::<WorkspaceScriptQueryResult>(r#"{"kind":"future"}"#).unwrap();
    assert_eq!(unknown, WorkspaceScriptQueryResult::Unknown);
    let empty = page(Vec::new(), None, ServiceSnapshotId::random()).unwrap();
    let ServicePageView {
        workspace: _,
        snapshot: _,
        configuration,
        rows,
        next,
    } = empty.view();
    assert_eq!(configuration, &ServiceConfiguration::None);
    assert!(rows.is_empty());
    assert!(next.is_none());
}
