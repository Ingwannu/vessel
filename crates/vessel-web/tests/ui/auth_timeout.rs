//! A resource deadline must not invalidate an unused UI capability.
use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::tungstenite::{Message, protocol::frame::coding::CloseCode};
use vessel_web::LaunchToken;
use vessel_web::wire::{
    AuthMode, CLOSE_UNAUTHORIZED, ClientFrame, Nonce, PresentedToken, ServerFrame,
};

use crate::server::socket;
use crate::support::{Source, Ui, sign, within};

#[tokio::test]
async fn an_expired_hello_allows_fresh_proof_for_the_same_unused_launch() {
    // Use the product's unchanged wait; the socket's close is the barrier.
    let ui = Ui::start(Source::default()).await;
    let mut pending = socket(&ui, Some(&ui.origin())).await.unwrap();
    let Some(Ok(Message::Text(challenge))) = within(pending.next()).await else {
        panic!("expected the server challenge")
    };
    assert!(matches!(
        serde_json::from_str::<ServerFrame>(&challenge),
        Ok(ServerFrame::Challenge { .. })
    ));
    let Some(Ok(Message::Close(Some(frame)))) = within(pending.next()).await else {
        panic!("expected the elapsed hello wait to close")
    };
    assert_eq!(frame.code, CloseCode::Again);
    // This helper opens a new socket and verifies its fresh-nonce welcome
    // with the original bootstrap. No code was restored or minted anew.
    let _tab = ui.tab().await;
    assert!(ui.session.lock().await.is_some());
}

#[tokio::test]
async fn a_well_formed_but_wrong_proof_is_still_refused_without_spending_the_code() {
    let ui = Ui::start(Source::default()).await;
    let mut pending = socket(&ui, Some(&ui.origin())).await.unwrap();
    let Some(Ok(Message::Text(challenge))) = within(pending.next()).await else {
        panic!("expected the server challenge")
    };
    let Ok(ServerFrame::Challenge { launch, nonce }) = serde_json::from_str(&challenge) else {
        panic!("expected a typed server challenge")
    };
    let client = Nonce::from_random(LaunchToken::mint().unwrap());
    let proof = PresentedToken::new(sign(
        "deliberately wrong key",
        &format!(
            "vessel-ui-v1\nclient\n{}\n{}\n{}\n",
            launch.as_str(),
            nonce.as_str(),
            client.as_str()
        ),
    ));
    let hello = ClientFrame::Hello {
        mode: AuthMode::Bootstrap,
        nonce: client,
        proof,
    };
    within(pending.send(Message::Text(serde_json::to_string(&hello).unwrap().into())))
        .await
        .unwrap();
    let Some(Ok(Message::Close(Some(frame)))) = within(pending.next()).await else {
        panic!("expected a bad proof to close")
    };
    assert_eq!(frame.code, CLOSE_UNAUTHORIZED.into());
    let _tab = ui.tab().await;
    assert!(ui.session.lock().await.is_some());
}
