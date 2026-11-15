//! Runtime capacity is checked through concurrent real provider operations.
use std::sync::Arc;

use super::{MAX_PULLS, Merge, Pull, decode, document, repo};
use crate::{Forge as _, Opened, Opening};
use vessel_proto::forge::{PullNumber, PullState};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_opens_share_one_capacity_admission() {
    let mut raw = document();
    let preserved = raw.repositories[0].pulls[0].pull.clone();
    for number in 1..MAX_PULLS.saturating_sub(1) {
        let mut pull = preserved.clone();
        pull.summary.number = PullNumber::new(u64::try_from(number).unwrap()).unwrap();
        pull.summary.state = PullState::Closed;
        raw.repositories[0].pulls.push(Pull {
            pull,
            merge: Merge::Normal,
        });
    }
    let forge = decode(&raw).into_forge();
    let mut jobs = tokio::task::JoinSet::new();
    for number in 0..16 {
        let forge = Arc::clone(&forge);
        let opening = Opening {
            head: format!("parallel-{number}").parse().unwrap(),
            base: "main".parse().unwrap(),
            title: "A concurrent fixture pull".parse().unwrap(),
            body: "fixture".parse().unwrap(),
            draft: false,
        };
        let _task = jobs.spawn(async move { forge.open(&repo(), &opening).await });
    }
    let mut created = 0_usize;
    while let Some(answer) = jobs.join_next().await {
        match answer.unwrap().unwrap() {
            Opened::Created(_) => created = created.saturating_add(1),
            Opened::Rejected(message) => {
                assert_eq!(message, "the pull request limit has been reached");
            }
            other @ (Opened::Exists | Opened::NoCommits) => {
                panic!("unexpected open outcome: {other:?}")
            }
        }
    }
    assert_eq!(created, 1);
    assert_eq!(forge.state().pulls.len(), MAX_PULLS);
    assert_eq!(
        forge.get(&repo(), preserved.summary.number).unwrap().pull,
        preserved
    );
}
