//! Terminal sources share generic transport and its final admission barriers.
//! Positive branches require the integrator's real assignment; pending does not
//! establish delivery/cancellation proof.
use super::*;
const SCENARIO_DEADLINE: std::time::Duration = std::time::Duration::from_secs(60);
use vessel_proto::notifications::{
    ClearTerminalAttention, ReportTerminalActivity, TerminalActivitySignal, TerminalAttention,
    TerminalAttentionRevision,
};
use vessel_proto::terminals::{CloseTerminal, CreateTerminal, TerminalInfo, TerminalPlace};

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
            assert!(receiver.inbox.try_recv().is_err());
        } else {
            let working = working.unwrap();
            let _finished = fixture
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
        }
        fixture.close().await;
        receiver.close().await;
    }))
    .await
    .unwrap();
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
