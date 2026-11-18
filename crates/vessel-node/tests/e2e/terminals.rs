//! Protocol 11 over the wire (milestone 11, track C, ADR 0031): a shell
//! beside a session, a project's scripts and services under the approval
//! of ADR 0027, what an agent may and may not do with terminals, and the
//! ports a terminal's tree listens on.
//!
//! Unix only: the shell is `/bin/sh` and the scripts are its commands
//! (Windows' `cmd.exe` and `PowerShell` paths are CI's).

use std::path::{Path, PathBuf};

use bytes::Bytes;
use vessel_client::{NodeClient, TerminalStream};
use vessel_node::{Config, Shell};
use vessel_proto::ports::{ListPorts, PortOwner, PortScan};
use vessel_proto::scripts::{
    ListScripts, RestartScript, RunScript, ScriptPlace, ScriptRefusal, ScriptRunOutcome,
};
use vessel_proto::terminals::{
    CloseTerminal, CreateTerminal, GetTerminal, KillTerminal, ListTerminals, ReadTerminal,
    RenameTerminal, SendTerminal, TerminalInfo, TerminalKind, TerminalPlace,
};
use vessel_proto::workspace::{AddProject, ApproveCommands, ProjectCommands, RemoveProject};
use vessel_proto::{
    ErrorCode, EventSeq, HarnessKind, SessionEvent, SessionId, SessionStatus, TerminalBytes,
    TerminalId, TerminalInput,
};
use vessel_testkit::Fake;

use crate::agents::{as_agent, code, credential, following, vessel_mcp};
use crate::support::{Fixture, step, structured_spec, terminal_spec, within};
use crate::workspace::{lookup, repository};

/// A node whose terminals run `/bin/sh`.
fn with_shell(config: &mut Config) {
    config.terminals.shell = Some(Shell::plain("/bin/sh"));
}

fn typed(text: &str) -> TerminalInput {
    TerminalInput::Data {
        data: TerminalBytes::new(Bytes::from(text.to_owned())),
    }
}

async fn type_in(client: &NodeClient, terminal: TerminalId, text: &str) {
    let _ack = client
        .call(SendTerminal {
            terminal,
            input: typed(text),
        })
        .await
        .unwrap();
}

fn beside(session: SessionId) -> TerminalPlace {
    TerminalPlace::Session { session }
}

async fn open(client: &NodeClient, place: TerminalPlace) -> TerminalInfo {
    client
        .call(CreateTerminal {
            place,
            size: None,
            title: None,
        })
        .await
        .unwrap()
}

/// Read `stream` until its output holds `needle`; the output so far.
async fn until_output(stream: &mut TerminalStream, seen: &mut String, needle: &str) {
    step(format!("the terminal prints {needle:?}"), async {
        while !seen.contains(needle) {
            let Some(record) = stream.next().await else {
                panic!("the terminal ended before {needle:?}; it printed {seen:?}");
            };
            if let SessionEvent::Terminal(output) = record.unwrap().event {
                seen.push_str(&String::from_utf8_lossy(output.data.bytes()));
            }
        }
    })
    .await;
}

/// Read `stream` to its end; the last status and everything printed.
async fn to_end(stream: &mut TerminalStream) -> (Option<SessionStatus>, String) {
    step("the terminal ends", async {
        let mut status = None;
        let mut text = String::new();
        while let Some(record) = stream.next().await {
            match record.unwrap().event {
                SessionEvent::Terminal(output) => {
                    text.push_str(&String::from_utf8_lossy(output.data.bytes()));
                }
                SessionEvent::Status { status: now } => status = Some(now),
                SessionEvent::Agent { event } => {
                    if let vessel_proto::AgentEvent::HarnessLog { line } = event {
                        text.push('[');
                        text.push_str(&line);
                        text.push(']');
                    }
                }
                other @ (SessionEvent::ProfileChanged { .. }
                | SessionEvent::ModelCall(_)
                | SessionEvent::OptionsChanged { .. }
                | SessionEvent::Renamed { .. }
                | SessionEvent::QueueChanged { .. }
                | SessionEvent::CommandsChanged { .. }
                | SessionEvent::UsageChanged { .. }
                | SessionEvent::Parked { .. }
                | SessionEvent::Woken
                | SessionEvent::Forked { .. }
                | SessionEvent::Rewound { .. }
                | SessionEvent::Reloaded { .. }) => panic!("a terminal logs no {other:?}"),
            }
        }
        (status, text)
    })
    .await
}

