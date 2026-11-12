use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vessel_proc::{EnvValue, Environment};
use vessel_proto::agents::{AgentDefinition, AgentSource, AgentVariable};
use vessel_proto::{ConfigValue, ResolvedValue, SecretName};

use super::*;
use crate::env::HarnessEnv;
use crate::error::HarnessError;
use crate::lookup::Lookup;

/// An absolute path on this platform.
fn absolute(rest: &str) -> PathBuf {
    let root = if cfg!(windows) { "C:\\" } else { "/" };
    Path::new(root).join(rest)
}

fn parse(text: &str) -> Result<AddedAgents, AgentsError> {
    parse_agents(text, Some(&absolute("home/u")))
}

fn problem(text: &str) -> AgentProblem {
    match parse(text) {
        Err(AgentsError::Agent { problem, .. }) => problem,
        other => panic!("expected a problem with one agent, got {other:?}"),
    }
}

const GOOSE: &str = r#"
# Goose, from its own install
[agents.goose]
label = "Goose"
program = "goose"
args = ["acp", "~/work"]
env = { GOOSE_MODE = "auto", ANTHROPIC_API_KEY = { secret = "anthropic" } }
inherit = ["GOOSE_PROVIDER"]
mcp = ["docs"]
source = { registry = "goose", version = "1.9.0" }
"#;

#[test]
fn a_file_reads_into_agents_whose_values_stay_on_the_node() {
    let agents = parse(GOOSE).unwrap();
    let goose = agents.get(&"goose".parse().unwrap()).unwrap();
    let info = goose.info();
    assert_eq!(info.program.as_str(), "goose");
    let work = absolute("home/u").join("work");
    assert_eq!(info.args, ["acp", work.to_str().unwrap()]);
    assert_eq!(info.env, ["GOOSE_MODE".parse().unwrap()]);
    assert_eq!(info.secrets.len(), 1);
    assert_eq!(info.secrets[0].secret.as_str(), "anthropic");
    assert_eq!(info.inherit, ["GOOSE_PROVIDER".parse().unwrap()]);
    assert_eq!(info.mcp, ["docs".parse().unwrap()]);
    assert!(matches!(info.source, AgentSource::Registry { .. }));
    assert_eq!(
        goose.secrets().map(SecretName::as_str).collect::<Vec<_>>(),
        ["anthropic"]
    );
    let debug = format!("{goose:?}");
    assert!(
        !debug.contains("auto") && !debug.contains("anthropic"),
        "{debug}"
    );
    assert!(agents.has_name("GOOSE"));
    assert!(parse("").unwrap().all().next().is_none());
}

#[test]
fn each_rule_is_held_and_named() {
    assert!(matches!(
        problem("[agents.claude]\nprogram = \"x\"\n"),
        AgentProblem::Name(_)
    ));
    assert!(matches!(
        problem("[agents.open-code]\nprogram = \"x\"\n"),
        AgentProblem::Name(_)
    ));
    assert!(matches!(
        problem("[agents.Goose]\nprogram = \"x\"\n[agents.goose]\nprogram = \"y\"\n"),
        AgentProblem::Duplicate { .. }
    ));
    assert_eq!(
        problem("[agents.a]\nprogram = \"bin/agent\"\n"),
        AgentProblem::RelativeProgram
    );
    assert_eq!(
        problem("[agents.a]\nprogram = \"x\"\nenv = { K = \"1\" }\ninherit = [\"k\"]\n"),
        AgentProblem::Repeated {
            key: "inherit",
            name: "k".to_owned()
        }
    );
    assert!(matches!(
        problem("[agents.a]\nprogram = \"x\"\ninherit = [\"1BAD\"]\n"),
        AgentProblem::InvalidName { key: "inherit", .. }
    ));
    assert!(matches!(
        problem("[agents.a]\nprogram = \"x\"\nlabel = \"two\\nlines\"\n"),
        AgentProblem::Text { key: "label", .. }
    ));
    let long = format!(
        "[agents.a]\nprogram = \"x\"\nargs = [\"{}\"]\n",
        "y".repeat(MAX_ARG + 1)
    );
    assert_eq!(problem(&long), AgentProblem::LongArgument);
    let many = format!(
        "[agents.a]\nprogram = \"x\"\nargs = [{}]\n",
        vec!["\"y\""; MAX_ITEMS + 1].join(", ")
    );
    assert_eq!(problem(&many), AgentProblem::TooMany { key: "args" });
    assert!(matches!(
        parse(&"#".repeat(MAX_FILE + 1)),
        Err(AgentsError::TooLarge)
    ));
    assert!(matches!(
        parse("[agents.a]\nprogram = \"x\"\nunknown = 1\n"),
        Err(AgentsError::Toml { .. })
    ));
}

