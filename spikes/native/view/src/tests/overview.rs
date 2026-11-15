//! Settings › Overview: Paseo's rename dialog rules over the machine book,
//! and the version badge's text.

use crate::board::NodeVersion;
use crate::machine_book::{Edit, Failure, Name};
use crate::machines::NameProblem;
use crate::overview::{Ask, Editing, ask};

fn name(text: &str) -> Name {
    Name::parse(text).unwrap()
}

#[test]
fn an_unchanged_name_sends_nothing_even_with_spaces_around_it() {
    assert_eq!(
        ask(&name("laptop"), "  laptop ", &["build-server"]),
        Ask::Unchanged
    );
}

#[test]
fn a_valid_new_name_renames_the_entry() {
    assert_eq!(
        ask(&name("laptop"), "studio-mac", &["build-server"]),
        Ask::Send(Edit::Rename {
            from: name("laptop"),
            to: name("studio-mac"),
        })
    );
}

#[test]
fn says_why_a_name_cannot_be_used() {
    assert_eq!(
        ask(&name("laptop"), "build-server", &["build-server"]),
        Ask::Problem(NameProblem::Taken("build-server".to_owned()))
    );
    assert_eq!(
        ask(&name("laptop"), "", &[]),
        Ask::Problem(NameProblem::Empty)
    );
    assert_eq!(
        ask(&name("laptop"), "내 노트북", &[]),
        Ask::Problem(NameProblem::Start)
    );
    assert!(matches!(
        ask(&name("laptop"), "lap top", &[]),
        Ask::Problem(NameProblem::Characters)
    ));
}

#[test]
fn the_dialog_closes_on_its_own_edit_succeeding_and_keeps_a_refusal() {
    let edit = Edit::Rename {
        from: name("laptop"),
        to: name("studio"),
    };
    let other = Edit::Forget(name("build-server"));
    let mut dialog = Editing::open(name("laptop"));
    dialog.sent(edit.clone());
    assert!(!dialog.answered(&other, &Ok(())));
    assert_eq!(dialog.pending.as_ref(), Some(&edit));
    assert!(!dialog.answered(&edit, &Err(Failure::Taken)));
    assert_eq!(dialog.refused, Some(Failure::Taken));
    assert_eq!(dialog.pending, None);
    dialog.sent(edit.clone());
    assert_eq!(dialog.refused, None);
    assert!(dialog.answered(&edit, &Ok(())));
}

#[test]
fn a_version_badge_has_one_v_and_none_when_blank() {
    assert_eq!(NodeVersion::new("0.1.0").label().as_deref(), Some("v0.1.0"));
    assert_eq!(NodeVersion::new(" v2.3 ").label().as_deref(), Some("v2.3"));
    assert_eq!(NodeVersion::new("  ").label(), None);
}
