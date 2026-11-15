use clap::Parser as _;
use vessel_proto::NodeId;

use super::*;

#[test]
fn run_takes_a_place_or_a_folder_never_both() {
    let Cli {
        command: Command::Run(run),
        ..
    } = Cli::try_parse_from([
        "vessel",
        "run",
        "claude",
        "--project",
        "app",
        "--new-worktree",
    ])
    .unwrap()
    else {
        panic!("expected run");
    };
    assert_eq!(run.project.as_deref(), Some("app"));
    assert!(run.new_worktree);
    for refused in [
        &["vessel", "run", "claude", "--cwd", "/x", "--project", "app"][..],
        &["vessel", "run", "claude", "--dir", "src"],
        &["vessel", "run", "claude", "--new-worktree"],
        &[
            "vessel",
            "run",
            "claude",
            "--project",
            "app",
            "--worktree",
            "ab12",
        ],
        &[
            "vessel",
            "run",
            "claude",
            "--project",
            "app",
            "--dir",
            "s",
            "--new-worktree",
        ],
    ] {
        assert!(Cli::try_parse_from(refused).is_err(), "{refused:?}");
    }
    assert!(
        Cli::try_parse_from([
            "vessel",
            "worktree",
            "remove",
            "ab",
            "--snapshot",
            "--discard"
        ])
        .is_err()
    );
    assert!(Cli::try_parse_from(["vessel", "project", "approve", "app", "00ff"]).is_ok());
}

#[test]
fn run_takes_a_profile_and_profiles_a_harness() {
    let Cli {
        command: Command::Run(run),
        ..
    } = Cli::try_parse_from(["vessel", "run", "claude", "--profile", "claude-alt"]).unwrap()
    else {
        panic!("expected run");
    };
    assert_eq!(run.profile.unwrap().as_str(), "claude-alt");
    assert!(Cli::try_parse_from(["vessel", "run", "claude", "--profile", "bad name"]).is_err());
    let Cli {
        command: Command::Profiles { harness, reload },
        ..
    } = Cli::try_parse_from(["vessel", "profiles", "--harness", "codex", "--reload"]).unwrap()
    else {
        panic!("expected profiles");
    };
    assert_eq!(harness, Some(vessel_proto::HarnessKind::Codex.into()));
    assert!(reload);
    let Cli {
        command: Command::Profiles { harness, .. },
        ..
    } = Cli::try_parse_from(["vessel", "profiles", "--harness", "goose"]).unwrap()
    else {
        panic!("expected profiles");
    };
    assert_eq!(harness, Some(HarnessRef::Added("goose".parse().unwrap())));
}

#[test]
fn copy_and_move_take_a_node_a_profile_and_a_directory() {
    let Cli {
        command:
            Command::Session {
                command: SessionCommand::Copy { to, start },
            },
        ..
    } = Cli::try_parse_from([
        "vessel",
        "session",
        "copy",
        "3f2a",
        "--to",
        "laptop",
        "--profile",
        "claude-alt",
        "--cwd",
        "/srv/app",
        "--start",
    ])
    .unwrap()
    else {
        panic!("expected session copy");
    };
    assert_eq!(to.to.as_deref(), Some("laptop"));
    assert_eq!(to.profile.unwrap().as_str(), "claude-alt");
    assert_eq!(to.cwd, Some(PathBuf::from("/srv/app")));
    assert!(start);
    assert!(matches!(
        Cli::try_parse_from(["vessel", "session", "move", "3f2a"])
            .unwrap()
            .command,
        Command::Session {
            command: SessionCommand::Move { .. }
        }
    ));
    // A move always resumes the copy: no --start.
    assert!(Cli::try_parse_from(["vessel", "session", "move", "3f2a", "--start"]).is_err());
}

#[test]
fn a_secret_is_copied_to_a_whole_node_id_only() {
    let id = NodeId::random().to_string();
    let parsed = Cli::try_parse_from(["vessel", "fleet", "secret", "copy", "TOKEN", "--to", &id]);
    assert!(parsed.is_ok(), "{parsed:?}");
    let prefix = id.get(..8).unwrap();
    for to in ["laptop", prefix] {
        assert!(
            Cli::try_parse_from(["vessel", "fleet", "secret", "copy", "TOKEN", "--to", to])
                .is_err(),
            "{to}"
        );
    }
}

