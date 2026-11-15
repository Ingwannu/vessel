use super::*;
use vessel_proto::{ProjectId, workspace::RelPath};

fn snapshot(kind: ContextKind, id: &str, text: &str) -> ContextAttachment {
    ContextAttachment::Snapshot {
        kind,
        payload: Payload {
            id: ContextId::parse(id).unwrap(),
            title: ContextTitle::parse("Context").unwrap(),
            text: ContextText::parse(text).unwrap(),
            url: None,
        },
    }
}
fn scope() -> ContextScope {
    ContextScope {
        machine: Name::parse("laptop").unwrap(),
        root: WorkspaceRoot::Project {
            project: ProjectId::random(),
        },
    }
}

#[test]
fn workspace_files_are_pointers_deduped_by_scope_path_and_selection() {
    let file = WorkspaceFile {
        scope: scope(),
        path: "src/main.rs".parse::<RelPath>().unwrap(),
        selection: Selection::Whole,
    };
    let item = ContextAttachment::WorkspaceFile(file.clone());
    assert_eq!(item.text(), "Workspace file: src/main.rs");
    let mut tray = ContextTray::default();
    tray.add(item.clone()).unwrap();
    tray.add(item.clone()).unwrap();
    assert_eq!(tray.visible().count(), 1);
    let lines = ContextAttachment::WorkspaceFile(WorkspaceFile {
        selection: Selection::Lines(LineRange::new(2, 5).unwrap()),
        ..file
    });
    assert_eq!(lines.text(), "Workspace file: src/main.rs\nLines: 2-5");
    tray.add(lines).unwrap();
    assert_eq!(tray.visible().count(), 2);
    tray.remove(&item);
    assert_eq!(tray.visible().count(), 1);
    assert!(LineRange::new(0, 1).is_err());
    assert!(LineRange::new(5, 2).is_err());
}

#[test]
fn replacing_context_keeps_only_the_latest_snapshot_without_changing_its_suppression() {
    for kind in [
        ContextKind::ChatHistory,
        ContextKind::PullComment,
        ContextKind::PullReview,
        ContextKind::PullCheck,
    ] {
        let mut tray = ContextTray::default();
        let first = snapshot(kind, "same", "old");
        tray.add(first.clone()).unwrap();
        tray.remove(&first);
        tray.add(snapshot(kind, "same", "updated")).unwrap();
        assert!(
            tray.is_empty(),
            "producer refresh must not resurrect a removed pill"
        );
        tray.reattach(snapshot(kind, "same", "latest")).unwrap();
        assert_eq!(
            tray.visible()
                .map(ContextAttachment::text)
                .collect::<Vec<_>>(),
            ["latest"]
        );
    }
}

#[test]
fn source_removal_is_suppression_and_failure_restores_it() {
    for kind in [
        ContextKind::BrowserElement,
        ContextKind::Review,
        ContextKind::ChatHistory,
        ContextKind::PullComment,
        ContextKind::PullReview,
        ContextKind::PullCheck,
    ] {
        let mut tray = ContextTray::default();
        let removed = snapshot(kind, "hidden", "hidden text");
        tray.add(removed.clone()).unwrap();
        tray.remove(&removed);
        tray.add(snapshot(kind, "sending", "sent text")).unwrap();
        let prepared = tray.prepare().unwrap().unwrap();
        assert_eq!(prepared.attachments().count(), 1);
        assert!(tray.settle(&prepared, SubmissionOutcome::Failed));
        assert_eq!(tray.visible().count(), 2);
    }
}

#[test]
fn accepted_or_queued_submission_does_not_remove_newer_replacements_or_new_items() {
    for outcome in [SubmissionOutcome::Sent, SubmissionOutcome::Queued] {
        let mut tray = ContextTray::default();
        tray.add(snapshot(ContextKind::PullCheck, "one", "old"))
            .unwrap();
        let sent = tray.prepare().unwrap().unwrap();
        assert_eq!(tray.prepare(), Err(ContextProblem::Busy));
        tray.add(snapshot(ContextKind::PullCheck, "one", "new"))
            .unwrap();
        tray.add(snapshot(ContextKind::ChatHistory, "two", "history"))
            .unwrap();
        assert!(tray.settle(&sent, outcome));
        assert_eq!(tray.visible().count(), 2);
        let next = tray.prepare().unwrap().unwrap();
        assert!(!tray.settle(&sent, SubmissionOutcome::Failed));
        assert!(tray.settle(&next, outcome));
        assert!(tray.is_empty());
    }
}

#[test]
fn external_picker_selection_toggles_and_submit_does_not_execute_or_fetch() {
    for kind in [
        ContextKind::ForgeIssue,
        ContextKind::ForgeChangeRequest,
        ContextKind::PluginResource,
    ] {
        let mut tray = ContextTray::default();
        let item = snapshot(kind, "producer-captured-key", "exact context");
        tray.toggle(item.clone()).unwrap();
        assert_eq!(tray.visible().count(), 1);
        tray.toggle(item.clone()).unwrap();
        assert!(tray.is_empty());
        tray.toggle(item).unwrap();
        let prepared = tray.prepare().unwrap().unwrap();
        assert_eq!(
            prepared.attachments().next().unwrap().text(),
            "exact context"
        );
        assert!(tray.settle(&prepared, SubmissionOutcome::Sent));
        assert!(tray.is_empty());
    }
}

#[test]
fn metadata_is_validated_and_unsafe_links_are_refused() {
    assert!(ContextId::parse("").is_err());
    assert!(ContextTitle::parse("a\nb").is_err());
    assert!(ContextText::parse("a\0b").is_err());
    assert!(ContextUrl::parse("javascript:alert(1)").is_err());
    assert!(ContextUrl::parse("https://example.invalid/context").is_ok());
}

#[test]
fn history_accepts_recent_unicode_context_and_bounds_the_complete_submission() {
    let history = "한".repeat(100_000);
    let wrapped = format!("<chat-history-summary>\n{history}\n</chat-history-summary>");
    let history = ChatHistory::new(Payload {
        id: ContextId::parse("history").unwrap(),
        title: ContextTitle::parse("Chat history").unwrap(),
        text: ContextText::parse(&wrapped).unwrap(),
        url: None,
    });
    let mut tray = ContextTray::default();
    tray.add(history.into_attachment()).unwrap();
    assert_eq!(
        tray.prepare()
            .unwrap()
            .unwrap()
            .attachments()
            .next()
            .unwrap()
            .text(),
        wrapped
    );
    let mut too_much = ContextTray::default();
    let large = "x".repeat(ContextText::MAX_BYTES);
    too_much
        .add(snapshot(ContextKind::Review, "one", &large))
        .unwrap();
    too_much
        .add(snapshot(ContextKind::Review, "two", "extra"))
        .unwrap();
    assert_eq!(too_much.prepare(), Err(ContextProblem::Text));
}
