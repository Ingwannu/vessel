use std::path::Path;

use vessel_proto::{EnvName, HarnessKind, HarnessRef};

use super::parse::without_values;
use super::*;

#[expect(
    clippy::unnecessary_wraps,
    reason = "parse_profiles takes an optional home"
)]
fn home() -> Option<&'static Path> {
    Some(Path::new("/home/u"))
}

/// `name` under [`home`], as `~/name` expands: the system's separator.
fn under_home(name: &str) -> String {
    format!("/home/u{}{name}", std::path::MAIN_SEPARATOR)
}

fn var(name: &str) -> EnvName {
    name.parse().unwrap()
}

/// Review finding 5: lookups follow the system's case rule, so a
/// profile's `codex_home` is Codex's home on Windows, where the
/// environment ignores case, and another variable elsewhere.
#[test]
fn lookups_follow_the_systems_case_rule() {
    let text =
        "[profiles.x]\nharness = \"codex\"\nenv = { codex_home = \"/w\" }\nunset = [\"Path\"]\n";
    let x = parse_profiles(text, home())
        .unwrap()
        .get(&"x".parse().unwrap())
        .cloned()
        .unwrap();
    let windows = cfg!(windows);
    assert_eq!(x.get(&var("CODEX_HOME")).is_some(), windows);
    assert_eq!(x.unsets(&var("PATH")), windows);
    assert_eq!(x.get(&var("codex_home")), Some("/w"));
    assert!(x.unsets(&var("Path")));
    assert_eq!(same_variable("Path", "PATH"), windows);
    assert!(same_variable("PATH", "PATH"));
}

fn problem(text: &str) -> ProfileProblem {
    match parse_profiles(text, home()) {
        Err(ProfilesError::Profile { problem, .. }) => problem,
        other => panic!("expected a profile problem, got {other:?}"),
    }
}

const ALT: &str = r#"
    [profiles.claude-alt]
    harness = "claude"
    label = "Claude (Alt)"
    env = { CLAUDE_CONFIG_DIR = "~/.claude-alt", ANTHROPIC_BASE_URL = "https://example.invalid" }
    unset = ["ANTHROPIC_API_KEY"]
    args = ["--model", "opus", "--settings", "~/s.json"]

    [profiles.codex-work]
    harness = "codex"
    env = { CODEX_HOME = "~" }
"#;

#[test]
fn the_adr_example_parses() {
    let profiles = parse_profiles(ALT, home()).unwrap();
    let alt = profiles.get(&"claude-alt".parse().unwrap()).unwrap();
    assert_eq!(alt.harness(), HarnessKind::ClaudeCode);
    assert_eq!(alt.label(), Some("Claude (Alt)"));
    assert_eq!(
        alt.get(&var("CLAUDE_CONFIG_DIR")),
        Some(under_home(".claude-alt").as_str())
    );
    assert!(alt.unsets(&var("ANTHROPIC_API_KEY")));
    assert_eq!(
        alt.args(),
        [
            "--model",
            "opus",
            "--settings",
            under_home("s.json").as_str()
        ]
    );
    assert!(!alt.is_default());
    let work = profiles.get(&"codex-work".parse().unwrap()).unwrap();
    assert_eq!(work.get(&var("CODEX_HOME")), Some("/home/u"));
    let names: Vec<_> = profiles.all().map(|p| p.name().to_string()).collect();
    assert_eq!(
        names,
        [
            "claude",
            "codex",
            "gemini",
            "opencode",
            "grok",
            "pi",
            "omp",
            "antigravity",
            "muse",
            "amp",
            "cursor",
            "claude-alt",
            "codex-work"
        ]
    );
}

#[test]
fn defaults_change_nothing_and_are_found_by_name() {
    let profiles = parse_profiles("", home()).unwrap();
    for harness in HarnessKind::ALL {
        let default = Profile::default_for(harness);
        assert!(default.is_default());
        assert_eq!(default.env().count(), 0);
        assert_eq!(default.unset().count(), 0);
        assert!(default.args().is_empty());
        assert_eq!(profiles.get(default.name()), Some(&default));
    }
    assert_eq!(
        Profile::default_for(HarnessKind::ClaudeCode)
            .name()
            .as_str(),
        "claude"
    );
    assert_eq!(profiles.get(&"nope".parse().unwrap()), None);
}

