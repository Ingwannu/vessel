//! A nonempty allocation choice is retained before any helper can run.

use super::*;

const BLOCK: &str =
    "[worktree.service_ports]\nrange = '7000-7009'\nport_script = 'tools/port-approval'\n";

#[test]
fn a_block_without_services_is_still_a_nonempty_version_three_approval() {
    let file = ProjectFile::parse(BLOCK.as_bytes()).unwrap();
    assert!(!file.is_empty());
    assert!(file.scripts.is_empty() && file.terminals.is_empty());
    let ports = file.service_ports.as_ref().unwrap();
    assert_eq!(ports.range().unwrap().to_string(), "7000-7009");
    assert_eq!(ports.port_script().unwrap().as_str(), "tools/port-approval");
    assert_eq!(
        file.digest().as_str(),
        "4a8d9ac2af45593540831e4997e5b981d024ca268e3bb397c21e83f4ffcc5d72"
    );
}

#[test]
fn either_choice_or_both_are_read_with_the_shared_javascript_trim_rule() {
    for choices in [
        "range = '7000-7009'",
        "port_script = 'tools/port-approval'",
        "range = '\u{feff}7000-7009\u{2028}'\nport_script = '\u{feff}tools/port-approval\u{2029}'",
    ] {
        let text = format!("[worktree.service_ports]\n{choices}\n");
        let file = ProjectFile::parse(text.as_bytes()).unwrap();
        let ports = file.service_ports.unwrap();
        if let Some(range) = ports.range() {
            assert_eq!(range.to_string(), "7000-7009");
        }
        if let Some(program) = ports.port_script() {
            assert_eq!(program.as_str(), "tools/port-approval");
        }
    }
}

#[test]
fn invalid_or_unknown_allocation_choices_refuse_the_whole_file() {
    for choices in [
        "",
        "range = '7009-7000'",
        "range = 7000",
        "port_script = ''",
        "port_script = '../port'",
        "range = '7000-7009'\nextra = true",
        "range = '\u{0085}7000-7009'",
        "port_script = '\u{0085}tools/port'",
    ] {
        let text =
            format!("[worktree]\nsetup = 'do-not-run'\n[worktree.service_ports]\n{choices}\n");
        assert!(ProjectFile::parse(text.as_bytes()).is_err(), "{choices}");
    }
    assert!(ProjectFile::parse(b"[worktree]\nservice_ports = []\n").is_err());
}

#[test]
fn either_approved_choice_and_its_removal_change_the_digest() {
    let original = ProjectFile::parse(BLOCK.as_bytes()).unwrap().digest();
    for changed in [
        BLOCK.replace("7000-7009", "7000-7010"),
        BLOCK.replace("tools/port-approval", "tools/port-next"),
        BLOCK.replace("range = '7000-7009'\n", ""),
        BLOCK.replace("port_script = 'tools/port-approval'\n", ""),
        String::new(),
    ] {
        assert_ne!(
            ProjectFile::parse(changed.as_bytes()).unwrap().digest(),
            original
        );
    }
}
