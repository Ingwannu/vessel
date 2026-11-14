//! Protocol 14's forking and rewinding (track A2, ADR 0052), with
//! `fake-acp`, whose replies carry message ids and whose `session/fork`
//! reads the anchor as the real adapters do: a fork from a turn's end
//! continues that conversation in a new session; a rewind takes a session
//! back and a fork from a turn it left behind still finds that turn; an
//! agent that ignores the anchor is caught by the check; and the refusals.

use vessel_client::{NodeClient, SessionStream};
use vessel_proto::forking::{ForkSession, RewindSession};
use vessel_proto::{
    AgentEvent, ErrorCode, EventRecord, EventSeq, SessionEvent, SessionId, SessionStatus, TurnPoint,
};

use crate::agents::code;
use crate::support::{Fixture, ask, read_until, status_of, structured_spec, within};

const PROFILES: &str = r#"
[profiles.ignores]
harness = "claude"
env = { FAKE_ACP_FORK_IGNORES_POINT = "1" }

[profiles.breaks]
harness = "claude"
env = { FAKE_ACP_LOAD_FAILS = "1" }

[profiles.cannot]
harness = "claude"
env = { FAKE_ACP_NO_FORK = "1" }
"#;

/// A running session of `profile` (the default one without), its stream
/// and what it saw.
async fn running(
    fixture: &Fixture,
    client: &NodeClient,
    profile: Option<&str>,
) -> (SessionId, SessionStream, Vec<EventRecord>) {
    let mut spec = structured_spec(fixture, None);
    spec.profile = profile.map(|name| name.parse().unwrap());
    let id = client.create_session(spec).await.unwrap().id;
    let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
    let mut seen = Vec::new();
    read_until(&mut stream, &mut seen, |record| {
        status_of(record) == Some(&SessionStatus::Running)
    })
    .await;
    (id, stream, seen)
}

/// Whether the agent wrote a line starting with `start` on stderr.
fn said_on_stderr(seen: &[EventRecord], start: &str) -> bool {
    seen.iter().any(|record| {
        matches!(
            &record.event,
            SessionEvent::Agent {
                event: AgentEvent::HarnessLog { line }
            } if line.starts_with(start)
        )
    })
}