#[test]
fn values_reach_neither_info_nor_debug() {
    let text = r#"
        [profiles.keyed]
        harness = "claude"
        env = { ANTHROPIC_API_KEY = "sk-very-secret" }
    "#;
    let profiles = parse_profiles(text, home()).unwrap();
    let keyed = profiles.get(&"keyed".parse().unwrap()).unwrap();
    let info = keyed.info();
    assert_eq!(info.sets, ["ANTHROPIC_API_KEY".parse::<EnvName>().unwrap()]);
    let json = serde_json::to_string(&info).unwrap();
    assert!(!json.contains("sk-very-secret"), "{json}");
    for shown in [format!("{keyed:?}"), format!("{profiles:?}")] {
        assert!(!shown.contains("sk-very-secret"), "{shown}");
        assert!(shown.contains("ANTHROPIC_API_KEY"), "{shown}");
    }
}

#[test]
fn toml_errors_name_the_key_and_position_without_quoting_the_file() {
    let text = "[profiles.x]\nharness = \"claude\"\nenv = { KEY = \"sk-secret\" \n";
    let error = parse_profiles(text, home()).unwrap_err();
    let shown = error.to_string();
    assert!(!shown.contains("sk-secret"), "{shown}");
    assert!(shown.contains("line 3"), "{shown}");

    let unknown = "[profiles.x]\nharness = \"claude\"\nenviron = {}\n";
    let shown = parse_profiles(unknown, home()).unwrap_err().to_string();
    assert!(shown.contains("environ"), "{shown}");
    assert!(shown.contains("line 3"), "{shown}");

    let missing = "[profiles.x]\nlabel = \"no harness\"\n";
    let shown = parse_profiles(missing, home()).unwrap_err().to_string();
    assert!(shown.contains("harness"), "{shown}");

    let top = "[profile.x]\nharness = \"claude\"\n";
    let shown = parse_profiles(top, home()).unwrap_err().to_string();
    assert!(shown.contains("profile"), "{shown}");

    let typed = "[profiles.x]\nharness = \"claude\"\nunset = \"HOME\"\n";
    let shown = parse_profiles(typed, home()).unwrap_err().to_string();
    assert!(shown.contains("line 3"), "{shown}");
}

#[test]
fn toml_errors_carry_a_typed_position() {
    let error =
        parse_profiles("[profiles.x]\nharness = \"claude\"\nenv = 3\n", home()).unwrap_err();
    let ProfilesError::Toml { position, .. } = error else {
        panic!("expected a TOML error");
    };
    assert_eq!(position, Some(Position { line: 3, column: 7 }));
}

/// Review finding: serde's messages quoted the value they found, and a
/// value may be a secret.
#[test]
fn errors_never_quote_a_value() {
    for (text, kind) in [
        ("env = { PIN = 98765432123 }", "integer, expected a string"),
        (
            "args = \"--token=sk-SECRET\"",
            "string, expected a sequence",
        ),
        ("env = \"sk-SECRET\"", "string, expected a map"),
        ("unset = \"sk-SECRET\"", "string, expected a sequence"),
        ("label = 1.5e300", "floating point, expected a string"),
        ("env = { K = true }", "boolean, expected a string"),
        (
            "env = { K = 98765432123.5 }",
            "floating point, expected a string",
        ),
    ] {
        let text = format!("[profiles.x]\nharness = \"claude\"\n{text}\n");
        let shown = parse_profiles(&text, home()).unwrap_err().to_string();
        for value in ["98765432123", "sk-SECRET", "1500000", "true"] {
            assert!(!shown.contains(value), "{value} in {shown}");
        }
        assert!(shown.contains(kind), "{shown}");
        assert!(shown.contains("line 3"), "{shown}");
    }
    for text in [
        "profiles = \"sk-SECRET\"\n",
        "[profiles]\nx = \"sk-SECRET\"\n",
        "[profiles.x]\nharness = \"claude\"\nenv = { K = sk-SECRET }\n",
        "[profiles.x]\nharness = \"claude\"\nenv = { K = \"\\q sk-SECRET\" }\n",
        "[profiles.x]\nharness = \"claude\"\nenv = { K = \"a\" } sk-SECRET\n",
    ] {
        let shown = parse_profiles(text, home()).unwrap_err().to_string();
        assert!(!shown.contains("SECRET"), "{shown}");
    }
    assert_eq!(
        without_values("invalid value: string \"a, expected b\", expected one of x"),
        "invalid value: string, expected one of x"
    );
    assert_eq!(
        without_values("invalid type: integer `5`"),
        "invalid type: a value of another type"
    );
    assert_eq!(
        without_values("missing field `harness`"),
        "missing field `harness`"
    );
}

