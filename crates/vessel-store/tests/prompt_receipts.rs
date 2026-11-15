//! Writer commit acknowledgements and restart replay, without a provider process.

use fixtures::{data_dir, new_session, open};
use vessel_proto::timeline::{ClientMessageId, SubmitPrompt, SubmitPromptReply};
use vessel_proto::{ContentBlock, Creator, QueueEntryId, SessionId};
use vessel_store::{ReceiptAdmission, Store, StoreConfig, StoreError};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(dead_code, reason = "only session and directory builders are needed")]
mod fixtures;

fn prompt(session: SessionId) -> SubmitPrompt {
    SubmitPrompt {
        native_command: None,
        session,
        message: ClientMessageId::random(),
        content: vec![ContentBlock::Text { text: "run".into() }],
        delivery: vessel_proto::timeline::SubmissionDelivery::Queue,
    }
}

#[tokio::test]
async fn committed_pending_and_accepted_outcomes_survive_restart() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let info = new_session(0);
    store.create_session(info.clone()).await.unwrap();
    let pending = prompt(info.id);
    let accepted = prompt(info.id);
    let ReceiptAdmission::New(pending_token) = store
        .admit_prompt(pending.clone(), Creator::Owner)
        .await
        .unwrap()
    else {
        panic!("fresh pending admission");
    };
    drop(pending_token);
    let ReceiptAdmission::New(token) = store
        .admit_prompt(accepted.clone(), Creator::Owner)
        .await
        .unwrap()
    else {
        panic!("fresh accepted admission");
    };
    let delivered = vessel_proto::timeline::SubmissionAcceptedKind::Prompt {
        disposition: vessel_proto::timeline::PromptDisposition::Queued {
            entry: QueueEntryId::random(),
        },
    };
    let receipt = store.accept_prompt(token, delivered).await.unwrap();
    store.close().await;
    drop(store);
    let store = open(&dir, StoreConfig::default()).await;
    assert!(
        matches!(store.admit_prompt(pending.clone(), Creator::Owner).await.unwrap(),
        ReceiptAdmission::Existing(SubmitPromptReply::OutcomeUnknown { message }) if message == pending.message)
    );
    let replay = store
        .admit_prompt(accepted.clone(), Creator::Owner)
        .await
        .unwrap();
    assert!(
        matches!(replay, ReceiptAdmission::Existing(SubmitPromptReply::Accepted { receipt: kept }) if kept == receipt)
    );
    let mut changed = accepted;
    changed.delivery = vessel_proto::timeline::SubmissionDelivery::Start;
    assert!(matches!(
        store.admit_prompt(changed, Creator::Owner).await.unwrap(),
        ReceiptAdmission::Existing(SubmitPromptReply::KeyConflict { .. })
    ));
    store.close().await;
}

#[tokio::test]
async fn empty_or_unknown_input_creates_no_replay_state() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let info = new_session(0);
    store.create_session(info.clone()).await.unwrap();
    let mut request = prompt(info.id);
    let original = request.clone();
    request.content.clear();
    assert!(matches!(
        store.admit_prompt(request, Creator::Owner).await,
        Err(StoreError::EmptyIdentifiedPrompt)
    ));
    assert!(matches!(
        store.admit_prompt(original, Creator::Owner).await.unwrap(),
        ReceiptAdmission::New(_)
    ));
    let unknown = SessionId::random();
    assert!(
        matches!(store.admit_prompt(prompt(unknown), Creator::Owner).await, Err(StoreError::UnknownSession(id)) if id == unknown)
    );
    store.close().await;
}

#[tokio::test]
async fn corrupt_accounting_refuses_open_without_discarding_pending_keys() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let info = new_session(0);
    store.create_session(info.clone()).await.unwrap();
    let request = prompt(info.id);
    assert!(matches!(
        store.admit_prompt(request, Creator::Owner).await.unwrap(),
        ReceiptAdmission::New(_)
    ));
    store.close().await;
    drop(store);
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    let _setup = connection
        .execute("UPDATE timeline_admission_meta SET records=0", [])
        .unwrap();
    drop(connection);
    assert!(matches!(
        Store::open(&dir, StoreConfig::default()).await,
        Err(StoreError::AdmissionIntegrity)
    ));
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    let retained: i64 = connection
        .query_row("SELECT count(*) FROM timeline_admissions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(retained, 1);
}

#[tokio::test]
async fn refusal_replays_after_restart_and_changed_native_intent_conflicts() {
    use vessel_proto::timeline::{SubmissionDelivery, SubmissionRefusal};
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let info = new_session(0);
    store.create_session(info.clone()).await.unwrap();
    let mut request = prompt(info.id);
    request.delivery = SubmissionDelivery::SteerOrInterrupt;
    let ReceiptAdmission::New(token) = store
        .admit_prompt(request.clone(), Creator::Owner)
        .await
        .unwrap()
    else {
        panic!("fresh");
    };
    let refused = store
        .refuse_prompt(token, SubmissionRefusal::TurnRunning)
        .await
        .unwrap();
    store.close().await;
    drop(store);
    let store = open(&dir, StoreConfig::default()).await;
    assert!(
        matches!(store.admit_prompt(request.clone(), Creator::Owner).await.unwrap(), ReceiptAdmission::Existing(replay) if replay == refused)
    );
    request.native_command = Some(vessel_proto::native_commands::NativeCommandIntent::Compact);
    assert!(matches!(
        store.admit_prompt(request, Creator::Owner).await.unwrap(),
        ReceiptAdmission::Existing(SubmitPromptReply::KeyConflict { .. })
    ));
    store.close().await;
}
