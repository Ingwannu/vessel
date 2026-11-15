//! The book's edits from the page (`forget`, `rename`, `default`), on a
//! real book in a temporary directory: each answered with the book as it
//! is then and seen in the file; names the book would not take, and this
//! machine's node where only a book entry will do, refused when the frame
//! is decoded and still answered by id; nodes the book does not have and
//! taken names refused by the book; and a forgotten node's connection and
//! streams ended with a typed reason.
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "tests reading the few frames they wait for among the others"
)]

use std::sync::atomic::Ordering;

use vessel_client::book::BookFile;
use vessel_proto::{
    Call, CreateSession, EventSeq, HarnessKind, ListSessions, Reply, SessionMode, SessionSpec,
    TerminalSize,
};
use vessel_web::wire::{
    BookName, BookNode, CallId, CallOutcome, ClientFrame, Failure, NodeKey, NodeStatus, Op,
    OpFailure, OpOutcome, OpResult, ServerFrame, StreamId, StreamState, StreamStop,
};

use crate::bridge::{op_reply, reply};
use crate::support::{Source, Tab, TestNode, Ui, within};

/// The nodes an edit answered with, or the panic that says what came.
async fn edited(tab: &mut Tab, op: Op) -> Vec<BookNode> {
    let id = tab.op(op).await;
    match op_reply(tab, id).await {
        OpOutcome::Ok(
            OpResult::Forget { nodes } | OpResult::Rename { nodes } | OpResult::Default { nodes },
        ) => nodes,
        other => panic!("{other:?}"),
    }
}

/// Why the op answered as `id` was refused.
async fn refusal(tab: &mut Tab, id: CallId) -> (OpFailure, String) {
    match op_reply(tab, id).await {
        OpOutcome::Err { reason, message } => (reason, message),
        OpOutcome::Ok(result) => panic!("expected a refusal, got {result:?}"),
    }
}

/// Why `op` was refused.
async fn refused(tab: &mut Tab, op: Op) -> (OpFailure, String) {
    let id = tab.op(op).await;
    refusal(tab, id).await
}

/// A book name, as the page's frames carry it.
fn book(name: &str) -> BookName {
    name.parse().unwrap()
}

fn keys(nodes: &[BookNode]) -> Vec<&str> {
    nodes.iter().map(|node| node.key.as_str()).collect()
}

fn default(nodes: &[BookNode]) -> Option<&str> {
    nodes
        .iter()
        .find(|node| node.default)
        .map(|node| node.key.as_str())
}