#[test]
fn names_are_checked() {
    assert!(matches!(
        problem("[profiles.\"bad name\"]\nharness = \"claude\"\n"),
        ProfileProblem::InvalidName(_)
    ));
    for reserved in ["claude", "Claude-Code", "codex", "gemini", "OpenCode"] {
        let text = format!("[profiles.{reserved}]\nharness = \"codex\"\n");
        assert!(
            matches!(problem(&text), ProfileProblem::Reserved { .. }),
            "{reserved}"
        );
    }
    let twice = "[profiles.Work]\nharness = \"codex\"\n[profiles.work]\nharness = \"claude\"\n";
    let error = parse_profiles(twice, home()).unwrap_err();
    assert!(
        matches!(
            &error,
            ProfilesError::Profile {
                problem: ProfileProblem::Duplicate { .. },
                ..
            }
        ),
        "{error:?}"
    );
    assert!(error.to_string().contains("work") || error.to_string().contains("Work"));
}

#[test]
fn harness_and_variables_are_checked() {
    assert_eq!(
        problem("[profiles.x]\nharness = \"vim\"\n"),
        ProfileProblem::UnknownHarness {
            value: "vim".to_owned()
        }
    );
    assert_eq!(
        problem("[profiles.x]\nharness = \"claude\"\nenv = { \"BAD-NAME\" = \"1\" }\n"),
        ProfileProblem::InvalidVariable {
            key: "env",
            name: "BAD-NAME".to_owned()
        }
    );
    assert_eq!(
        problem("[profiles.x]\nharness = \"claude\"\nunset = [\"1X\"]\n"),
        ProfileProblem::InvalidVariable {
            key: "unset",
            name: "1X".to_owned()
        }
    );
    assert!(matches!(
        problem("[profiles.x]\nharness = \"claude\"\nenv = { Path = \"a\", PATH = \"b\" }\n"),
        ProfileProblem::RepeatedVariable { key: "env", .. }
    ));
    assert!(matches!(
        problem("[profiles.x]\nharness = \"claude\"\nunset = [\"A\", \"a\"]\n"),
        ProfileProblem::RepeatedVariable { key: "unset", .. }
    ));
    assert_eq!(
        problem(
            "[profiles.x]\nharness = \"claude\"\nenv = { HOME = \"/h\" }\nunset = [\"home\"]\n"
        ),
        ProfileProblem::SetAndUnset {
            name: "home".parse().unwrap()
        }
    );
    let error = parse_profiles(
        "[profiles.x]\nharness = \"claude\"\nunset = [\"A\", \"a\"]\n",
        home(),
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("profile `x`") && error.contains("`unset`"),
        "{error}"
    );
}

