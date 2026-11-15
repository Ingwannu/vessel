//! What a launch adds to Claude Code's command, and what it refuses before
//! anything starts.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use vessel_proc::CommandSpec;
use vessel_proto::{
    ChoiceValue, HarnessSessionId, McpTransport, ModelId, OptionChoice, OptionId, OptionValueId,
    ResolvedMcpServers, ResolvedValue,
};

use super::{Launched, Mode, launch};
use crate::error::{AgentError, LaunchRefusal};
use crate::handshake::{AgentSetup, ForkFrom, ForkRefusal};
use crate::options::OptionError;

/// The command `vessel_harness::launch_direct` gives the driver: the
/// program, its stream-json arguments and the profile's own, and an
/// environment the driver must leave as it is.
fn command() -> CommandSpec {
    CommandSpec::new("/opt/claude/bin/claude")
        .args([
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--model",
            "opus",
        ])
        .env("CLAUDE_CONFIG_DIR", "/home/someone/.claude-alt")
        .env_remove("ANTHROPIC_API_KEY")
        // Whatever the machine running the tests has: the provider is the
        // command's to say (`auto` is unavailable on Bedrock and Vertex).
        .env_remove("CLAUDE_CODE_USE_BEDROCK")
        .env_remove("CLAUDE_CODE_USE_VERTEX")
}

fn setup() -> AgentSetup {
    AgentSetup {
        cwd: PathBuf::from("/work"),
        model: None,
        resume: None,
        fork: None,
        options: Vec::new(),
        mcp_servers: ResolvedMcpServers::new(),
        handshake_timeout: Duration::from_secs(5),
        questions: false,
        system_prompt: None,
        private_dir: None,
    }
}

/// The arguments the launch added, after the command's own.
fn added(command: &CommandSpec) -> Vec<String> {
    let own = self::command().args.len();
    assert_eq!(
        command.args[..own],
        self::command().args[..],
        "the command's own come first"
    );
    command.args[own..]
        .iter()
        .map(|arg| arg.to_str().unwrap().to_owned())
        .collect()
}

fn launched(setup: &AgentSetup) -> (Vec<String>, Launched) {
    let (shaped, launched) = launch(&command(), setup).unwrap();
    assert_eq!(shaped.program, command().program);
    assert_eq!(
        shaped.env,
        command().env,
        "the driver adds nothing to the environment"
    );
    assert_eq!(shaped.environment, command().environment);
    (added(&shaped), launched)
}

fn refused(setup: &AgentSetup) -> LaunchRefusal {
    match launch(&command(), setup) {
        Err(AgentError::Refused(refusal)) => refusal,
        Err(other) => panic!("not a refusal: {other:?}"),
        Ok((shaped, _)) => panic!("launched: {:?}", added(&shaped)),
    }
}

fn mode(value: &str) -> OptionChoice {
    OptionChoice {
        id: OptionId::new("mode"),
        value: ChoiceValue::Select {
            value: OptionValueId::new(value),
        },
    }
}

/// Every argument a launch may add: enumerated flags, and flags whose
/// value is an id the driver checked or minted, or a path in the node's
/// private directory. Nothing else may ever reach the command line.
fn allowed(arg: &str, private: Option<&Path>) -> bool {
    const ENUMERATED: [&str; 4] = [
        "--include-partial-messages",
        "--permission-prompt-tool=stdio",
        "--disallowedTools=AskUserQuestion",
        "--allow-dangerously-skip-permissions",
    ];
    if ENUMERATED.contains(&arg) {
        return true;
    }
    if let Some(mode) = arg.strip_prefix("--permission-mode=") {
        return Mode::ALL.iter().any(|known| known.name() == mode);
    }
    if let Some(id) = arg
        .strip_prefix("--session-id=")
        .or_else(|| arg.strip_prefix("--resume="))
    {
        return uuid::Uuid::parse_str(id).is_ok();
    }
    if let Some(model) = arg.strip_prefix("--model=") {
        return model == "claude-opus-5-5";
    }
    if let Some(file) = arg.strip_prefix("--mcp-config=") {
        return private.is_some_and(|dir| Path::new(file).parent() == Some(dir));
    }
    false
}

