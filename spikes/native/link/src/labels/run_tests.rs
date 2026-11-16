use std::time::Duration;

use chrono::DateTime;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream};
use vessel_app_view::labels::{Assignment, Color, Count, Failure, Label, Name, Tickets};
use vessel_client::{CallError, NodeClient};
use vessel_proto::workspace_labels::{
    CatalogCursor, CatalogList, CatalogMode, CatalogRead, WorkspaceLabelAssigned,
    WorkspaceLabelDeleted, WorkspaceLabelFailure, WorkspaceLabelUpdated,
};
use vessel_proto::{
    Call, ClientFrame, ClientInfo, ErrorCode, ErrorDetail, NodeId, NodeInfo, NodeState, Outcome,
    ProtocolRange, Reply, ServerFrame, Welcome, WireError, WorkspaceId, WorkspaceLabelGeneration,
    WorkspaceLabelSeq,
};

use super::{failure, late, remote, update};
use crate::connections::RequestFailure;
use crate::labels::{Answer, Command, Event, Request};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const PIPE_BYTES: usize = 64 * 1024;
const DEADLINE: Duration = Duration::from_secs(10);

fn name(text: &str) -> Name {
    text.parse().unwrap()
}

fn label(text: &str, color: Color) -> Label {
    Label {
        name: name(text),
        color,
    }
}

fn refusal(reason: WorkspaceLabelFailure) -> WireError {
    let mut error = WireError::new(reason.code(), "the node's own words");
    error.detail = Some(ErrorDetail::WorkspaceLabel { reason });
    error
}

#[test]
fn an_edit_sends_the_new_name_only_when_its_spelling_changes() {
    let same = update(&name("Ready"), &label("Ready", Color::Sky));
    assert_eq!(same.new_name, None);
    assert_eq!(same.color, Some(Color::Sky));
    let renamed = update(&name("Ready"), &label("Done", Color::Violet));
    assert_eq!(renamed.name, name("Ready"));
    assert_eq!(renamed.new_name, Some(name("Done")));
    // A change of case is a change of spelling the host keeps.
    let recased = update(&name("ready"), &label("Ready", Color::Violet));
    assert_eq!(recased.new_name, Some(name("Ready")));
}

#[test]
fn the_node_s_reasons_become_the_form_s_failures_and_its_words_stay_behind() {
    assert_eq!(
        remote(&refusal(WorkspaceLabelFailure::NameTaken)),
        Failure::NameTaken
    );
    assert_eq!(
        remote(&refusal(WorkspaceLabelFailure::LabelNotFound)),
        Failure::NotFound
    );
    for reason in [
        WorkspaceLabelFailure::WorkspaceNotFound,
        WorkspaceLabelFailure::WorkspaceInactive,
        WorkspaceLabelFailure::Unknown,
    ] {
        assert_eq!(remote(&refusal(reason)), Failure::Update);
    }
    assert_eq!(
        remote(&WireError::new(ErrorCode::Unsupported, "old")),
        Failure::Unsupported
    );
    assert_eq!(
        remote(&WireError::new(ErrorCode::Forbidden, "reader")),
        Failure::Update
    );
    assert_eq!(
        failure(RequestFailure::Call(CallError::Disconnected)),
        Failure::Offline
    );
    assert_eq!(
        failure(RequestFailure::Call(CallError::UnassignedProtocol {
            method: vessel_proto::Method::ListWorkspaceLabels,
        })),
        Failure::Unsupported
    );
    assert_eq!(
        failure(RequestFailure::Refused(
            vessel_app_view::connection::Refusal::Changed
        )),
        Failure::Changed
    );
}

#[test]
fn a_late_list_is_a_load_failure_and_a_late_write_an_update_failure() {
    let list = Command::List {
        cursor: None,
        read: CatalogRead::Observe,
    };
    assert_eq!(late(&list), Failure::Load);
    assert_eq!(late(&Command::Delete(name("Ready"))), Failure::Update);
}

/// What the in-memory node received, in order.
type Received = std::sync::Arc<std::sync::Mutex<Vec<Call>>>;

