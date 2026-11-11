//! The book's edits from the page (`forget`, `rename`, `default`): each
//! answered with the book as it is then, refused for this machine's node,
//! a node the book does not have and a name the book does not take, and a
//! forgotten node's connection and streams ended with a typed reason.
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "tests reading the few frames they wait for among the others"
)]

use std::sync::atomic::Ordering;

use vessel_proto::{
    Call, CreateSession, EventSeq, HarnessKind, ListSessions, Reply, SessionMode, SessionSpec,
    TerminalSize,
};
use vessel_web::wire::{
    BookNode, CallOutcome, ClientFrame, Failure, NodeKey, NodeStatus, Op, OpOutcome, OpResult,
    ServerFrame, StreamId, StreamState, StreamStop,
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

/// Why an edit was refused.
async fn refused(tab: &mut Tab, op: Op) -> String {
    let id = tab.op(op).await;
    match op_reply(tab, id).await {
        OpOutcome::Err { message } => message,
        OpOutcome::Ok(result) => panic!("expected a refusal, got {result:?}"),
    }
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
            .with("box", node.address(), node.token())
            .with("lab", node.address(), node.token());
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
            node: NodeKey::new("box"),
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

        let id = tab
            .op(Op::Forget {
                node: NodeKey::new("box"),
            })
            .await;
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
        let id = tab.call("box", &Call::ListSessions(ListSessions)).await;
        match reply(&mut tab, id).await {
            CallOutcome::Err(Failure::Unreachable { message }) => {
                assert!(message.contains("no node named box"), "{message}");
            }
            other => panic!("{other:?}"),
        }
        let message = refused(
            &mut tab,
            Op::Retry {
                node: NodeKey::new("box"),
            },
        )
        .await;
        assert!(message.contains("no node named box"), "{message}");
        assert_eq!(dials.load(Ordering::SeqCst), before);
        // The other name for the same node is untouched.
        let id = tab.call("lab", &Call::ListSessions(ListSessions)).await;
        assert!(matches!(reply(&mut tab, id).await, CallOutcome::Ok(_)));
    }))
    .await;
}

#[tokio::test]
async fn edits_refuse_this_machine_unknown_nodes_and_names_the_book_does_not_take() {
    within(Box::pin(async {
        let node = TestNode::start().await;
        let source = Source::default()
            .with("box", node.address(), node.token())
            .with("lab", node.address(), node.token());
        let ui = Ui::start(source).await;
        let mut tab = ui.tab().await;

        for op in [
            Op::Forget {
                node: NodeKey::here(),
            },
            Op::Rename {
                node: NodeKey::here(),
                name: "desk".to_owned(),
            },
        ] {
            let message = refused(&mut tab, op).await;
            assert!(message.contains("not in the book"), "{message}");
        }
        for op in [
            Op::Forget {
                node: NodeKey::new("ghost"),
            },
            Op::Rename {
                node: NodeKey::new("ghost"),
                name: "spirit".to_owned(),
            },
            Op::Default {
                node: NodeKey::new("ghost"),
            },
        ] {
            let message = refused(&mut tab, op).await;
            assert!(message.contains("no node named ghost"), "{message}");
        }
        for name in ["", "two words", "-flag", "_here", "a/b"] {
            let op = Op::Rename {
                node: NodeKey::new("box"),
                name: name.to_owned(),
            };
            let message = refused(&mut tab, op).await;
            assert!(!message.is_empty(), "{name:?}");
        }
        let taken = refused(
            &mut tab,
            Op::Rename {
                node: NodeKey::new("box"),
                name: "lab".to_owned(),
            },
        )
        .await;
        assert!(taken.contains("already"), "{taken}");

        // Nothing changed.
        let id = tab.op(Op::Nodes).await;
        match op_reply(&mut tab, id).await {
            OpOutcome::Ok(OpResult::Nodes { nodes }) => {
                assert_eq!(keys(&nodes), ["box", "lab"]);
                assert_eq!(default(&nodes), None);
            }
            other => panic!("{other:?}"),
        }
    }))
    .await;
}

#[tokio::test]
async fn rename_and_default_answer_with_the_book_and_the_default_follows_a_rename() {
    within(Box::pin(async {
        let node = TestNode::start().await;
        let source = Source::default()
            .with("box", node.address(), node.token())
            .with("lab", node.address(), node.token());
        let ui = Ui::start(source).await;
        let mut tab = ui.tab().await;

        let nodes = edited(
            &mut tab,
            Op::Default {
                node: NodeKey::new("box"),
            },
        )
        .await;
        assert_eq!(default(&nodes), Some("box"));
        let nodes = edited(
            &mut tab,
            Op::Rename {
                node: NodeKey::new("box"),
                name: "big".to_owned(),
            },
        )
        .await;
        assert_eq!(keys(&nodes), ["big", "lab"]);
        assert_eq!(default(&nodes), Some("big"));
        let id = tab.call("big", &Call::ListSessions(ListSessions)).await;
        assert!(matches!(reply(&mut tab, id).await, CallOutcome::Ok(_)));

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
    }))
    .await;
}
