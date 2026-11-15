//! Two held real loads bound boot dispatch; a person can take a queued row.
use super::*;
use crate::recovery::Recovery;
const MANUAL: usize = Recovery::MAX_ATTEMPTS;
const LAST: usize = MANUAL.saturating_add(1);
const CANDIDATES: usize = LAST.saturating_add(1);
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn only_two_auto_loads_start_and_a_manual_queued_resume_consumes_its_intent() {
    vessel_testkit::within(
        CASE_DEADLINE,
        Box::pin(async {
            let f = Fixture::new();
            let node = f.start().await;
            let owner = f.client(&node).await;
            set(&node, true).await;
            let candidates = active_candidates(&node, &owner, &f).await;
            node.stop().await.unwrap();
            let gate = LoadGate::install(&f).await;
            let node = f.start().await;
            let owner = f.client(&node).await;
            let (first, mut held_first) = gate.arrive().await;
            let (second, mut held_second) = gate.arrive().await;
            let expected = std::collections::BTreeSet::from([
                candidates[0].2.as_str().to_owned(),
                candidates[1].2.as_str().to_owned(),
            ]);
            assert_eq!(
                std::collections::BTreeSet::from([first.clone(), second.clone()]),
                expected
            );
            assert_held_load_bound(&owner, &candidates).await;
            // A queued candidate can be resumed explicitly while both automatic
            // slots are held. Its ordinary resume consumes that old boot intent.
            let manual_id = candidates[MANUAL].1;
            let manual = {
                let owner = owner.clone();
                tokio::spawn(async move { owner.resume_session(manual_id).await })
            };
            let (manual_native, mut held_manual) = gate.arrive().await;
            assert_eq!(manual_native, candidates[MANUAL].2.as_str());
            held_manual.write_all(&[1]).await.unwrap();
            drop(held_manual);
            assert_eq!(
                manual.await.unwrap().unwrap().status,
                SessionStatus::Running
            );
            let first_candidate = candidates
                .iter()
                .find(|candidate| candidate.2.as_str() == first)
                .unwrap();
            let mut first_stream = owner
                .follow(first_candidate.1, first_candidate.3)
                .await
                .unwrap();
            held_first.write_all(&[1]).await.unwrap();
            drop(held_first);
            completed(&mut first_stream).await;
            let (fourth, mut held_fourth) = gate.arrive().await;
            assert_eq!(fourth, candidates[LAST].2.as_str());
            let second_candidate = candidates
                .iter()
                .find(|candidate| candidate.2.as_str() == second)
                .unwrap();
            let mut second_stream = owner
                .follow(second_candidate.1, second_candidate.3)
                .await
                .unwrap();
            let mut fourth_stream = owner
                .follow(candidates[LAST].1, candidates[LAST].3)
                .await
                .unwrap();
            held_second.write_all(&[1]).await.unwrap();
            drop(held_second);
            held_fourth.write_all(&[1]).await.unwrap();
            drop(held_fourth);
            for stream in [&mut second_stream, &mut fourth_stream] {
                completed(stream).await;
            }
            for (index, (_, id, _, _)) in candidates.iter().enumerate() {
                let expected = usize::from(index != MANUAL);
                assert_eq!(
                    owner
                        .read(*id, EventSeq::ZERO, EVENT_PAGE)
                        .await
                        .unwrap()
                        .records
                        .iter()
                        .filter(|record| started(record, "loaded"))
                        .count(),
                    expected
                );
            }
            node.stop().await.unwrap();
            settled_projection(&f).await;
        }),
    )
    .await
    .unwrap();
}

type Candidate = (
    chrono::DateTime<Utc>,
    SessionId,
    vessel_proto::HarnessSessionId,
    EventSeq,
);
async fn active_candidates(node: &Node, owner: &NodeClient, f: &Fixture) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    for _ in 0..CANDIDATES {
        let info = f.active(node, owner).await;
        let id = info.id;
        let records = owner
            .read(id, EventSeq::ZERO, EVENT_PAGE)
            .await
            .unwrap()
            .records;
        let active = records
            .iter()
            .find(|record| started(record, "wait"))
            .unwrap();
        candidates.push((active.at, id, info.harness_session.unwrap(), info.next_seq));
    }
    candidates.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.to_string().cmp(&b.1.to_string()))
    });

    candidates
}
async fn settled_projection(f: &Fixture) {
    let store = vessel_store::Store::open_with_restart_policy(
        &f.data,
        vessel_store::StoreConfig::default(),
        true,
    )
    .await
    .unwrap()
    .store;
    assert!(
        store.next_restart().await.unwrap().is_none(),
        "the idle manual run cannot revive the old queued intent"
    );
    store.close().await;
}

async fn completed(stream: &mut vessel_client::SessionStream) {
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
fn started(record: &EventRecord, text: &str) -> bool {
    matches!(&record.event, SessionEvent::Agent { event: AgentEvent::TurnStarted { content } } if content == &vec![ContentBlock::Text { text: text.to_owned() }])
}

async fn assert_held_load_bound(owner: &NodeClient, candidates: &[Candidate]) {
    let listed = owner.sessions().await.unwrap();
    assert_eq!(
        listed
            .iter()
            .filter(|info| info.status == SessionStatus::Starting)
            .count(),
        Recovery::MAX_ATTEMPTS
    );
    for (_, id, _, _) in candidates.iter().skip(Recovery::MAX_ATTEMPTS) {
        assert_eq!(
            owner.session(*id).await.unwrap().status,
            SessionStatus::Interrupted
        );
    }
}
