//! Adversarial peer chunks are refused before append.
use super::*;
use bytes::Bytes;
fn chunk(data: &'static [u8], total: u64) -> BlobChunk {
    BlobChunk {
        data: Bytes::from_static(data),
        total,
    }
}
#[test]
fn pieces_make_one_image_with_monotone_offsets() {
    let mut image = Collector::default();
    assert_eq!(image.take(&chunk(b"0123", 10)), Ok(Next::More));
    assert_eq!(image.offset(), 4);
    assert_eq!(image.take(&chunk(b"4567", 10)), Ok(Next::More));
    assert_eq!(image.take(&chunk(b"89", 10)), Ok(Next::Done));
    assert_eq!(image.bytes, b"0123456789");
}
#[test]
fn oversized_empty_and_overrunning_first_chunks_are_not_kept() {
    for (input, why) in [
        (chunk(b"", 0), ReadProblem::Empty),
        (
            chunk(b"a", AttachmentLimits::DEFAULT.image.saturating_add(1)),
            ReadProblem::TooLarge,
        ),
        (chunk(b"ab", 1), ReadProblem::Overrun),
        (chunk(b"", 2), ReadProblem::Stopped),
    ] {
        let mut image = Collector::default();
        assert_eq!(image.take(&input), Err(why));
        assert!(image.bytes.is_empty());
    }
}
#[test]
fn changed_total_stall_and_overrun_do_not_mutate_a_partial_image() {
    for (input, why) in [
        (chunk(b"a", 4), ReadProblem::Changed),
        (chunk(b"", 5), ReadProblem::Stopped),
        (chunk(b"abcd", 5), ReadProblem::Overrun),
    ] {
        let mut image = Collector::default();
        assert_eq!(image.take(&chunk(b"ab", 5)), Ok(Next::More));
        assert_eq!(image.take(&input), Err(why));
        assert_eq!(image.bytes, b"ab");
    }
}

pub(in crate::attachments) mod peer;

#[tokio::test]
async fn the_real_client_reads_chunks_from_an_in_memory_peer_and_delivers_one_answer() {
    let (client, task) = peer::connect(vec![chunk(b"abc", 5), chunk(b"de", 5)]).await;
    let sent = Sent {
        node: client.node().id,
        machine: vessel_app_view::machine_book::Name::parse("laptop").unwrap(),
        session: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        attachment: "00000000-0000-4000-8000-000000000002".parse().unwrap(),
    };
    let (events, mut incoming) = mpsc::channel(1);
    spawn(Ok(client), sent.clone(), events);
    let event = incoming.recv().await.expect("read completed");
    assert!(
        matches!(event, Event::Files(Answer::Read { sent: ref actual, bytes: Some(ref bytes) })
        if actual == &sent && bytes.as_ref() == b"abcde")
    );
    let calls = task.await.unwrap();
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().zip([0, 3]).all(|(call, offset)| matches!(call,
        vessel_proto::Call::ReadAttachment(ask) if ask.session == sent.session && ask.attachment == sent.attachment && ask.offset == offset)));
}

#[tokio::test]
async fn a_malformed_peer_finishes_as_a_failed_picture_instead_of_running_forever() {
    let (client, task) = peer::connect(vec![chunk(b"abc", 5), chunk(b"", 5)]).await;
    let sent = Sent {
        node: client.node().id,
        machine: vessel_app_view::machine_book::Name::parse("laptop").unwrap(),
        session: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        attachment: "00000000-0000-4000-8000-000000000002".parse().unwrap(),
    };
    assert!(matches!(
        read(&client, &sent).await,
        Err(LinkError::AttachmentRead(ReadProblem::Stopped))
    ));
    drop(client);
    assert_eq!(task.await.unwrap().len(), 2);
}

#[tokio::test]
async fn a_rebound_node_cannot_receive_a_sent_image_read() {
    let (client, task) = peer::connect(Vec::new()).await;
    let sent = Sent {
        node: vessel_proto::NodeId::random(),
        machine: vessel_app_view::machine_book::Name::parse("laptop").unwrap(),
        session: vessel_proto::SessionId::random(),
        attachment: vessel_proto::AttachmentId::random(),
    };
    assert!(matches!(
        read(&client, &sent).await,
        Err(LinkError::AttachmentTarget(
            super::super::TargetProblem::Changed { .. }
        ))
    ));
    drop(client);
    assert!(task.await.unwrap().is_empty());
}
