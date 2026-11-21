//! Terminals that belong to a workspace folder over the wire (ADR 0121):
//! a worktree's and an unregistered folder's shells outlive archiving the
//! session in view; a worktree's removal ends its shells only when asked
//! and no live agent keeps it anyway; a peer below the protocol never sees
//! them.
//!
//! Unix only, as the terminals' tests are: the shell is `/bin/sh`.

use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use vessel_client::NodeClient;
use vessel_node::{Node, Shell};
use vessel_proto::terminals::{
    CreateTerminal, GetTerminal, ListTerminals, TerminalInfo, TerminalList, TerminalPlace,
    WORKSPACE_TERMINALS_PROTOCOL,
};
use vessel_proto::workspace::{
    AddProject, IfLost, KeptReason, ProjectInfo, RemoveWorktree, RemoveWorktreeOutcome,
    SessionPlace,
};
use vessel_proto::{
    Call, ClientFrame, ErrorCode, Hello, Mark, Outcome, ProtocolRange, ProtocolVersion, Reply,
    RequestId, ServerFrame,
};

use crate::support::{Fixture, client, step, url, within};
use crate::workspace::{in_place, lookup, new_worktree, repository};

async fn node_with_a_project(fixture: &Fixture) -> (Node, NodeClient, ProjectInfo) {
    let node = fixture
        .start_with(|config| {
            config.harness_lookup = lookup(fixture);
            config.terminals.shell = Some(Shell::plain("/bin/sh"));
        })
        .await;
    let owner = fixture.connect(&node).await;
    let root = fixture.cwd().join("app");
    std::fs::create_dir(&root).unwrap();
    repository(&root);
    let project = owner
        .call(AddProject {
            path: root,
            name: None,
        })
        .await
        .unwrap();
    (node, owner, project)
}

async fn open(client: &NodeClient, place: TerminalPlace) -> TerminalInfo {
    client
        .call(CreateTerminal {
            place,
            size: None,
            title: None,
            profile: None,
        })
        .await
        .unwrap()
}

async fn listed(client: &NodeClient, place: TerminalPlace) -> TerminalList {
    client
        .call(ListTerminals {
            place: Some(place),
            changed_from: None,
        })
        .await
        .unwrap()
}

