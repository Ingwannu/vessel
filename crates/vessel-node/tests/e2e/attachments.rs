//! Attachments over the wire (protocol 10, ADR 0025): uploads checked at
//! every step and kept to the caller that began them, an image and a file
//! reaching the agent, the node's record replacing the client's, reading
//! one back, and what a restart, a delete and a swapped file do.

use bytes::Bytes;
use vessel_client::{NodeClient, SessionStream};
use vessel_proto::conversation::MimeType;
use vessel_proto::{
    AgentEvent, AttachmentId, AttachmentKind, AttachmentLimits, AttachmentName, AttachmentRef,
    ContentBlock, Delivery, ErrorCode, EventRecord, EventSeq, HarnessKind, SessionId,
    SessionStatus, UploadId,
};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{
    Fixture, agent_of, read_until, status_of, step, structured_spec, terminal_spec, within,
};

pub(crate) fn name(text: &str) -> AttachmentName {
    text.parse().unwrap()
}

pub(crate) fn mime(text: &str) -> MimeType {
    text.parse().unwrap()
}

/// A PNG's signature followed by `size - 8` zero bytes.
pub(crate) fn png(size: usize) -> Bytes {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.resize(size, 0);
    Bytes::from(bytes)
}

pub(crate) fn text(text: &str) -> ContentBlock {
    ContentBlock::Text {
        text: text.to_owned(),
    }
}

pub(crate) fn attached(attachment: &AttachmentRef) -> ContentBlock {
    ContentBlock::Attachment {
        attachment: attachment.clone(),
    }
}

/// A structured session, followed until it runs.
pub(crate) async fn running(
    client: &NodeClient,
    fixture: &Fixture,
    profile: Option<&str>,
) -> (SessionId, SessionStream, Vec<EventRecord>) {
    let mut spec = structured_spec(fixture, None);
    spec.profile = profile.map(|profile| profile.parse().unwrap());
    let id = client.create_session(spec).await.unwrap().id;
    let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
    let mut seen = Vec::new();
    read_until(&mut stream, &mut seen, |record| {
        status_of(record) == Some(&SessionStatus::Running)
    })
    .await;
    (id, stream, seen)
}

/// Start a turn with `content` and return the agent's answer.
pub(crate) async fn answer(
    client: &NodeClient,
    (stream, seen): (&mut SessionStream, &mut Vec<EventRecord>),
    id: SessionId,
    content: Vec<ContentBlock>,
) -> String {
    let start = seen.len();
    let _started = step(
        "a prompt with attachments",
        client.prompt(id, content, Delivery::Start),
    )
    .await
    .unwrap();
    read_until(stream, seen, |record| {
        matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))
    })
    .await;
    seen[start..]
        .iter()
        .filter_map(|record| match agent_of(record) {
            Some(AgentEvent::MessageChunk {
                content: ContentBlock::Text { text },
            }) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// An attachment of `session`, read back whole, piece by piece.
async fn read_back(client: &NodeClient, session: SessionId, id: AttachmentId) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let offset = u64::try_from(bytes.len()).unwrap();
        let chunk = client.read_attachment(session, id, offset).await.unwrap();
        assert!(chunk.data.len() <= vessel_proto::BlobChunk::MAX_LEN);
        bytes.extend_from_slice(&chunk.data);
        if u64::try_from(bytes.len()).unwrap() >= chunk.total {
            return bytes;
        }
        assert!(!chunk.data.is_empty(), "no progress at {offset}");
    }
}

