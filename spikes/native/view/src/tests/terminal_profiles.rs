//! Settings › Terminals' list and edits (ADR 0117): Paseo's host page and
//! edit modal rules.

use vessel_proto::ErrorCode;
use vessel_proto::terminal_profiles::{Icon, Id, Profiles};

use crate::devices::Tickets;
use crate::terminal_profiles::{
    Draft, Listing, Problem, Step, Target, command_line, draft_of, moved, new_id, parse_args,
    removed, with_draft,
};

fn shipped() -> Profiles {
    Profiles::shipped().unwrap()
}

fn ids(list: &Profiles) -> Vec<&str> {
    list.entries().iter().map(|p| p.id().as_str()).collect()
}

#[test]
fn only_the_latest_answer_counts_and_a_refused_save_keeps_the_list() {
    let mut tickets = Tickets::default();
    let mut listing = Listing::default();
    let first = tickets.next_ticket().unwrap();
    listing.reading(first);
    assert!(listing.waiting());
    let second = tickets.next_ticket().unwrap();
    listing.reading(second);
    assert_eq!(listing.answered(first, Ok(shipped())), None);
    assert!(listing.profiles().is_none());
    assert_eq!(listing.answered(second, Ok(shipped())), None);
    assert_eq!(listing.profiles(), Some(&shipped()));
    let save = tickets.next_ticket().unwrap();
    assert!(listing.saving_with(save));
    assert!(!listing.saving_with(tickets.next_ticket().unwrap()));
    let refused = Problem::Refused(ErrorCode::Forbidden);
    assert_eq!(listing.answered(save, Err(refused)), Some(Err(refused)));
    assert_eq!(listing.profiles(), Some(&shipped()));
    assert_eq!(listing.refused(), Some(refused));
    assert!(!listing.saving());
}

#[test]
fn rows_move_one_place_and_not_past_the_ends() {
    let list = shipped();
    assert_eq!(moved(&list, 0, Step::Up), None);
    assert_eq!(moved(&list, 3, Step::Down), None);
    let down = moved(&list, 0, Step::Down).unwrap();
    assert_eq!(ids(&down), ["codex", "claude", "opencode", "pi"]);
    let up = moved(&list, 3, Step::Up).unwrap();
    assert_eq!(ids(&up), ["claude", "codex", "pi", "opencode"]);
    let gone = removed(&list, &Id::parse("codex").unwrap());
    assert_eq!(ids(&gone), ["claude", "opencode", "pi"]);
}

#[test]
fn a_draft_needs_a_name_and_a_command_and_splits_its_arguments() {
    let list = shipped();
    let add = Target::Add(Id::parse("profile_x_1").unwrap());
    let empty = with_draft(&list, &add, &Draft::default()).unwrap_err();
    assert!(empty.name && empty.command);
    let blank_name = Draft {
        name: "  ".to_owned(),
        command: "zsh".to_owned(),
        args: String::new(),
    };
    let errors = with_draft(&list, &add, &blank_name).unwrap_err();
    assert!(errors.name && !errors.command);
    assert_eq!(parse_args("  --login   -i "), ["--login", "-i"]);
    let draft = Draft {
        name: " Login shell ".to_owned(),
        command: " /bin/zsh ".to_owned(),
        args: "--login   -i".to_owned(),
    };
    let added = with_draft(&list, &add, &draft).unwrap();
    let new_row = added.entries().last().unwrap();
    assert_eq!(new_row.name().as_str(), "Login shell");
    assert_eq!(command_line(new_row), "/bin/zsh --login -i");
    assert_eq!(new_row.icon(), None);
}

#[test]
fn an_edit_keeps_the_id_and_icon_and_opens_with_the_arguments_joined() {
    let list = shipped();
    let claude = Id::parse("claude").unwrap();
    let opened = draft_of(list.get(&claude).unwrap());
    assert_eq!(opened.args, "{{{prompt}}}");
    let draft = Draft {
        name: "Claude".to_owned(),
        ..opened
    };
    let edited = with_draft(&list, &Target::Edit(claude.clone()), &draft).unwrap();
    let row = edited.get(&claude).unwrap();
    assert_eq!(row.name().as_str(), "Claude");
    assert_eq!(row.icon(), Some(&Icon::parse("claude").unwrap()));
    assert_eq!(ids(&edited), ids(&list));
}

#[test]
fn a_new_id_has_paseos_shape() {
    assert_eq!(
        new_id(1_739_308_800_000, 35).unwrap().as_str(),
        "profile_m70zjv9c_z"
    );
    assert_eq!(new_id(0, 0).unwrap().as_str(), "profile_0_0");
    assert_eq!(new_id(u64::MAX, u64::MAX).unwrap().as_str().len(), 35);
}
