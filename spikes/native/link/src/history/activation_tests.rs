//! Activating imported history must not create a replacement session.

use super::*;
use vessel_proto::{SessionId, SessionInfo, SessionStatus};

fn existing(node: vessel_proto::NodeId) -> (Batch, SessionInfo) {
    let mut batch = batch(node);
    batch.entries.truncate(1);
    let entry = batch.entries.first_mut().unwrap();
    let id = SessionId::random();
    entry.in_vessel = Some(id);
    let mut info = SessionInfo::new(id, entry.spec(&batch.scope), chrono::DateTime::UNIX_EPOCH);
    info.harness_session = Some(entry.id.clone());
    info.status = SessionStatus::Running;
    (batch, info)
}

#[tokio::test]
async fn an_archived_native_conversation_resumes_the_original_session() {
    crate::testing::within(async {
        let (request, mut archived) = existing(vessel_proto::NodeId::random());
        archived.status = SessionStatus::Exited { code: Some(0) };
        archived.marks.archived_at = Some(chrono::DateTime::UNIX_EPOCH);
        let mut resumed = archived.clone();
        resumed.status = SessionStatus::Running;
        resumed.marks.archived_at = None;
        let peer = peer::connect(vec![
            peer::Behaviour::Session(Box::new(archived)),
            peer::Behaviour::Session(Box::new(resumed.clone())),
        ])
        .await;
        let mut request = request;
        request.node = peer.client.node().id;
        request.scope.context = peer::admitted(&peer).3;
        let Reply::Imported { outcomes, .. } =
            bounded_answer(&peer::admitted(&peer).2, Ask::Import(request)).await
        else {
            panic!("activation")
        };
        assert_eq!(
            outcomes.first().unwrap().result.as_ref().unwrap().as_ref(),
            &resumed
        );
        {
            let calls = peer.calls.lock().unwrap();
            assert_eq!(calls.len(), 2);
            assert!(
                matches!(calls.first(),Some(Call::GetSession(asked)) if asked.session==resumed.id)
            );
            assert!(
                matches!(calls.last(),Some(Call::ResumeSession(asked)) if asked.session==resumed.id)
            );
        }

        peer.finish().await;
    })
    .await;
}

#[tokio::test]
async fn an_existing_live_session_is_opened_without_resuming_or_creating() {
    crate::testing::within(async {
        let (mut request, live) = existing(vessel_proto::NodeId::random());
        let peer = peer::connect(vec![peer::Behaviour::Session(Box::new(live.clone()))]).await;
        request.node = peer.client.node().id;
        request.scope.context = peer::admitted(&peer).3;
        let Reply::Imported { outcomes, .. } =
            bounded_answer(&peer::admitted(&peer).2, Ask::Import(request)).await
        else {
            panic!("activation")
        };
        assert_eq!(
            outcomes.first().unwrap().result.as_ref().unwrap().id,
            live.id
        );
        assert_eq!(peer.calls.lock().unwrap().len(), 1);

        peer.finish().await;
    })
    .await;
}

#[tokio::test]
async fn a_changed_account_is_refused_before_reactivation() {
    crate::testing::within(async {
        let (mut request, mut info) = existing(vessel_proto::NodeId::random());
        info.profile = Some("moved-account".parse().unwrap());
        let peer = peer::connect(vec![peer::Behaviour::Session(Box::new(info))]).await;
        request.node = peer.client.node().id;
        request.scope.context = peer::admitted(&peer).3;
        let Reply::Imported { outcomes, .. } =
            bounded_answer(&peer::admitted(&peer).2, Ask::Import(request)).await
        else {
            panic!("activation")
        };
        assert_eq!(outcomes.first().unwrap().result, Err(Problem::InvalidReply));
        assert_eq!(peer.calls.lock().unwrap().len(), 1);

        peer.finish().await;
    })
    .await;
}

#[tokio::test]
async fn lost_reactivation_receipt_is_uncertain_and_never_creates_a_duplicate() {
    crate::testing::within(async {
        let (mut request, mut info) = existing(vessel_proto::NodeId::random());
        info.status = SessionStatus::Interrupted;
        let peer = peer::connect(vec![
            peer::Behaviour::Session(Box::new(info)),
            peer::Behaviour::Disconnect,
        ])
        .await;
        request.node = peer.client.node().id;
        request.scope.context = peer::admitted(&peer).3;
        let Reply::Imported { outcomes, .. } =
            bounded_answer(&peer::admitted(&peer).2, Ask::Import(request)).await
        else {
            panic!("activation")
        };
        assert_eq!(outcomes.first().unwrap().result, Err(Problem::Uncertain));
        {
            let calls = peer.calls.lock().unwrap();
            assert_eq!(calls.len(), 2);
            assert!(
                calls
                    .iter()
                    .all(|call| !matches!(call, Call::CreateSession(_)))
            );
        }

        peer.finish().await;
    })
    .await;
}

async fn bounded_answer(selected: &crate::connections::SelectedConnection, request: Ask) -> Reply {
    crate::testing::phase(
        "history adapter result",
        crate::history::answer(selected, request),
    )
    .await
}
