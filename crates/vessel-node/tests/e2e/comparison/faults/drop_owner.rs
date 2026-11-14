//! Dropping the sole owner leaves passive recovery state, never a restart handle.
use super::*;
use vessel_client::comparison::{JobStatus, Phase};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owner_drop_keeps_started_targets_and_a_lost_prompt_as_uncertain() {
    within(async {
        let fixture = Fixture::new();
        let node = start_node(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&owner).await;
        let bridge = Bridge::open(&fixture, &node).await;
        let job = prepared(&bridge.client, project.id, &["fake-small", "fake-large"]).await;
        let mut updates = job.updates();
        let arrival = bridge.hold_after(Method::PromptSession, 1).await;
        {
            let sending = job.start(input("plain", None));
            tokio::pin!(sending);
            tokio::select! {
                reply = &mut sending => panic!("held prompt completed {reply:?}"),
                held = arrival => held.unwrap(),
            }
        }
        drop(job);
        let final_report = step("dropped owner settles an admitted prompt", async {
            loop {
                let report = updates.borrow_and_update().clone();
                if report.status == JobStatus::Cancelled {
                    return report;
                }
                updates.changed().await.unwrap();
            }
        })
        .await;
        assert!(matches!(
            final_report.targets[0].status,
            TargetStatus::Started
        ));
        assert!(
            matches!(&final_report.targets[1].status, TargetStatus::Uncertain(failure)
            if failure.phase == Phase::Prompt)
        );
        for target in &final_report.targets {
            assert_eq!(
                turns(&owner, target.resources.session.unwrap()).await.len(),
                1
            );
        }
        assert_eq!(bridge.count(Method::PromptSession).await, 2);
        bridge.stop().await;
        node.stop().await.unwrap();
    })
    .await;
}
