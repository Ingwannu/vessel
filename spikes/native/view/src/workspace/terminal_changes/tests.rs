use super::*;
use crate::text::workspace::{En, Ko, Local, WorkspaceWords};
use crate::workspace::command_changes::{Compared, ComparedWith, Listed, compare_approved};
use chrono::{TimeZone as _, Utc};
use vessel_proto::workspace::{ApprovedCommands, CommandSet, ProjectCommands};

fn terminal(name: Option<&str>, command: &str) -> FileTerminal {
    FileTerminal {
        name: name.map(|name| name.parse().unwrap()),
        command: command.parse().unwrap(),
    }
}

fn commands(
    terminals: Vec<FileTerminal>,
    approved: bool,
    digest: char,
    last: Option<ApprovedCommands>,
) -> ProjectCommands {
    ProjectCommands::Listed {
        setup: vec![],
        teardown: vec![],
        scripts: vec![],
        terminals,
        service_ports: None,
        digest: digest.to_string().repeat(64).parse().unwrap(),
        approved,
        last_approved: last.map(Box::new),
    }
}

#[test]
fn name_only_edits_keep_both_complete_definitions() {
    let was = terminal(None, "npm run dev");
    let now = terminal(Some("server"), "npm run dev");
    let lines = compare(std::slice::from_ref(&was), std::slice::from_ref(&now));
    assert!(
        matches!(&lines[..], [TerminalLine::Changed { position, terminal, was: before }]
        if position.ordinal() == 1 && terminal == &now && before == &was)
    );
}

#[test]
fn duplicate_titles_and_order_changes_never_collapse_into_one_terminal() {
    let first = terminal(Some("watch"), "npm run api");
    let second = terminal(Some("watch"), "npm run web");
    let lines = compare(
        &[first.clone(), second.clone()],
        &[second.clone(), first.clone()],
    );
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].position().ordinal(), 1);
    assert_eq!(lines[1].position().ordinal(), 2);
    assert!(lines.iter().all(TerminalLine::changed));
    assert_eq!(lines[0].terminal(), &second);
    assert_eq!(lines[1].terminal(), &first);
}

#[test]
fn removing_all_terminals_retains_the_last_approved_commands_and_names() {
    let old = terminal(Some("old server"), "npm run old");
    let prior = ApprovedCommands {
        commands: CommandSet {
            terminals: vec![old.clone()],
            ..CommandSet::default()
        },
        digest: "a".repeat(64).parse().unwrap(),
        approved_at: Utc.timestamp_opt(100, 0).unwrap(),
    };
    let asked = commands(vec![], false, 'b', Some(prior));
    let Compared::Changed {
        terminals, against, ..
    } = compare_approved(&Listed::of(&asked).unwrap(), None)
    else {
        panic!()
    };
    assert!(matches!(against, ComparedWith::LastApproved { .. }));
    assert!(
        matches!(&terminals[..], [TerminalLine::Line { change: Change::Removed, terminal, .. }] if terminal == &old)
    );
}

#[test]
fn approved_folder_fallback_includes_terminal_changes_in_both_language_summaries() {
    let folder = commands(vec![terminal(Some("was"), "watch")], true, 'a', None);
    let asked = commands(vec![terminal(Some("now"), "watch")], false, 'b', None);
    let compared = compare_approved(&Listed::of(&asked).unwrap(), Some(&folder));
    let Compared::Changed {
        terminals, against, ..
    } = &compared
    else {
        panic!()
    };
    assert_eq!(*against, ComparedWith::Folder);
    assert!(terminals.iter().any(TerminalLine::changed));
    let local = Local {
        offset: chrono::FixedOffset::east_opt(0).unwrap(),
        today: Utc::now().date_naive(),
    };
    let english = En.compared(&compared, &local);
    let korean = Ko.compared(&compared, &local);
    assert!(english.contains("Automatic terminal changes"), "{english}");
    assert!(!english.contains("nothing that runs reads differently"));
    assert!(korean.contains("자동 터미널 변경"), "{korean}");
    assert!(!korean.contains("달라진 것은 없습니다"));
}
