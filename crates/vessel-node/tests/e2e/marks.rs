//! Protocol 10's marks and deletion over the wire (milestone 11, track A,
//! ADR 0026): pinned, archived and unread kept by the node for live and
//! ended sessions alike, archive stopping a live session, agents kept from
//! marking read, `session.delete`, and an older client's board.

use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use vessel_node::Node;
use vessel_proto::{
    ClientFrame, ErrorCode, EventSeq, HarnessKind, Hello, KanbanColumn, ListSessions, Mark,
    Outcome, ProtocolRange, ProtocolVersion, Reply, Request as _, RequestId, ServerFrame,
    SessionBoard, SessionId, SessionInfo, SessionStatus,
};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{
    Fixture, ask, client, read_all, read_until, status_of, structured_spec, terminal_spec, url,
    within,
};

/// A session's info as listed.
async fn listed(client: &vessel_client::NodeClient, id: SessionId) -> SessionInfo {
    client
        .sessions()
        .await
        .unwrap()
        .into_iter()
        .find(|info| info.id == id)
        .unwrap_or_else(|| panic!("{id} is not listed"))
}

/// Every mark on a live session, then on the same session read back from
/// the store after a restart; `get`, `list` and the mark's own answer
/// agree each time. `archive` on a live session stops it and answers once
/// it has ended.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn marks_are_kept_live_and_after_a_restart() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let id = client
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap()
            .id;
        let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;

        let pinned = client.mark(id, Mark::Pin).await.unwrap();
        assert!(pinned.marks.pinned);
        assert!(!pinned.marks.unread, "nothing asked for attention yet");
        assert_eq!(
            ask(&client, &mut stream, &mut seen, id, "plain").await,
            "echo: plain"
        );
        let got = client.session(id).await.unwrap();
        assert!(got.marks.pinned && got.marks.unread, "{:?}", got.marks);
        assert_eq!(listed(&client, id).await.marks, got.marks);

        // Read through everything this client has seen.
        let through = seen.last().unwrap().seq.next().unwrap();
        let read = client.mark(id, Mark::Read { through }).await.unwrap();
        assert!(!read.marks.unread);
        assert_eq!(client.session(id).await.unwrap().marks, read.marks);
        assert_eq!(
            code(
                client
                    .mark(
                        id,
                        Mark::Read {
                            through: EventSeq::new(through.get().saturating_add(100))
                        }
                    )
                    .await
            ),
            ErrorCode::InvalidRequest,
            "past the end of the log"
        );
        let unread = client.mark(id, Mark::Unread).await.unwrap();
        assert!(unread.marks.unread);

        // Archiving a live session stops it first.
        let archived = client.mark(id, Mark::Archive).await.unwrap();
        assert!(!archived.status.is_live(), "{:?}", archived.status);
        let ended_at = archived.ended_at.unwrap();
        assert!(archived.marks.archived_at.is_some());
        assert_eq!(archived.board, Some(SessionBoard::archived(ended_at)));
        assert_eq!(client.session(id).await.unwrap().board, archived.board);
        let log = read_all(&client, id).await;
        assert!(
            log.last()
                .and_then(status_of)
                .is_some_and(|status| !status.is_live()),
            "its log ends with its end"
        );
        node.stop().await.unwrap();

        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let stored = client.session(id).await.unwrap();
        assert_eq!(stored.marks, archived.marks, "kept by the store");
        assert_eq!(stored.board, archived.board);
        assert_eq!(listed(&client, id).await.marks, stored.marks);
        let back = client.mark(id, Mark::Unarchive).await.unwrap();
        assert_eq!(back.marks.archived_at, None);
        assert_eq!(back.board, Some(SessionBoard::exited(ended_at)));
        let unpinned = client.mark(id, Mark::Unpin).await.unwrap();
        assert!(!unpinned.marks.pinned);
        let end = EventSeq::new(u64::try_from(log.len()).unwrap());
        let seen_all = client.mark(id, Mark::Read { through: end }).await.unwrap();
        assert!(seen_all.marks.is_empty(), "{:?}", seen_all.marks);
        assert_eq!(
            code(client.mark(SessionId::random(), Mark::Pin).await),
            ErrorCode::NotFound
        );

        // A terminal session takes marks too.
        let terminal = client
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap()
            .id;
        assert!(client.mark(terminal, Mark::Pin).await.unwrap().marks.pinned);
        node.stop().await.unwrap();
    })
    .await;
}