#[test]
fn memory_copy_needs_a_source_project_for_a_target_one() {
    let parsed = Cli::try_parse_from([
        "vessel",
        "fleet",
        "memory",
        "copy",
        "--from-profile",
        "claude-alt",
        "--from-project",
        "/srv/a",
        "--profile",
        "claude-work",
        "--project",
        "/srv/b",
    ]);
    assert!(parsed.is_ok(), "{parsed:?}");
    assert!(
        Cli::try_parse_from(["vessel", "fleet", "memory", "copy", "--project", "/srv/b"]).is_err()
    );
}

#[test]
fn move_profile_takes_a_session_a_profile_and_force() {
    let Cli {
        command:
            Command::Session {
                command: SessionCommand::MoveProfile { profile, force, .. },
            },
        ..
    } = Cli::try_parse_from(["vessel", "session", "move-profile", "3f2a", "claude-alt"]).unwrap()
    else {
        panic!("expected session move-profile");
    };
    assert_eq!(profile.as_str(), "claude-alt");
    assert!(!force);
    assert!(Cli::try_parse_from(["vessel", "session", "move-profile", "3f2a", "a b"]).is_err());
    assert!(Cli::try_parse_from(["vessel", "session", "move-profile", "3f2a"]).is_err());
    assert!(
        Cli::try_parse_from(["vessel", "session", "move-profile", "3f2a", "x", "--force"]).is_ok()
    );
}

/// Review item 3: a pairing link never comes from the command line,
/// where other users and the shell's history see it.
#[test]
fn a_pairing_link_on_the_command_line_is_refused() {
    let parse = |args: &[&str]| Cli::try_parse_from(args);
    let error = parse(&[
        "vessel",
        "nodes",
        "add",
        "box",
        "--pair",
        "vessel://pair#e30",
    ])
    .unwrap_err()
    .to_string();
    assert!(error.contains("standard input"), "{error}");
    assert!(error.contains("Make a new link"), "{error}");
    for args in [
        &["vessel", "nodes", "add", "box", "--pair"][..],
        &["vessel", "nodes", "add", "box", "--pair", "-"],
        &["vessel", "nodes", "add", "box", "--pair", "--yes"],
    ] {
        let Cli {
            command:
                Command::Nodes {
                    command: NodesCommand::Add { pair, address, .. },
                },
            ..
        } = parse(args).unwrap()
        else {
            panic!("{args:?}");
        };
        assert_eq!((pair, address), (Some(FromStdin), None), "{args:?}");
    }
    assert!(parse(&["vessel", "nodes", "add", "box", "me@box", "--pair"]).is_err());
}

#[test]
fn pairing_link_lifetimes_read_like_durations_and_stay_within_a_day() {
    assert_eq!(ttl_secs("90"), Ok(90));
    assert_eq!(ttl_secs("90s"), Ok(90));
    assert_eq!(ttl_secs("10m"), Ok(600));
    assert_eq!(ttl_secs(" 2h "), Ok(7200));
    assert_eq!(ttl_secs("24h"), Ok(MAX_TTL_SECS));
    assert!(ttl_secs("25h").is_err());
    assert!(ttl_secs("0m").is_err());
    assert!(ttl_secs("ten").is_err());
    assert!(ttl_secs("1d").is_err());
    assert!(ttl_secs("4294967295h").is_err(), "overflow is an error");
}

#[test]
fn agents_add_takes_its_command_after_the_options() {
    let Cli {
        command: Command::Agents {
            command: Some(AgentsCommand::Add(add)),
        },
        ..
    } = Cli::try_parse_from([
        "vessel",
        "agents",
        "add",
        "goose",
        "--env",
        "GOOSE_MODE=auto=yes",
        "--secret",
        "ANTHROPIC_API_KEY=GOOSE_KEY",
        "--inherit",
        "SSH_AUTH_SOCK",
        "--mcp",
        "docs",
        "--",
        "goose",
        "acp",
        "--verbose",
    ])
    .unwrap()
    else {
        panic!("expected agents add");
    };
    assert_eq!(add.name.as_str(), "goose");
    // Split at the first `=`: the value keeps the rest.
    assert_eq!(
        add.env[0].1,
        "GOOSE_MODE=auto=yes".split_once('=').unwrap().1
    );
    assert_eq!(add.secrets[0].1.as_str(), "GOOSE_KEY");
    assert_eq!(add.command, ["goose", "acp", "--verbose"]);
    assert!(!add.replace);

    let Cli {
        command: Command::Agents { command: None },
        ..
    } = Cli::try_parse_from(["vessel", "agents"]).unwrap()
    else {
        panic!("a list without a command");
    };
    for refused in [
        // One of the four's names, in any case.
        &["vessel", "agents", "add", "Claude", "--", "x"][..],
        // No command.
        &["vessel", "agents", "add", "goose"],
        // Not NAME=VALUE; a secret name breaking its rule.
        &["vessel", "agents", "add", "goose", "--env", "X", "--", "x"],
        &[
            "vessel", "agents", "add", "goose", "--secret", "X=no-way", "--", "x",
        ],
    ] {
        assert!(Cli::try_parse_from(refused).is_err(), "{refused:?}");
    }
}

