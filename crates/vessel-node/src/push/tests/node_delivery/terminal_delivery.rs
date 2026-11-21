//! Terminal sources share generic transport and its final admission barriers.
//! Positive branches require the integrator's real assignment; pending does not
//! establish delivery/cancellation proof.
use super::*;
const SCENARIO_DEADLINE: std::time::Duration = std::time::Duration::from_secs(60);
use vessel_proto::notifications::{
    ClearTerminalAttention, ReportTerminalActivity, TerminalActivity, TerminalActivityAuth,
    TerminalActivitySecret, TerminalActivitySignal, TerminalAttention, TerminalAttentionReason,
    TerminalAttentionRevision,
};
use vessel_proto::terminals::{
    CloseTerminal, CreateTerminal, KillTerminal, TerminalInfo, TerminalPlace,
};

impl Fixture {
    async fn terminal(&self) -> TerminalInfo {
        let session = self
            .owner
            .create_session(
                SessionSpec::new(
                    HarnessKind::ClaudeCode,
                    SessionMode::Structured {
                        model: None,
                        resume: None,
                        options: Vec::new(),
                    },
                )
                .with_cwd(self.temp.path().to_path_buf()),
            )
            .await
            .unwrap();
        self.owner
            .call(CreateTerminal {
                place: TerminalPlace::Session {
                    session: session.id,
                },
                size: None,
                title: None,
            })
            .await
            .unwrap()
    }
}
fn fake_shell(config: &mut Config) {
    config.terminals.shell = Some(crate::terminals::Shell {
        program: Fake::Tui.path().unwrap(),
        args: Vec::new(),
    });
}
async fn activity(
    owner: &NodeClient,
    state: TerminalAttention,
    signal: TerminalActivitySignal,
) -> TerminalAttention {
    owner
        .report_terminal_activity(ReportTerminalActivity {
            terminal: state.terminal,
            revision: Some(state.revision),
            signal,
        })
        .await
        .unwrap()
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn terminal_attention_uses_the_same_encrypted_generic_delivery_after_assignment() {
    Box::pin(vessel_testkit::within(SCENARIO_DEADLINE, async {
        let mut receiver = Receiver::start(StatusCode::CREATED).await;
        let fixture = Fixture::start_with(receiver.url.clone(), fake_shell).await;
        let (_, device) = fixture.pair().await;
        let (private, subscription) = subscriber();
        let _lease = device.subscribe_push(subscription.clone()).await.unwrap();
        let terminal = fixture.terminal().await;
        let report = ReportTerminalActivity {
            terminal: terminal.id,
            revision: Some(
                terminal
                    .attention
                    .map_or(TerminalAttentionRevision::new(0), |state| state.revision),
            ),
            signal: TerminalActivitySignal::Working,
        };
        let working = fixture.owner.report_terminal_activity(report).await;
        if vessel_proto::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL.is_none() {
            assert!(matches!(
                working,
                Err(vessel_client::CallError::UnassignedProtocol { .. })
            ));
            // The node's own path, which no client reaches while the feature
            // is unassigned (owed item 5 of the node review of TRACKER 815):
            // the terminal's handle goes Working, then Idle, a Finished
            // revision. The worker wakes for it and pushes nothing.
            let handle = fixture
                .node
                .secure
                .service
                .0
                .terminals()
                .get(terminal.id)
                .unwrap()
                .handle;
            // The created terminal's reply has its attention cut, as every
            // peer's has while unassigned; the node's own handle holds it.
            let start = handle.info().attention.unwrap();
            let working = handle
                .report_activity(start.revision, TerminalActivitySignal::Working)
                .unwrap();
            let finished = handle
                .report_activity(working.revision, TerminalActivitySignal::Idle)
                .unwrap();
            assert!(
                matches!(
                    finished.activity,
                    TerminalActivity::Attention {
                        reason: TerminalAttentionReason::Finished
                    }
                ),
                "{finished:?}"
            );
            assert!(
                tokio::time::timeout(std::time::Duration::from_secs(2), receiver.inbox.recv())
                    .await
                    .is_err(),
                "no Web Push while the feature is unassigned"
            );
        } else {
            let working = working.unwrap();
            let finished = fixture
                .owner
                .report_terminal_activity(ReportTerminalActivity {
                    terminal: terminal.id,
                    revision: Some(working.revision),
                    signal: TerminalActivitySignal::Idle,
                })
                .await
                .unwrap();
            let message = step("terminal generic encrypted push", receiver.inbox.recv())
                .await
                .unwrap();
            let decoded = web_push_native::decrypt(
                message.body.to_vec(),
                &private,
                &web_push_native::Auth::from(*subscription.auth.expose()),
            )
            .unwrap();
            assert_eq!(decoded, crate::push::crypto::PAYLOAD);
            finishes_again_inside_the_gap(&fixture.owner, &mut receiver, terminal.id, finished)
                .await;
        }
        fixture.close().await;
        receiver.close().await;
    }))
    .await
    .unwrap();
}

/// The worker recorded the terminal as pushed in its burst, which opens the
/// terminal's gap (`push/terminal/spacing.rs`): a turn that finishes again at
/// once is not pushed again (ADR 0089, owed at assignment, beside the IPC
/// route's test in `vessel-cli/tests/cli/terminals/hooks.rs`).
async fn finishes_again_inside_the_gap(
    owner: &NodeClient,
    receiver: &mut Receiver,
    terminal: vessel_proto::TerminalId,
    finished: TerminalAttention,
) {
    let again = owner
        .report_terminal_activity(ReportTerminalActivity {
            terminal,
            revision: Some(finished.revision),
            signal: TerminalActivitySignal::Working,
        })
        .await
        .unwrap();
    let _finished_again = owner
        .report_terminal_activity(ReportTerminalActivity {
            terminal,
            revision: Some(again.revision),
            signal: TerminalActivitySignal::Idle,
        })
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(2), receiver.inbox.recv())
            .await
            .is_err(),
        "a second finish inside the terminal's gap is not pushed"
    );
}

