//! Cache hits must not revive registry facts replaced during their disk open.
//! Occupy the real blocking pool; no sleeps or test-only production hook.
use super::*;
use std::future::Future as _;
use std::task::Poll;
use vessel_proto::workspace::{
    CompareGit, CompareGitFile, GitComparison, GitHunkOffset, GitReadReason, GitWhitespace,
};

#[derive(Clone, Copy)]
enum ChangedFact {
    Path,
    DefaultBranch,
}
#[derive(Clone, Copy)]
enum Payload {
    Summary,
    File,
}

#[test]
fn cache_hits_refuse_registry_changes_during_the_original_root_open() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        vessel_testkit::within(std::time::Duration::from_secs(60), async {
            for payload in [Payload::Summary, Payload::File] {
                for changed in [ChangedFact::Path, ChangedFact::DefaultBranch] {
                    changed_during_probe(payload, changed).await;
                }
            }
        })
        .await
        .unwrap();
    });
}

async fn read(
    owner: &Arc<Workspaces>,
    query: CompareGit,
    payload: Payload,
) -> Result<(), WorkspaceError> {
    match payload {
        Payload::Summary => owner.git_compare(query).await.map(drop),
        Payload::File => owner
            .git_compare_file(CompareGitFile {
                query,
                path: "README.md".parse().unwrap(),
                from_hunk: GitHunkOffset::default(),
            })
            .await
            .map(drop),
    }
}

async fn changed_during_probe(payload: Payload, changed: ChangedFact) {
    let fixture = Fixture::new().await;
    let project = fixture.project("cached-probe-before", None).await;
    let replacement = fixture.repo("cached-probe-after", None);
    let query = CompareGit {
        root: WorkspaceRoot::Project {
            project: project.id,
        },
        comparison: GitComparison::Uncommitted,
        whitespace: GitWhitespace::Show,
        reason: GitReadReason::View,
    };
    read(&fixture.workspaces, query.clone(), payload)
        .await
        .unwrap();
    assert_eq!(fixture.workspaces.comparisons.cached_payloads_for_test(), 1);
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, finish) = std::sync::mpsc::channel();
    let blocker = tokio::task::spawn_blocking(move || {
        entered.send(()).unwrap();
        // Unwinding closes release too, allowing runtime cleanup after a failed assertion.
        let _released = finish.recv();
    });
    ready.await.unwrap();
    let probe = read(&fixture.workspaces, query, payload);
    tokio::pin!(probe);
    assert!(matches!(
        std::future::poll_fn(|cx| Poll::Ready(probe.as_mut().poll(cx))).await,
        Poll::Pending
    ));
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
    release.send(()).unwrap();
    blocker.await.unwrap();
    assert!(matches!(probe.await, Err(WorkspaceError::TargetChanged)));
    // Refusal is authority revalidation, not cache invalidation masquerading as a miss.
    assert_eq!(fixture.workspaces.comparisons.cached_payloads_for_test(), 1);
    fixture.stop.cancel();
    assert!(fixture.tasks.close());
    fixture.tasks.wait().await;
}