#[test]
fn a_new_conversation_gets_its_id_on_the_command_line_and_nothing_else_is_added() {
    let (args, launched) = launched(&setup());
    let session = format!("--session-id={}", launched.conversation);
    assert_eq!(
        args,
        [
            "--include-partial-messages",
            "--permission-prompt-tool=stdio",
            "--disallowedTools=AskUserQuestion",
            session.as_str(),
            "--permission-mode=auto",
        ]
    );
    assert!(uuid::Uuid::parse_str(launched.conversation.as_str()).is_ok());
    assert!(args.iter().all(|arg| allowed(arg, None)));
    let (_, again) = self::launched(&setup());
    assert_ne!(
        again.conversation, launched.conversation,
        "a fresh id each time"
    );
}

#[test]
fn a_resumed_conversation_is_named_by_resume_alone() {
    let id = "5d0c3c1e-8f3a-4e8e-9c55-2f0f3a1b7c42";
    let resumed = AgentSetup {
        resume: Some(HarnessSessionId::new(id)),
        ..setup()
    };
    let (args, launched) = launched(&resumed);
    assert_eq!(launched.conversation.as_str(), id);
    assert!(args.contains(&format!("--resume={id}")));
    assert!(!args.iter().any(|arg| arg.starts_with("--session-id")));
    assert!(!args.iter().any(|arg| arg.starts_with("--fork-session")));
    assert!(args.iter().all(|arg| allowed(arg, None)));
}

#[test]
fn an_id_claude_code_cannot_have_made_is_refused_never_read_as_a_path() {
    for id in [
        "/home/someone/.claude/projects/x/5d0c.jsonl",
        "--dangerously-skip-permissions",
        "my-session-name",
        "",
    ] {
        let resumed = AgentSetup {
            resume: Some(HarnessSessionId::new(id)),
            ..setup()
        };
        assert!(
            matches!(refused(&resumed), LaunchRefusal::Conversation(ref refused) if refused == id),
            "{id}"
        );
    }
}

#[test]
fn a_fork_is_refused_until_the_history_slice() {
    for point in [None, Some(vessel_proto::AgentMessageId::new("m1"))] {
        let fork = AgentSetup {
            fork: Some(Box::new(ForkFrom {
                session: HarnessSessionId::new("5d0c3c1e-8f3a-4e8e-9c55-2f0f3a1b7c42"),
                point,
            })),
            ..setup()
        };
        assert!(matches!(
            refused(&fork),
            LaunchRefusal::Fork(ForkRefusal::CannotFork)
        ));
    }
}

#[test]
fn each_mode_is_its_flag_and_only_bypass_skips_permissions() {
    for known in Mode::ALL {
        let chosen = AgentSetup {
            options: vec![mode(known.name())],
            ..setup()
        };
        let (args, _) = launched(&chosen);
        assert!(args.contains(&format!("--permission-mode={}", known.name())));
        let skips = args
            .iter()
            .any(|arg| arg.contains("dangerously") || arg.contains("skip-permissions"));
        assert_eq!(skips, known == Mode::Bypass, "{known:?}");
        assert!(args.iter().all(|arg| allowed(arg, None)));
    }
    // No mode chosen: Paseo's default, passed, never the person's own
    // setting the node cannot see.
    let (args, _) = launched(&setup());
    let modes: Vec<&String> = args
        .iter()
        .filter(|arg| arg.starts_with("--permission-mode"))
        .collect();
    assert_eq!(modes, ["--permission-mode=auto"]);
}

/// The command with `name` set to `value`, as a profile would set it.
fn with(name: &str, value: &str) -> CommandSpec {
    command().env(name, value)
}

#[test]
fn on_bedrock_or_vertex_auto_is_refused_and_the_default_is_default() {
    for (name, value) in [
        ("CLAUDE_CODE_USE_BEDROCK", "1"),
        ("CLAUDE_CODE_USE_VERTEX", " TRUE "),
    ] {
        let elsewhere = with(name, value);
        let (shaped, _) = launch(&elsewhere, &setup()).unwrap();
        assert!(
            shaped
                .args
                .iter()
                .any(|arg| arg == "--permission-mode=default"),
            "{name}"
        );
        let auto = AgentSetup {
            options: vec![mode("auto")],
            ..setup()
        };
        let Err(AgentError::Refused(LaunchRefusal::Option(OptionError::NotOffered {
            offered,
            ..
        }))) = launch(&elsewhere, &auto)
        else {
            panic!("{name}: auto launched");
        };
        assert!(!offered.iter().any(|value| value.as_str() == "auto"));
        assert_eq!(offered.len(), Mode::OFFERED.len() - 1);
    }
    for off in ["0", "false", "no", "off", ""] {
        let (shaped, _) = launch(&with("CLAUDE_CODE_USE_BEDROCK", off), &setup()).unwrap();
        assert!(
            shaped
                .args
                .iter()
                .any(|arg| arg == "--permission-mode=auto"),
            "{off:?} is not on"
        );
    }
}