#[tokio::test]
async fn forgetting_a_node_closes_its_connection_and_ends_its_streams_as_removed() {
    within(Box::pin(async {
        let node = TestNode::start().await;
        let source = Source::default()
            .with("box", node.address(), &node.token())
            .with("lab", node.address(), &node.token());
        let dials = source.dials();
        let ui = Ui::start(source).await;
        let mut tab = ui.tab().await;

        let mut spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        );
        spec.cwd = Some(node.dir().to_path_buf());
        let id = tab
            .call("box", &Call::CreateSession(CreateSession { spec }))
            .await;
        let session = match reply(&mut tab, id).await {
            CallOutcome::Ok(reply) => match *reply {
                Reply::CreateSession(info) => info.id,
                other => panic!("{other:?}"),
            },
            CallOutcome::Err(failure) => panic!("{failure:?}"),
        };
        let stream = StreamId::new(1);
        tab.send(&ClientFrame::Follow {
            stream,
            node: "box".parse().unwrap(),
            session,
            from: EventSeq::ZERO,
        })
        .await;
        let _first = tab
            .until(|frame| match frame {
                ServerFrame::Event { stream: from, .. } if from == stream => Some(()),
                _other => None,
            })
            .await;

        let id = tab.op(Op::Forget { node: book("box") }).await;
        let (mut answered, mut ended, mut released) = (None, None, false);
        while answered.is_none() || ended.is_none() || !released {
            match tab.next().await {
                ServerFrame::OpReply {
                    id: got,
                    outcome: OpOutcome::Ok(OpResult::Forget { nodes }),
                } if got == id => answered = Some(nodes),
                ServerFrame::OpReply { id: got, outcome } if got == id => {
                    panic!("{outcome:?}")
                }
                ServerFrame::Stream {
                    stream: from,
                    state: StreamState::Failed { reason, .. },
                } if from == stream => ended = Some(reason),
                ServerFrame::Node {
                    node,
                    status: NodeStatus::Failed { .. },
                } if node.as_str() == "box" => released = true,
                _other => {}
            }
        }
        assert_eq!(answered.as_deref().map(keys), Some(vec!["lab"]));
        assert_eq!(ended, Some(StreamStop::Removed));

        // Gone for calls, follows and retries, with no dial to it again.
        let before = dials.load(Ordering::SeqCst);
        let id = tab
            .call("box", &Call::ListSessions(ListSessions::default()))
            .await;
        match reply(&mut tab, id).await {
            CallOutcome::Err(Failure::Unreachable { message }) => {
                assert!(message.contains("no node named box"), "{message}");
            }
            other => panic!("{other:?}"),
        }
        let (_reason, message) = refused(
            &mut tab,
            Op::Retry {
                node: "box".parse().unwrap(),
            },
        )
        .await;
        assert!(message.contains("no node named box"), "{message}");
        assert_eq!(dials.load(Ordering::SeqCst), before);
        // The other name for the same node is untouched.
        let id = tab
            .call("lab", &Call::ListSessions(ListSessions::default()))
            .await;
        assert!(matches!(reply(&mut tab, id).await, CallOutcome::Ok(_)));
    }))
    .await;
}

