//! Recovery options are validated before any node connection or mutation.
use super::*;
use crate::args::{Cli, Command};
use clap::Parser as _;
#[test]
fn startup_and_verbatim_prompt_are_independent_typed_options() {
    let cli = Cli::try_parse_from([
        "vessel",
        "node",
        "settings",
        "--resume-on-startup",
        "off",
        "--resume-prompt",
        "  계속하세요.\n",
    ])
    .unwrap();
    let Command::Node {
        command:
            NodeCommand::Settings {
                resume_on_startup,
                resume_prompt,
                ..
            },
    } = cli.command
    else {
        panic!("settings command")
    };
    assert_eq!(resume_on_startup, Some(ResumeOnStartup::Off));
    assert_eq!(resume_prompt.unwrap().as_str(), "  계속하세요.\n");
    for args in [
        vec!["vessel", "node", "settings", "--resume-prompt", "  "],
        vec!["vessel", "node", "settings", "--resume-on-startup", "yes"],
    ] {
        assert!(Cli::try_parse_from(args).is_err());
    }
}
#[test]
fn legacy_settings_does_not_construct_a_recovery_patch() {
    let cli = Cli::try_parse_from(["vessel", "node", "settings", "--park-after", "never"]).unwrap();
    let Command::Node {
        command:
            NodeCommand::Settings {
                resume_on_startup,
                resume_prompt,
                ..
            },
    } = cli.command
    else {
        panic!("settings command")
    };
    assert_eq!(resume_on_startup, None);
    assert_eq!(resume_prompt, None);
}
