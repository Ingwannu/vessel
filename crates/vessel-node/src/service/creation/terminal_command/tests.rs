//! Inspect actual std Command values without executing any process or shell.
use super::*;
use std::ffi::OsStr;
use vessel_proto::terminal_profiles::{
    Argument, Id, Name, Profile, ProfileRevision, Profiles, Program,
};

const SENTINEL: &str = "PRIVATE-TERMINAL-PROMPT-CANARY ; $(never-execute)";

fn settings(command: &str, args: &[&str]) -> ProfileSettings {
    ProfileSettings {
        revision: ProfileRevision::INITIAL,
        profiles: Profiles::new(vec![
            Profile::new(
                Id::parse("configured").unwrap(),
                Name::parse("Configured").unwrap(),
                Program::parse(command).unwrap(),
                args.iter()
                    .map(|arg| Argument::parse(*arg).unwrap())
                    .collect(),
                None,
            )
            .unwrap(),
        ])
        .unwrap(),
    }
}

#[test]
fn literal_profile_command_reuses_scrubbed_environment_and_private_diagnostics() {
    let directory = tempfile::tempdir().unwrap();
    let root = Root::open(directory.path()).unwrap();
    let settings = settings(
        "private-fixture-program",
        &["--prompt={{{prompt}}}", "literal space"],
    );
    let selection = settings.summaries()[0].selection();
    let prepared = PreparedTerminal::profile(
        &settings,
        &selection,
        &Prompt::parse(SENTINEL).unwrap(),
        &root,
        &[OsString::from("VESSEL_AGENT_SECRET")],
    )
    .unwrap();
    let debug = format!("{prepared:?}");
    assert!(!debug.contains(SENTINEL));
    assert!(!debug.contains("private-fixture-program"));
    let (spec, kind) = prepared.into_parts();
    assert_eq!(spec.diagnostic_policy(), CommandDiagnostics::Redacted);
    let command = spec.to_command();
    assert_eq!(command.get_program(), OsStr::new("private-fixture-program"));
    assert_eq!(command.get_current_dir(), Some(root.path()));
    let expected = format!("--prompt={SENTINEL}");
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        [OsStr::new(&expected), OsStr::new("literal space")]
    );
    assert!(
        command
            .get_envs()
            .any(|(name, value)| name == "VESSEL_AGENT_SECRET" && value.is_none())
    );
    assert!(
        command
            .get_envs()
            .any(|(name, value)| name == "TERM" && value == Some(OsStr::new("xterm-256color")))
    );
    assert_eq!(
        kind,
        TerminalKind::Profile {
            profile: selection.profile,
            revision: selection.revision
        }
    );
}

#[test]
fn a_stale_selection_or_removed_profile_cannot_prepare_another_command() {
    let directory = tempfile::tempdir().unwrap();
    let root = Root::open(directory.path()).unwrap();
    let mut settings = settings("fixture", &["{{{prompt}}}"]);
    let selected = settings.summaries()[0].selection();
    settings.revision = settings.revision.next().unwrap();
    assert!(matches!(
        PreparedTerminal::profile(
            &settings,
            &selected,
            &Prompt::parse("").unwrap(),
            &root,
            &[]
        ),
        Err(TerminalPreparationError::Profile(
            ProfileError::StaleRevision
        ))
    ));
    let missing = ProfileSelection {
        profile: Id::parse("missing").unwrap(),
        revision: settings.revision,
    };
    assert!(matches!(
        PreparedTerminal::profile(&settings, &missing, &Prompt::parse("").unwrap(), &root, &[]),
        Err(TerminalPreparationError::Missing)
    ));
}

#[test]
fn readonly_profiles_refuse_text_and_empty_prompt_omits_only_whole_prompt_arguments() {
    let directory = tempfile::tempdir().unwrap();
    let root = Root::open(directory.path()).unwrap();
    let readonly = settings("fixture", &["fixed"]);
    let selection = readonly.summaries()[0].selection();
    assert!(matches!(
        PreparedTerminal::profile(
            &readonly,
            &selection,
            &Prompt::parse(SENTINEL).unwrap(),
            &root,
            &[]
        ),
        Err(TerminalPreparationError::ReadOnly)
    ));
    let fixed = PreparedTerminal::profile(
        &readonly,
        &selection,
        &Prompt::parse("").unwrap(),
        &root,
        &[],
    )
    .unwrap();
    let (fixed, _) = fixed.into_parts();
    assert_eq!(
        fixed.to_command().get_args().collect::<Vec<_>>(),
        [OsStr::new("fixed")]
    );
    let template = settings(
        "fixture",
        &["{{{prompt}}}", "--prompt={{{prompt}}}", "kept"],
    );
    let prepared = PreparedTerminal::profile(
        &template,
        &template.summaries()[0].selection(),
        &Prompt::parse("").unwrap(),
        &root,
        &[],
    )
    .unwrap();
    let (spec, _) = prepared.into_parts();
    assert_eq!(
        spec.to_command().get_args().collect::<Vec<_>>(),
        [OsStr::new("kept")]
    );
}

#[test]
fn blank_terminal_reuses_the_detected_shell_without_inventing_prompt_argv() {
    let directory = tempfile::tempdir().unwrap();
    let root = Root::open(directory.path()).unwrap();
    let shell = Shell {
        program: "fixture-shell".into(),
        args: vec!["--login".to_owned()],
    };
    let (spec, kind) = PreparedTerminal::blank(&shell, &root, &[]).into_parts();
    assert_eq!(spec.diagnostic_policy(), CommandDiagnostics::Redacted);
    let command = spec.to_command();
    assert_eq!(command.get_program(), OsStr::new("fixture-shell"));
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        [OsStr::new("--login")]
    );
    assert_eq!(
        kind,
        TerminalKind::Shell {
            program: shell.program
        }
    );
}
