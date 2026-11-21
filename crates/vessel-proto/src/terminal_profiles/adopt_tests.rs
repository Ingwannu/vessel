//! Adoption and marks, case for case with Paseo's
//! `protocol/src/terminal-profiles.test.ts` (28-100, 140-176, 178-248).

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
    let bare = kept("claude", "claude", &[], Some("claude"));
    assert_eq!(args(&bare.adopted()), [PROMPT_SENTINEL]);
    let generated = kept("profile_1739308800000_a1b2c3", "codex", &["--yolo"], None);
    assert_eq!(args(&generated.adopted()), ["--yolo", PROMPT_SENTINEL]);
    let path = kept("profile_b", "/usr/local/bin/codex", &[], None);
    assert_eq!(args(&path.adopted()), [PROMPT_SENTINEL]);
    let windows = kept("profile_c", r"C:\Program Files\nodejs\claude.cmd", &[], None);
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
    for (command, given) in [
        ("codex", &["--profile", "work"][..]),
        ("opencode", &["--model", "anthropic/claude"][..]),
        ("pi", &["--thinking", "high"][..]),
        ("claude", &["--model=opus"][..]),
    ] {
        let adopted = kept("profile_g", command, given, None).adopted();
        assert_eq!(&args(&adopted)[..given.len()], given, "{command}");
        assert!(adopted.takes_prompt(), "{command}");
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
    assert_eq!(Profiles::shipped().unwrap().adopted(), Profiles::shipped().unwrap());
}

#[test]
fn a_command_guesses_its_mark_by_its_whole_base_name() {
    for (command, expected) in [
        ("claude", "claude"),
        ("codex", "codex"),
        ("opencode", "opencode"),
        ("pi", "pi"),
        ("gemini", "gemini"),
        ("cursor", "cursor"),
        ("cursor-agent", "cursor"),
        ("Claude", "claude"),
        ("/usr/local/bin/codex", "codex"),
        (r"C:\tools\Codex.CMD", "codex"),
        ("agy", "agy"),
    ] {
        assert_eq!(guess(command), Some(expected), "{command}");
    }
    for command in ["zsh", "claude-code", "my-codex", ".codex", "bash"] {
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
    assert_eq!(kept("b", "claude", &[], None).mark(), ProfileIcon::Known("claude"));
    assert_eq!(kept("c", "cursor-agent", &[], None).mark(), ProfileIcon::Known("cursor"));
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
