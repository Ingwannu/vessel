//! The cases in Paseo's `model.test.ts`, plus the differences ADR 0071 states.
//! A visit that cannot grow is included because the counter is checked
//! arithmetic and the failure has to leave the phase alone.

use super::*;

fn at() -> CompletedAt {
    CompletedAt::parse("2026-11-14T00:00:00Z").unwrap()
}

fn pending_with(facts: MachineFacts) -> Onboarding {
    let mut model = Onboarding::new();
    model.note_completion(Completion::Pending);
    model.note_client(Client::Desktop);
    model.note_machines(facts);
    model
}

fn opened(model: &Onboarding) -> &Open {
    match model.phase() {
        Phase::Open(open) => Some(open),
        Phase::Deciding | Phase::Hidden(_) => None,
    }
    .unwrap()
}

#[test]
fn opens_on_the_theme_when_nothing_is_paired() {
    let model = pending_with(MachineFacts::None);
    assert_eq!(opened(&model).step, Step::Theme);
}

#[test]
fn stays_deciding_while_project_lists_are_unknown() {
    let mut model = Onboarding::new();
    model.note_completion(Completion::Pending);
    model.note_client(Client::Desktop);
    assert_eq!(model.phase(), &Phase::Deciding);
    model.note_machines(MachineFacts::ProjectsUnknown);
    assert_eq!(model.phase(), &Phase::Deciding);

    model.note_machines(MachineFacts::Projects(Projects::None));
    assert_eq!(opened(&model).step, Step::Theme);
}

#[test]
fn stays_hidden_when_a_machine_already_had_a_project() {
    let mut model = pending_with(MachineFacts::Projects(Projects::Some));
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::AlreadySetUp));

    model.note_machines(MachineFacts::Projects(Projects::None));
    model.note_machines(MachineFacts::None);
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::AlreadySetUp));
}

#[test]
fn stays_hidden_once_completed() {
    let mut model = Onboarding::new();
    model.note_completion(Completion::Completed(at()));
    model.note_machines(MachineFacts::None);
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::Finished));
}

#[test]
fn decides_only_after_the_record_is_known() {
    let mut model = Onboarding::new();
    model.note_machines(MachineFacts::None);
    assert_eq!(model.phase(), &Phase::Deciding);

    model.note_completion(Completion::Completed(at()));
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::Finished));
}

#[test]
fn a_completed_record_hides_without_waiting_for_machines() {
    let mut model = Onboarding::new();
    model.note_completion(Completion::Completed(at()));
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::Finished));
}

#[test]
fn stays_open_when_a_project_appears_during_setup() {
    let mut model = pending_with(MachineFacts::None);
    let moved = model.next(opened(&model).visit, at());
    assert_eq!(moved, Advance::Moved);
    assert_eq!(opened(&model).step, Step::Machine);

    model.note_machines(MachineFacts::Projects(Projects::Some));
    assert_eq!(opened(&model).step, Step::Machine);
}

#[test]
fn the_desktop_list_includes_the_phone_step() {
    assert_eq!(
        Step::DESKTOP,
        [
            Step::Theme,
            Step::Machine,
            Step::Agents,
            Step::Project,
            Step::Orchestration,
            Step::Phone
        ]
    );
    assert_eq!(Step::PHONE.len(), 5);
    assert_eq!(Step::Phone.following(Client::Desktop), None);
    assert_eq!(Step::Orchestration.following(Client::Phone), None);
    assert_eq!(Step::Theme.previous(Client::Desktop), None);
    assert_eq!(ToolsChoice::default(), ToolsChoice::On);
}

#[test]
fn moves_inside_the_list_and_back_stops_at_the_first() {
    let mut model = pending_with(MachineFacts::None);
    let first = opened(&model).visit;
    model.back();
    assert_eq!(opened(&model).visit, first);
    assert_eq!(opened(&model).step, Step::Theme);

    assert_eq!(model.next(first, at()), Advance::Moved);
    assert_ne!(opened(&model).visit, first);
    assert_eq!(model.next(opened(&model).visit, at()), Advance::Moved);
    assert_eq!(model.next(opened(&model).visit, at()), Advance::Moved);
    assert_eq!(opened(&model).step, Step::Project);

    model.go_to(Step::Machine);
    let on_machine = opened(&model).visit;
    model.go_to(Step::Machine);
    assert_eq!(opened(&model).visit, on_machine);
    assert_eq!(opened(&model).step, Step::Machine);
    model.back();
    assert_eq!(opened(&model).step, Step::Theme);
    assert_ne!(opened(&model).visit, on_machine);
}