async fn read(stream: &mut DuplexStream) -> TestResult<Option<ClientFrame>> {
    let length = match stream.read_u32().await {
        Ok(length) => length,
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = vec![0; usize::try_from(length)?];
    let _read = stream.read_exact(&mut bytes).await?;
    Ok(Some(serde_json::from_slice(&bytes)?))
}

async fn write(stream: &mut DuplexStream, frame: &ServerFrame) -> TestResult {
    let bytes = serde_json::to_vec(frame)?;
    stream.write_u32(u32::try_from(bytes.len())?).await?;
    stream.write_all(&bytes).await?;
    Ok(())
}

fn head() -> CatalogCursor {
    CatalogCursor {
        generation: WorkspaceLabelGeneration::random(),
        after: WorkspaceLabelSeq::new(3),
    }
}

/// The node's answer to each label call: the real reply shapes, and one
/// structured refusal (a rename onto a taken name). Any other call is
/// refused as invalid.
fn reply(call: &Call) -> Outcome {
    if let Call::ListWorkspaceLabels(_) = call {
        return Outcome::Ok(Reply::ListWorkspaceLabels(CatalogList {
            data: CatalogMode::Snapshot {
                labels: vec![label("Ready", Color::Sky)],
            },
            head: head(),
        }));
    }
    if let Call::AssignWorkspaceLabel(request) = call {
        return Outcome::Ok(Reply::AssignWorkspaceLabel(WorkspaceLabelAssigned {
            label: request.label.clone(),
            labels: vec![name("Ready"), request.label.name.clone()],
        }));
    }
    if let Call::UpdateWorkspaceLabel(request) = call {
        if request.new_name == Some(name("Taken")) {
            return Outcome::Err(refusal(WorkspaceLabelFailure::NameTaken));
        }
        return Outcome::Ok(Reply::UpdateWorkspaceLabel(WorkspaceLabelUpdated {
            label: Label {
                name: request
                    .new_name
                    .clone()
                    .unwrap_or_else(|| request.name.clone()),
                color: request.color.unwrap_or_default(),
            },
            affected: Count::new(2),
        }));
    }
    let deleted = WorkspaceLabelDeleted {
        affected: Count::new(4),
    };
    if let Call::InspectWorkspaceLabelDelete(_) = call {
        return Outcome::Ok(Reply::InspectWorkspaceLabelDelete(deleted));
    }
    if let Call::DeleteWorkspaceLabel(_) = call {
        return Outcome::Ok(Reply::DeleteWorkspaceLabel(deleted));
    }
    Outcome::Err(WireError::new(
        ErrorCode::InvalidRequest,
        "not a label call",
    ))
}

/// A node at the current protocol (labels are protocol 28) that answers
/// every request with `reply` and records it.
async fn serve(mut stream: DuplexStream, received: Received) -> TestResult {
    if !matches!(read(&mut stream).await?, Some(ClientFrame::Hello(_))) {
        return Err(std::io::Error::other("hello required").into());
    }
    let node = NodeInfo {
        id: NodeId::random(),
        name: "fake".to_owned(),
        hostname: "fake".to_owned(),
        os: "test".to_owned(),
        arch: "test".to_owned(),
        version: "0".to_owned(),
        protocol: ProtocolRange::CURRENT,
        started_at: DateTime::UNIX_EPOCH,
        state: NodeState::Serving,
        public_key: None,
        timezone: None,
    };
    let welcome = Welcome {
        protocol: ProtocolRange::CURRENT.max,
        node,
    };
    write(&mut stream, &ServerFrame::Welcome(welcome)).await?;
    while let Some(frame) = read(&mut stream).await? {
        let ClientFrame::Request { id, call } = frame else {
            continue;
        };
        let outcome = reply(&call);
        received.lock().map_err(|_poisoned| "lock")?.push(call);
        write(&mut stream, &ServerFrame::Response { id, outcome }).await?;
    }
    Ok(())
}

#[tokio::test]
async fn each_command_reaches_the_node_as_its_call_and_comes_back_typed() -> TestResult {
    let (client_pipe, peer_pipe) = tokio::io::duplex(PIPE_BYTES);
    let received = Received::default();
    let peer = tokio::spawn(serve(peer_pipe, std::sync::Arc::clone(&received)));
    let (reader, writer) = tokio::io::split(client_pipe);
    let client = NodeClient::over(
        vessel_rpc::Framed::new(reader, writer, vessel_rpc::MAX_FRAME),
        ClientInfo {
            name: "test".to_owned(),
            version: "0".to_owned(),
        },
        &vessel_rpc::ClientConfig::default(),
    )
    .await?;
    let home = crate::testing::Home::new();
    let (mut state, mut events) = crate::testing::state(&home);
    let binding = vessel_client::book::BookEntry {
        address: "ws://127.0.0.1:1/rpc".parse()?,
        id: Some(client.node().id),
        token: None,
        credential: None,
    };
    let connected = state.admit_connection("fake", &binding, client.clone(), Vec::new())?;
    let context = connected.context.clone();
    let _previous = state.clients.insert("fake".to_owned(), connected);
    let workspace = WorkspaceId::random();
    let asks = vec![
        Command::List {
            cursor: None,
            read: CatalogRead::Now,
        },
        Command::Assign {
            workspace,
            label: label("Blocked", Color::Violet),
            assignment: Assignment::Assigned,
        },
        Command::Update {
            original: name("Ready"),
            replacement: label("Ready", Color::Emerald),
        },
        Command::Update {
            original: name("Ready"),
            replacement: label("Taken", Color::Sky),
        },
        Command::InspectDelete(name("Ready")),
        Command::Delete(name("Ready")),
    ];
    let mut tickets = Tickets::default();
    let mut answers = Vec::new();
    for ask in asks {
        let ticket = tickets.issue()?;
        state
            .labels(Request {
                context: context.clone(),
                ticket,
                ask,
            })
            .await;
        let event = tokio::time::timeout(DEADLINE, events.recv())
            .await?
            .ok_or("the link closed")?;
        let crate::Event::Labels(Event {
            context: answered,
            ticket: answered_ticket,
            answer,
        }) = event
        else {
            return Err("not a labels answer".into());
        };
        assert_eq!(answered, context);
        assert_eq!(answered_ticket, ticket);
        answers.push(answer);
    }
    let calls = received.lock().map_err(|_poisoned| "lock")?.clone();
    assert!(matches!(
        &calls[0],
        Call::ListWorkspaceLabels(list) if list.cursor.is_none() && list.read == CatalogRead::Now
    ));
    assert!(matches!(
        &calls[1],
        Call::AssignWorkspaceLabel(assign)
            if assign.workspace == workspace && assign.assignment == Assignment::Assigned
    ));
    assert!(matches!(
        &calls[2],
        Call::UpdateWorkspaceLabel(edit)
            if edit.new_name.is_none() && edit.color == Some(Color::Emerald)
    ));
    assert!(matches!(&calls[4], Call::InspectWorkspaceLabelDelete(_)));
    assert!(matches!(&calls[5], Call::DeleteWorkspaceLabel(_)));
    assert!(matches!(
        &answers[0],
        Answer::Listed(Ok(CatalogList { data: CatalogMode::Snapshot { labels }, .. }))
            if labels == &vec![label("Ready", Color::Sky)]
    ));
    assert!(matches!(
        &answers[1],
        Answer::Assigned { workspace: actual, name: asked, result: Ok(labels) }
            if *actual == workspace && *asked == name("Blocked")
                && labels == &vec![name("Ready"), name("Blocked")]
    ));
    assert!(matches!(
        &answers[2],
        Answer::Updated(Ok(updated)) if *updated == label("Ready", Color::Emerald)
    ));
    assert!(matches!(
        &answers[3],
        Answer::Updated(Err(Failure::NameTaken))
    ));
    assert!(matches!(&answers[4], Answer::Inspected(Ok(count)) if *count == Count::new(4)));
    assert!(matches!(&answers[5], Answer::Deleted(Ok(count)) if *count == Count::new(4)));
    drop(client);
    drop(state);
    peer.abort();
    Ok(())
}
