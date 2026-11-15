//! Inert commands use only testkit executable paths and synthetic environments.

use super::*;
use vessel_proc::{EnvValue, Environment};

fn prefix() -> CommandPrefix {
    CommandPrefix {
        program: "fake-acp".parse().unwrap(),
        args: Arguments::new(vec!["--native".into()]).unwrap(),
    }
}

fn environment(rows: &[(&str, &str)]) -> ProviderEnvironment {
    ProviderEnvironment::new(
        rows.iter()
            .map(|(key, value)| (key.parse().unwrap(), ConfigValue::Text((*value).into())))
            .collect(),
    )
    .unwrap()
}

#[test]
fn command_resolution_obeys_default_append_replace_without_a_shell() {
    let base = prefix();
    let (resolved, origin) = resolve_prefix(Some(&base), &CommandEdit::Default).unwrap();
    assert_eq!(resolved, base);
    assert_eq!(origin, LaunchOrigin::Default);
    let edit = CommandEdit::Append {
        args: Arguments::new(vec!["literal $(not-a-shell)".into()]).unwrap(),
    };
    let (resolved, origin) = resolve_prefix(Some(&base), &edit).unwrap();
    assert_eq!(resolved.args.all(), ["--native", "literal $(not-a-shell)"]);
    assert_eq!(origin, LaunchOrigin::Append);
    let replacement = CommandEdit::Replace {
        program: "replacement".parse().unwrap(),
        args: Arguments::default(),
    };
    let (resolved, origin) = resolve_prefix(None, &replacement).unwrap();
    assert_eq!(resolved.program.as_str(), "replacement");
    assert_eq!(origin, LaunchOrigin::Override);
    assert!(matches!(
        resolve_prefix(None, &CommandEdit::Default),
        Err(ProviderLaunchError::NoDefault)
    ));
    assert!(matches!(
        resolve_prefix(None, &edit),
        Err(ProviderLaunchError::NoDefault)
    ));
}

#[test]
fn launch_inheritance_overlays_variables_and_concatenates_denied_tools() {
    let base = LaunchSettings {
        command: CommandEdit::Replace {
            program: "fake-acp".parse().unwrap(),
            args: Arguments::default(),
        },
        env: environment(&[("BASE", "one"), ("Case", "old")]),
        disallowed_tools: DisallowedTools::new(vec!["base-tool".into()]).unwrap(),
    };
    let merged = merge_launch(
        &base,
        None,
        &environment(&[("CASE", "new")]),
        &DisallowedTools::new(vec!["child-tool".into()]).unwrap(),
    )
    .unwrap();
    assert_eq!(merged.command, base.command);
    assert_eq!(merged.env.values().len(), base.env.values().len());
    assert_eq!(
        merged.env.values().get(&"CASE".parse().unwrap()),
        Some(&ConfigValue::Text("new".into()))
    );
    assert_eq!(
        merged
            .disallowed_tools
            .patterns()
            .iter()
            .map(vessel_proto::providers::ToolPattern::as_str)
            .collect::<Vec<_>>(),
        ["base-tool", "child-tool"]
    );
}

#[test]
fn a_configured_launch_cannot_inherit_daemon_keys_or_parent_markers() {
    let fake = vessel_testkit::Fake::Acp.path().unwrap();
    let root = tempfile::tempdir().unwrap();
    let lookup = Lookup::new([fake.parent().unwrap().to_path_buf()]);
    let account = HarnessEnv::new(root.path().to_path_buf());
    let settings = LaunchSettings {
        command: CommandEdit::Replace {
            program: fake.to_str().unwrap().parse().unwrap(),
            args: Arguments::default(),
        },
        env: environment(&[
            ("SAFE_SETTING", "synthetic"),
            ("claudecode", "must-not-survive"),
        ]),
        disallowed_tools: DisallowedTools::default(),
    };
    let inherited = |name: &str| match name {
        "LANG" => Some(OsString::from("C")),
        "OPENAI_API_KEY" | "CLAUDECODE" => Some(OsString::from("synthetic-parent-key")),
        "HTTP_PROXY" => Some(OsString::from("http://user:synthetic@example.invalid")),
        _ => None,
    };
    let launch = provider_launch(
        None,
        &settings,
        &lookup,
        &account,
        &BTreeMap::new(),
        inherited,
    )
    .unwrap();
    assert_eq!(launch.origin(), LaunchOrigin::Override);
    let (command, tools) = launch.into_parts(ToolEnforcement::Unsupported).unwrap();
    assert!(tools.patterns().is_empty());
    assert_eq!(command.environment, Environment::Clear);
    assert_eq!(command.env_change("OPENAI_API_KEY"), None);
    assert_eq!(command.env_change("HTTP_PROXY"), None);
    assert_eq!(command.env_change("claudecode"), None);
    for name in PARENT_SESSION_VARIABLES {
        assert_eq!(command.env_change(name), Some(&EnvValue::Remove));
    }
    assert_eq!(command.env_change("LANG"), Some(&EnvValue::Set("C".into())));
    assert_eq!(
        command.env_change("SAFE_SETTING"),
        Some(&EnvValue::Set("synthetic".into()))
    );
}

#[test]
fn missing_secrets_and_unenforceable_tool_policy_fail_before_a_command_is_yielded() {
    let fake = vessel_testkit::Fake::Acp.path().unwrap();
    let root = tempfile::tempdir().unwrap();
    let lookup = Lookup::new([fake.parent().unwrap().to_path_buf()]);
    let account = HarnessEnv::new(root.path().to_path_buf());
    let mut settings = LaunchSettings {
        command: CommandEdit::Replace {
            program: fake.to_str().unwrap().parse().unwrap(),
            args: Arguments::default(),
        },
        env: ProviderEnvironment::new(BTreeMap::from([(
            "SYNTHETIC_KEY".parse().unwrap(),
            ConfigValue::Secret("synthetic".parse().unwrap()),
        )]))
        .unwrap(),
        disallowed_tools: DisallowedTools::new(vec!["dangerous-tool".into()]).unwrap(),
    };
    assert!(matches!(
        provider_launch(None, &settings, &lookup, &account, &BTreeMap::new(), |_| {
            None
        }),
        Err(ProviderLaunchError::MissingSecret)
    ));
    settings.env = ProviderEnvironment::default();
    let launch = provider_launch(None, &settings, &lookup, &account, &BTreeMap::new(), |_| {
        None
    })
    .unwrap();
    assert!(matches!(
        launch.into_parts(ToolEnforcement::Unsupported),
        Err(ProviderLaunchError::ToolPolicyUnsupported)
    ));
    let launch = provider_launch(None, &settings, &lookup, &account, &BTreeMap::new(), |_| {
        None
    })
    .unwrap();
    let (_command, tools) = launch.into_parts(ToolEnforcement::Native).unwrap();
    assert_eq!(tools, settings.disallowed_tools);
}

#[test]
fn relative_program_paths_never_resolve_against_a_daemon_directory() {
    let root = tempfile::tempdir().unwrap();
    let account = HarnessEnv::new(root.path().to_path_buf());
    let settings = LaunchSettings {
        command: CommandEdit::Replace {
            program: "relative/agent".parse().unwrap(),
            args: Arguments::default(),
        },
        ..LaunchSettings::default()
    };
    assert!(matches!(
        provider_launch(
            None,
            &settings,
            &Lookup::new([]),
            &account,
            &BTreeMap::new(),
            |_| None
        ),
        Err(ProviderLaunchError::RelativeProgram)
    ));
}
