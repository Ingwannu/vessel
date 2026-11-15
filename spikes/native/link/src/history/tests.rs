//! Existing calls are exercised through the in-memory RPC transport.

#[path = "peer.rs"]
pub(crate) mod peer;
use super::*;
use std::path::PathBuf;
use vessel_app_view::{
    devices::{MachineKey, Tickets},
    history::{Entry, Scope},
};
use vessel_proto::{Call, HarnessKind, HarnessSessionId, SessionInfo, SessionMode};

fn batch(node: vessel_proto::NodeId) -> Batch {
    let scope = Scope {
        context: vessel_app_view::connection::ConnectionEpochs::default()
            .admit(MachineKey::from_book("fixture"), node)
            .unwrap(),
        machine: MachineKey::from_book("fixture"),
        harness: HarnessKind::Codex.into(),
        profile: Some("other-account".parse().unwrap()),
        cwd: None,
    };
    let entries = ["one", "two"]
        .into_iter()
        .map(|id| Entry {
            id: HarnessSessionId::new(id),
            cwd: PathBuf::from(format!("/work/{id}")),
            title: Some(id.parse().unwrap()),
            updated_at: None,
            in_vessel: None,
        })
        .collect();
    Batch {
        node,
        scope,
        ticket: Tickets::default().next_ticket().unwrap(),
        entries,
    }
}

#[tokio::test]
async fn batch_resumes_each_native_id_in_its_original_directory_and_account() {
    let peer = peer::connect(vec![]).await;
    let batch = batch(peer.client.node().id);
    let Reply::Imported { outcomes, .. } =
        answer(&peer::admitted(&peer).2, Ask::Import(batch.clone())).await
    else {
        panic!("batch receipt");
    };
    assert_eq!(outcomes.len(), batch.entries.len());
    assert!(outcomes.iter().all(|outcome| outcome.result.is_ok()));
    let calls = peer.calls.lock().unwrap();
    assert_eq!(calls.len(), batch.entries.len());
    for (call, entry) in calls.iter().zip(&batch.entries) {
        let Call::CreateSession(request) = call else {
            panic!("only existing session.create is used");
        };
        let spec = &request.spec;
        assert_eq!(spec.profile, batch.scope.profile);
        assert_eq!(spec.cwd.as_ref(), Some(&entry.cwd));
        assert_eq!(spec.title, entry.title);
        assert!(
            matches!(&spec.mode, SessionMode::Structured { resume: Some(id), .. } if id == &entry.id)
        );
    }
}

#[tokio::test]
async fn definite_refusals_are_attributed_to_their_row_without_losing_successes() {
    let peer = peer::connect(vec![
        peer::Behaviour::Refuse(vessel_proto::ErrorCode::Conflict),
        peer::Behaviour::Success,
    ])
    .await;
    let Reply::Imported { outcomes, .. } = answer(
        &peer::admitted(&peer).2,
        Ask::Import(batch(peer.client.node().id)),
    )
    .await
    else {
        panic!("batch");
    };
    assert!(matches!(
        outcomes.first().unwrap().result,
        Err(Problem::Refused(vessel_proto::ErrorCode::Conflict))
    ));
    assert!(outcomes.last().unwrap().result.is_ok());
    assert_eq!(peer.calls.lock().unwrap().len(), outcomes.len());
}

