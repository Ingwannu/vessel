//! An agent's shells in its session's workspace (TRACKER 832, ADR 0121
//! amendment 1), over the wire: an agent naming its session gets a shell in
//! that session's worktree or folder, which outlives the session's archive
//! as a person's shell there does and ends with the worktree only when its
//! remover asks; an agent call below the protocol, or for a session with
//! no folder of its own, keeps the place it named.
//!
//! Unix only, as the terminals' tests are: the shell is `/bin/sh`.

use futures_util::{SinkExt as _, StreamExt as _};
use vessel_client::NodeClient;
use vessel_node::{Node, Shell};
use vessel_proto::terminals::{
    CreateTerminal, GetTerminal, ListTerminals, TerminalInfo, TerminalPlace,
    WORKSPACE_TERMINALS_PROTOCOL,
};
use vessel_proto::workspace::{
    AddProject, IfLost, ProjectInfo, RemoveWorktree, RemoveWorktreeOutcome, SessionPlace,
};
use vessel_proto::{
    AgentAuth, Call, ClientFrame, ErrorCode, Hello, Mark, Outcome, ProtocolRange, ProtocolVersion,
    Reply, RequestId, ServerFrame, SessionId, SessionSpec,
};

use crate::agents::{as_agent, credential, following, vessel_mcp};
use crate::support::{Fixture, client, step, structured_spec, within};
use crate::workspace::{lookup, new_worktree, repository};