/// `terminal.get` until `done` holds of the terminal, waiting on its
/// revision.
async fn get_until(
    client: &NodeClient,
    terminal: TerminalId,
    what: &str,
    done: impl Fn(&TerminalInfo) -> bool,
) -> TerminalInfo {
    step(what, async {
        let mut info = client
            .call(GetTerminal {
                terminal,
                changed_from: None,
            })
            .await
            .unwrap();
        while !done(&info) {
            info = client
                .call(GetTerminal {
                    terminal,
                    changed_from: Some(info.revision),
                })
                .await
                .unwrap();
        }
        info
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[expect(
    clippy::too_many_lines,
    reason = "one story told in order; split, its steps would lose their state"
)]
async fn a_shell_beside_a_session_runs_what_is_typed_and_ends_when_asked() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start_with(with_shell).await;
        let owner = fixture.connect(&node).await;
        let session = owner
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap();

        let shell = open(&owner, beside(session.id)).await;
        assert_eq!(shell.place, beside(session.id));
        assert_eq!(
            shell.folder,
            fixture.cwd(),
            "it opens in the session's folder"
        );
        assert!(matches!(shell.kind, TerminalKind::Shell { .. }));
        assert!(shell.is_live());
        let mut stream = owner
            .follow_terminal(shell.id, EventSeq::ZERO)
            .await
            .unwrap();
        let mut seen = String::new();
        // The terminal's own variables, and none of Vessel's credentials.
        type_in(
            &owner,
            shell.id,
            "echo vessel-$((40+2)) term=$TERM token=${VESSEL_TOKEN:-none}\n",
        )
        .await;
        until_output(
            &mut stream,
            &mut seen,
            "vessel-42 term=xterm-256color token=none",
        )
        .await;

        let page = owner
            .call(ReadTerminal {
                terminal: shell.id,
                from: EventSeq::ZERO,
                limit: 512,
                wait: None,
            })
            .await
            .unwrap();
        assert_eq!(
            page.records.first().map(|record| record.seq),
            Some(EventSeq::ZERO)
        );

        let renamed = owner
            .call(RenameTerminal {
                terminal: shell.id,
                title: "build".parse().unwrap(),
            })
            .await
            .unwrap();
        assert_eq!(
            renamed.title.as_ref().map(ToString::to_string).as_deref(),
            Some("build")
        );
        let listed = owner
            .call(ListTerminals {
                place: Some(beside(session.id)),
            })
            .await
            .unwrap();
        assert_eq!(listed.terminals.len(), 1);

        // Killed: ended, kept, its output still readable.
        let _ack = owner
            .call(KillTerminal { terminal: shell.id })
            .await
            .unwrap();
        let (status, _) = to_end(&mut stream).await;
        assert!(
            matches!(status, Some(SessionStatus::Exited { .. })),
            "{status:?}"
        );
        let ended = get_until(&owner, shell.id, "the shell is ended", |info| {
            !info.is_live()
        })
        .await;
        assert!(ended.ended_at.is_some());
        assert_eq!(
            code(
                owner
                    .call(SendTerminal {
                        terminal: shell.id,
                        input: typed("x"),
                    })
                    .await
            ),
            ErrorCode::Conflict
        );

        // Closed: forgotten.
        let _ack = owner
            .call(CloseTerminal { terminal: shell.id })
            .await
            .unwrap();
        assert_eq!(
            code(
                owner
                    .call(GetTerminal {
                        terminal: shell.id,
                        changed_from: None,
                    })
                    .await
            ),
            ErrorCode::NotFound
        );
        // Beside a session the node does not have: not found.
        assert_eq!(
            code(
                owner
                    .call(CreateTerminal {
                        place: beside(SessionId::random()),
                        size: None,
                        title: None,
                    })
                    .await
            ),
            ErrorCode::NotFound
        );
        owner.kill(session.id).await.unwrap();
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_node_without_a_shell_opens_no_terminal() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let session = owner
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap();
        assert_eq!(
            code(
                owner
                    .call(CreateTerminal {
                        place: beside(session.id),
                        size: None,
                        title: None,
                    })
                    .await
            ),
            ErrorCode::Unsupported
        );
        owner.kill(session.id).await.unwrap();
        node.stop().await.unwrap();
    })
    .await;
}