#[test]
fn a_mode_or_an_option_claude_code_does_not_take_is_refused() {
    let unknown_mode = AgentSetup {
        options: vec![mode("yolo")],
        ..setup()
    };
    assert!(matches!(
        refused(&unknown_mode),
        LaunchRefusal::Option(OptionError::NotOffered { ref offered, .. }) if offered.len() == Mode::OFFERED.len()
    ));
    let toggled = AgentSetup {
        options: vec![OptionChoice {
            id: OptionId::new("mode"),
            value: ChoiceValue::Toggle { on: true },
        }],
        ..setup()
    };
    assert!(matches!(
        refused(&toggled),
        LaunchRefusal::Option(OptionError::WrongKind { .. })
    ));
    let other = AgentSetup {
        options: vec![
            mode("default"),
            OptionChoice {
                id: OptionId::new("effort"),
                value: ChoiceValue::Select {
                    value: OptionValueId::new("high"),
                },
            },
        ],
        ..setup()
    };
    assert!(matches!(
        refused(&other),
        LaunchRefusal::Option(OptionError::Unknown { ref id, .. }) if id.as_str() == "effort"
    ));
}

#[test]
fn a_model_is_one_flag() {
    let chosen = AgentSetup {
        model: Some(ModelId::new("claude-opus-5-5")),
        ..setup()
    };
    let (args, _) = launched(&chosen);
    assert!(args.contains(&"--model=claude-opus-5-5".to_owned()));
    assert!(args.iter().all(|arg| allowed(arg, None)));
}

fn servers() -> ResolvedMcpServers {
    BTreeMap::from([(
        "docs".parse().unwrap(),
        McpTransport::Stdio {
            command: PathBuf::from("/opt/docs-mcp"),
            args: vec!["--stdio".to_owned()],
            env: BTreeMap::from([(
                "DOCS_TOKEN".parse().unwrap(),
                ResolvedValue::new("secret-value".to_owned()),
            )]),
        },
    )])
}

/// An owner-only directory, as the node's `mcp/` is.
#[cfg(unix)]
fn private_dir(root: &Path) -> PathBuf {
    use std::os::unix::fs::DirBuilderExt as _;
    let dir = root.join("mcp");
    std::fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
    dir
}

#[cfg(unix)]
#[test]
fn mcp_servers_go_in_a_private_file_the_session_holds() {
    let root = tempfile::tempdir().unwrap();
    let dir = private_dir(root.path());
    let with = AgentSetup {
        mcp_servers: servers(),
        system_prompt: None,
        private_dir: Some(dir.clone()),
        ..setup()
    };
    let (args, launched) = launched(&with);
    assert!(args.iter().all(|arg| allowed(arg, Some(&dir))), "{args:?}");
    assert!(!args.iter().any(|arg| arg.contains("secret-value")));
    let file = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--mcp-config="))
        .map(PathBuf::from)
        .unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("secret-value"));
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(&file).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    drop(launched);
    assert!(!file.exists(), "the file goes with the session");
}

#[test]
fn mcp_servers_without_the_private_directory_are_refused() {
    let without = AgentSetup {
        mcp_servers: servers(),
        ..setup()
    };
    assert!(matches!(refused(&without), LaunchRefusal::NoPrivateDir));
}

#[cfg(unix)]
#[test]
fn a_private_directory_open_to_others_is_refused_and_left_empty() {
    use std::os::unix::fs::PermissionsExt as _;
    let root = tempfile::tempdir().unwrap();
    let dir = private_dir(root.path());
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    let tampered = AgentSetup {
        mcp_servers: servers(),
        system_prompt: None,
        private_dir: Some(dir.clone()),
        ..setup()
    };
    assert!(matches!(refused(&tampered), LaunchRefusal::Mcp(_)));
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
}

#[test]
fn the_allow_list_refuses_what_a_launch_must_never_add() {
    for arg in [
        "--dangerously-skip-permissions",
        "--permission-mode=yolo",
        "--resume=/etc/passwd",
        "--mcp-config={\"mcpServers\":{}}",
        "--settings=/tmp/x.json",
    ] {
        assert!(!allowed(arg, None), "{arg}");
    }
}