/// A node whose terminals run `/bin/sh`, whose sessions get the `vessel`
/// server, with a profile that lets agents open terminals and a project.
async fn node_with_a_project(fixture: &Fixture) -> (Node, NodeClient, ProjectInfo) {
    std::fs::write(
        fixture.data_dir().profiles_path(),
        "[profiles.claude-term]\nharness = \"claude\"\nagent_terminals = true\n",
    )
    .unwrap();
    let node = fixture
        .start_with(|config| {
            config.harness_lookup = lookup(fixture);
            config.terminals.shell = Some(Shell::plain("/bin/sh"));
            config.vessel_mcp = Some(vessel_mcp(fixture));
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

/// A running session with the marked profile, and its agent's credential.
async fn agent_session(
    owner: &NodeClient,
    mut spec: SessionSpec,
) -> (SessionId, std::path::PathBuf, AgentAuth) {
    spec.profile = Some("claude-term".parse().unwrap());
    let created = owner.create_session(spec).await.unwrap();
    let mut running = following(owner, created.id, vessel_proto::EventSeq::ZERO).await;
    let (_, socket, auth) = credential(owner, &mut running).await;
    (created.id, socket, auth)
}

fn beside(session: SessionId) -> CreateTerminal {
    CreateTerminal {
        place: TerminalPlace::Session { session },
        size: None,
        title: None,
    }
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

async fn archived_and_ended(owner: &NodeClient, session: SessionId) {
    let _archived = owner.mark(session, Mark::Archive).await.unwrap();
    step("the archived session ends", async {
        while owner.session(session).await.unwrap().status.is_live() {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agents_worktree_shell_outlives_its_archive_and_ends_with_the_worktree_when_asked() {
    within(async {
        let fixture = Fixture::new();
        let (node, owner, project) = node_with_a_project(&fixture).await;
        let worktree = new_worktree(&owner, project.id).await;
        let mut spec = structured_spec(&fixture, None).with_place(SessionPlace::Worktree {
            worktree: worktree.id,
        });
        spec.cwd = None;
        let (session, socket, auth) = agent_session(&owner, spec).await;
        let agent = as_agent(&socket, &auth).await;

        // Named by its session, placed in that session's worktree, and
        // listed there, not beside the session.
        let shell = agent.call(beside(session)).await.unwrap();
        let in_worktree = TerminalPlace::Worktree {
            worktree: worktree.id,
        };
        assert_eq!(shell.place, in_worktree);
        let listed = |place| {
            owner.call(ListTerminals {
                place: Some(place),
                changed_from: None,
            })
        };
        let there = listed(in_worktree.clone()).await.unwrap();
        assert_eq!(there.terminals.first().map(|info| info.id), Some(shell.id));
        let beside_it = listed(TerminalPlace::Session { session }).await.unwrap();
        assert!(beside_it.terminals.is_empty(), "{beside_it:?}");

        // Its session put away: the worktree's shell stays, as Paseo's.
        archived_and_ended(&owner, session).await;
        assert!(info_of(&owner, &shell).await.is_live());

        // Removing the worktree with its terminals ends it.
        let removed = owner
            .call(RemoveWorktree {
                worktree: worktree.id,
                if_lost: IfLost::Discard,
                end_terminals: true,
            })
            .await
            .unwrap();
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agents_folder_shell_outlives_its_archive_and_older_or_folderless_calls_stay_beside() {
    within(async {
        let fixture = Fixture::new();
        let (node, owner, _project) = node_with_a_project(&fixture).await;
        let (session, socket, auth) = agent_session(&owner, structured_spec(&fixture, None)).await;
        let agent = as_agent(&socket, &auth).await;

        // Outside projects: the session's folder, kept canonical.
        let in_folder = agent.call(beside(session)).await.unwrap();
        assert_eq!(
            in_folder.place,
            TerminalPlace::Folder {
                path: std::fs::canonicalize(fixture.cwd()).unwrap(),
            }
        );

        // A session with no folder of its own keeps its agent's shell beside
        // it, as the page keeps such a session's shells: it cannot name that
        // folder (node review R1).
        let mut folderless = structured_spec(&fixture, None);
        folderless.cwd = None;
        let (other, other_socket, other_auth) = agent_session(&owner, folderless).await;
        let other_agent = as_agent(&other_socket, &other_auth).await;
        let kept = other_agent.call(beside(other)).await.unwrap();
        assert_eq!(kept.place, TerminalPlace::Session { session: other });

        // The same call from an agent below the protocol: beside the
        // session, as it named (it could not read a folder place).
        let since = WORKSPACE_TERMINALS_PROTOCOL.unwrap();
        let older = ProtocolVersion::new(since.get().saturating_sub(1));
        let outcome =
            agent_call_as_older(&socket, &auth, older, Call::CreateTerminal(beside(session))).await;
        let Outcome::Ok(Reply::CreateTerminal(beside_it)) = outcome else {
            panic!("{outcome:?}");
        };
        assert_eq!(beside_it.place, TerminalPlace::Session { session });

        // Its session put away: the shell beside it ends, the folder's
        // stays. Waiting for the one to end orders the check after both
        // ends ran.
        archived_and_ended(&owner, session).await;
        step("the shell beside the session ends", async {
            while info_of(&owner, &beside_it).await.is_live() {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
        .await;
        assert!(info_of(&owner, &in_folder).await.is_live());
        node.stop().await.unwrap();
    })
    .await;
}

/// `call` answered to an agent that speaks only `version`, over the node's
/// local endpoint.
async fn agent_call_as_older(
    socket: &std::path::Path,
    auth: &AgentAuth,
    version: ProtocolVersion,
    call: Call,
) -> Outcome {
    let stream = tokio::net::UnixStream::connect(socket).await.unwrap();
    let (reader, writer) = tokio::io::split(stream);
    let mut framed = vessel_rpc::Framed::new(reader, writer, vessel_rpc::MAX_FRAME);
    framed
        .send(serde_json::to_string(auth).unwrap())
        .await
        .unwrap();
    let hello = ClientFrame::Hello(Hello {
        protocol: ProtocolRange {
            min: version,
            max: version,
        },
        client: client(),
    });
    framed
        .send(serde_json::to_string(&hello).unwrap())
        .await
        .unwrap();
    let Some(Ok(_welcome)) = framed.next().await else {
        panic!("a welcome");
    };
    let frame = ClientFrame::Request {
        id: RequestId::new(1),
        call,
    };
    framed
        .send(serde_json::to_string(&frame).unwrap())
        .await
        .unwrap();
    let Some(Ok(text)) = framed.next().await else {
        panic!("the node answers the call");
    };
    let ServerFrame::Response { outcome, .. } = serde_json::from_str::<ServerFrame>(&text).unwrap()
    else {
        panic!("a response: {text}");
    };
    outcome
}