/// An image goes to the agent inline and a file as a link it reads; the
/// node's record replaces a client's relabelled reference; the log keeps
/// references only; another device reads them back.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_image_and_a_file_reach_the_agent_and_read_back() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let (id, mut stream, mut seen) = running(&client, &fixture, None).await;
        let conversation = client.conversation(id).await.unwrap();
        assert_eq!(conversation.limits, AttachmentLimits::DEFAULT);

        // Larger than a piece, so it travels in two.
        let image_bytes = png(1_500_000);
        let image = client
            .attach(id, name("shot.png"), mime("image/png"), image_bytes.clone())
            .await
            .unwrap();
        assert_eq!(
            (image.kind, image.size, image.name.as_str()),
            (AttachmentKind::Image, 1_500_000, "shot.png")
        );
        let file = client
            .attach(
                id,
                name("notes v1.txt"),
                mime("text/plain"),
                Bytes::from_static(b"hello, file"),
            )
            .await
            .unwrap();
        assert_eq!(file.kind, AttachmentKind::File);

        // A client calling the image a small file: the node's record wins.
        let relabelled = AttachmentRef {
            kind: AttachmentKind::File,
            size: 1,
            name: name("small.txt"),
            mime: mime("text/plain"),
            ..image.clone()
        };
        let said = answer(
            &client,
            (&mut stream, &mut seen),
            id,
            vec![text("describe"), attached(&relabelled), attached(&file)],
        )
        .await;
        assert_eq!(
            said,
            "text 8; image image/png 1500000 bytes; link notes v1.txt text/plain 11 read 11"
        );
        let started = seen
            .iter()
            .rev()
            .find_map(|record| match agent_of(record) {
                Some(AgentEvent::TurnStarted {
                    provenance: None,
                    content,
                    client_message_id: None,
                }) => Some((record, content)),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            started.1,
            &vec![text("describe"), attached(&image), attached(&file)],
            "the log keeps the node's references"
        );
        let logged = serde_json::to_string(started.0).unwrap();
        let data_dir = fixture.data_dir().root().display().to_string();
        assert!(!logged.contains(&data_dir), "never the path: {logged}");
        assert!(logged.len() < 4096, "never the bytes");

        // Another device shows what was sent.
        let other = fixture.connect(&node).await;
        assert_eq!(read_back(&other, id, image.id).await, image_bytes);
        assert_eq!(read_back(&other, id, file.id).await, b"hello, file");
        let past = client.read_attachment(id, file.id, 12).await;
        assert_eq!(code(past), ErrorCode::InvalidRequest);
        let unknown = client.read_attachment(id, AttachmentId::random(), 0).await;
        assert_eq!(code(unknown), ErrorCode::NotFound);
        let nowhere = client
            .read_attachment(SessionId::random(), file.id, 0)
            .await;
        assert_eq!(code(nowhere), ErrorCode::NotFound);
        let (theirs, _, _) = running(&client, &fixture, None).await;
        let elsewhere = client.read_attachment(theirs, file.id, 0).await;
        assert_eq!(code(elsewhere), ErrorCode::NotFound, "another session's");
        node.stop().await.unwrap();
    })
    .await;
}

/// Every refusal of `attachment.begin`; open uploads go at a restart.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn uploads_begin_within_their_bounds() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let (full, _, _) = running(&client, &fixture, None).await;
        let begin = |session, file: &str, kind: &str, size| {
            client.begin_upload(session, name(file), mime(kind), size)
        };

        assert_eq!(
            code(begin(SessionId::random(), "a.txt", "text/plain", 1).await),
            ErrorCode::NotFound
        );
        let terminal = client
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap()
            .id;
        assert_eq!(
            code(begin(terminal, "a.txt", "text/plain", 1).await),
            ErrorCode::Unsupported
        );
        assert_eq!(
            code(begin(full, "note.wav", "audio/wav", 4).await),
            ErrorCode::Unsupported,
            "the agent takes no audio"
        );
        assert_eq!(
            code(begin(full, "big.png", "image/png", (10 << 20) + 1).await),
            ErrorCode::InvalidRequest
        );
        assert_eq!(
            code(begin(full, "big.bin", "application/octet-stream", (50 << 20) + 1).await),
            ErrorCode::InvalidRequest
        );

        // A session's 100 MiB counts its open uploads.
        let half = begin(full, "a.bin", "application/octet-stream", 50 << 20)
            .await
            .unwrap();
        let _other_half = begin(full, "b.bin", "application/octet-stream", 50 << 20)
            .await
            .unwrap();
        assert_eq!(
            code(begin(full, "c.txt", "text/plain", 1).await),
            ErrorCode::InvalidRequest
        );
        let too_long = Bytes::from(vec![0_u8; vessel_proto::BlobChunk::MAX_LEN + 1]);
        assert_eq!(
            code(client.write_upload(half.upload, 0, too_long).await),
            ErrorCode::InvalidRequest
        );

        // Four places a caller.
        let (other, _, _) = running(&client, &fixture, None).await;
        let _third = begin(other, "c.txt", "text/plain", 1).await.unwrap();
        let _fourth = begin(other, "d.txt", "text/plain", 1).await.unwrap();
        assert_eq!(
            code(begin(other, "e.txt", "text/plain", 1).await),
            ErrorCode::Unavailable
        );

        // A restart drops every open upload.
        node.stop().await.unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        assert_eq!(
            code(
                client
                    .write_upload(half.upload, 0, Bytes::from_static(b"x"))
                    .await
            ),
            ErrorCode::NotFound
        );
        let staged = std::fs::read_dir(fixture.data_dir().uploads_dir()).map_or(0, Iterator::count);
        assert_eq!(staged, 0, "nothing staged is left");
        node.stop().await.unwrap();
    })
    .await;
}

