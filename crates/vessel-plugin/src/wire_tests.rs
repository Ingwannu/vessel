use std::error::Error;

use tokio::io::AsyncWriteExt;

use super::*;

#[test]
fn malformed_helper_fields_never_escape_error_chains() {
    for text in [
        r#"{"type":"PRIVATE-PIPE-CANARY"}"#,
        r#"{"type":"hooks_changed","PRIVATE-PIPE-CANARY":true}"#,
        r#"{"type":"error","request":1,"kind":"PRIVATE-PIPE-CANARY","diagnostic":"private"}"#,
    ] {
        let error = decode_helper(text.as_bytes()).expect_err("reject malformed frame");
        let mut cause: &dyn Error = &error;
        loop {
            assert!(!format!("{cause:?} {cause}").contains("PRIVATE-PIPE-CANARY"));
            match cause.source() {
                Some(next) => cause = next,
                None => break,
            }
        }
    }
}

#[tokio::test]
async fn fragmented_and_coalesced_frames_preserve_boundaries() {
    let message = NodeToHelper::Shutdown {};
    let mut encoded = Vec::new();
    write_node(&mut encoded, &message).await.expect("encode");
    let (mut writer, mut reader) = tokio::io::duplex(2);
    let sent = tokio::spawn(async move {
        for byte in encoded.iter().chain(encoded.iter()) {
            writer.write_all(&[*byte]).await.expect("fragment");
        }
    });
    assert_eq!(
        read_node(&mut reader).await.expect("frame"),
        Some(message.clone())
    );
    assert_eq!(read_node(&mut reader).await.expect("frame"), Some(message));
    sent.await.expect("join");
    assert!(read_node(&mut reader).await.expect("eof").is_none());
}

#[tokio::test]
async fn oversized_length_is_refused_without_waiting_for_body() {
    let length = u32::try_from(MAX_HELPER_FRAME_BYTES)
        .expect("fits")
        .checked_add(1)
        .expect("increment");
    let (mut writer, mut reader) = tokio::io::duplex(4);
    writer
        .write_all(&length.to_be_bytes())
        .await
        .expect("header");
    // The writer remains open and supplies no body: a decoder that reads or
    // allocates before checking the header would hang rather than refuse.
    assert!(matches!(
        read_helper(&mut reader).await,
        Err(FrameError::Length)
    ));
}

#[tokio::test]
async fn clean_eof_partial_header_and_partial_body_are_distinct() {
    assert!(read_node(&mut &b""[..]).await.expect("eof").is_none());
    for bytes in [
        b"\0".as_slice(),
        b"\0\0\0".as_slice(),
        b"\0\0\0\x05{}".as_slice(),
    ] {
        assert!(
            matches!(read_node(&mut &*bytes).await, Err(FrameError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof)
        );
    }
    assert!(matches!(
        read_node(&mut &b"\0\0\0\0"[..]).await,
        Err(FrameError::Length)
    ));
}

#[test]
fn encoded_budget_stops_before_growing_past_limit() {
    let mut body = BoundedBody {
        bytes: Vec::new(),
        exceeded: false,
    };
    let oversized = "x".repeat(MAX_HELPER_FRAME_BYTES);
    assert!(serde_json::to_writer(&mut body, &oversized).is_err());
    assert!(body.exceeded);
    assert!(body.bytes.len() <= MAX_HELPER_FRAME_BYTES);
}