#[tokio::test]
async fn names_the_book_would_not_take_are_refused_by_id_before_any_op_runs() {
    within(Box::pin(async {
        let node = TestNode::start().await;
        let source = Source::default()
            .with("box", node.address(), &node.token())
            .with("lab", node.address(), &node.token());
        let book_file = source.path();
        let before = std::fs::read_to_string(&book_file).unwrap();
        let ui = Ui::start(source).await;
        let mut tab = ui.tab().await;

        // This machine's node is not a book entry: it cannot be named
        // where only one will do.
        for op in [
            r#"{"op":"forget","node":"_here"}"#,
            r#"{"op":"rename","node":"_here","name":"desk"}"#,
            r#"{"op":"rename","node":"box","name":"_here"}"#,
        ] {
            let id = tab
                .frame_text(&format!(r#"{{"type":"op","id":{{id}},"op":{op}}}"#))
                .await;
            let (reason, message) = refusal(&mut tab, id).await;
            assert_eq!(reason, OpFailure::Unreadable, "{op}");
            assert!(message.contains("not in the book"), "{message}");
        }
        // Names the book does not take, as a new name or as a node.
        for name in ["", "two words", "-flag", "a/b"] {
            for op in [
                format!(r#"{{"op":"rename","node":"box","name":"{name}"}}"#),
                format!(r#"{{"op":"forget","node":"{name}"}}"#),
                format!(r#"{{"op":"default","node":"{name}"}}"#),
            ] {
                let id = tab
                    .frame_text(&format!(r#"{{"type":"op","id":{{id}},"op":{op}}}"#))
                    .await;
                let (reason, message) = refusal(&mut tab, id).await;
                assert_eq!(reason, OpFailure::Unreadable, "{op}: {message}");
            }
        }
        // A call or a follow naming a node badly is answered by its id too.
        let id = tab
            .frame_text(
                r#"{"type":"call","id":{id},"node":"two words","call":{"method":"session.list","params":null}}"#,
            )
            .await;
        assert!(matches!(
            reply(&mut tab, id).await,
            CallOutcome::Err(Failure::Refused { .. })
        ));
        let _id = tab
            .frame_text(
                r#"{"type":"follow","stream":7,"node":"a/b","session":"00000000-0000-0000-0000-000000000000","from":0}"#,
            )
            .await;
        let state = tab
            .until(|frame| match frame {
                ServerFrame::Stream { stream, state } if stream == StreamId::new(7) => Some(state),
                _other => None,
            })
            .await;
        assert!(
            matches!(
                state,
                StreamState::Failed {
                    reason: StreamStop::Error,
                    ..
                }
            ),
            "{state:?}"
        );

        // None of it reached the book.
        assert_eq!(std::fs::read_to_string(&book_file).unwrap(), before);
    }))
    .await;
}

#[tokio::test]
async fn the_book_refuses_unknown_nodes_and_taken_names() {
    within(Box::pin(async {
        let node = TestNode::start().await;
        let source = Source::default()
            .with("box", node.address(), &node.token())
            .with("lab", node.address(), &node.token());
        let book_file = source.path();
        let before = std::fs::read_to_string(&book_file).unwrap();
        let ui = Ui::start(source).await;
        let mut tab = ui.tab().await;

        for op in [
            Op::Forget {
                node: book("ghost"),
            },
            Op::Rename {
                node: book("ghost"),
                name: book("spirit"),
            },
            Op::Default {
                node: "ghost".parse().unwrap(),
            },
        ] {
            let (reason, message) = refused(&mut tab, op).await;
            assert_eq!(reason, OpFailure::Failed);
            assert!(message.contains("no node `ghost`"), "{message}");
        }
        let (reason, taken) = refused(
            &mut tab,
            Op::Rename {
                node: book("box"),
                name: book("lab"),
            },
        )
        .await;
        assert_eq!(reason, OpFailure::Failed);
        assert!(taken.contains("already"), "{taken}");
        assert_eq!(std::fs::read_to_string(&book_file).unwrap(), before);
    }))
    .await;
}

#[tokio::test]
async fn rename_and_default_answer_with_the_book_the_file_holds() {
    within(Box::pin(async {
        let node = TestNode::start().await;
        let source = Source::default()
            .with("box", node.address(), &node.token())
            .with("lab", node.address(), &node.token());
        let book_file = source.path();
        let ui = Ui::start(source).await;
        let mut tab = ui.tab().await;
        let on_disk = || BookFile::open(&book_file).unwrap().book().clone();

        let nodes = edited(
            &mut tab,
            Op::Default {
                node: "box".parse().unwrap(),
            },
        )
        .await;
        assert_eq!(default(&nodes), Some("box"));
        assert_eq!(on_disk().default_node, Some("box".parse().unwrap()));
        let nodes = edited(
            &mut tab,
            Op::Rename {
                node: book("box"),
                name: book("big"),
            },
        )
        .await;
        assert_eq!(keys(&nodes), ["big", "lab"]);
        assert_eq!(default(&nodes), Some("big"));
        assert_eq!(on_disk().default_node, Some("big".parse().unwrap()));
        let id = tab
            .call("big", &Call::ListSessions(ListSessions::default()))
            .await;
        assert!(matches!(reply(&mut tab, id).await, CallOutcome::Ok(_)));

        // Saving a name that did not change is no change.
        let nodes = edited(
            &mut tab,
            Op::Rename {
                node: book("big"),
                name: book("big"),
            },
        )
        .await;
        assert_eq!(keys(&nodes), ["big", "lab"]);

        // A tab that resumed the launch's session (a reload) is answered as
        // the first one: the edits stand where `pair` stands, behind the
        // socket's mutual proof, and no tab is more trusted than another.
        let mut reloaded = ui.tab().await;
        let nodes = edited(
            &mut reloaded,
            Op::Default {
                node: NodeKey::here(),
            },
        )
        .await;
        assert_eq!(
            default(&nodes),
            None,
            "this machine's node: no book default"
        );
        assert_eq!(on_disk().default_node, None);
    }))
    .await;
}
