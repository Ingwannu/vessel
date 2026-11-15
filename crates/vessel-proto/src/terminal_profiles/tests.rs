//! Reference substitution fixtures plus adversarial decode and allocation bounds.

use super::*;

fn profile(command: &str, args: &[&str]) -> Profile {
    Profile::new(
        Id::parse("test").unwrap(),
        Name::parse("Test").unwrap(),
        Program::parse(command).unwrap(),
        args.iter()
            .map(|arg| Argument::parse(*arg).unwrap())
            .collect(),
        None,
    )
    .unwrap()
}

#[test]
fn shipped_defaults_match_reference_order_and_empty_prompt_behavior() {
    let defaults = Profiles::shipped().unwrap();
    assert_eq!(
        defaults
            .entries()
            .iter()
            .map(|profile| profile.id().as_str())
            .collect::<Vec<_>>(),
        ["claude", "codex", "opencode", "pi"]
    );
    let empty = Prompt::parse("").unwrap();
    for entry in defaults.entries() {
        assert!(entry.takes_prompt());
        assert!(entry.resolve(&empty).unwrap().args().is_empty());
    }
    let open = defaults.get(&Id::parse("opencode").unwrap()).unwrap();
    let launch = open.resolve(&Prompt::parse("안녕 world").unwrap()).unwrap();
    assert_eq!(
        launch
            .args()
            .iter()
            .map(Argument::as_str)
            .collect::<Vec<_>>(),
        ["--prompt=안녕 world"]
    );
}

#[test]
fn substitution_preserves_metacharacters_and_argv_boundaries() {
    let template = format!("prefix-{PROMPT_SENTINEL}-{PROMPT_SENTINEL}");
    let profile = profile("tool", &["--message", PROMPT_SENTINEL, &template]);
    let prompt = "a b; $(touch nope) 'quoted'\nsecond line";
    let launch = profile.resolve(&Prompt::parse(prompt).unwrap()).unwrap();
    assert_eq!(launch.command().as_str(), "tool");
    assert_eq!(
        launch
            .args()
            .iter()
            .map(Argument::as_str)
            .collect::<Vec<_>>(),
        ["--message", prompt, &format!("prefix-{prompt}-{prompt}")]
    );
}

#[test]
fn empty_prompt_drops_only_prompt_only_arguments_and_preserves_whitespace() {
    let assignment = format!("--prompt={PROMPT_SENTINEL}");
    let embedded = format!("before {PROMPT_SENTINEL} after");
    let empty_option = format!("={PROMPT_SENTINEL}");
    let profile = profile(
        "tool",
        &[PROMPT_SENTINEL, &assignment, &embedded, "", &empty_option],
    );
    let launch = profile.resolve(&Prompt::parse("").unwrap()).unwrap();
    assert_eq!(
        launch
            .args()
            .iter()
            .map(Argument::as_str)
            .collect::<Vec<_>>(),
        ["before  after", "", "="]
    );
    let whitespace = profile.resolve(&Prompt::parse(" ").unwrap()).unwrap();
    assert_eq!(whitespace.args().first().unwrap().as_str(), " ");
}

#[test]
fn read_only_profiles_do_not_inject_a_prompt_and_command_templates_also_expand() {
    let static_profile = profile("tool", &["--version"]);
    assert!(!static_profile.takes_prompt());
    let launch = static_profile
        .resolve(&Prompt::parse("unused").unwrap())
        .unwrap();
    assert_eq!(launch.args().first().unwrap().as_str(), "--version");
    let template = profile(PROMPT_SENTINEL, &[]);
    assert_eq!(
        template
            .resolve(&Prompt::parse("/path/to/tool").unwrap())
            .unwrap()
            .command()
            .as_str(),
        "/path/to/tool"
    );
    assert_eq!(
        template.resolve(&Prompt::parse("").unwrap()),
        Err(ProfileError::Empty(TextKind::Program))
    );
}

#[test]
fn reserved_duplicate_and_oversized_values_are_rejected_on_decode() {
    assert_eq!(Id::parse("blank"), Err(ProfileError::ReservedId));
    assert!(Id::parse("white space").is_err());
    for text in ["", "\0", " \t"] {
        assert!(Program::parse(text).is_err());
    }
    assert!(Argument::parse("a\0b").is_err());
    assert!(Name::parse("line\nbreak").is_err());
    let one = profile("tool", &[]);
    assert_eq!(
        Profiles::new(vec![one.clone(), one.clone()]),
        Err(ProfileError::DuplicateId)
    );
    let raw = serde_json::to_string(&vec![one.clone(), one.clone()]).unwrap();
    assert!(serde_json::from_str::<Profiles>(&raw).is_err());
    let mut wire = serde_json::to_value(one).unwrap();
    wire["command"] = serde_json::json!("bad\0program");
    assert!(serde_json::from_value::<Profile>(wire).is_err());
}

#[test]
fn aggregate_and_expanded_bounds_are_checked_and_debug_redacts_content() {
    let argument = Argument::parse("x".repeat(Argument::MAX_BYTES)).unwrap();
    assert!(Argument::parse("x".repeat(Argument::MAX_BYTES.saturating_add(1))).is_err());
    assert!(
        Profile::new(
            Id::parse("test").unwrap(),
            Name::parse("Test").unwrap(),
            Program::parse("tool").unwrap(),
            vec![argument; MAX_ARGUMENTS],
            None
        )
        .is_err()
    );
    let repeated = PROMPT_SENTINEL.repeat(100);
    let profile = profile("tool", &[&repeated]);
    let prompt = Prompt::parse("private-token".repeat(100)).unwrap();
    assert!(matches!(
        profile.resolve(&prompt),
        Err(ProfileError::TooLong { .. })
    ));
    assert!(!format!("{profile:?} {prompt:?}").contains("private-token"));
    assert_eq!(Profiles::new(Vec::new()).unwrap().entries(), &[]);
}
