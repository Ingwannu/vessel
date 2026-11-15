use super::*;

fn host() -> MachineKey {
    MachineKey::from_book("test-node")
}
fn open(model: &Model) -> &Open {
    let State::Open(open) = model.state() else {
        panic!("setup should be open");
    };
    open
}

#[test]
fn readiness_completion_and_once_only_decision() {
    let mut model = Model::default();
    model
        .decide(Completion::Loading, SetupHost::Empty(host()), Layout::Wide)
        .unwrap();
    assert_eq!(model.state(), &State::Deciding);
    model
        .decide(Completion::Pending, SetupHost::Waiting, Layout::Wide)
        .unwrap();
    assert_eq!(model.state(), &State::Deciding);
    model
        .decide(Completion::Pending, SetupHost::Empty(host()), Layout::Wide)
        .unwrap();
    assert_eq!(open(&model).step(), Step::Appearance);
    model
        .decide(Completion::Completed, SetupHost::Existing, Layout::Compact)
        .unwrap();
    assert_eq!(open(&model).steps(), WIDE_STEPS);
}

#[test]
fn existing_users_and_unreadable_completion_hide_but_can_reopen() {
    for (completion, setup) in [
        (Completion::Pending, SetupHost::Existing),
        (Completion::Completed, SetupHost::Waiting),
    ] {
        let mut model = Model::default();
        model.decide(completion, setup, Layout::Wide).unwrap();
        assert_eq!(model.state(), &State::Hidden);
        model
            .decide(Completion::Pending, SetupHost::Empty(host()), Layout::Wide)
            .unwrap();
        assert_eq!(model.state(), &State::Hidden);
        model.reopen(host(), Layout::Compact).unwrap();
        assert_eq!(open(&model).steps(), COMPACT_STEPS);
        model.go_to(Step::Phone).unwrap();
        assert_eq!(open(&model).step(), Step::Appearance);
    }
}

#[test]
fn stale_work_cannot_advance_after_back_skip_or_reopen() {
    let mut model = Model::default();
    model.reopen(host(), Layout::Wide).unwrap();
    model.go_to(Step::Projects).unwrap();
    let old = open(&model).visit();
    model.back().unwrap();
    assert_eq!(model.next(old).unwrap(), Effect::None);
    assert_eq!(open(&model).step(), Step::Providers);
    assert_eq!(model.complete(), Effect::Complete);
    model.reopen(host(), Layout::Wide).unwrap();
    assert_eq!(model.next(old).unwrap(), Effect::None);
    assert_eq!(open(&model).step(), Step::Appearance);
}

#[test]
fn exact_step_order_and_completion_effect() {
    let mut model = Model::default();
    model.reopen(host(), Layout::Wide).unwrap();
    let initial = open(&model).visit();
    model.back().unwrap();
    assert_eq!(open(&model).visit(), initial);
    for step in WIDE_STEPS {
        assert_eq!(open(&model).step(), *step);
        let visit = open(&model).visit();
        let effect = model.next(visit).unwrap();
        assert_eq!(
            effect,
            if *step == Step::Phone {
                Effect::Complete
            } else {
                Effect::None
            }
        );
    }
    assert_eq!(model.state(), &State::Hidden);
}

#[test]
fn exhausted_visits_refuse_without_mutating_open_state() {
    let mut model = Model::default();
    model.reopen(host(), Layout::Wide).unwrap();
    model.visits = u64::MAX;
    let before = model.state().clone();
    assert_eq!(model.go_to(Step::Projects), Err(VisitExhausted));
    assert_eq!(model.state(), &before);
}
