//! The actual framed client must carry the captured continuation, not zero.
use super::tests::{answer, install, status};
use super::*;
use crate::testing::{Home, peer, phase, state, within};
use vessel_app_view::workspace::git::RequestKeys;
use vessel_app_view::workspace::git::panel::{
    Comparison, ComparisonMode, PanelReads, RepositoryKey, Whitespace,
};
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::workspace::{
    ChangeStatus, DiffContent, DiffSummary, FileChange, FileDiff, GitStatus, Hunk,
};
use vessel_proto::{Call, ErrorCode, Outcome as WireOutcome, WireError};

fn continuation(repository: RepositoryKey) -> (PanelReads, FileTicket) {
    let mut keys = RequestKeys::default();
    let mut reads = PanelReads::new(
        repository,
        Comparison::new(ComparisonMode::Uncommitted, None, Whitespace::Show),
    );
    let status = reads.begin_status(keys.issue().unwrap());
    assert!(reads.finish_status(
        status,
        Ok(GitStatus {
            branch: None,
            head: None,
            upstream: None,
            base: None,
            operation: None,
            files: Vec::new(),
            more: false,
        })
    ));
    let path: vessel_proto::workspace::RelPath = "src/한글.rs".parse().unwrap();
    let summary = reads.begin_summary(keys.issue().unwrap());
    assert!(reads.finish_summary(
        &summary,
        Ok(DiffSummary {
            from: None,
            more: false,
            files: vec![FileChange {
                path: path.clone(),
                old_path: None,
                status: ChangeStatus::Modified,
                additions: None,
                deletions: None,
                binary: false,
            }],
        })
    ));
    let first = reads
        .begin_captured_file(&reads.file_target(&path).unwrap(), keys.issue().unwrap())
        .unwrap();
    assert!(reads.finish_file(
        &first,
        Ok(FileDiff {
            path: path.clone(),
            old_path: None,
            mode: None,
            status: ChangeStatus::Modified,
            more_from: Some(1),
            content: DiffContent::Hunks {
                hunks: vec![Hunk {
                    old_start: 1,
                    old_lines: 0,
                    new_start: 1,
                    new_lines: 0,
                    section: None,
                    lines: Vec::new(),
                }]
            },
        })
    ));
    let next = reads
        .begin_captured_file(
            &reads.next_file_target(&path).unwrap(),
            keys.issue().unwrap(),
        )
        .unwrap();
    (reads, next)
}

#[tokio::test]
async fn the_original_client_receives_the_nonzero_hunk_and_echoes_it_on_failure() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = peer::controlled("fixture").await;
        let context = install(&mut state, peer.client.take().unwrap());
        let Ask::Status {
            address, ticket, ..
        } = status(context.clone(), context.node(), "fixture")
        else {
            panic!("status fixture");
        };
        let (mut reads, ticket) = continuation(ticket.repository());
        state
            .git_panel(Ask::File {
                address: address.clone(),
                context: context.clone(),
                ticket: ticket.clone(),
            })
            .await;
        let request = phase(
            "actual nonzero file frame",
            peer.requests.as_mut().unwrap().recv(),
        )
        .await
        .unwrap();
        let Call::GitDiff(asked) = request.call else {
            panic!("legacy exact uncommitted file query");
        };
        assert_eq!(asked.from_hunk, Some(1));
        assert_eq!(asked.path, *ticket.path());
        assert_eq!(asked.root, ticket.comparison().repository().root);
        request
            .answer
            .send(WireOutcome::Err(WireError::new(
                ErrorCode::Unavailable,
                "fixture read failed",
            )))
            .unwrap();
        let returned = answer(&mut events).await;
        assert_eq!(returned.address, address);
        assert_eq!(returned.context, context);
        let Outcome::File(got, result) = returned.outcome else {
            panic!("file outcome");
        };
        assert_eq!(got, ticket);
        assert_eq!(
            result,
            Err(ReadFailure::Call(Problem::Node {
                code: ErrorCode::Unavailable,
                message: "fixture read failed".to_owned()
            }))
        );
        assert!(reads.finish_file(&got, result));
        assert!(reads.next_file_target(got.path()).is_none());
        assert_eq!(peer.calls.lock().unwrap().len(), 1);
    })
    .await;
}
