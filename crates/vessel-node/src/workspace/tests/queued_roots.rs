//! Real owned comparison/history jobs revalidate after their execution queue.
use super::*;

const DEADLINE: std::time::Duration = std::time::Duration::from_secs(60);
use vessel_proto::workspace::{
    CompareGit, GitComparison, GitReadReason, GitWhitespace, ReadGitHistory,
};

#[derive(Clone, Copy)]
enum ChangedFact {
    Path,
    DefaultBranch,
}
#[derive(Clone, Copy)]
enum Reader {
    Comparison,
    History,
}

#[tokio::test]
async fn queued_reads_do_not_follow_replaced_paths_or_use_superseded_base_facts() {
    vessel_testkit::within(DEADLINE, async {
        for reader in [Reader::Comparison, Reader::History] {
            for changed in [ChangedFact::Path, ChangedFact::DefaultBranch] {
                queued_change(reader, changed).await;
            }
        }
    })
    .await
    .expect("registered-root operation should settle after releasing its real queue barrier");
}

async fn queued_change(reader: Reader, changed: ChangedFact) {
    let fixture = Fixture::new().await;
    let project = fixture.project("queued-before", None).await;
    let replacement = fixture.repo("queued-after", None);
    let root = WorkspaceRoot::Project {
        project: project.id,
    };
    let occupied = fixture.workspaces.comparisons.hold_execution_for_test();
    let submitted = fixture
        .workspaces
        .comparisons
        .submitted_for_test()
        .notified();
    tokio::pin!(submitted);
    let _enabled = submitted.as_mut().enable();
    let workspaces = Arc::clone(&fixture.workspaces);
    let answer = tokio::spawn(async move {
        match reader {
            Reader::Comparison => workspaces
                .git_compare(CompareGit {
                    root,
                    comparison: GitComparison::Committed { base: None },
                    whitespace: GitWhitespace::Show,
                    reason: GitReadReason::View,
                })
                .await
                .map(drop),
            Reader::History => workspaces
                .git_history(ReadGitHistory { root })
                .await
                .map(drop),
        }
    });
    submitted.await;
    {
        let mut state = fixture.workspaces.state();
        let entry = state.projects.get_mut(&project.id).unwrap();
        match changed {
            ChangedFact::Path => entry.info.root = replacement,
            ChangedFact::DefaultBranch => {
                entry.info.default_branch = Some("release".parse().unwrap());
            }
        }
    }
    drop(occupied);
    let error = answer.await.unwrap().unwrap_err();
    match reader {
        Reader::History => assert!(matches!(&error, WorkspaceError::TargetChanged), "{error:?}"),
        Reader::Comparison => assert!(
            matches!(&error, WorkspaceError::SharedComparison(source)
            if matches!(source.as_ref(), WorkspaceError::TargetChanged)),
            "{error:?}"
        ),
    }
    fixture.stop.cancel();
    assert!(fixture.tasks.close());
    fixture.tasks.wait().await;
}
