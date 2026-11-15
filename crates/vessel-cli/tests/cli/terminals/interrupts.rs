//! Real signals leave the observed script/session running, using owned CLI children.

use std::path::Path;

use bytes::Bytes;
use vessel_client::NodeClient;
use vessel_proto::ports::{ListPorts, ListeningPort, PortOwner};
use vessel_proto::scripts::{RunScript, ScriptPlace, ScriptRunOutcome};
use vessel_proto::terminals::{GetTerminal, KillTerminal};
use vessel_proto::workspace::{AddProject, ApproveCommands, GetProjectCommands, ProjectCommands};
use vessel_proto::{SessionId, SessionInput, TerminalBytes, TerminalId, TerminalInput};
use vessel_testkit::Fake;

use super::{repository, start};
use crate::support::{Captured, ChildGuard, Fixture, interrupt, within};

fn script_file(root: &Path) {
    repository(root);
    let fake = Fake::Tui.path().unwrap();
    let quoted = format!("'{}'", fake.to_str().unwrap().replace('\'', "'\\''"));
    let command = format!("printf 'script-ready\\n'; exec {quoted} --sleeper");
    // This is project configuration; the executable is the already-built fake.
    std::fs::write(
        root.join("vessel.toml"),
        format!(
            "[scripts.wait]\ncommand = {}\n",
            serde_json::to_string(&command).unwrap()
        ),
    )
    .unwrap();
}

async fn script(owner: &NodeClient, root: &Path) -> TerminalId {
    script_file(root);
    let project = owner
        .call(AddProject {
            path: root.to_path_buf(),
            name: Some("app".parse().unwrap()),
        })
        .await
        .unwrap();
    let ProjectCommands::Listed { digest, .. } = owner
        .call(GetProjectCommands {
            project: project.id,
            base: None,
        })
        .await
        .unwrap()
    else {
        panic!("the fake script must be listed")
    };
    let approved = owner
        .call(ApproveCommands {
            project: project.id,
            digest,
            approved: true,
        })
        .await
        .unwrap();
    assert!(matches!(
        approved,
        ProjectCommands::Listed { approved: true, .. }
    ));
    let ScriptRunOutcome::Started { terminal } = owner
        .call(RunScript {
            place: ScriptPlace::Project {
                project: project.id,
            },
            script: "wait".parse().unwrap(),
        })
        .await
        .unwrap()
    else {
        panic!("the fake script must start")
    };
    terminal.id
}

async fn listen(owner: &NodeClient, session: SessionId) {
    owner
        .send(
            session,
            SessionInput::Terminal {
                input: TerminalInput::Data {
                    data: TerminalBytes::new(Bytes::from_static(b"listen\r")),
                },
            },
        )
        .await
        .unwrap();
}

async fn next_listener(
    owner: &NodeClient,
    session: SessionId,
    known: &[ListeningPort],
) -> ListeningPort {
    let mut list = owner.call(ListPorts::default()).await.unwrap();
    loop {
        if let Some(port) = list
            .ports
            .iter()
            .find(|port| port.owner == PortOwner::Session { session } && !known.contains(port))
        {
            return port.clone();
        }
        // The revision call waits for a change; this is not snapshot polling.
        list = owner
            .call(ListPorts {
                changed_from: Some(list.revision),
            })
            .await
            .unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interrupting_read_only_script_output_detaches_without_stopping_the_script() {
    within(async {
        let fixture = start().await;
        let root = tempfile::tempdir().unwrap();
        let owner = fixture.owner().await;
        let terminal = script(&owner, root.path()).await;
        let id = terminal.to_string();
        let mut child = ChildGuard(
            fixture.spawn_vessel_at(&fixture.node_address(), &["script", "output", &id]),
        );
        drop(child.0.stdin.take());
        let mut stdout = Captured::start(child.0.stdout.take().unwrap());
        let mut stderr = Captured::start(child.0.stderr.take().unwrap());
        stdout.wait_for("script-ready", 1).await;
        // The public harness acknowledges output, not a particular pending write.
        // The receiver's lifetime across writes is enforced by the pinned future.
        interrupt(&mut child.0).await;
        assert!(child.0.wait().await.unwrap().success(), "{}", stderr.text());
        stderr
            .wait_for(&format!("detached from {terminal}"), 1)
            .await;
        let current = owner
            .call(GetTerminal {
                terminal,
                changed_from: None,
            })
            .await
            .unwrap();
        assert!(current.status.is_live(), "{current:?}");
        let _stopped = owner.call(KillTerminal { terminal }).await.unwrap();
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interrupting_ports_after_an_update_keeps_the_session_and_listeners_running() {
    within(async {
        let fixture = Fixture::start().await;
        let session: SessionId = fixture.run_detached().await.parse().unwrap();
        let owner = fixture.owner().await;
        listen(&owner, session).await;
        let first = next_listener(&owner, session, &[]).await;
        let mut child =
            ChildGuard(fixture.spawn_vessel_at(&fixture.node_address(), &["ports", "--watch"]));
        drop(child.0.stdin.take());
        let mut stdout = Captured::start(child.0.stdout.take().unwrap());
        let stderr = Captured::start(child.0.stderr.take().unwrap());
        stdout
            .wait_for(&format!("{}:{}", first.address, first.port), 1)
            .await;
        listen(&owner, session).await;
        let second = next_listener(&owner, session, std::slice::from_ref(&first)).await;
        stdout
            .wait_for(&format!("{}:{}", second.address, second.port), 1)
            .await;
        // A changed snapshot is acknowledged; signal-during-write scheduling
        // cannot be forced by the public scanner/CLI harness without a test hook.
        interrupt(&mut child.0).await;
        assert!(child.0.wait().await.unwrap().success(), "{}", stderr.text());
        assert!(owner.session(session).await.unwrap().status.is_live());
        let current = owner.call(ListPorts::default()).await.unwrap();
        assert!(current.ports.contains(&first) && current.ports.contains(&second));
        owner.kill(session).await.unwrap();
        fixture.stop().await;
    })
    .await;
}