#[derive(Clone, Copy)]
enum DuringPreparation {
    Clear,
    NewWorking,
    Remove,
    Focus,
    Revoke,
    Renew,
}
async fn cancels_before_admission(stage: crate::push::PreparedStage, action: DuringPreparation) {
    let mut receiver = Receiver::start(StatusCode::CREATED).await;
    let (ready, mut paused) = mpsc::channel(1);
    let (finished, mut completed) = mpsc::channel(1);
    let hook = std::sync::Arc::new(crate::push::Preparation {
        stages: vec![stage],
        ready,
        finished,
    });
    let fixture = Fixture::start_with(receiver.url.clone(), |config| {
        fake_shell(config);
        config.push_preparation = Some(hook);
    })
    .await;
    let (id, device) = fixture.pair().await;
    let (_, subscription) = subscriber();
    let _lease = device.subscribe_push(subscription.clone()).await.unwrap();
    let terminal = fixture.terminal().await;
    let needs = fixture
        .owner
        .report_terminal_activity(ReportTerminalActivity {
            terminal: terminal.id,
            revision: Some(terminal.attention.unwrap().revision),
            signal: TerminalActivitySignal::NeedsInput,
        })
        .await
        .unwrap();
    let release = step("terminal push preparation barrier", paused.recv())
        .await
        .unwrap();
    match action {
        DuringPreparation::Clear => {
            let _idle = fixture
                .owner
                .clear_terminal_attention(ClearTerminalAttention {
                    terminal: terminal.id,
                    revision: needs.revision,
                })
                .await
                .unwrap();
        }
        DuringPreparation::NewWorking => {
            let _working = activity(&fixture.owner, needs, TerminalActivitySignal::Working).await;
        }
        DuringPreparation::Remove => {
            let _closed = fixture
                .owner
                .call(CloseTerminal {
                    terminal: terminal.id,
                })
                .await
                .unwrap();
        }
        DuringPreparation::Focus => fixture
            .owner
            .report_push_presence(ReportPushPresence {
                idle_age: None,
                notifies: true,
                focus: PushFocus::Terminal {
                    terminal: terminal.id,
                },
                activity: PushActivity::Active,
            })
            .await
            .unwrap(),
        DuringPreparation::Revoke => fixture.owner.revoke_device(id).await.unwrap(),
        DuringPreparation::Renew => {
            let _lease = device.subscribe_push(subscription).await.unwrap();
        }
    }
    release.send(()).unwrap();
    assert!(
        step(
            "terminal attempt finishes before shutdown",
            completed.recv()
        )
        .await
        .unwrap()
        .is_none()
    );
    assert!(receiver.inbox.try_recv().is_err());
    fixture.close().await;
    receiver.close().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn terminal_state_recipient_and_lease_changes_cancel_after_crypto_and_transport() {
    if vessel_proto::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL.is_none() {
        // Do not assign a test protocol or silently fabricate runtime support.
        // The paired positive/refusal fixture exercises pending SDK admission.
        assert!(
            vessel_proto::Method::ReportTerminalActivity
                .since()
                .is_none()
        );
        return;
    }
    Box::pin(vessel_testkit::within(SCENARIO_DEADLINE, async {
        for stage in [
            crate::push::PreparedStage::Crypto,
            crate::push::PreparedStage::Transport,
        ] {
            for action in [
                DuringPreparation::Clear,
                DuringPreparation::NewWorking,
                DuringPreparation::Remove,
                DuringPreparation::Focus,
                DuringPreparation::Revoke,
                DuringPreparation::Renew,
            ] {
                cancels_before_admission(stage, action).await;
            }
        }
    }))
    .await
    .unwrap();
}

/// A terminal's activity capability admits that terminal with its own
/// secret only, and closes when the terminal is forgotten or ends (owed
/// item 5 of the node review of TRACKER 815). The node gives no capability
/// while the feature is unassigned, so the test gives one; the IPC route
/// itself is checked by the wire test with the assigned protocol.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_terminal_capability_admits_only_its_terminal_and_closes_with_it() {
    Box::pin(vessel_testkit::within(SCENARIO_DEADLINE, async {
        let receiver = Receiver::start(StatusCode::CREATED).await;
        let fixture = Fixture::start_with(receiver.url.clone(), fake_shell).await;
        let terminals = std::sync::Arc::clone(fixture.node.secure.service.0.terminals());
        let closed = fixture.terminal().await;
        let killed = fixture.terminal().await;
        let auth = |terminal, byte| TerminalActivityAuth {
            terminal,
            secret: TerminalActivitySecret::from_bytes([byte; 32]),
        };
        assert!(terminals.give_producer(closed.id, &auth(closed.id, 1).secret));
        assert!(terminals.give_producer(killed.id, &auth(killed.id, 2).secret));
        if vessel_proto::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL.is_none() {
            assert!(
                terminals.admit_producer(&auth(closed.id, 1)).is_none(),
                "the gate refuses even the right secret while unassigned"
            );
        }
        let (_handle, closed_capability) = terminals.admit_assigned(&auth(closed.id, 1)).unwrap();
        assert!(
            terminals.admit_assigned(&auth(closed.id, 2)).is_none(),
            "another terminal's secret"
        );
        assert!(
            terminals.admit_assigned(&auth(killed.id, 1)).is_none(),
            "a secret for another terminal"
        );
        let (_handle, killed_capability) = terminals.admit_assigned(&auth(killed.id, 2)).unwrap();

        let _closed = fixture
            .owner
            .call(CloseTerminal {
                terminal: closed.id,
            })
            .await
            .unwrap();
        assert!(closed_capability.is_cancelled(), "forgotten, it closes");
        assert!(terminals.admit_assigned(&auth(closed.id, 1)).is_none());

        let _killed = fixture
            .owner
            .call(KillTerminal {
                terminal: killed.id,
            })
            .await
            .unwrap();
        step(
            "the capability closes when its terminal ends",
            killed_capability.cancelled(),
        )
        .await;
        assert!(terminals.admit_assigned(&auth(killed.id, 2)).is_none());
        fixture.close().await;
        receiver.close().await;
    }))
    .await
    .unwrap();
}