#[test]
fn agents_status_refreshes_one_agent_and_the_registry_replaces_the_command() {
    let Cli {
        command:
            Command::Agents {
                command:
                    Some(AgentsCommand::Status {
                        agent: Some(agent),
                        profile: Some(profile),
                        refresh: true,
                    }),
            },
        ..
    } = Cli::try_parse_from([
        "vessel",
        "agents",
        "status",
        "codex",
        "--profile",
        "work",
        "--refresh",
    ])
    .unwrap()
    else {
        panic!("expected agents status");
    };
    assert_eq!(agent.to_string(), "codex");
    assert_eq!(profile.as_str(), "work");
    let Cli {
        command: Command::Agents {
            command: Some(AgentsCommand::Add(add)),
        },
        ..
    } = Cli::try_parse_from([
        "vessel",
        "agents",
        "add",
        "example",
        "--from-registry",
        "example-agent",
        "--yes",
    ])
    .unwrap()
    else {
        panic!("expected agents add");
    };
    assert_eq!(add.from_registry.unwrap().as_str(), "example-agent");
    assert!(add.yes && add.command.is_empty());
    for refused in [
        // A refresh names its agent.
        &["vessel", "agents", "status", "--refresh"][..],
        // An entry or a command, not both.
        &[
            "vessel",
            "agents",
            "add",
            "example",
            "--from-registry",
            "example-agent",
            "--",
            "x",
        ],
    ] {
        assert!(Cli::try_parse_from(refused).is_err(), "{refused:?}");
    }
}

#[test]
fn agents_install_takes_one_of_the_four_and_its_part() {
    let Cli {
        command:
            Command::Agents {
                command:
                    Some(AgentsCommand::Install {
                        agent,
                        adapter: true,
                        yes: false,
                    }),
            },
        ..
    } = Cli::try_parse_from(["vessel", "agents", "install", "claude", "--adapter"]).unwrap()
    else {
        panic!("expected agents install");
    };
    assert_eq!(agent, vessel_proto::HarnessKind::ClaudeCode);
    // Only the four have documented installers.
    assert!(Cli::try_parse_from(["vessel", "agents", "install", "goose"]).is_err());
}

#[test]
fn native_creation_listing_is_optional_and_does_not_repurpose_paseo_wait_aliases() {
    let Cli {
        command: Command::Run(run),
        ..
    } = Cli::try_parse_from([
        "vessel",
        "run",
        "claude",
        "--listing",
        "shown",
        "--label",
        "purpose=one",
        "--detach",
    ])
    .unwrap()
    else {
        panic!("native run expected")
    };
    assert_eq!(
        run.metadata.listing,
        Some(vessel_proto::SessionListing::Shown)
    );
    assert_eq!(run.metadata.label, ["purpose=one"]);
    assert!(run.detach);
    let Cli {
        command: Command::Run(run),
        ..
    } = Cli::try_parse_from(["vessel", "run", "claude"]).unwrap()
    else {
        panic!("native run expected")
    };
    assert_eq!(run.metadata.listing, None);
    assert!(run.metadata.label.is_empty());
    assert!(Cli::try_parse_from(["vessel", "run", "claude", "--listing", "unknown"]).is_err());
    let Cli {
        command: Command::Sessions(query),
        ..
    } = Cli::try_parse_from([
        "vessel",
        "sessions",
        "--background",
        "--label",
        "purpose=one",
    ])
    .unwrap()
    else {
        panic!("native sessions expected")
    };
    assert!(query.background);
    assert_eq!(query.label, ["purpose=one"]);
}