/// A project with a script and a service in its `vessel.toml`.
fn project_with_scripts(folder: &Path) -> PathBuf {
    let root = std::fs::canonicalize(folder).unwrap().join("app");
    std::fs::create_dir(&root).unwrap();
    repository(&root);
    std::fs::write(
        root.join("vessel.toml"),
        "[scripts.hello]\ncommand = 'echo hello-$VESSEL_SCRIPT'\n\n\
         [services.web]\ncommand = 'echo web-up; exec sleep 300'\n",
    )
    .unwrap();
    root
}

fn started(outcome: ScriptRunOutcome) -> TerminalInfo {
    match outcome {
        ScriptRunOutcome::Started { terminal } => *terminal,
        other @ (ScriptRunOutcome::NeedsApproval { .. } | ScriptRunOutcome::Refused { .. }) => {
            panic!("not started: {other:?}")
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[expect(
    clippy::too_many_lines,
    reason = "one story told in order; split, its steps would lose their state"
)]
async fn scripts_run_once_approved_and_a_service_holds_its_project() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| {
                with_shell(config);
                config.harness_lookup = lookup(&fixture);
            })
            .await;
        let owner = fixture.connect(&node).await;
        let folder = tempfile::tempdir().unwrap();
        let root = project_with_scripts(folder.path());
        let project = owner
            .call(AddProject {
                path: root.clone(),
                name: None,
            })
            .await
            .unwrap();
        let place = ScriptPlace::Project {
            project: project.id,
        };

        let listed = owner.call(ListScripts { place }).await.unwrap();
        assert_eq!(listed.root, root);
        let ProjectCommands::Listed {
            scripts,
            approved,
            digest,
            ..
        } = listed.commands
        else {
            panic!("no scripts: {:?}", listed.commands);
        };
        assert!(!approved);
        let names: Vec<String> = scripts
            .iter()
            .map(|script| script.name.to_string())
            .collect();
        assert_eq!(names, ["hello", "web"]);

        // Not approved: nothing runs, and the set is shown to approve.
        let run = |script: &str| RunScript {
            place,
            script: script.parse().unwrap(),
        };
        let asked = owner.call(run("hello")).await.unwrap();
        assert!(
            matches!(asked, ScriptRunOutcome::NeedsApproval { .. }),
            "{asked:?}"
        );
        let _commands = owner
            .call(ApproveCommands {
                project: project.id,
                digest,
                approved: true,
            })
            .await
            .unwrap();

        let hello = started(owner.call(run("hello")).await.unwrap());
        assert!(matches!(hello.kind, TerminalKind::Script { .. }));
        let mut stream = owner
            .follow_terminal(hello.id, EventSeq::ZERO)
            .await
            .unwrap();
        let (status, text) = to_end(&mut stream).await;
        assert!(text.contains("hello-hello"), "{text}");
        assert_eq!(status, Some(SessionStatus::Exited { code: Some(0) }));
        assert_eq!(
            owner.call(run("nothing")).await.unwrap(),
            ScriptRunOutcome::Refused {
                reason: ScriptRefusal::NoSuchScript
            }
        );

        // A service: one run at a time, even asked twice at once; it holds
        // its project.
        let (first, second) = tokio::join!(owner.call(run("web")), owner.call(run("web")));
        let (first, second) = (first.unwrap(), second.unwrap());
        let (web, other) = match (first, second) {
            (ScriptRunOutcome::Started { terminal }, other)
            | (other, ScriptRunOutcome::Started { terminal }) => (*terminal, other),
            both => panic!("neither started: {both:?}"),
        };
        assert_eq!(
            other,
            ScriptRunOutcome::Refused {
                reason: ScriptRefusal::AlreadyRunning { terminal: web.id }
            }
        );
        let mut stream = owner.follow_terminal(web.id, EventSeq::ZERO).await.unwrap();
        let mut seen = String::new();
        until_output(&mut stream, &mut seen, "web-up").await;
        assert_eq!(
            owner.call(run("web")).await.unwrap(),
            ScriptRunOutcome::Refused {
                reason: ScriptRefusal::AlreadyRunning { terminal: web.id }
            }
        );
        assert_eq!(
            code(
                owner
                    .call(RemoveProject {
                        project: project.id,
                    })
                    .await
            ),
            ErrorCode::Conflict
        );
        // Restarted in place: the same terminal, its output going on.
        let again = started(
            owner
                .call(RestartScript { terminal: web.id })
                .await
                .unwrap(),
        );
        assert_eq!(again.id, web.id);
        let mut more = String::new();
        until_output(&mut stream, &mut more, "web-up").await;

        let _ack = owner.call(KillTerminal { terminal: web.id }).await.unwrap();
        let (status, _) = to_end(&mut stream).await;
        assert!(
            matches!(status, Some(SessionStatus::Exited { .. })),
            "{status:?}"
        );
        // Ended, it no longer holds the project; its runs go with it.
        let _ack = owner
            .call(RemoveProject {
                project: project.id,
            })
            .await
            .unwrap();
        let left = owner.call(ListTerminals { place: None }).await.unwrap();
        assert!(left.terminals.is_empty(), "{left:?}");
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[expect(
    clippy::too_many_lines,
    reason = "one story told in order; split, its steps would lose their state"
)]
async fn an_agent_opens_terminals_only_where_a_person_allowed_and_uses_only_its_own() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(
            fixture.data_dir().profiles_path(),
            "[profiles.claude-term]\nharness = \"claude\"\nagent_terminals = true\n",
        )
        .unwrap();
        let node = fixture
            .start_with(|config| {
                with_shell(config);
                config.vessel_mcp = Some(vessel_mcp(&fixture));
            })
            .await;
        let owner = fixture.connect(&node).await;

        // Its session's profile does not allow it: forbidden.
        let plain = owner
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap();
        let mut plain = following(&owner, plain.id, EventSeq::ZERO).await;
        let (_, socket, auth) = credential(&owner, &mut plain).await;
        let refused = as_agent(&socket, &auth).await;
        assert_eq!(
            code(
                refused
                    .call(CreateTerminal {
                        place: beside(plain.id),
                        size: None,
                        title: None,
                    })
                    .await
            ),
            ErrorCode::Forbidden
        );
        // Nor by starting a session whose profile allows it: its own
        // profile must too.
        let mut marked = structured_spec(&fixture, None);
        marked.profile = Some("claude-term".parse().unwrap());
        let child = refused.create_session(marked).await.unwrap();
        assert_eq!(
            code(
                refused
                    .call(CreateTerminal {
                        place: beside(child.id),
                        size: None,
                        title: None,
                    })
                    .await
            ),
            ErrorCode::Forbidden
        );
        refused.kill(child.id).await.unwrap();
        // Scripts are a person's.
        assert_eq!(
            code(
                refused
                    .call(ListScripts {
                        place: ScriptPlace::Session { session: plain.id },
                    })
                    .await
            ),
            ErrorCode::Forbidden
        );

        // A profile a person marked: its own terminals, four at once.
        let mut spec = structured_spec(&fixture, None);
        spec.profile = Some("claude-term".parse().unwrap());
        let allowed = owner.create_session(spec).await.unwrap();
        let mut allowed = following(&owner, allowed.id, EventSeq::ZERO).await;
        let (_, socket, auth) = credential(&owner, &mut allowed).await;
        let agent = as_agent(&socket, &auth).await;
        let mut own = Vec::new();
        for _ in 0..4 {
            own.push(open(&agent, beside(allowed.id)).await);
        }
        assert_eq!(
            code(
                agent
                    .call(CreateTerminal {
                        place: beside(allowed.id),
                        size: None,
                        title: None,
                    })
                    .await
            ),
            ErrorCode::Forbidden,
            "a fifth"
        );
        // Not beside a session it may not operate.
        assert_eq!(
            code(
                agent
                    .call(CreateTerminal {
                        place: beside(plain.id),
                        size: None,
                        title: None,
                    })
                    .await
            ),
            ErrorCode::Forbidden
        );
        // A person's terminal is not the agent's to see or use.
        let persons = open(&owner, beside(allowed.id)).await;
        assert_eq!(
            code(
                agent
                    .call(SendTerminal {
                        terminal: persons.id,
                        input: typed("id\n"),
                    })
                    .await
            ),
            ErrorCode::Forbidden
        );
        let seen = agent.call(ListTerminals { place: None }).await.unwrap();
        assert_eq!(seen.terminals.len(), 4);
        assert!(seen.terminals.iter().all(|info| info.id != persons.id));

        // Its terminals end with its session; the person's stays.
        owner.kill(allowed.id).await.unwrap();
        for info in &own {
            let _ended = get_until(
                &owner,
                info.id,
                "an agent's terminal ends with it",
                |info| !info.is_live(),
            )
            .await;
        }
        let still = owner
            .call(GetTerminal {
                terminal: persons.id,
                changed_from: None,
            })
            .await
            .unwrap();
        assert!(still.is_live());
        owner.kill(plain.id).await.unwrap();
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_ports_a_terminals_tree_listens_on_are_found() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start_with(with_shell).await;
        let owner = fixture.connect(&node).await;
        let session = owner
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap();
        let shell = open(&owner, beside(session.id)).await;
        let mut stream = owner
            .follow_terminal(shell.id, EventSeq::ZERO)
            .await
            .unwrap();
        let mut seen = String::new();
        let tui = Fake::Tui.path().unwrap();
        type_in(&owner, shell.id, &format!("{}\n", tui.display())).await;
        until_output(&mut stream, &mut seen, "ready").await;
        type_in(&owner, shell.id, "listen\n").await;
        until_output(&mut stream, &mut seen, "listening ").await;
        until_output(&mut stream, &mut seen, "\n").await;
        let port: u16 = seen
            .split("listening ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|port| port.parse().ok())
            .unwrap_or_else(|| panic!("no port in {seen:?}"));

        let found = step("the port is found", async {
            let mut list = owner.call(ListPorts { changed_from: None }).await.unwrap();
            loop {
                if let Some(found) = list
                    .ports
                    .iter()
                    .find(|listener| listener.port == port)
                    .cloned()
                {
                    return (found, list.scan);
                }
                list = owner
                    .call(ListPorts {
                        changed_from: Some(list.revision),
                    })
                    .await
                    .unwrap();
            }
        })
        .await;
        let (listener, scan) = found;
        assert_eq!(listener.owner, PortOwner::Terminal { terminal: shell.id });
        assert!(listener.address.is_loopback());
        assert!(matches!(scan, PortScan::Scanned { .. }), "{scan:?}");

        owner.kill(session.id).await.unwrap();
        node.stop().await.unwrap();
    })
    .await;
}

/// Deleting a session ends its terminals and forgets them, before its
/// worktree is tidied (they would hold it).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_a_session_ends_and_forgets_its_terminals() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start_with(with_shell).await;
        let owner = fixture.connect(&node).await;
        let session = owner
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap();
        let shell = open(&owner, beside(session.id)).await;
        let mut log = owner.follow(session.id, EventSeq::ZERO).await.unwrap();
        owner.kill(session.id).await.unwrap();
        step("the session ends", async {
            while let Some(record) = log.next().await {
                let _record = record.unwrap();
            }
        })
        .await;
        // Ending the session alone leaves its terminal running.
        let live = owner
            .call(GetTerminal {
                terminal: shell.id,
                changed_from: None,
            })
            .await
            .unwrap();
        assert!(live.is_live());

        let _deleted = owner.delete_session(session.id).await.unwrap();
        assert_eq!(
            code(
                owner
                    .call(GetTerminal {
                        terminal: shell.id,
                        changed_from: None,
                    })
                    .await
            ),
            ErrorCode::NotFound
        );
        node.stop().await.unwrap();
    })
    .await;
}
