//! A frame that does not decode is answered and logged with where and what
//! kind of mistake it had, never with a value it carried (`DecodeFailure`).
//! Each case puts a unique marker where serde would quote it and checks the
//! marker is in neither the answer nor the error the server ends with, its
//! Display, Debug or source chain.

use std::error::Error as _;
use std::fmt::Write as _;

use super::*;

/// A string no product text contains, so finding it means it was echoed.
const MARKER: &str = "vessel-privacy-marker-7f3c19";

/// Everything the node may log about `error`: Display, Debug and the
/// Display of every source.
fn everything_logged(error: &ServeError<Disconnected>) -> String {
    let mut text = format!("{error} {error:?}");
    let mut source = error.source();
    while let Some(cause) = source {
        let _infallible = write!(text, " {cause}");
        source = cause.source();
    }
    text
}

async fn ended(served: Served) -> ServeError<Disconnected> {
    match served.task.await.unwrap() {
        Err(error) => error,
        Ok(()) => panic!("the server should end the connection on an undecodable frame"),
    }
}

async fn answer_to(client: &mut MemoryTransport, frame: &str) -> WireError {
    client.send(frame.to_owned()).await.unwrap();
    refusal(&client.next().await.unwrap().unwrap())
}

#[tokio::test]
async fn bad_parameters_are_answered_with_their_place_not_their_value() {
    let (mut client, _served, _) = raw_client_with_node(vessel_proto::PROTOCOL_VERSION.get()).await;
    let frame = format!(
        r#"{{"type":"request","id":3,"call":{{"method":"session.get","params":{{"session":"{MARKER}"}}}}}}"#
    );
    let error = answer_to(&mut client, &frame).await;
    assert_eq!(error.code, ErrorCode::InvalidRequest);
    assert!(!error.message.contains(MARKER), "{}", error.message);
    assert!(error.message.contains("`session.get`"), "{}", error.message);
    assert!(error.message.contains("session"), "{}", error.message);

    // A value of the wrong JSON type, where serde names what it found.
    let frame = format!(
        r#"{{"type":"request","id":4,"call":{{"method":"session.get","params":{{"session":["{MARKER}"]}}}}}}"#
    );
    let error = answer_to(&mut client, &frame).await;
    assert!(!error.message.contains(MARKER), "{}", error.message);

    // A method name chosen by the caller is not repeated either.
    let frame =
        format!(r#"{{"type":"request","id":5,"call":{{"method":"{MARKER}","params":null}}}}"#);
    let error = answer_to(&mut client, &frame).await;
    assert_eq!(error.code, ErrorCode::InvalidRequest);
    assert!(!error.message.contains(MARKER), "{}", error.message);

    // A map key chosen by the caller becomes a placeholder in the path.
    let frame = format!(
        r#"{{"type":"request","id":6,"call":{{"method":"session.get","params":{{"{MARKER}-Key":1}}}}}}"#
    );
    let error = answer_to(&mut client, &frame).await;
    assert!(!error.message.contains(MARKER), "{}", error.message);
}

#[tokio::test]
async fn a_frame_without_an_id_ends_the_connection_without_its_values() {
    let (mut client, served, _) = raw_client_with_node(vessel_proto::PROTOCOL_VERSION.get()).await;
    let frame = format!(r#"{{"type":"request","id":"{MARKER}"}}"#);
    client.send(frame).await.unwrap();
    let logged = everything_logged(&ended(served).await);
    assert!(!logged.contains(MARKER), "{logged}");
}

#[tokio::test]
async fn a_bad_hello_is_refused_without_its_values() {
    let (mut client, server_end) = pair();
    let served = serve(server_end);
    let frame = format!(r#"{{"type":"hello","protocol":"{MARKER}","client":null}}"#);
    client.send(frame).await.unwrap();
    let text = client.next().await.unwrap().unwrap();
    let Ok(ServerFrame::Rejected { error }) = serde_json::from_str::<ServerFrame>(&text) else {
        panic!("expected the hello to be rejected, got {text}");
    };
    assert_eq!(error.code, ErrorCode::InvalidRequest);
    assert!(!error.message.contains(MARKER), "{}", error.message);
    let logged = everything_logged(&ended(served).await);
    assert!(!logged.contains(MARKER), "{logged}");
}
