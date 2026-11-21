//! Adoption and marks, with every case of Paseo's
//! `protocol/src/terminal-profiles.test.ts` 28-100, 140-176 and 178-212
//! (the line each mirrors is named beside it).

use super::*;

fn kept(id: &str, command: &str, args: &[&str], icon: Option<&str>) -> Profile {
    Profile::new(
        Id::parse(id).unwrap(),
        Name::parse("Kept").unwrap(),
        Program::parse(command).unwrap(),
        args.iter()
            .map(|arg| Argument::parse(*arg).unwrap())
            .collect(),
        icon.map(|icon| Icon::parse(icon).unwrap()),
    )
    .unwrap()
}

fn args(profile: &Profile) -> Vec<&str> {
    profile.args().iter().map(Argument::as_str).collect()
}

#[test]
fn a_shipped_agents_program_takes_the_prompt_by_its_command_not_its_id() {
    // 28-38: the whole profile, only its arguments grown.
    let bare = kept("claude", "claude", &[], Some("claude"));
    assert_eq!(
        bare.adopted(),
        kept("claude", "claude", &[PROMPT_SENTINEL], Some("claude"))
    );
    // 40-47 and 50-54: a generated id, and the shipped id, keep their own
    // arguments first.
    let generated = kept("profile_1739308800000_a1b2c3", "codex", &["--yolo"], None);
    assert_eq!(args(&generated.adopted()), ["--yolo", PROMPT_SENTINEL]);
    let same_id = kept("codex", "codex", &["--yolo"], None);
    assert_eq!(args(&same_id.adopted()), ["--yolo", PROMPT_SENTINEL]);
    let path = kept("profile_b", "/usr/local/bin/codex", &[], None);
    assert_eq!(args(&path.adopted()), [PROMPT_SENTINEL]);
    let windows = kept(
        "profile_c",
        r"C:\Program Files\nodejs\claude.cmd",
        &[],
        None,
    );
    assert_eq!(args(&windows.adopted()), [PROMPT_SENTINEL]);
    // The shipped form per program: OpenCode's positional is a folder.
    let open = kept("profile_d", "opencode", &[], None);
    assert_eq!(args(&open.adopted()), ["--prompt={{{prompt}}}"]);
    let pi = kept("profile_e", "pi", &["--thinking", "high"], None);
    assert_eq!(args(&pi.adopted()), ["--thinking", "high", PROMPT_SENTINEL]);
}

#[test]
fn a_profile_with_the_placeholder_a_positional_or_another_program_is_kept() {
    let has = kept("codex", "codex", &[PROMPT_SENTINEL, "--yolo"], None);
    assert_eq!(has.adopted(), has);
    for other in [
        kept("zsh-custom", "zsh", &[], None),
        kept("claude", "gemini", &[], None),
    ] {
        assert_eq!(other.adopted(), other);
    }
    for (command, given) in [
        ("codex", &["--profile", "work", "login"][..]),
        ("opencode", &["--model", "anthropic/claude", "run"][..]),
        ("claude", &["--", "x"][..]),
    ] {
        let subcommand = kept("profile_f", command, given, None);
        assert_eq!(subcommand.adopted(), subcommand, "{command}");
    }
    // 155-165: options are the only arguments, so exactly the shipped
    // prompt form is appended.
    for (command, given, appended) in [
        ("codex", &["--profile", "work"][..], PROMPT_SENTINEL),
        ("opencode", &["--model", "anthropic/claude"][..], "--prompt={{{prompt}}}"),
        ("pi", &["--thinking", "high"][..], PROMPT_SENTINEL),
        ("claude", &["--model=opus"][..], PROMPT_SENTINEL),
    ] {
        let adopted = kept("profile_g", command, given, None).adopted();
        let mut expected = given.to_vec();
        expected.push(appended);
        assert_eq!(args(&adopted), expected, "{command}");
    }
}

#[test]
fn a_list_adopts_only_its_matching_rows_in_order() {
    let list = Profiles::new(vec![
        kept("claude", "claude", &[], None),
        kept("profile_h", "gemini", &[], None),
    ])
    .unwrap();
    let adopted = list.adopted();
    assert_eq!(args(&adopted.entries()[0]), [PROMPT_SENTINEL]);
    assert_eq!(adopted.entries()[1], list.entries()[1]);
    assert_eq!(
        Profiles::shipped().unwrap().adopted(),
        Profiles::shipped().unwrap()
    );
}