#[test]
fn ignores_a_late_advance_from_a_visit_already_left() {
    let mut model = Onboarding::new();
    model.note_completion(Completion::Completed(at()));
    model.reopen(Client::Desktop);
    model.go_to(Step::Agents);
    let adding_from = opened(&model).visit;

    model.go_to(Step::Theme);
    model.go_to(Step::Agents);
    assert_eq!(model.next(adding_from, at()), Advance::Ignored);
    assert_eq!(opened(&model).step, Step::Agents);

    let remembered = model.complete(at());
    model.reopen(Client::Desktop);
    model.go_to(Step::Agents);
    assert_eq!(model.next(adding_from, at()), Advance::Ignored);
    assert_eq!(opened(&model).step, Step::Agents);
    assert_eq!(remembered.at, at());
}

#[test]
fn the_last_step_finishes_unless_that_visit_was_left() {
    let mut model = pending_with(MachineFacts::Projects(Projects::None));
    model.go_to(Step::Phone);
    let finishing_from = opened(&model).visit;
    model.back();
    assert_eq!(model.next(finishing_from, at()), Advance::Ignored);
    assert_eq!(opened(&model).step, Step::Orchestration);

    assert_eq!(model.next(opened(&model).visit, at()), Advance::Moved);
    assert_eq!(
        model.next(opened(&model).visit, at()),
        Advance::Finished(Remember { at: at() })
    );
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::Finished));
}

#[test]
fn closing_is_remembered_so_the_next_launch_stays_hidden() {
    let mut model = pending_with(MachineFacts::None);
    let remembered = model.complete(at());
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::Finished));
    assert_eq!(remembered.at, at());

    let mut again = Onboarding::new();
    again.note_completion(Completion::Completed(remembered.at));
    again.note_machines(MachineFacts::None);
    assert_eq!(again.phase(), &Phase::Hidden(Hidden::Finished));
}

#[test]
fn reopen_starts_at_the_theme_for_someone_who_finished() {
    let mut model = Onboarding::new();
    model.note_completion(Completion::Completed(at()));
    model.note_machines(MachineFacts::Projects(Projects::Some));
    model.reopen(Client::Desktop);
    assert_eq!(opened(&model).step, Step::Theme);
}

#[test]
fn an_unreadable_file_does_not_open_setup() {
    let mut model = Onboarding::new();
    model.note_file_unreadable();
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::Finished));
    model.note_machines(MachineFacts::None);
    model.note_completion(Completion::Pending);
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::Finished));
}

#[test]
fn a_second_record_note_does_not_replace_the_first() {
    let mut model = Onboarding::new();
    model.note_completion(Completion::Pending);
    model.note_file_unreadable();
    model.note_client(Client::Desktop);
    model.note_machines(MachineFacts::None);
    assert_eq!(opened(&model).step, Step::Theme);
}

#[test]
fn a_visit_that_cannot_grow_does_not_move() {
    let mut model = Onboarding::new();
    model.next_visit = u64::MAX;
    model.note_completion(Completion::Pending);
    model.note_client(Client::Desktop);
    model.note_machines(MachineFacts::None);
    assert_eq!(model.phase(), &Phase::Deciding);
    model.reopen(Client::Desktop);
    assert_eq!(model.phase(), &Phase::Deciding);

    // The record and the client were already noted, and the phase is still
    // deciding, so a later machine note can open it once the counter can grow.
    model.next_visit = 1;
    model.note_machines(MachineFacts::None);
    let theme = opened(&model).visit;
    assert_eq!(opened(&model).step, Step::Theme);

    model.next_visit = u64::MAX;
    assert_eq!(model.next(theme, at()), Advance::Ignored);
    assert_eq!(opened(&model).step, Step::Theme);
    assert_eq!(opened(&model).visit, theme);

    // The opening issued visit 1, so the next stay is 2.
    model.next_visit = 2;
    model.go_to(Step::Phone);
    let visit = opened(&model).visit;
    model.next_visit = u64::MAX;
    model.back();
    assert_eq!(opened(&model).step, Step::Phone);
    assert_eq!(opened(&model).visit, visit);
    assert_eq!(
        model.next(visit, at()),
        Advance::Finished(Remember { at: at() })
    );
}

