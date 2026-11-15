//! Pull requests' rules end to end (protocol 11, ADR 0029): an agent
//! opens only what it pushed and merges nothing; a session that is not
//! placed has none; an ended one is not watched; a forge that is down
//! never holds a session, and its last facts are kept; feedback is sent
//! once.

use std::sync::Arc;

use vessel_forge::fake::{Failure, FakeForge};
use vessel_proto::forge::{
    ForgeAccess, GetForgeStatus, GetPullRequest, MergeMethod, MergePullRequest, OpenOutcome,
    OpenPullRequest, OpenRefusal, PullNumber, PullTitle, SetFeedback,
};
use vessel_proto::workspace::{CommitId, SessionPlace};
use vessel_proto::{AgentEvent, ErrorCode, EventRecord, GetSession, SessionEvent, SessionInput};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::forge::{git, info_until, project, pull, running_in};
use crate::support::{Fixture, read_all, within};
use crate::workspace::{in_place, lookup, new_worktree};

/// How many prompts the node sent the session as feedback.
fn feedback_prompts(records: &[EventRecord]) -> usize {
    records
        .iter()
        .filter(|record| {
            matches!(
                            &record.event,
                            SessionEvent::Agent {
                                event: AgentEvent::TurnStarted { provenance: None, content ,
            client_message_id: None,
            },
                            } if format!("{content:?}").contains("Vessel: pull request feedback")
                        )
        })
        .count()
}

/// An agent opens a pull request only of a branch it pushed itself (the
/// node never pushes for it), and may not merge, switch feedback or read
/// the node's sign-in.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_opens_only_what_it_pushed_and_merges_nothing() {
    within(async {
        let forge = FakeForge::new();
        let fixture = Fixture::new();
        let forge_for = Arc::clone(&forge);
        let node = fixture
            .start_with(|config| {
                config.vessel_mcp = Some(vessel_mcp(&fixture));
                config.harness_lookup = lookup(&fixture);
                config.forge = vessel_node::ForgeSetup {
                    custom: Some(forge_for),
                    ..vessel_node::ForgeSetup::default()
                };
            })
            .await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;
        let mut first = structured(&owner, &fixture).await;
        let (_, socket, auth) = credential(&owner, &mut first).await;
        let agent = as_agent(&socket, &auth).await;
        let own = new_worktree(&agent, project.id).await;
        let session = agent
            .create_session(in_place(
                &fixture,
                SessionPlace::Worktree { worktree: own.id },
            ))
            .await
            .unwrap()
            .id;
        let open = OpenPullRequest {
            session,
            title: PullTitle::lossy("A change"),
            body: None,
            base: None,
            draft: false,
        };
        assert_eq!(
            agent.call(open).await.unwrap(),
            OpenOutcome::Refused {
                refusal: OpenRefusal::NotPushed
            }
        );
        assert_eq!(forge.calls("open"), 0);
        let head: CommitId = git(&own.path, &["rev-parse", "HEAD"]).parse().unwrap();
        let merge = MergePullRequest {
            session,
            number: PullNumber::new(1).unwrap(),
            method: MergeMethod::Squash,
            head,
        };
        assert_eq!(code(agent.call(merge).await), ErrorCode::Forbidden);
        let feedback = SetFeedback {
            session,
            enabled: true,
        };
        assert_eq!(code(agent.call(feedback).await), ErrorCode::Forbidden);
        assert_eq!(
            code(agent.call(GetForgeStatus {}).await),
            ErrorCode::Forbidden
        );
        // Not placed: the owner's first session has no pull request.
        let unplaced = owner.call(GetSession { session: first.id }).await.unwrap();
        assert_eq!(unplaced.pull_request, None);
    })
    .await;
}

/// A forge that is down holds no session: it still takes prompts, its
/// last facts stay on its card, and the card says why they are old; an
/// ended session is not watched.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_forge_that_is_down_holds_nothing_and_an_end_clears_it() {
    within(async {
        let forge = FakeForge::new();
        let fixture = Fixture::new();
        let node = crate::forge::start(&fixture, &forge).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;
        let worktree = new_worktree(&owner, project.id).await;
        let head: CommitId = git(&worktree.path, &["rev-parse", "HEAD"]).parse().unwrap();
        let session = running_in(&owner, &fixture, &worktree).await;
        forge.put(
            worktree.branch.clone().unwrap(),
            pull(5, &worktree, &head, false),
        );
        let refresh = || GetPullRequest {
            session,
            refresh: true,
        };
        assert!(owner.call(refresh()).await.unwrap().pull_request.is_some());

        forge.fail(Some(Failure::Down));
        // A session's refreshes are held to one per 10 s; the next looks.
        tokio::time::sleep(std::time::Duration::from_secs(11)).await;
        let view = owner.call(refresh()).await.unwrap();
        assert!(
            matches!(view.access, ForgeAccess::Unreachable { .. }),
            "{:?}",
            view.access
        );
        assert!(view.pull_request.is_some(), "the last facts are kept");
        owner
            .send(
                session,
                SessionInput::Prompt {
                    content: vec![vessel_proto::ContentBlock::Text {
                        text: "plain".to_owned(),
                    }],
                },
            )
            .await
            .unwrap();

        owner.kill(session).await.unwrap();
        let ended = info_until(&owner, session, |info| !info.status.is_live()).await;
        assert_eq!(ended.pull_request, None);
        let view = owner.call(refresh()).await.unwrap();
        assert_eq!(view.access, ForgeAccess::Ended);
    })
    .await;
}

/// Feedback goes once: looking again at the same failing check sends
/// nothing more.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn feedback_is_sent_once_per_event() {
    within(async {
        let forge = FakeForge::new();
        let fixture = Fixture::new();
        let node = crate::forge::start(&fixture, &forge).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;
        let worktree = new_worktree(&owner, project.id).await;
        let head: CommitId = git(&worktree.path, &["rev-parse", "HEAD"]).parse().unwrap();
        let session = running_in(&owner, &fixture, &worktree).await;
        forge.put(
            worktree.branch.clone().unwrap(),
            pull(6, &worktree, &head, true),
        );
        let _switched = owner
            .call(SetFeedback {
                session,
                enabled: true,
            })
            .await
            .unwrap();
        let refresh = || GetPullRequest {
            session,
            refresh: true,
        };
        let _first = owner.call(refresh()).await.unwrap();
        crate::support::step("the feedback prompt", async {
            while feedback_prompts(&read_all(&owner, session).await) == 0 {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
        .await;
        // A whole read again (a person's refresh is never conditional),
        // after the 10 s a session's refreshes are held to.
        tokio::time::sleep(std::time::Duration::from_secs(11)).await;
        let _second = owner.call(refresh()).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        assert_eq!(feedback_prompts(&read_all(&owner, session).await), 1);
    })
    .await;
}
