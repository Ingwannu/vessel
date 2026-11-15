//! Real repositories through the node's root authority and tracked entry points.
use std::collections::BTreeSet;

use vessel_proto::workspace::{
    CompareGit, CompareGitFile, GitCompared, GitComparison, GitHunkOffset, GitWhitespace,
};

use super::*;

fn paths(value: GitCompared<vessel_proto::workspace::DiffSummary>) -> BTreeSet<String> {
    match value {
        GitCompared::Ready { data } => data
            .files
            .iter()
            .map(|file| file.path.to_string())
            .collect(),
        other @ (GitCompared::NoBase | GitCompared::TooLarge) => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn node_comparisons_keep_committed_working_and_all_distinct_and_reject_invalid_bases() {
    let fixture = Fixture::new().await;
    let repo = fixture.repo("comparison-node", None);
    let project = fixture
        .workspaces
        .add_project(repo.clone(), None)
        .await
        .unwrap();
    fixture.git(&repo, &["checkout", "-q", "-b", "topic"]);
    std::fs::write(repo.join("committed"), "committed\n").unwrap();
    fixture.git(&repo, &["add", "committed"]);
    fixture.git(&repo, &["commit", "-q", "-m", "topic"]);
    std::fs::write(repo.join("working"), "working\n").unwrap();
    let index = std::fs::read(repo.join(".git/index")).unwrap();
    let root = WorkspaceRoot::Project {
        project: project.id,
    };
    let query = |comparison| CompareGit {
        root,
        comparison,
        whitespace: GitWhitespace::Show,
    };
    assert_eq!(
        paths(
            fixture
                .workspaces
                .git_compare(query(GitComparison::Committed { base: None }))
                .await
                .unwrap()
        ),
        BTreeSet::from(["committed".to_owned()])
    );
    assert_eq!(
        paths(
            fixture
                .workspaces
                .git_compare(query(GitComparison::Uncommitted))
                .await
                .unwrap()
        ),
        BTreeSet::from(["working".to_owned()])
    );
    assert_eq!(
        paths(
            fixture
                .workspaces
                .git_compare(query(GitComparison::All { base: None }))
                .await
                .unwrap()
        ),
        BTreeSet::from(["committed".to_owned(), "working".to_owned()])
    );
    let file = fixture
        .workspaces
        .git_compare_file(CompareGitFile {
            query: query(GitComparison::Committed { base: None }),
            path: "committed".parse().unwrap(),
            from_hunk: GitHunkOffset::default(),
        })
        .await
        .unwrap();
    assert!(matches!(file, GitCompared::Ready { data } if data.path.as_str() == "committed"));
    let invalid = fixture
        .workspaces
        .git_compare(query(GitComparison::Committed {
            base: Some(vessel_proto::workspace::WorktreeBase::Ref {
                name: "missing-ref".parse().unwrap(),
            }),
        }))
        .await;
    assert!(matches!(invalid, Err(WorkspaceError::Invalid(_))));
    assert_eq!(std::fs::read(repo.join(".git/index")).unwrap(), index);
    assert_eq!(
        std::fs::read_dir(&fixture.workspaces.dirs.scratch)
            .unwrap()
            .count(),
        0
    );
    fixture.stop.cancel();
    fixture.tasks.close();
    fixture.tasks.wait().await;
}

#[tokio::test]
async fn an_unknown_root_fails_before_a_job_and_a_stopped_node_refuses_the_resolved_one() {
    let fixture = Fixture::new().await;
    let unknown = CompareGit {
        root: WorkspaceRoot::Project {
            project: ProjectId::random(),
        },
        comparison: GitComparison::Uncommitted,
        whitespace: GitWhitespace::Show,
    };
    assert!(matches!(
        fixture.workspaces.git_compare(unknown).await,
        Err(WorkspaceError::UnknownProject(_))
    ));
    assert!(fixture.tasks.is_empty());
    let repo = fixture.repo("stopped-comparison", None);
    let project = fixture.workspaces.add_project(repo, None).await.unwrap();
    fixture.stop.cancel();
    let result = fixture
        .workspaces
        .git_compare(CompareGit {
            root: WorkspaceRoot::Project {
                project: project.id,
            },
            comparison: GitComparison::Uncommitted,
            whitespace: GitWhitespace::Show,
        })
        .await;
    assert!(matches!(
        result,
        Err(WorkspaceError::ComparisonJob(
            super::super::comparison::JobError::Closed
        ))
    ));
    fixture.tasks.close();
    fixture.tasks.wait().await;
}