async fn info_of(client: &NodeClient, terminal: &TerminalInfo) -> TerminalInfo {
    client
        .call(GetTerminal {
            terminal: terminal.id,
            changed_from: None,
        })
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_worktree_and_a_folder_shell_outlive_archiving_the_session_in_view() {
    within(async {
        let fixture = Fixture::new();
        let (node, owner, project) = node_with_a_project(&fixture).await;
        let worktree = new_worktree(&owner, project.id).await;
        let session = owner
            .create_session(in_place(
                &fixture,
                SessionPlace::Worktree {
                    worktree: worktree.id,
                },
            ))
            .await
            .unwrap();
        let in_worktree = open(
            &owner,
            TerminalPlace::Worktree {
                worktree: worktree.id,
            },
        )
        .await;
        let folder = tempfile::tempdir().unwrap();
        let in_folder = open(
            &owner,
            TerminalPlace::Folder {
                path: folder.path().join("."),
            },
        )
        .await;
        let canonical = std::fs::canonicalize(folder.path()).unwrap();
        assert_eq!(
            in_folder.place,
            TerminalPlace::Folder {
                path: canonical.clone()
            },
            "a folder place is kept canonical"
        );

        // The agent in view is put away: neither shell is beside it.
        let _archived = owner.mark(session.id, Mark::Archive).await.unwrap();
        assert!(info_of(&owner, &in_worktree).await.is_live());
        assert!(info_of(&owner, &in_folder).await.is_live());

        // Each workspace lists its own: a worktree is not its project's.
        let worktrees = listed(
            &owner,
            TerminalPlace::Worktree {
                worktree: worktree.id,
            },
        )
        .await;
        assert_eq!(worktrees.terminals.len(), 1);
        let projects = listed(
            &owner,
            TerminalPlace::Project {
                project: project.id,
                dir: None,
            },
        )
        .await;
        assert!(projects.terminals.is_empty(), "{projects:?}");
        let folders = listed(
            &owner,
            TerminalPlace::Folder {
                path: folder.path().to_path_buf(),
            },
        )
        .await;
        assert_eq!(
            folders.terminals.first().map(|info| info.id),
            Some(in_folder.id)
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn removing_a_worktree_ends_its_shells_only_when_asked() {
    within(async {
        let fixture = Fixture::new();
        let (node, owner, project) = node_with_a_project(&fixture).await;
        let worktree = new_worktree(&owner, project.id).await;
        let shell = open(
            &owner,
            TerminalPlace::Worktree {
                worktree: worktree.id,
            },
        )
        .await;
        let remove = |end_terminals| RemoveWorktree {
            worktree: worktree.id,
            if_lost: IfLost::Discard,
            end_terminals,
        };

        // Not asked (every older client): the live shell keeps it.
        let kept = owner.call(remove(false)).await.unwrap();
        assert!(
            matches!(&kept, RemoveWorktreeOutcome::Kept { reasons, .. } if reasons == &[KeptReason::InUse]),
            "{kept:?}"
        );
        assert!(info_of(&owner, &shell).await.is_live());

        // Asked, but an agent runs in it, which refuses the removal anyway:
        // the shell is not ended for nothing.
        let agent = owner
            .create_session(in_place(
                &fixture,
                SessionPlace::Worktree {
                    worktree: worktree.id,
                },
            ))
            .await
            .unwrap();
        let held = owner.call(remove(true)).await.unwrap();
        assert!(
            matches!(&held, RemoveWorktreeOutcome::Kept { reasons, .. } if reasons == &[KeptReason::InUse]),
            "{held:?}"
        );
        assert!(info_of(&owner, &shell).await.is_live());
        owner.kill(agent.id).await.unwrap();
        step("the agent ends", async {
            while owner.session(agent.id).await.unwrap().status.is_live() {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
        .await;
        assert!(
            info_of(&owner, &shell).await.is_live(),
            "the agent's end leaves the worktree's shell"
        );

        // Asked: the shell ends, the worktree goes, and so does the shell.
        let removed = owner.call(remove(true)).await.unwrap();
        assert!(
            matches!(removed, RemoveWorktreeOutcome::Removed { .. }),
            "{removed:?}"
        );
        let gone = owner
            .call(GetTerminal {
                terminal: shell.id,
                changed_from: None,
            })
            .await
            .unwrap_err();
        assert_eq!(gone.code(), Some(ErrorCode::NotFound));
        node.stop().await.unwrap();
    })
    .await;
}

/// `calls` answered to a raw peer that speaks only `version`.
pub(crate) async fn as_older(
    fixture: &Fixture,
    node: &Node,
    version: ProtocolVersion,
    calls: Vec<Call>,
) -> Vec<Outcome> {
    let mut request = url(node).as_uri().clone().into_client_request().unwrap();
    let bearer = format!("Bearer {}", fixture.token().expose());
    let _previous = request
        .headers_mut()
        .insert(AUTHORIZATION, bearer.parse().unwrap());
    let (mut socket, _response) = tokio_tungstenite::connect_async(request).await.unwrap();
    let hello = ClientFrame::Hello(Hello {
        protocol: ProtocolRange {
            min: version,
            max: version,
        },
        client: client(),
    });
    socket
        .send(Message::text(serde_json::to_string(&hello).unwrap()))
        .await
        .unwrap();
    let Some(Ok(Message::Text(_welcome))) = socket.next().await else {
        panic!("a welcome");
    };
    let mut outcomes = Vec::new();
    for (at, call) in (1_u64..).zip(calls) {
        let frame = ClientFrame::Request {
            id: RequestId::new(at),
            call,
        };
        socket
            .send(Message::text(serde_json::to_string(&frame).unwrap()))
            .await
            .unwrap();
        let Some(Ok(Message::Text(text))) = socket.next().await else {
            panic!("the node answers each call");
        };
        let ServerFrame::Response { outcome, .. } =
            serde_json::from_str::<ServerFrame>(text.as_str()).unwrap()
        else {
            panic!("a response: {text}");
        };
        outcomes.push(outcome);
    }
    outcomes
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_older_peer_never_sees_a_workspace_place() {
    within(async {
        let fixture = Fixture::new();
        let (node, owner, _project) = node_with_a_project(&fixture).await;
        let folder = tempfile::tempdir().unwrap();
        let shell = open(
            &owner,
            TerminalPlace::Folder {
                path: folder.path().to_path_buf(),
            },
        )
        .await;
        let since = WORKSPACE_TERMINALS_PROTOCOL.unwrap();
        let older = ProtocolVersion::new(since.get().saturating_sub(1));
        let outcomes = as_older(
            &fixture,
            &node,
            older,
            vec![
                Call::ListTerminals(ListTerminals::default()),
                Call::GetTerminal(GetTerminal {
                    terminal: shell.id,
                    changed_from: None,
                }),
            ],
        )
        .await;
        match outcomes.as_slice() {
            [Outcome::Ok(Reply::ListTerminals(list)), Outcome::Err(error)] => {
                assert!(list.terminals.iter().all(|info| info.id != shell.id));
                assert_eq!(error.code, ErrorCode::NotFound);
            }
            other => panic!("{other:?}"),
        }
        node.stop().await.unwrap();
    })
    .await;
}
