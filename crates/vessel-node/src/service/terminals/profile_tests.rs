//! ADR 0117 below the server's protocol gate (the number is the integrator's
//! to assign, so the server refuses the fields until then), and the gate
//! itself. The profiles run the testkit's `fake-tui`, never a real agent or
//! a login shell; `args` makes it print its arguments.
#![cfg(unix)]

use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use vessel_client::NodeClient;
use vessel_harness::{HarnessEnv, Lookup};
use vessel_proto::settings::{GetNodeSettings, SetNodeSettings};
use vessel_proto::terminal_profiles::{
    Argument, Id, Name, Profile, ProfileLaunch, Profiles, Program, Prompt,
    TERMINAL_PROFILES_PROTOCOL,
};
use vessel_proto::terminals::{
    CreateTerminal, KillTerminal, SendTerminal, TerminalInfo, TerminalKind, TerminalPlace,
};
use vessel_proto::{
    ClientInfo, ErrorCode, EventSeq, Outcome, Reply, Request as _, SessionEvent, TerminalBytes,
    TerminalInput, WireError,
};
use vessel_rpc::{ClientConfig, Server, memory};
use vessel_store::DataDir;
use vessel_testkit::Fake;

use super::super::Connection;
use crate::guard::Guarded;
use crate::principal::Principal;
use crate::terminals::Shell;
use crate::{Config, Node};

struct Fixture {
    node: Node,
    guarded: Arc<Guarded>,
    connection: Connection,
    client: NodeClient,
    bin: PathBuf,
    work: PathBuf,
    _temporary: tempfile::TempDir,
}

impl Fixture {
    /// A node whose shell is `/bin/sh` and whose search path holds only a
    /// `claude` that is `fake-tui`.
    async fn open() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let bin = temporary.path().join("bin");
        let work = temporary.path().join("work");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&work).unwrap();
        std::os::unix::fs::symlink(Fake::Tui.path().unwrap(), bin.join("claude")).unwrap();
        let mut config = Config::new(
            DataDir::new(temporary.path().join("data")),
            HarnessEnv::new(temporary.path().join("user")),
        );
        config.bind = None;
        config.harness_lookup = Lookup::new(Vec::<PathBuf>::new());
        config.terminals.shell = Some(Shell::plain("/bin/sh"));
        config.terminals.search_path = Some(bin.clone().into_os_string());
        let node = Node::start(config).await.unwrap();
        let gateway = &node.secure;
        let guarded = Arc::clone(&gateway.service);
        let server = Server::new(
            Arc::clone(&guarded),
            gateway.lifecycle.watch_info(),
            gateway.config.clone(),
            CancellationToken::new(),
        );
        let (client_end, node_end) = memory::pair(NonZeroUsize::new(16).unwrap());
        let (made, contexts) = oneshot::channel();
        drop(tokio::spawn(async move {
            server
                .serve(node_end, move |peer| {
                    let connection = Connection::owner(peer);
                    drop(made.send(connection.acting_as(Principal::Owner)));
                    connection
                })
                .await
                .unwrap();
        }));
        let client = NodeClient::over(
            client_end,
            ClientInfo {
                name: "terminal-profiles-test".to_owned(),
                version: "0".to_owned(),
            },
            &ClientConfig::default(),
        )
        .await
        .unwrap();
        Self {
            node,
            guarded,
            connection: contexts.await.unwrap(),
            client,
            bin,
            work,
            _temporary: temporary,
        }
    }

    /// `call` answered by the guarded service, past the server's gate.
    async fn direct(&self, call: impl vessel_proto::Request) -> Result<Reply, WireError> {
        match vessel_proto::dispatch(&*self.guarded, &self.connection, call.into_call()).await {
            Outcome::Ok(reply) => Ok(reply),
            Outcome::Err(error) => Err(error),
        }
    }

    async fn launch(&self, id: &str, prompt: Option<&str>) -> Result<TerminalInfo, WireError> {
        let request = CreateTerminal {
            place: TerminalPlace::Folder {
                path: self.work.clone(),
            },
            size: None,
            title: None,
            profile: Some(ProfileLaunch {
                id: Id::parse(id).unwrap(),
                prompt: prompt.map(|text| Prompt::parse(text).unwrap()),
            }),
        };
        match self.direct(request).await? {
            Reply::CreateTerminal(info) => Ok(info),
            other => panic!("not a terminal: {other:?}"),
        }
    }

    /// Type `args` and wait for the line it prints.
    async fn args_of(&self, terminal: &TerminalInfo, expected: &str) {
        let mut stream = self
            .client
            .follow_terminal(terminal.id, EventSeq::ZERO)
            .await
            .unwrap();
        let mut seen = String::new();
        let mut asked = false;
        let wait = async {
            while !seen.contains(expected) {
                if !asked && seen.contains("ready") {
                    asked = true;
                    let _ack = self
                        .client
                        .call(SendTerminal {
                            terminal: terminal.id,
                            input: TerminalInput::Data {
                                data: TerminalBytes::new(Bytes::from_static(b"args\n")),
                            },
                        })
                        .await
                        .unwrap();
                }
                let Some(record) = stream.next().await else {
                    panic!("it ended before {expected:?}; it printed {seen:?}");
                };
                if let SessionEvent::Terminal(output) = record.unwrap().event {
                    seen.push_str(&String::from_utf8_lossy(output.data.bytes()));
                }
            }
        };
        tokio::time::timeout(std::time::Duration::from_secs(30), wait)
            .await
            .unwrap_or_else(|_| panic!("no {expected:?}"));
        let _ack = self
            .client
            .call(KillTerminal {
                terminal: terminal.id,
            })
            .await
            .unwrap();
    }
}