#[test]
fn tilde_needs_a_home() {
    let text = "[profiles.x]\nharness = \"claude\"\nenv = { CLAUDE_CONFIG_DIR = \"~/.c\" }\n";
    let error = parse_profiles(text, None).unwrap_err();
    assert!(matches!(
        error,
        ProfilesError::Profile { problem: ProfileProblem::NoHome { ref key }, .. } if key == "env.CLAUDE_CONFIG_DIR"
    ));
    let plain =
        "[profiles.x]\nharness = \"claude\"\nenv = { A = \"~user/x\" }\nargs = [\"a~/b\"]\n";
    let profiles = parse_profiles(plain, None).unwrap();
    let x = profiles.get(&"x".parse().unwrap()).unwrap();
    assert_eq!(x.get(&var("A")), Some("~user/x"));
    assert_eq!(x.args(), ["a~/b"]);
    assert_eq!(expand("~/a", Some("/h/")), Some("/h/a".to_owned()));
    assert_eq!(expand("~", Some("/h")), Some("/h".to_owned()));
    // The system's separator throughout, as `Path::join` writes it.
    let home = std::env::temp_dir();
    let home_text = home.to_str().unwrap().trim_end_matches(['/', '\\']);
    assert_eq!(
        expand("~/.claude-work/sub", Some(home_text)),
        Some(
            Path::new(home_text)
                .join(".claude-work")
                .join("sub")
                .display()
                .to_string()
        )
    );
}

/// Profiles of an agent added on the node (ADR 0032): `env` and `unset`
/// only, and its name is its default profile's.
#[test]
fn a_profile_extends_an_added_agent_with_its_environment_only() {
    let agents =
        crate::added::parse_agents("[agents.goose]\nprogram = \"goose\"\n", home()).unwrap();
    let with = |text: &str| parse_profiles_with(text, home(), agents.clone());
    let profiles = with(
        "[profiles.goose-work]\nharness = \"goose\"\nenv = { GOOSE_MODE = \"auto\" }\nunset = [\"X\"]\n",
    )
    .unwrap();
    let goose: HarnessRef = "goose".parse().unwrap();
    let work = profiles.get(&"goose-work".parse().unwrap()).unwrap();
    assert_eq!(work.harness(), goose);
    assert!(work.known().is_none() && work.added().is_some());
    assert_eq!(work.info().harness, goose);
    // The agent's default profile is in the set, under its name.
    let default = profiles.get(&"goose".parse().unwrap()).unwrap();
    assert!(default.is_default());
    assert_eq!(
        profiles.default_of(&goose).unwrap().name().as_str(),
        "goose"
    );
    assert!(profiles.added().get(&"goose".parse().unwrap()).is_some());

    let problem = |text: &str| match with(text) {
        Err(ProfilesError::Profile { problem, .. }) => problem,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        problem("[profiles.g]\nharness = \"goose\"\nargs = [\"--x\"]\n"),
        ProfileProblem::ArgsUnsupported {
            harness: goose.clone()
        }
    );
    assert_eq!(
        problem("[profiles.g]\nharness = \"goose\"\nmodel = \"qwen\"\n"),
        ProfileProblem::NotRoutable { harness: goose }
    );
    assert_eq!(
        problem("[profiles.GOOSE]\nharness = \"codex\"\n"),
        ProfileProblem::ReservedForAgent {
            agent: "GOOSE".to_owned()
        }
    );
    assert!(matches!(
        problem("[profiles.g]\nharness = \"nobody\"\n"),
        ProfileProblem::UnknownHarness { .. }
    ));
}

/// ADR 0031: agents' terminals are off unless a person wrote them on.
#[test]
fn agent_terminals_are_off_unless_written() {
    let text = "[profiles.on]\nharness = \"claude\"\nagent_terminals = true\n\n[profiles.off]\nharness = \"codex\"\n";
    let profiles = parse_profiles(text, home()).unwrap();
    let get = |name: &str| profiles.get(&name.parse().unwrap()).unwrap();
    assert!(get("on").agent_terminals());
    assert!(!get("off").agent_terminals());
    assert!(
        profiles
            .all()
            .filter(|p| p.is_default())
            .all(|p| !p.agent_terminals())
    );
    assert!(matches!(
        parse_profiles(
            "[profiles.x]\nharness = \"claude\"\nagent_terminals = \"yes\"\n",
            home()
        ),
        Err(ProfilesError::Toml { .. })
    ));
}