#[test]
fn errors_never_quote_a_value() {
    let error = parse("[agents.a]\nprogram = \"x\"\nenv = { K = 98765 }\n").unwrap_err();
    assert!(!error.to_string().contains("98765"), "{error}");
}

fn definition() -> AgentDefinition {
    AgentDefinition {
        label: None,
        program: "goose".parse().unwrap(),
        args: vec!["acp".to_owned()],
        env: vec![AgentVariable {
            name: "KEY".parse().unwrap(),
            value: ConfigValue::Secret("key".parse().unwrap()),
        }],
        inherit: Vec::new(),
        mcp: Vec::new(),
    }
}

#[test]
fn adding_keeps_the_file_and_refuses_a_taken_name() {
    let name: AgentName = "goose".parse().unwrap();
    let text = "# mine\n[agents.other]\nprogram = \"x\" # kept\n";
    let added = add_agent(text, &name, &definition(), &AgentSource::Command, false).unwrap();
    assert!(added.starts_with("# mine\n"), "{added}");
    assert!(added.contains("# kept"), "{added}");
    let agents = parse(&added).unwrap();
    assert_eq!(agents.all().count(), 2);
    let goose = agents.get(&name).unwrap().info();
    assert_eq!(goose.secrets[0].variable.as_str(), "KEY");

    let shouting: AgentName = "GOOSE".parse().unwrap();
    assert_eq!(
        add_agent(
            &added,
            &shouting,
            &definition(),
            &AgentSource::Command,
            false
        ),
        Err(EditError::Taken("goose".to_owned()))
    );
    let replaced = add_agent(
        &added,
        &shouting,
        &definition(),
        &AgentSource::Command,
        true,
    )
    .unwrap();
    let agents = parse(&replaced).unwrap();
    assert!(agents.get(&name).is_none() && agents.get(&shouting).is_some());

    let removed = remove_agent(&replaced, &shouting).unwrap();
    assert_eq!(parse(&removed).unwrap().all().count(), 1);
    assert_eq!(
        remove_agent(&removed, &shouting),
        Err(EditError::NotFound(shouting))
    );
    assert_eq!(
        add_agent(
            "not = [toml",
            &name,
            &definition(),
            &AgentSource::Command,
            false
        ),
        Err(EditError::NotToml)
    );
}