#[test]
fn a_command_guesses_its_mark_by_its_whole_base_name() {
    // 179-205, then Vessel's own forms (a Windows script, a relative path).
    for (command, expected) in [
        ("claude", "claude"),
        ("codex", "codex"),
        ("opencode", "opencode"),
        ("copilot", "copilot"),
        ("kiro", "kiro"),
        ("omp", "omp"),
        ("pi", "pi"),
        ("gemini", "gemini"),
        ("cursor", "cursor"),
        ("goose", "goose"),
        ("grok", "grok"),
        ("kimi", "kimi"),
        ("Claude", "claude"),
        ("CODEX", "codex"),
        ("/usr/local/bin/claude", "claude"),
        (r"C:\Program Files\Codex\codex.exe", "codex"),
        ("/usr/local/bin/gemini", "gemini"),
        ("agy", "agy"),
        ("/usr/local/bin/agy", "agy"),
        ("cursor-agent", "cursor"),
        ("/usr/local/bin/cursor-agent", "cursor"),
        (r"C:\tools\Codex.CMD", "codex"),
        ("bin/codex", "codex"),
    ] {
        assert_eq!(guess(command), Some(expected), "{command}");
    }
    // 206-212, then near misses.
    for command in [
        "zsh",
        "bash",
        "fish",
        "",
        "/usr/bin/foo",
        "cursor-tool",
        "claude-code",
        "my-codex",
        ".codex",
    ] {
        assert_eq!(guess(command), None, "{command}");
    }
    // A leading dot is part of the name (Paseo cuts only at a dot past 0).
    assert_eq!(base_name(".hidden.sh"), ".hidden.sh");
    assert_eq!(base_name("tool.v2.exe"), "tool");
}

#[test]
fn a_row_draws_its_set_mark_else_the_guess() {
    assert_eq!(
        kept("a", "claude", &[], Some("claude")).mark(),
        ProfileIcon::Known("claude")
    );
    assert_eq!(
        kept("b", "claude", &[], None).mark(),
        ProfileIcon::Known("claude")
    );
    assert_eq!(
        kept("c", "cursor-agent", &[], None).mark(),
        ProfileIcon::Known("cursor")
    );
    // A set mark wins over the guess, as Paseo's `icon ??`.
    assert_eq!(
        kept("d", "cursor-agent", &[], Some("claude")).mark(),
        ProfileIcon::Known("claude")
    );
    assert_eq!(kept("e", "zsh", &[], None).mark(), ProfileIcon::Terminal);
    assert_eq!(
        kept("f", "zsh", &[], Some("not-a-provider")).mark(),
        ProfileIcon::Unknown
    );
    let shipped = Profiles::shipped().unwrap();
    // 102-109, 167-175: the defaults carry their icon, not only a guess.
    let icons: Vec<Option<&str>> = shipped
        .entries()
        .iter()
        .map(|profile| profile.icon().map(Icon::as_str))
        .collect();
    assert_eq!(
        icons,
        [Some("claude"), Some("codex"), Some("opencode"), Some("pi")]
    );
    let marks: Vec<ProfileIcon> = shipped.entries().iter().map(Profile::mark).collect();
    assert_eq!(
        marks,
        [
            ProfileIcon::Known("claude"),
            ProfileIcon::Known("codex"),
            ProfileIcon::Known("opencode"),
            ProfileIcon::Known("pi"),
        ]
    );
}

#[test]
fn every_prompt_the_wire_takes_launches_a_profile_that_holds_the_placeholder_once() {
    // R3 of the node review: the wire's bound is one argument's, and a
    // substituted argument may hold a template's bound and a prompt's.
    let longest = Prompt::parse("x".repeat(Prompt::MAX_BYTES)).unwrap();
    assert!(Prompt::parse("x".repeat(Prompt::MAX_BYTES.saturating_add(1))).is_err());
    assert_eq!(Prompt::MAX_BYTES, Argument::MAX_BYTES);
    for profile in Profiles::shipped().unwrap().entries() {
        let resolved = profile.resolve(&longest).unwrap();
        assert!(resolved.args()[0].as_str().ends_with(longest.as_str()));
    }
    let padded = "p".repeat(Argument::MAX_BYTES.saturating_sub(PROMPT_SENTINEL.len()));
    let wide = kept("profile_w", "tool", &[&format!("{padded}{PROMPT_SENTINEL}")], None);
    assert_eq!(
        wide.resolve(&longest).unwrap().args()[0].as_str().len(),
        RESOLVED_ARGUMENT_BYTES
    );
    // Twice in one argument is past the bound: refused, naming it.
    let shorter = "p".repeat(Argument::MAX_BYTES.saturating_sub(PROMPT_SENTINEL.len().saturating_mul(2)));
    let twice = kept(
        "profile_t",
        "tool",
        &[&format!("{shorter}{PROMPT_SENTINEL}{PROMPT_SENTINEL}")],
        None,
    );
    assert!(matches!(
        twice.resolve(&longest),
        Err(ProfileError::TooLong { kind: TextKind::Argument, max: RESOLVED_ARGUMENT_BYTES })
    ));
}