#[test]
fn a_phone_client_has_no_phone_step() {
    let mut model = Onboarding::new();
    model.note_completion(Completion::Pending);
    model.note_machines(MachineFacts::None);
    assert_eq!(model.phase(), &Phase::Deciding);
    model.note_client(Client::Phone);
    assert_eq!(opened(&model).step, Step::Theme);
    assert_eq!(opened(&model).client, Client::Phone);
    model.go_to(Step::Phone);
    assert_eq!(opened(&model).step, Step::Theme);
    model.go_to(Step::Orchestration);
    assert_eq!(
        model.next(opened(&model).visit, at()),
        Advance::Finished(Remember { at: at() })
    );
}

#[test]
fn completion_time_is_rfc3339() {
    assert!(CompletedAt::parse("").is_err());
    assert!(CompletedAt::parse("2026-11-14").is_err());
    let zulu = CompletedAt::parse("2026-11-14T00:00:00Z").unwrap();
    let offset = CompletedAt::parse("2026-11-14T01:00:00+01:00").unwrap();
    assert_eq!(zulu, offset);
    assert_eq!(String::from(zulu), "2026-11-14T00:00:00.000Z");

    let record = SetupRecord { completed_at: zulu };
    let text = serde_json::to_string(&record).unwrap();
    let read: SetupRecord = serde_json::from_str(&text).unwrap();
    assert_eq!(read, record);
    assert!(serde_json::from_str::<SetupRecord>(r#"{"completed_at":"no"}"#).is_err());
    assert!(serde_json::from_str::<SetupRecord>("{}").is_err());
    assert!(serde_json::from_str::<SetupRecord>("null").is_err());
}

#[test]
fn a_project_known_before_the_record_still_hides_setup() {
    let mut model = Onboarding::new();
    model.note_client(Client::Desktop);
    model.note_machines(MachineFacts::Projects(Projects::Some));
    assert_eq!(model.phase(), &Phase::Deciding);
    model.note_completion(Completion::Pending);
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::AlreadySetUp));
}

#[test]
fn an_unknown_list_that_gains_a_project_hides_setup() {
    let mut model = Onboarding::new();
    model.note_completion(Completion::Pending);
    model.note_client(Client::Desktop);
    model.note_machines(MachineFacts::ProjectsUnknown);
    assert_eq!(model.phase(), &Phase::Deciding);
    model.note_machines(MachineFacts::Projects(Projects::Some));
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::AlreadySetUp));
}

#[test]
fn the_step_list_stays_once_setup_has_opened() {
    let mut model = pending_with(MachineFacts::None);
    assert_eq!(opened(&model).client, Client::Desktop);
    model.note_client(Client::Phone);
    assert_eq!(opened(&model).client, Client::Desktop);
    model.go_to(Step::Phone);
    assert_eq!(opened(&model).step, Step::Phone);
}

#[test]
fn reopen_uses_the_client_it_is_given() {
    let mut model = Onboarding::new();
    model.note_completion(Completion::Pending);
    model.note_client(Client::Phone);
    model.note_machines(MachineFacts::None);
    assert_eq!(opened(&model).client, Client::Phone);
    model.reopen(Client::Desktop);
    assert_eq!(opened(&model).step, Step::Theme);
    assert_eq!(opened(&model).client, Client::Desktop);
    model.go_to(Step::Phone);
    assert_eq!(opened(&model).step, Step::Phone);
}

#[test]
fn reopen_from_an_already_set_up_launch_starts_at_the_theme() {
    let mut model = pending_with(MachineFacts::Projects(Projects::Some));
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::AlreadySetUp));
    model.reopen(Client::Desktop);
    assert_eq!(opened(&model).step, Step::Theme);
    model.note_machines(MachineFacts::None);
    assert_eq!(opened(&model).step, Step::Theme);
}

#[test]
fn a_move_after_setup_has_closed_does_nothing() {
    let mut model = pending_with(MachineFacts::None);
    let visit = opened(&model).visit;
    let remembered = model.complete(at());
    assert_eq!(model.next(visit, at()), Advance::Ignored);
    model.back();
    model.go_to(Step::Agents);
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::Finished));
    assert_eq!(remembered.at, at());
}

#[test]
fn closing_while_still_deciding_is_remembered() {
    let mut model = Onboarding::new();
    let remembered = model.complete(at());
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::Finished));
    assert_eq!(remembered.at, at());
    model.note_completion(Completion::Pending);
    model.note_machines(MachineFacts::None);
    assert_eq!(model.phase(), &Phase::Hidden(Hidden::Finished));
}