/// Every refusal of `attachment.write` and `.finish`, and an upload kept
/// to the caller that began it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pieces_are_checked_and_belong_to_their_caller() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let client = fixture.connect(&node).await;
        let begin = |session, file: &str, kind: &str, size| {
            client.begin_upload(session, name(file), mime(kind), size)
        };
        let mut session = structured(&client, &fixture).await;
        let other = session.id;
        let upload = begin(other, "hello.txt", "text/plain", 10)
            .await
            .unwrap()
            .upload;
        let big = begin(other, "big.bin", "application/octet-stream", 2 << 20)
            .await
            .unwrap()
            .upload;
        let too_long = Bytes::from(vec![0_u8; vessel_proto::BlobChunk::MAX_LEN + 1]);
        assert_eq!(
            code(client.write_upload(big, 0, too_long).await),
            ErrorCode::InvalidRequest
        );

        let write = |offset, data: &'static [u8]| {
            client.write_upload(upload, offset, Bytes::from_static(data))
        };
        assert_eq!(write(0, b"hello").await.unwrap().received, 5);
        assert_eq!(
            write(0, b"hello").await.unwrap().received,
            5,
            "a piece sent again is answered, not written twice"
        );
        assert_eq!(
            code(write(7, b"rld").await),
            ErrorCode::InvalidRequest,
            "a gap"
        );
        assert_eq!(
            code(write(3, b"lo wor").await),
            ErrorCode::InvalidRequest,
            "overlaps"
        );
        assert_eq!(
            code(write(5, b"world!").await),
            ErrorCode::InvalidRequest,
            "past the size"
        );
        assert_eq!(
            code(client.finish_upload(upload).await),
            ErrorCode::InvalidRequest,
            "bytes missing"
        );
        assert_eq!(write(5, b"world").await.unwrap().received, 10);

        // Another caller cannot see it.
        let (_program, socket, auth) = credential(&client, &mut session).await;
        let agent = as_agent(&socket, &auth).await;
        assert_eq!(
            code(agent.write_upload(upload, 10, Bytes::new()).await),
            ErrorCode::NotFound
        );
        assert_eq!(code(agent.finish_upload(upload).await), ErrorCode::NotFound);
        assert_eq!(
            code(
                agent
                    .begin_upload(other, name("a.txt"), mime("text/plain"), 1)
                    .await
            ),
            ErrorCode::Forbidden,
            "the owner's session"
        );

        let kept = client.finish_upload(upload).await.unwrap();
        assert_eq!((kept.size, kept.name.as_str()), (10, "hello.txt"));
        assert_eq!(
            code(client.finish_upload(upload).await),
            ErrorCode::NotFound
        );
        assert_eq!(
            code(
                client
                    .write_upload(UploadId::random(), 0, Bytes::new())
                    .await
            ),
            ErrorCode::NotFound
        );

        node.stop().await.unwrap();
    })
    .await;
}

/// `attachment.finish` keeps no image that does not start as its type
/// does, and nothing for a session that ended meanwhile.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn finishing_checks_the_image_and_the_session() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let begin = |session, file: &str, kind: &str, size| {
            client.begin_upload(session, name(file), mime(kind), size)
        };
        let (other, _, _) = running(&client, &fixture, None).await;
        // An image that does not start as its type does is not kept.
        let fake = begin(other, "fake.png", "image/png", 8)
            .await
            .unwrap()
            .upload;
        let _written = client
            .write_upload(fake, 0, Bytes::from_static(b"GIF89a.."))
            .await
            .unwrap();
        assert_eq!(
            code(client.finish_upload(fake).await),
            ErrorCode::InvalidRequest
        );
        assert_eq!(
            code(client.write_upload(fake, 8, Bytes::new()).await),
            ErrorCode::NotFound,
            "dropped"
        );

        // A session that ended meanwhile keeps nothing new.
        let held = begin(other, "held.txt", "text/plain", 2)
            .await
            .unwrap()
            .upload;
        let _written = client
            .write_upload(held, 0, Bytes::from_static(b"ok"))
            .await
            .unwrap();
        let mut stream = client.follow(other, EventSeq::ZERO).await.unwrap();
        client.kill(other).await.unwrap();
        read_until(&mut stream, &mut Vec::new(), |record| {
            status_of(record).is_some_and(|status| !status.is_live())
        })
        .await;
        assert_eq!(code(client.finish_upload(held).await), ErrorCode::Conflict);
        assert_eq!(
            code(begin(other, "late.txt", "text/plain", 1).await),
            ErrorCode::Conflict
        );

        node.stop().await.unwrap();
    })
    .await;
}