/// An agent may pin or archive the sessions it started, but never mark
/// one read or unread (review item 13): an agent reading a session is not
/// its owner seeing it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_may_not_mark_read_or_unread() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let mut parent = structured(&owner, &fixture).await;
        let (_program, socket, auth) = credential(&owner, &mut parent).await;
        let agent = as_agent(&socket, &auth).await;
        let child = agent
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap()
            .id;

        assert!(agent.mark(child, Mark::Pin).await.unwrap().marks.pinned);
        let through = owner.session(child).await.unwrap().next_seq;
        assert_eq!(
            code(agent.mark(child, Mark::Read { through }).await),
            ErrorCode::Forbidden
        );
        assert_eq!(
            code(agent.mark(child, Mark::Unread).await),
            ErrorCode::Forbidden
        );
        // The owner may.
        assert!(owner.mark(child, Mark::Unread).await.unwrap().marks.unread);
        node.stop().await.unwrap();
    })
    .await;
}

/// `session.delete`: an ended session goes whole, its attachment folder
/// included; a live one is `conflict`; an unknown one `not_found`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_ended_session_is_deleted_and_a_live_one_is_not() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let ended = client
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap()
            .id;
        // The restart ends it, and the store serves it from then on.
        node.stop().await.unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let live = client
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap()
            .id;
        let folder = fixture.data_dir().attachments_dir().join(ended.to_string());
        std::fs::create_dir_all(folder.join("an-attachment")).unwrap();
        std::fs::write(folder.join("an-attachment").join("notes.txt"), b"x").unwrap();

        assert_eq!(code(client.delete_session(live).await), ErrorCode::Conflict);
        let deleted = client.delete_session(ended).await.unwrap();
        assert_eq!(deleted.worktree, None, "it ran in no worktree");
        assert!(!folder.exists(), "its files went with it");
        assert_eq!(code(client.session(ended).await), ErrorCode::NotFound);
        assert_eq!(
            code(client.read(ended, EventSeq::ZERO, 10).await),
            ErrorCode::NotFound
        );
        assert!(
            client
                .sessions()
                .await
                .unwrap()
                .iter()
                .all(|info| info.id != ended)
        );
        assert_eq!(
            code(client.delete_session(ended).await),
            ErrorCode::NotFound
        );
        assert!(client.session(live).await.unwrap().status.is_live());
        node.stop().await.unwrap();
    })
    .await;
}

/// The protocol-9 view of `session.list`, through a raw connection that
/// offers only version 9.
async fn listed_for_version_9(fixture: &Fixture, node: &Node) -> Vec<SessionInfo> {
    let mut request = url(node).as_uri().clone().into_client_request().unwrap();
    let bearer = format!("Bearer {}", fixture.token().expose());
    let _previous = request
        .headers_mut()
        .insert(AUTHORIZATION, bearer.parse().unwrap());
    let (mut socket, _response) = tokio_tungstenite::connect_async(request).await.unwrap();
    let version = ProtocolVersion::new(9);
    let hello = ClientFrame::Hello(Hello {
        protocol: ProtocolRange {
            min: version,
            max: version,
        },
        client: client(),
    });
    let mut frames = Vec::new();
    for frame in [
        hello,
        ClientFrame::Request {
            id: RequestId::new(1),
            call: ListSessions::default().into_call(),
        },
    ] {
        socket
            .send(Message::text(serde_json::to_string(&frame).unwrap()))
            .await
            .unwrap();
        let Some(Ok(Message::Text(text))) = socket.next().await else {
            panic!("the node answers each frame");
        };
        frames.push(serde_json::from_str::<ServerFrame>(text.as_str()).unwrap());
    }
    let [
        ServerFrame::Welcome(welcome),
        ServerFrame::Response { outcome, .. },
    ] = frames.as_slice()
    else {
        panic!("a welcome, then the list: {frames:?}");
    };
    assert_eq!(welcome.protocol, version);
    let Outcome::Ok(Reply::ListSessions(sessions)) = outcome else {
        panic!("the list: {outcome:?}");
    };
    sessions.clone()
}

/// A version 8-9 client still finds every ended session in the archive
/// (review item 11): the cards it can neither start again nor archive do
/// not pile up in its building lane. A current client sees it in building
/// as exited until it is archived.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_older_client_finds_ended_sessions_in_the_archive() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let id = client
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap()
            .id;
        node.stop().await.unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;

        let current = client.session(id).await.unwrap();
        let ended_at = current.ended_at.unwrap();
        assert_eq!(current.board, Some(SessionBoard::exited(ended_at)));
        let older = listed_for_version_9(&fixture, &node).await;
        let [older] = older.as_slice() else {
            panic!("one session: {older:?}");
        };
        assert_eq!(older.board, Some(SessionBoard::archived(ended_at)));
        assert_eq!(
            older.board.as_ref().map(|board| board.column),
            Some(KanbanColumn::Archive)
        );
        node.stop().await.unwrap();
    })
    .await;
}
