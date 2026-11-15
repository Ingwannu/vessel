//! In-memory port contract; no fake implementation is callable in a product build.
use super::*;
use vessel_app_view::devices::MachineKey;
use vessel_app_view::side_question::State;

struct Fake {
    calls: Vec<Ask>,
}

impl Fake {
    fn send(&mut self, ask: Ask) -> Answer {
        self.calls.push(ask.clone());
        ask.refused(Problem::Offline)
    }
}

fn target() -> Target {
    Target {
        machine: MachineKey::from_book("fake-node"),
        parent: SessionId::random(),
    }
}

#[test]
fn fake_port_settles_the_original_scope_and_never_changes_the_main_state() {
    let main = State::default();
    let mut side = State::default();
    side.select(target());
    let pending = side.begin(Operation::Open).unwrap();
    let mut fake = Fake { calls: Vec::new() };
    let answer = fake.send(Ask::new(pending.clone(), Action::Open).unwrap());
    assert_eq!(answer.pending, pending);
    let ResultKind::Opened(result) = answer.result else {
        panic!("open result")
    };
    assert!(side.opened(&answer.pending, result.map(|info| info.id)));
    assert_eq!(side.problem(), Some(Problem::Offline));
    assert_eq!(main.target(), None);
    assert_eq!(main.pending(), None);
    assert_eq!(fake.calls.len(), 1);
}

#[test]
fn invalid_parent_prompt_and_wrong_operation_are_refused_before_fake_work() {
    let mut side = State::default();
    let target = target();
    side.select(target.clone());
    let pending = side.begin(Operation::Open).unwrap();
    let question = Question::parse("why?").unwrap();
    assert!(
        Ask::new(
            pending.clone(),
            Action::Prompt {
                side: target.parent,
                question
            }
        )
        .is_err()
    );
    assert!(Ask::new(pending, Action::Reset(SessionId::random())).is_err());
}

#[test]
fn correct_prompt_kind_still_cannot_address_the_parent() {
    let target = target();
    let mut state = State::default();
    state.select(target.clone());
    let opened = state.begin(Operation::Open).unwrap();
    assert!(state.opened(&opened, Ok(SessionId::random())));
    let prompt = state.begin(Operation::Prompt).unwrap();
    let question = Question::parse("why?").unwrap();
    assert!(
        Ask::new(
            prompt.clone(),
            Action::Prompt {
                side: target.parent,
                question
            }
        )
        .is_err()
    );
    assert!(Ask::new(prompt, Action::Open).is_err());
}
