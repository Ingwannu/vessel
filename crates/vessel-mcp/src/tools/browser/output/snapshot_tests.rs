use super::*;
use vessel_proto::browser::{
    BrowserRevision, PageText, SnapshotDocument, SnapshotMetric, SnapshotStats, TabArgs,
    TextByteCount,
};

#[test]
fn snapshot_projects_observed_document_text_and_named_stats_without_later_lookups() {
    let id: BrowserId = "00000000-0000-4000-8000-000000000001".parse().unwrap();
    let command = BrowserCommand::Snapshot(TabArgs {
        browser_id: id.clone(),
    });
    let content = PageText::capped("- document \"Own snapshot\"".to_owned());
    let length = u32::try_from(content.text.as_str().len()).unwrap();
    let reply = render(
        &command,
        BrowserResult::Snapshot {
            browser_id: id,
            revision: BrowserRevision::ZERO,
            document: SnapshotDocument {
                address: PageText::try_from("https://example.test/observed".to_owned()).unwrap(),
                title: PageText::try_from("Observed snapshot title".to_owned()).unwrap(),
            },
            content,
            stats: SnapshotStats {
                nodes: SnapshotMetric::new(3),
                references: SnapshotMetric::new(1),
                text_bytes: TextByteCount::new(length),
                frames: SnapshotMetric::new(0),
            },
        },
    )
    .unwrap();
    let structured = reply.structured_content.unwrap();
    let result = &structured["result"];
    assert_eq!(result["url"], "https://example.test/observed");
    assert_eq!(result["title"], "Observed snapshot title");
    assert_eq!(result["format"], "aria-yaml");
    assert_eq!(result["snapshot"], "- document \"Own snapshot\"");
    assert_eq!(result["truncated"], false);
    assert_eq!(result["stats"]["nodeCount"], 3);
    assert_eq!(result["stats"]["textLength"], length);
    assert!(result.get("document").is_none());
    assert!(result.get("content").is_none());
    assert!(
        result.get("workspaceId").is_none(),
        "absent owner context is not fabricated"
    );
}