#[tokio::test]
async fn connection_loss_is_uncertain_instead_of_a_repeatable_create_failure() {
    let peer = peer::connect(vec![peer::Behaviour::Disconnect]).await;
    let Reply::Imported { outcomes, .. } = answer(
        &peer::admitted(&peer).2,
        Ask::Import(batch(peer.client.node().id)),
    )
    .await
    else {
        panic!("batch");
    };
    assert!(matches!(
        outcomes.first().unwrap().result,
        Err(Problem::Uncertain)
    ));
    assert_eq!(peer.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn account_choices_use_the_existing_profile_list_call() {
    let peer = peer::connect(vec![]).await;
    let batch = batch(peer.client.node().id);
    let Reply::Profiles { result, .. } = answer(
        &peer::admitted(&peer).2,
        Ask::Profiles {
            scope: batch.scope.clone(),
            ticket: batch.ticket,
        },
    )
    .await
    else {
        panic!("accounts");
    };
    assert!(result.unwrap().is_empty());
    assert!(
        matches!(peer.calls.lock().unwrap().first(), Some(Call::ListProfiles(request)) if request.harness.as_ref() == Some(&batch.scope.harness))
    );
}

#[test]
fn lost_create_receipts_cannot_be_retried_as_definite_failures() {
    assert_eq!(
        create_problem(vessel_client::CallError::Disconnected),
        Problem::Uncertain
    );
    assert_eq!(
        read_problem(vessel_client::CallError::Disconnected),
        Problem::Offline
    );
}

#[test]
fn node_messages_do_not_reach_history_failures() {
    let error = vessel_client::CallError::Remote(vessel_proto::WireError::new(
        vessel_proto::ErrorCode::Forbidden,
        "untrusted diagnostic",
    ));
    assert_eq!(
        read_problem(error),
        Problem::Refused(vessel_proto::ErrorCode::Forbidden)
    );
}

#[tokio::test]
async fn reusing_a_book_alias_for_a_different_node_refuses_before_any_create() {
    let peer = peer::connect(vec![]).await;
    let mut request = batch(peer.client.node().id);
    request.node = vessel_proto::NodeId::random();
    let Reply::Imported { outcomes, .. } =
        answer(&peer::admitted(&peer).2, Ask::Import(request)).await
    else {
        panic!("batch");
    };
    assert!(
        outcomes
            .iter()
            .all(|outcome| matches!(outcome.result, Err(Problem::IdentityChanged)))
    );
    assert!(peer.calls.lock().unwrap().is_empty());
}

#[path = "listing_tests.rs"]
mod listing;

#[path = "activation_tests.rs"]
mod activation;

#[tokio::test]
async fn same_node_reconnect_rejects_original_history_admission_before_any_rpc() {
    let peer = peer::connect(vec![]).await;
    let (_home, mut state, _selected, old, mut events) = peer::admitted(&peer);
    let previous = state.clients["fixture"].clone();
    let next = state
        .admissions
        .admit(MachineKey::from_book("fixture"), old.node())
        .unwrap();
    assert_ne!(old.epoch(), next.epoch());
    let _previous = state
        .clients
        .insert("fixture".into(), previous.renamed(next));
    let mut request = batch(old.node());
    request.scope.context = old;
    state.history(Ask::Import(request)).await;
    let event = crate::testing::within(async { events.recv().await.unwrap() }).await;
    let crate::Event::History(answer) = event else {
        panic!("history")
    };
    let Reply::Imported { outcomes, .. } = answer.into_reply() else {
        panic!("import refusal")
    };
    assert!(
        outcomes
            .iter()
            .all(|row| row.result == Err(Problem::IdentityChanged))
    );
    assert!(peer.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn full_history_budget_refuses_before_spawning_or_sending() {
    let peer = peer::connect(vec![]).await;
    let (_home, state, _selected, context, mut events) = peer::admitted(&peer);
    let _held = std::sync::Arc::clone(&state.history_calls.0)
        .try_acquire_many_owned(u32::try_from(MAX_CALLS).unwrap())
        .unwrap();
    let mut request = batch(context.node());
    request.scope.context = context;
    state
        .history(Ask::Profiles {
            scope: request.scope,
            ticket: request.ticket,
        })
        .await;
    let event = crate::testing::within(async { events.recv().await.unwrap() }).await;
    let crate::Event::History(answer) = event else {
        panic!("history")
    };
    assert!(matches!(
        answer.into_reply(),
        Reply::Profiles {
            result: Err(Problem::Unsent),
            ..
        }
    ));
    assert!(peer.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn oversized_import_batch_never_reaches_the_peer() {
    let peer = peer::connect(vec![]).await;
    let mut request = batch(peer.client.node().id);
    let row = request.entries.first().unwrap().clone();
    request.entries = vec![row; vessel_app_view::history::MAX_IMPORT_ENTRIES + 1];
    let Reply::Imported { outcomes, .. } =
        answer(&peer::admitted(&peer).2, Ask::Import(request)).await
    else {
        panic!("batch refusal")
    };
    assert!(
        outcomes
            .iter()
            .all(|row| row.result == Err(Problem::Capacity))
    );
    assert!(peer.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn a_mismatched_create_receipt_is_uncertain_instead_of_opening_another_conversation() {
    let request = batch(vessel_proto::NodeId::random());
    let mut wrong = SessionInfo::new(
        vessel_proto::SessionId::random(),
        request.entries.first().unwrap().spec(&request.scope),
        chrono::DateTime::UNIX_EPOCH,
    );
    wrong.spec.cwd = Some("/wrong-conversation".into());
    let peer = peer::connect(vec![peer::Behaviour::Session(Box::new(wrong))]).await;
    let mut request = batch(peer.client.node().id);
    request.entries.truncate(1);
    let Reply::Imported { outcomes, .. } =
        answer(&peer::admitted(&peer).2, Ask::Import(request)).await
    else {
        panic!("batch")
    };
    assert_eq!(outcomes.first().unwrap().result, Err(Problem::Uncertain));
    assert_eq!(peer.calls.lock().unwrap().len(), 1);
}
