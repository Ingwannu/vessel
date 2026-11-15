//! The real draft producer must retain the node's same listing admission.

use std::future::Future;
use std::num::NonZeroUsize;
use std::task::Poll;
use std::time::Duration;

use vessel_harness::fakes::{Role, hold_lookup, install};
use vessel_harness::{HarnessEnv, Lookup, Profile};
use vessel_store::DataDir;
use vessel_testkit::Fake;

use super::*;
use crate::{Config, Node};

const STEP: Duration = Duration::from_secs(20);
const SAFETY_NET: Duration = Duration::from_secs(90);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn draft_caller_drop_keeps_the_original_node_listing_budget_until_physical_settlement() {
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async {
            let temp = tempfile::tempdir().unwrap();
            let bin = temp.path().join("bin");
            std::fs::create_dir(&bin).unwrap();
            drop(
                install(
                    &bin,
                    HarnessKind::ClaudeCode,
                    Role::Structured,
                    &Fake::Acp.path().unwrap(),
                )
                .unwrap(),
            );
            let capacity = NonZeroUsize::new(super::super::super::history::AT_ONCE).unwrap();
            let (lookup, mut barrier) = hold_lookup(
                Lookup::new([bin]),
                HarnessKind::ClaudeCode,
                Role::Structured,
                capacity,
            );
            let mut setup = Config::new(
                DataDir::new(temp.path().join("data")),
                HarnessEnv::new(temp.path().join("user")),
            );
            setup.bind = None;
            setup.harness_lookup = lookup;
            let node = Node::start(setup).await.unwrap();
            let service = Arc::clone(&node.secure.service);
            let mut held = Vec::new();
            for _ in 0..capacity.get() {
                let service = Arc::clone(&service);
                let cwd = temp.path().to_owned();
                let probing = tokio::spawn(async move {
                    let profile = Profile::default_of(&HarnessKind::ClaudeCode.into()).unwrap();
                    service.0.probe_command_draft(config(), profile, cwd).await
                });
                held.push(
                    tokio::time::timeout(STEP, barrier.arrived())
                        .await
                        .unwrap()
                        .unwrap(),
                );
                probing.abort();
                assert!(probing.await.unwrap_err().is_cancelled());
            }
            assert_eq!(service.0.listings.available_permits(), 0);
            let profile = Profile::default_of(&HarnessKind::ClaudeCode.into()).unwrap();
            let refused = service
                .0
                .probe_command_draft(config(), profile, temp.path().to_owned())
                .await;
            assert_eq!(refused.options.unwrap_err().code, ErrorCode::Unavailable);
            assert_eq!(barrier.entered().get(), capacity.get());
            let mut stopping = Box::pin(node.stop());
            assert!(
                std::future::poll_fn(|cx| Poll::Ready(stopping.as_mut().poll(cx).is_pending()))
                    .await
            );
            drop(held);
            tokio::time::timeout(STEP, stopping).await.unwrap().unwrap();
            assert_eq!(service.0.listings.available_permits(), capacity.get());
        }),
    )
    .await
    .unwrap();
}
