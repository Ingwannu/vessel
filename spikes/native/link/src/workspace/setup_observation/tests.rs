use super::*;
use std::sync::Mutex;
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
use vessel_app_view::workspace::git::RequestKeys;
use vessel_proto::{NodeId, SetupRunId, WorktreeId};

struct Fake {
    support: Support,
    calls: Mutex<Vec<Address>>,
}
impl Backend for Fake {
    fn support(&self, _address: &Address) -> Support {
        self.support
    }
    async fn read(
        &self,
        address: &Address,
        request: ReadSetupProgress,
    ) -> Result<SetupProgressReply, Problem> {
        assert_eq!(address.worktree, request.worktree);
        self.calls.lock().unwrap().push(address.clone());
        Ok(SetupProgressReply { progress: None })
    }
    async fn cancel(
        &self,
        address: &Address,
        request: CancelSetup,
    ) -> Result<CancelSetupOutcome, Problem> {
        assert_eq!(address.worktree, request.worktree);
        self.calls.lock().unwrap().push(address.clone());
        Ok(CancelSetupOutcome::Requested)
    }
}
fn address() -> Address {
    Address {
        context: ConnectionEpochs::default()
            .admit(MachineKey::from_book("fixture"), NodeId::random())
            .unwrap(),
        worktree: WorktreeId::random(),
    }
}
fn request(address: Address, operation: Operation) -> Ask {
    Ask {
        address,
        key: RequestKeys::default().issue().unwrap(),
        operation,
    }
}
#[tokio::test]
async fn unsupported_or_different_backing_cannot_poll_a_backend() {
    let address = address();
    let fake = Fake {
        support: Support::Unavailable,
        calls: Mutex::default(),
    };
    let read = request(
        address.clone(),
        Operation::Read(ReadSetupProgress {
            worktree: address.worktree,
            after: None,
            wait_ms: None,
        }),
    );
    let answer = run(&fake, read).await;
    assert_eq!(answer.ask.address, address);
    assert!(matches!(answer.result, Err(Problem::Unsent)));
    assert!(fake.calls.lock().unwrap().is_empty());
    let fake = Fake {
        support: Support::ObserveAndCancel,
        ..fake
    };
    let cancel = request(
        address,
        Operation::Cancel(CancelSetup {
            worktree: WorktreeId::random(),
            run: SetupRunId::random(),
        }),
    );
    assert!(matches!(
        run(&fake, cancel).await.result,
        Err(Problem::Unsent)
    ));
    assert!(fake.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn read_only_support_cannot_cancel_and_receipts_never_become_progress() {
    let address = address();
    let fake = Fake {
        support: Support::Observe,
        calls: Mutex::default(),
    };
    let read = request(
        address.clone(),
        Operation::Read(ReadSetupProgress {
            worktree: address.worktree,
            after: None,
            wait_ms: None,
        }),
    );
    assert!(matches!(
        run(&fake, read).await.result,
        Ok(Reply::Read(SetupProgressReply { progress: None }))
    ));
    let cancel = CancelSetup {
        worktree: address.worktree,
        run: SetupRunId::random(),
    };
    assert!(matches!(
        run(&fake, request(address.clone(), Operation::Cancel(cancel)))
            .await
            .result,
        Err(Problem::Unsent)
    ));
    assert_eq!(fake.calls.lock().unwrap().len(), 1);
    let fake = Fake {
        support: Support::ObserveAndCancel,
        ..fake
    };
    let answer = run(&fake, request(address.clone(), Operation::Cancel(cancel))).await;
    assert_eq!(answer.ask.address, address);
    assert!(matches!(answer.ask.operation, Operation::Cancel(request) if request == cancel));
    assert!(matches!(
        answer.result,
        Ok(Reply::Cancel(CancelSetupOutcome::Requested))
    ));
}

#[tokio::test]
async fn default_backend_preserves_the_original_cancel_without_polling_a_transport() {
    let address = address();
    let cancel = CancelSetup {
        worktree: address.worktree,
        run: SetupRunId::random(),
    };
    let ask = request(address.clone(), Operation::Cancel(cancel));
    let key = ask.key;
    let answer = unavailable(ask).await;
    assert_eq!(answer.ask.address, address);
    assert_eq!(answer.ask.key, key);
    assert!(matches!(answer.ask.operation, Operation::Cancel(original) if original == cancel));
    assert!(matches!(answer.result, Err(Problem::Unsent)));
}