/// The last turn's end in `seen`: its record and its point.
fn last_point(seen: &[EventRecord]) -> (EventSeq, TurnPoint) {
    seen.iter()
        .rev()
        .find_map(|record| {
            if let SessionEvent::Agent {
                event:
                    AgentEvent::TurnEnded {
                        point: Some(point), ..
                    },
            } = &record.event
            {
                Some((record.seq, point.clone()))
            } else {
                None
            }
        })
        .expect("a turn's end with a point")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_fork_continues_a_turns_conversation_in_a_new_session() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let (source, mut stream, mut seen) = running(&fixture, &client, None).await;
        assert_eq!(
            ask(&client, &mut stream, &mut seen, source, "plain").await,
            "echo: plain"
        );
        let (first, point) = last_point(&seen);
        let _later = ask(&client, &mut stream, &mut seen, source, "plain").await;

        let fork = client
            .call(ForkSession {
                session: source,
                after: Some(first),
            })
            .await
            .unwrap();
        assert_ne!(fork.id, source);
        assert!(
            fork.title
                .as_ref()
                .is_some_and(|title| title.as_str().ends_with("(fork)")),
            "{:?}",
            fork.title
        );
        let mut forked = client.follow(fork.id, EventSeq::ZERO).await.unwrap();
        let mut forked_seen = Vec::new();
        read_until(&mut forked, &mut forked_seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        assert!(
            forked_seen.iter().any(|record| record.event
                == SessionEvent::Forked {
                    from: source,
                    after: Some(first),
                }),
            "{forked_seen:#?}"
        );
        // Its agent loaded the fork made at the first turn's message, and
        // the check's replay is not in its log.
        let expected = format!("fork~{}~{}", point.conversation, point.message);
        assert_eq!(
            ask(&client, &mut forked, &mut forked_seen, fork.id, "loaded").await,
            format!("loaded {expected}")
        );
        assert!(!forked_seen.iter().any(|record| matches!(
            &record.event,
            SessionEvent::Agent {
                event: AgentEvent::UserMessageChunk { .. }
            }
        )));
        assert_eq!(
            client
                .session(fork.id)
                .await
                .unwrap()
                .harness_session
                .unwrap()
                .as_str(),
            expected
        );
        // The source is untouched.
        assert_eq!(
            client.session(source).await.unwrap().harness_session,
            Some(point.conversation.clone())
        );
        // The whole conversation, between turns.
        let whole = client
            .call(ForkSession {
                session: source,
                after: None,
            })
            .await
            .unwrap();
        let mut whole_stream = client.follow(whole.id, EventSeq::ZERO).await.unwrap();
        let mut whole_seen = Vec::new();
        read_until(&mut whole_stream, &mut whole_seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        assert_eq!(
            client
                .session(whole.id)
                .await
                .unwrap()
                .harness_session
                .unwrap()
                .as_str(),
            format!("fork~{}~head", point.conversation)
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_rewind_goes_back_and_a_turn_it_left_behind_can_still_be_forked() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let (id, mut stream, mut seen) = running(&fixture, &client, None).await;
        let _first = ask(&client, &mut stream, &mut seen, id, "plain").await;
        let (first, first_point) = last_point(&seen);
        let _second = ask(&client, &mut stream, &mut seen, id, "plain").await;
        let (second, second_point) = last_point(&seen);
        assert_ne!(first_point.message, second_point.message);

        let rewound = client
            .call(RewindSession {
                session: id,
                after: first,
            })
            .await
            .unwrap();
        let conversation = format!("fork~{}~{}", first_point.conversation, first_point.message);
        assert_eq!(rewound.status, SessionStatus::Running);
        assert_eq!(
            rewound
                .harness_session
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            Some(conversation.clone())
        );
        read_until(&mut stream, &mut seen, |record| {
            matches!(record.event, SessionEvent::Rewound { .. })
        })
        .await;
        assert!(seen.iter().any(|record| matches!(
            &record.event,
            SessionEvent::Rewound { after, files_changed: 0, harness_session }
                if *after == first && harness_session.as_str() == conversation
        )));
        // The conversation goes on from the fork.
        assert_eq!(
            ask(&client, &mut stream, &mut seen, id, "loaded").await,
            format!("loaded {conversation}")
        );
        // The second turn, left behind, is in the old conversation, which
        // its point names: a fork from it still finds it.
        let fork = client
            .call(ForkSession {
                session: id,
                after: Some(second),
            })
            .await
            .unwrap();
        let mut forked = client.follow(fork.id, EventSeq::ZERO).await.unwrap();
        let mut forked_seen = Vec::new();
        read_until(&mut forked, &mut forked_seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        assert_eq!(
            client
                .session(fork.id)
                .await
                .unwrap()
                .harness_session
                .unwrap()
                .as_str(),
            format!(
                "fork~{}~{}",
                second_point.conversation, second_point.message
            )
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_that_ignores_the_point_is_caught_and_nothing_changes() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(fixture.data_dir().profiles_path(), PROFILES).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let (id, mut stream, mut seen) = running(&fixture, &client, Some("ignores")).await;
        let _first = ask(&client, &mut stream, &mut seen, id, "plain").await;
        let (first, _point) = last_point(&seen);
        let before = client.session(id).await.unwrap().harness_session;

        assert_eq!(
            code(
                client
                    .call(RewindSession {
                        session: id,
                        after: first
                    })
                    .await
            ),
            ErrorCode::Unsupported
        );
        let after = client.session(id).await.unwrap();
        assert_eq!(
            after.harness_session, before,
            "the old conversation goes on"
        );
        assert_eq!(
            ask(&client, &mut stream, &mut seen, id, "loaded").await,
            format!("loaded {}", before.unwrap())
        );
        assert!(
            !seen
                .iter()
                .any(|record| matches!(record.event, SessionEvent::Rewound { .. }))
        );
        // A fork of it is created, then fails its check: a failed session
        // that never held a conversation, its agent's fork deleted.
        let fork = client
            .call(ForkSession {
                session: id,
                after: Some(first),
            })
            .await
            .unwrap();
        let mut forked = client.follow(fork.id, EventSeq::ZERO).await.unwrap();
        let mut forked_seen = Vec::new();
        read_until(&mut forked, &mut forked_seen, |record| {
            matches!(status_of(record), Some(SessionStatus::Failed { .. }))
        })
        .await;
        let Some(SessionStatus::Failed {
            message,
            empty_fork,
        }) = forked_seen.iter().rev().find_map(status_of)
        else {
            panic!("failed");
        };
        assert_eq!(message, "the agent did not fork at the chosen point");
        assert!(empty_fork);
        assert!(
            said_on_stderr(&forked_seen, "fake-acp: deleted fork~"),
            "{forked_seen:#?}"
        );
        assert_eq!(client.session(fork.id).await.unwrap().harness_session, None);
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn what_is_not_a_point_and_what_cannot_fork_are_refused() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(fixture.data_dir().profiles_path(), PROFILES).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let (id, mut stream, mut seen) = running(&fixture, &client, None).await;
        let _first = ask(&client, &mut stream, &mut seen, id, "plain").await;
        let (first, _point) = last_point(&seen);
        // A record that is not a turn's end, one that does not exist, a
        // session that does not exist.
        assert_eq!(
            code(
                client
                    .call(RewindSession {
                        session: id,
                        after: EventSeq::ZERO
                    })
                    .await
            ),
            ErrorCode::Unsupported
        );
        assert_eq!(
            code(
                client
                    .call(RewindSession {
                        session: id,
                        after: EventSeq::new(first.get() + 1_000),
                    })
                    .await
            ),
            ErrorCode::NotFound
        );
        assert_eq!(
            code(
                client
                    .call(ForkSession {
                        session: SessionId::random(),
                        after: None,
                    })
                    .await
            ),
            ErrorCode::NotFound
        );
        // A turn that runs: no rewind, no fork of the whole conversation.
        client
            .send(id, crate::support::prompt("wait"))
            .await
            .unwrap();
        read_until(&mut stream, &mut seen, |record| {
            matches!(
                &record.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnStarted { .. }
                }
            )
        })
        .await;
        assert_eq!(
            code(
                client
                    .call(RewindSession {
                        session: id,
                        after: first
                    })
                    .await
            ),
            ErrorCode::Conflict
        );
        assert_eq!(
            code(
                client
                    .call(ForkSession {
                        session: id,
                        after: None
                    })
                    .await
            ),
            ErrorCode::Conflict
        );
        client.kill(id).await.unwrap();

        // An agent that does not offer `session/fork`.
        let (cannot, mut stream, mut seen) = running(&fixture, &client, Some("cannot")).await;
        let _first = ask(&client, &mut stream, &mut seen, cannot, "plain").await;
        let (first, _point) = last_point(&seen);
        assert_eq!(
            code(
                client
                    .call(RewindSession {
                        session: cannot,
                        after: first
                    })
                    .await
            ),
            ErrorCode::Unsupported
        );
        assert_eq!(
            ask(&client, &mut stream, &mut seen, cannot, "plain").await,
            "echo: plain",
            "it goes on"
        );
        node.stop().await.unwrap();
    })
    .await;
}

/// A rewind whose fork fails, after which the old conversation does not
/// start again either (the agent cannot load any conversation): the
/// session ends `failed` with a sentence that says to resume it, its
/// conversation id the one it had, and no agent left running.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_rewind_that_cannot_start_any_conversation_ends_the_session_failed() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(fixture.data_dir().profiles_path(), PROFILES).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let (id, mut stream, mut seen) = running(&fixture, &client, Some("breaks")).await;
        let _first = ask(&client, &mut stream, &mut seen, id, "plain").await;
        let (first, _point) = last_point(&seen);
        let before = client.session(id).await.unwrap().harness_session;
        assert!(before.is_some());

        assert_eq!(
            code(
                client
                    .call(RewindSession {
                        session: id,
                        after: first
                    })
                    .await
            ),
            ErrorCode::SpawnFailed
        );
        read_until(&mut stream, &mut seen, |record| {
            matches!(status_of(record), Some(SessionStatus::Failed { .. }))
        })
        .await;
        let Some(SessionStatus::Failed {
            message,
            empty_fork,
        }) = seen.iter().rev().find_map(status_of)
        else {
            panic!("failed");
        };
        assert!(
            message.starts_with(
                "the rewind failed and the agent could not start the conversation again"
            ) && message.ends_with("resume the session to go on from before the rewind"),
            "{message}"
        );
        assert!(!empty_fork);
        assert!(
            !seen
                .iter()
                .any(|record| matches!(record.event, SessionEvent::Rewound { .. }))
        );
        let after = client.session(id).await.unwrap();
        assert!(matches!(after.status, SessionStatus::Failed { .. }));
        assert_eq!(
            after.harness_session, before,
            "its conversation is the one it had"
        );
        node.stop().await.unwrap();
    })
    .await;
}
