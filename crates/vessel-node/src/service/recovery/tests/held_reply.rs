//! Real provider response and real committed Arm ACK held independently.
//! The ACP driver accepts at send, so the held Store ACK is what keeps the
//! session delivery reply unresolved; no fake successful ACK is substituted.
use super::*;
use crate::persist::reply_gate;
use crate::recovery::{Claim, Recovery};
use tokio::net::TcpStream;
use vessel_proto::SessionInfo;

const PROMPT: &str = "plain";
const CANDIDATES: usize = Recovery::MAX_ATTEMPTS.saturating_add(1);
struct Scenario {
    fixture: Fixture,
    node: Node,
    owner: NodeClient,
    provider: LoadGate,
    candidates: Vec<(SessionInfo, reply_gate::Registration)>,
}
struct Held {
    session: SessionId,
    ack: reply_gate::Held,
    provider: TcpStream,
}
impl Scenario {
    async fn new() -> Self {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let owner = fixture.client(&node).await;
        let mut policy = change(true);
        policy.restart_recovery.as_mut().unwrap().prompt = Some(PROMPT.parse().unwrap());
        let installed = node.secure.service.0.settings.change(policy).await.unwrap();
        assert!(installed.restart_recovery.enabled);
        assert_eq!(installed.restart_recovery.prompt.as_str(), PROMPT);
        let mut originals = Vec::new();
        for _ in 0..CANDIDATES {
            originals.push(fixture.active(&node, &owner).await);
        }
        node.stop().await.unwrap();
        let candidates = originals
            .into_iter()
            .map(|info| {
                let arm = reply_gate::arm(info.id);
                (info, arm)
            })
            .collect();
        let provider = LoadGate::install_prompt(&fixture).await;
        let node = fixture.start().await;
        let owner = fixture.client(&node).await;
        Self {
            fixture,
            node,
            owner,
            provider,
            candidates,
        }
    }
    async fn held(&mut self) -> Held {
        let (native, provider) = self.provider.arrive().await;
        let (info, registration) = self
            .candidates
            .iter_mut()
            .find(|(info, _)| info.harness_session.as_ref().unwrap().as_str() == native)
            .unwrap();
        let ack = tokio::time::timeout(WAIT, registration.arrived())
            .await
            .unwrap();
        Held {
            session: info.id,
            ack,
            provider,
        }
    }
    fn full(&self) {
        assert!(
            matches!(
                self.node
                    .secure
                    .service
                    .0
                    .recovery
                    .claim(SessionId::random()),
                Claim::Wait(_)
            ),
            "unresolved real replies retain the actual attempt slots"
        );
    }
    async fn finish(&self, held: Held) {
        let Held {
            session,
            ack,
            mut provider,
        } = held;
        let through = self
            .node
            .secure
            .service
            .0
            .sessions
            .store()
            .session(session)
            .await
            .unwrap()
            .unwrap()
            .next_seq;
        let mut stream = self.owner.follow(session, through).await.unwrap();
        drop(ack);
        provider.write_all(&[1]).await.unwrap();
        loop {
            let record = stream.next().await.unwrap().unwrap();
            if matches!(
                record.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded { .. }
                }
            ) {
                break;
            }
        }
    }
    async fn killed(&self, first: Held) -> SessionHandle {
        let sessions = &self.node.secure.service.0.sessions;
        let mut ended = sessions.ends();
        let original = sessions.runtime_handle(first.session).unwrap();
        let generation = original.restart_generation();
        // Real Guarded RPC must commit suppression before either reply release.
        tokio::time::timeout(WAIT, self.owner.kill(first.session))
            .await
            .expect("a person's kill must not wait for the recovery prompt reply")
            .unwrap();
        assert!(original.restart_generation() > generation);
        self.full();
        assert!(sessions.store().next_restart().await.unwrap().is_some());
        let first_id = first.session;
        let Held { ack, provider, .. } = first;
        drop(ack); // Actual delivery settlement; no new capacity before this.
        let _ended = tokio::time::timeout(WAIT, original.ended()).await.unwrap();
        tokio::time::timeout(WAIT, async {
            while ended.recv().await.unwrap() != first_id {}
        })
        .await
        .unwrap();
        assert!(matches!(
            sessions.find(first_id).await.unwrap(),
            crate::sessions::Found::Stored(_)
        ));
        // Actual process ended before the provider reply was ever released.
        drop(provider);
        original
    }
    async fn no_retry(self, session: SessionId) {
        self.node.stop().await.unwrap();
        let node = self.fixture.start().await;
        let owner = self.fixture.client(&node).await;
        done(&node).await;
        assert_eq!(
            owner.session(session).await.unwrap().status,
            SessionStatus::Interrupted
        );
        let kept = owner
            .read(session, EventSeq::ZERO, EVENT_PAGE)
            .await
            .unwrap();
        assert_eq!(
            kept.records
                .iter()
                .filter(|record| matches!(&record.event,
                    SessionEvent::Agent { event: AgentEvent::TurnStarted { content } }
                        if content == &vec![ContentBlock::Text { text: PROMPT.into() }]
                ))
                .count(),
            1,
            "no retry or reconstructed recovery intent"
        );
        node.stop().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn guarded_kill_reaches_suppression_while_the_admitted_delivery_reply_is_held() {
    vessel_testkit::within(
        CASE_DEADLINE,
        Box::pin(async {
            let mut s = Scenario::new().await;
            let first = s.held().await;
            let second = s.held().await;
            s.full();
            let original = s.killed(first).await;
            let session = original.id();
            let third = s.held().await;
            assert_ne!(third.session, session);
            assert_ne!(third.session, second.session);
            s.full();
            let replacement = tokio::time::timeout(WAIT, s.owner.resume_session(session))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(replacement.status, SessionStatus::Running);
            let replacement_run = s
                .node
                .secure
                .service
                .0
                .sessions
                .runtime_handle(session)
                .unwrap();
            // Execute the old pinned target after takeover, not a new id lookup.
            s.node
                .secure
                .service
                .0
                .sessions
                .kill_pinned(original)
                .await
                .unwrap();
            s.finish(second).await;
            s.finish(third).await;
            done(&s.node).await;
            assert!(
                s.node
                    .secure
                    .service
                    .0
                    .sessions
                    .runtime_handle(session)
                    .unwrap()
                    .same_run(&replacement_run)
            );
            assert_eq!(
                s.owner.session(session).await.unwrap().status,
                SessionStatus::Running
            );
            s.no_retry(session).await;
        }),
    )
    .await
    .unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn off_does_not_kill_a_prompt_while_its_real_delivery_reply_is_held() {
    vessel_testkit::within(
        CASE_DEADLINE,
        Box::pin(async {
            let mut s = Scenario::new().await;
            let first = s.held().await;
            let second = s.held().await;
            s.full();
            set(&s.node, false).await;
            s.full();
            for session in [first.session, second.session] {
                assert_eq!(
                    s.owner.session(session).await.unwrap().status,
                    SessionStatus::Running
                );
            }
            assert!(
                s.node
                    .secure
                    .service
                    .0
                    .sessions
                    .store()
                    .next_restart()
                    .await
                    .unwrap()
                    .is_none()
            );
            s.finish(first).await;
            s.finish(second).await;
            done(&s.node).await;
            s.node.stop().await.unwrap();
        }),
    )
    .await
    .unwrap();
}