/// The node's environment as a test gives it.
fn node(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> + use<> {
    let vars: BTreeMap<String, OsString> = vars
        .iter()
        .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
        .collect();
    move |name| vars.get(name).cloned()
}

#[test]
fn the_command_starts_from_an_empty_environment() {
    let dir = std::env::temp_dir().join(format!("vessel-added-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let program = dir.join(if cfg!(windows) { "agent.exe" } else { "agent" });
    std::fs::write(&program, b"").unwrap();
    let text = format!(
        "[agents.a]\nprogram = '{}'\nargs = [\"acp\"]\nenv = {{ MODE = \"x\", KEY = {{ secret = \"key\" }} }}\ninherit = [\"WANTED\"]\n",
        program.display()
    );
    let agents = parse(&text).unwrap();
    let agent = agents.get(&"a".parse().unwrap()).unwrap();
    let env = HarnessEnv::new(absolute("home/u"));
    let lookup = Lookup::new([dir.clone()]);
    let secrets = BTreeMap::from([(
        "key".parse().unwrap(),
        ResolvedValue::new("s3cret".to_owned()),
    )]);
    let reader = node(&[
        ("ANTHROPIC_API_KEY", "leak"),
        ("VESSEL_TOKEN", "leak"),
        ("WANTED", "yes"),
        ("LANG", "C.UTF-8"),
        ("HTTPS_PROXY", "http://user:pw@proxy:3128"),
        ("NO_PROXY", "localhost"),
        ("SSH_AUTH_SOCK", "/tmp/agent"),
    ]);
    let command = command(agent, &lookup, &env, &secrets, &reader).unwrap();
    assert_eq!(command.program, program);
    let changes: BTreeMap<String, String> = command
        .env
        .iter()
        .filter_map(|(name, value)| match value {
            EnvValue::Set(value) => Some((
                name.to_string_lossy().into_owned(),
                value.to_string_lossy().into_owned(),
            )),
            EnvValue::Remove => None,
        })
        .collect();
    assert_eq!(command.environment, Environment::Clear);
    assert_eq!(changes.get("MODE").map(String::as_str), Some("x"));
    assert_eq!(changes.get("KEY").map(String::as_str), Some("s3cret"));
    assert_eq!(changes.get("WANTED").map(String::as_str), Some("yes"));
    assert_eq!(
        changes.get("NO_PROXY").map(String::as_str),
        Some("localhost")
    );
    let home = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    assert!(changes.contains_key(home));
    assert!(changes.contains_key("PATH"));
    for kept_out in [
        "ANTHROPIC_API_KEY",
        "VESSEL_TOKEN",
        "HTTPS_PROXY",
        "SSH_AUTH_SOCK",
    ] {
        assert!(
            !changes.contains_key(kept_out),
            "{kept_out} reached the agent"
        );
    }
    if cfg!(unix) {
        assert_eq!(changes.get("LANG").map(String::as_str), Some("C.UTF-8"));
    }

    assert!(matches!(
        super::command(agent, &lookup, &env, &BTreeMap::new(), &reader),
        Err(HarnessError::SecretMissing { .. })
    ));
    let gone = parse("[agents.b]\nprogram = \"no-such-agent-here\"\n").unwrap();
    let gone = gone.get(&"b".parse().unwrap()).unwrap();
    assert!(matches!(
        super::command(gone, &lookup, &env, &secrets, &reader),
        Err(HarnessError::AgentMissing { .. })
    ));
    drop(std::fs::remove_dir_all(&dir));
}

#[test]
fn an_added_agent_launches_structured_only_and_as_its_own_profile() {
    use vessel_proto::{HarnessRef, ResolvedMcpServers, SessionMode, SessionSpec, TerminalSize};

    use crate::profile::Profile;

    let dir = std::env::temp_dir().join(format!("vessel-added-launch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let program = dir.join(if cfg!(windows) { "agent.exe" } else { "agent" });
    std::fs::write(&program, b"").unwrap();
    let text = format!("[agents.a]\nprogram = '{}'\n", program.display());
    let agents = parse(&text).unwrap();
    let agent = Arc::clone(agents.get(&"a".parse().unwrap()).unwrap());
    let profile = Profile::default_added(agent);
    let env = HarnessEnv::new(dir.clone());
    let lookup = Lookup::new([dir.clone()]);
    let none = ResolvedMcpServers::new();
    let harness = HarnessRef::Added("a".parse().unwrap());
    let structured = SessionSpec::new(
        harness.clone(),
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    )
    .with_cwd(dir.clone());
    let launched = crate::launch(&lookup, &env, &structured, &profile, None, &none, None).unwrap();
    assert_eq!(launched.command.program, program);
    assert_eq!(launched.command.environment, Environment::Clear);

    let terminal = SessionSpec::new(
        harness,
        SessionMode::Terminal {
            args: Vec::new(),
            size: TerminalSize::default(),
        },
    )
    .with_cwd(dir.clone());
    assert!(matches!(
        crate::launch(&lookup, &env, &terminal, &profile, None, &none, None),
        Err(HarnessError::NoTerminal { .. })
    ));
    // One of the four's profile is not the added agent's.
    let codex = Profile::default_for(vessel_proto::HarnessKind::Codex);
    assert!(matches!(
        crate::launch(&lookup, &env, &structured, &codex, None, &none, None),
        Err(HarnessError::ProfileMismatch { .. })
    ));
    drop(std::fs::remove_dir_all(&dir));
}

#[test]
fn an_installer_gets_the_clean_environment_and_npm_settings_only() {
    let dir = std::env::temp_dir().join(format!("vessel-installer-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let npm = dir.join(if cfg!(windows) { "npm.exe" } else { "npm" });
    std::fs::write(&npm, b"").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&npm, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let install = crate::setup::installs(vessel_proto::HarnessKind::Codex)
        .into_iter()
        .find(|install| install.part == crate::setup::InstallPart::Harness)
        .unwrap();
    let env = HarnessEnv::new(absolute("home/u"));
    let lookup = Lookup::new([dir.clone()]);
    let reader = node(&[("ANTHROPIC_API_KEY", "leak"), ("LANG", "C.UTF-8")]);
    let vars = [
        ("npm_config_cache", "/cache"),
        ("NPM_CONFIG_REGISTRY", "https://registry.example"),
        ("OPENAI_API_KEY", "leak"),
    ]
    .map(|(name, value)| (OsString::from(name), OsString::from(value)));
    let command = installer(&install, &lookup, &env, &reader, vars).unwrap();
    assert_eq!(command.program, npm);
    assert_eq!(command.args, ["install", "-g", "@openai/codex"]);
    assert_eq!(command.environment, Environment::Clear);
    assert_eq!(command.cwd.as_deref(), Some(absolute("home/u").as_path()));
    let set: Vec<String> = command
        .env
        .iter()
        .map(|(name, _value)| name.to_string_lossy().into_owned())
        .collect();
    for wanted in ["npm_config_cache", "NPM_CONFIG_REGISTRY", "PATH"] {
        assert!(set.iter().any(|name| name == wanted), "{wanted}: {set:?}");
    }
    for kept_out in ["ANTHROPIC_API_KEY", "OPENAI_API_KEY"] {
        assert!(
            !set.iter().any(|name| name == kept_out),
            "{kept_out} reached it"
        );
    }
    let empty = Lookup::new([dir.join("nothing")]);
    assert!(installer(&install, &empty, &env, &reader, []).is_none());
    drop(std::fs::remove_dir_all(&dir));
}