fn profile(id: &str, command: &Path, args: &[&str]) -> Profile {
    Profile::new(
        Id::parse(id).unwrap(),
        Name::parse(format!("Profile {id}")).unwrap(),
        Program::parse(command.to_str().unwrap()).unwrap(),
        args.iter()
            .map(|arg| Argument::parse(*arg).unwrap())
            .collect(),
        None,
    )
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_shipped_profile_runs_its_program_with_the_prompt_and_takes_its_name() {
    let fixture = Fixture::open().await;
    let with = fixture.launch("claude", Some("fix it now")).await.unwrap();
    assert_eq!(
        with.kind,
        TerminalKind::Shell {
            program: fixture.bin.join("claude"),
        }
    );
    assert_eq!(
        with.title.as_ref().map(ToString::to_string).as_deref(),
        Some("Claude Code")
    );
    // One argument, spaces kept: no shell split it.
    fixture.args_of(&with, r#"args ["fix it now"]"#).await;
    // No prompt: the placeholder's argument is left out.
    let without = fixture.launch("claude", None).await.unwrap();
    fixture.args_of(&without, "args []").await;
    fixture.node.stop().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_kept_list_replaces_the_shipped_one_and_refusals_say_why() {
    let fixture = Fixture::open().await;
    let tui = Fake::Tui.path().unwrap();
    let kept = Profiles::new(vec![
        profile("mine", Path::new("codex"), &[]),
        profile("tui", &tui, &["one two", "{{{prompt}}}"]),
    ])
    .unwrap();
    let change = SetNodeSettings {
        terminal_profiles: Some(kept.clone()),
        ..SetNodeSettings::default()
    };
    let Reply::SetNodeSettings(settings) = fixture.direct(change).await.unwrap() else {
        panic!("not settings");
    };
    // As kept: nothing adopted is written back.
    assert_eq!(settings.terminal_profiles, Some(kept));
    let gone = fixture.launch("claude", None).await.unwrap_err();
    assert_eq!(gone.code, ErrorCode::NotFound);
    assert_eq!(gone.message, "no terminal profile claude");
    let missing = fixture.launch("mine", None).await.unwrap_err();
    assert_eq!(missing.code, ErrorCode::SpawnFailed);
    assert_eq!(missing.message, "codex: command not found");
    let tui_run = fixture.launch("tui", Some("x")).await.unwrap();
    assert_eq!(tui_run.kind, TerminalKind::Shell { program: tui });
    fixture.args_of(&tui_run, r#"args ["one two","x"]"#).await;
    fixture.node.stop().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_server_refuses_and_hides_the_fields_while_their_number_is_pending() {
    if TERMINAL_PROFILES_PROTOCOL.is_some() {
        return;
    }
    let fixture = Fixture::open().await;
    let kept = Profiles::new(Vec::new()).unwrap();
    let change = SetNodeSettings {
        terminal_profiles: Some(kept),
        ..SetNodeSettings::default()
    };
    // Kept below the gate, so the reply over the wire has something to hide.
    drop(fixture.direct(change.clone()).await.unwrap());
    assert!(fixture.client.call(change).await.is_err());
    let read = fixture.client.call(GetNodeSettings {}).await.unwrap();
    assert_eq!(read.terminal_profiles, None);
    let launch = CreateTerminal {
        place: TerminalPlace::Folder {
            path: fixture.work.clone(),
        },
        size: None,
        title: None,
        profile: Some(ProfileLaunch {
            id: Id::parse("claude").unwrap(),
            prompt: None,
        }),
    };
    assert!(fixture.client.call(launch).await.is_err());
    fixture.node.stop().await.unwrap();
}
