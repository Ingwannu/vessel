//! Two held real loads bound boot dispatch; a person can take a queued row.
use super::{fixture, gated::Gate, policy, running, started, text};
use crate::support::{agent_of, read_all, read_until, within};
use tokio::io::AsyncWriteExt as _;
use vessel_proto::{AgentEvent, Delivery, SessionStatus};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn only_two_auto_loads_start_and_a_manual_queued_resume_consumes_its_intent() {
    within(async {
        let f = fixture();
        let node = f.start().await;
        let owner = f.connect(&node).await;
        policy(&owner, true, Some("loaded")).await;
        let candidates = active_candidates(&owner, &f).await;
        node.stop().await.unwrap();
        let gate = Gate::install(&f).await;
        let node = f.start().await;
        let owner = f.connect(&node).await;
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
        let listed = owner.sessions().await.unwrap();
        assert_eq!(
            listed
                .iter()
                .filter(|info| info.status == SessionStatus::Starting)
                .count(),
            2
        );
        for (_, id, _, _) in candidates.iter().skip(2) {
            assert_eq!(
                owner.session(*id).await.unwrap().status,
                SessionStatus::Interrupted
            );
        }
        // A queued candidate can be resumed explicitly while both automatic
        // slots are held. Its ordinary resume consumes that old boot intent.
        let manual_id = candidates[2].1;
        let manual = {
            let owner = owner.clone();
            tokio::spawn(async move { owner.resume_session(manual_id).await })
        };
        let (manual_native, mut held_manual) = gate.arrive().await;
        assert_eq!(manual_native, candidates[2].2.as_str());
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
        assert_eq!(fourth, candidates[3].2.as_str());
        let second_candidate = candidates
            .iter()
            .find(|candidate| candidate.2.as_str() == second)
            .unwrap();
        let mut second_stream = owner
            .follow(second_candidate.1, second_candidate.3)
            .await
            .unwrap();
        let mut fourth_stream = owner
            .follow(candidates[3].1, candidates[3].3)
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
            let expected = usize::from(index != 2);
            assert_eq!(
                read_all(&owner, *id)
                    .await
                    .iter()
                    .filter(|record| started(record, "loaded"))
                    .count(),
                expected
            );
        }
        node.stop().await.unwrap();
        settled_projection(&f).await;
    })
    .await;
}

type Candidate = (
    chrono::DateTime<chrono::Utc>,
    vessel_proto::SessionId,
    vessel_proto::HarnessSessionId,
    vessel_proto::EventSeq,
);
async fn active_candidates(
    owner: &vessel_client::NodeClient,
    f: &crate::support::Fixture,
) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    for _ in 0..4 {
        let id = running(owner, f).await;
        assert_eq!(
            owner
                .prompt(id, text("wait"), Delivery::Start)
                .await
                .unwrap(),
            vessel_proto::Delivered::Started
        );
        let info = owner.session(id).await.unwrap();
        let records = read_all(owner, id).await;
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
async fn settled_projection(f: &crate::support::Fixture) {
    let store = vessel_store::Store::open_with_restart_policy(
        &f.data_dir(),
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
    let mut seen = Vec::new();
    read_until(stream, &mut seen, |record| started(record, "loaded")).await;
    read_until(stream, &mut seen, |record| {
        matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))
    })
    .await;
}
