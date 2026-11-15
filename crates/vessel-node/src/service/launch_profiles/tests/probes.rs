//! A real fake ACP startup barrier fixes the reload/cancel ordering without sleeps.
use super::*;
use crate::principal::Principal;
use std::sync::Arc;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use vessel_proto::launch_profiles::*;
use vessel_proto::{ErrorDetail, Handler as _, HarnessKind};

async fn gated() -> (Fixture, tokio::net::TcpListener) {
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let text = format!(
        "[profiles.gated]\nharness = \"claude\"\nenv = {{ FAKE_ACP_START_GATE = \"{}\" }}\n",
        listener.local_addr().unwrap()
    );
    (Fixture::configured(Some(&text)).await, listener)
}
async fn accepted(listener: &tokio::net::TcpListener) -> (tokio::net::TcpStream, u32) {
    let (mut stream, _) = phase("actual fake reaches startup barrier", listener.accept())
        .await
        .unwrap();
    let pid = phase("barrier reports child pid", stream.read_u32())
        .await
        .unwrap();
    (stream, pid)
}
fn selection() -> LaunchSelection {
    LaunchSelection::new(
        HarnessKind::ClaudeCode.into(),
        Some("gated".parse().unwrap()),
        LaunchChoices::default(),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reload_during_actual_selected_probe_refuses_publication_then_recovers() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            for choices in [LaunchChoices::default(), bundle(&[("mode", "locked")])] {
                let (fixture, listener) = gated().await;
                let owner = fixture.context(Principal::Owner).await;
                let service = fixture.service();
                let context = owner.connection.acting_as(Principal::Owner);
                let running = Arc::clone(&service);
                let probe = tokio::spawn(async move {
                    running
                        .launch_profile_options(
                            &context,
                            GetLaunchProfileOptions {
                                selection: LaunchSelection::new(
                                    HarnessKind::ClaudeCode.into(),
                                    Some("gated".parse().unwrap()),
                                    choices,
                                ),
                            },
                        )
                        .await
                });
                let (mut gate, _pid) = accepted(&listener).await;
                let profiles = Arc::clone(&service.0.profiles);
                let _reloaded = phase(
                    "reload retained account generation",
                    tokio::task::spawn_blocking(move || profiles.reload()),
                )
                .await
                .unwrap()
                .unwrap();
                phase("release actual fake initialization", gate.write_all(&[1]))
                    .await
                    .unwrap();
                let error = phase("old probe cannot publish", probe)
                    .await
                    .unwrap()
                    .unwrap_err();
                assert!(matches!(
                    error.detail,
                    Some(ErrorDetail::LaunchProfile {
                        problem: LaunchProfileRefusal::ConfigurationChanged
                    })
                ));
                let fresh = phase(
                    "fresh default probe recovers",
                    service.launch_profile_options(
                        &owner.connection,
                        GetLaunchProfileOptions {
                            selection: LaunchSelection::new(
                                HarnessKind::ClaudeCode.into(),
                                None,
                                LaunchChoices::default(),
                            ),
                        },
                    ),
                )
                .await
                .unwrap();
                assert_eq!(fresh.account().kind(), LaunchAccountKind::Default);
                owner.finish().await;
                fixture.finish().await;
            }
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_caller_keeps_real_probe_until_node_stop_reaps_the_child() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let (fixture, listener) = gated().await;
            let owner = fixture.context(Principal::Owner).await;
            let service = fixture.service();
            let context = owner.connection.acting_as(Principal::Owner);
            let running = Arc::clone(&service);
            let probe = tokio::spawn(async move {
                running
                    .launch_profile_options(
                        &context,
                        GetLaunchProfileOptions {
                            selection: selection(),
                        },
                    )
                    .await
            });
            let (mut gate, pid) = accepted(&listener).await;
            probe.abort();
            assert!(
                phase("caller cancellation acknowledged", probe)
                    .await
                    .unwrap_err()
                    .is_cancelled()
            );
            #[cfg(target_os = "linux")]
            assert!(
                std::path::Path::new(&format!("/proc/{pid}")).exists(),
                "caller cancellation must not fabricate process cleanup"
            );
            #[cfg(not(target_os = "linux"))]
            let _pid = pid;
            // Closing the fixture's transport does not cancel the retained node job.
            owner.finish().await;
            fixture.finish().await;
            let mut byte = [0_u8; 1];
            assert_eq!(
                phase("killed child closes its held barrier", gate.read(&mut byte))
                    .await
                    .unwrap(),
                0
            );
            #[cfg(target_os = "linux")]
            assert!(
                !std::path::Path::new(&format!("/proc/{pid}")).exists(),
                "stop must reap the real fake child"
            );
            drop(service);
        }),
    )
    .await
    .unwrap();
}
