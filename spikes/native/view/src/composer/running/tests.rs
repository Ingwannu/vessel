use vessel_proto::{ChoiceValue, HarnessKind, OptionKind, OptionValueId};

use super::*;
use crate::localization::{ENGLISH, Locale};

fn option(id: &str, category: OptionCategory) -> SessionOption {
    SessionOption {
        id: OptionId::new(id),
        name: id.to_owned(),
        description: None,
        category,
        kind: OptionKind::Toggle { on: false },
    }
}

fn select(id: &str, value: &str) -> OptionChoice {
    OptionChoice {
        id: OptionId::new(id),
        value: ChoiceValue::Select {
            value: OptionValueId::new(value),
        },
    }
}

fn change(machine: &str, session: SessionId, choice: OptionChoice) -> Change {
    Change {
        machine: machine.to_owned(),
        session,
        agent: HarnessRef::Known(HarnessKind::ClaudeCode),
        choice,
        keep: KeepAt::Sending,
    }
}

#[test]
fn the_model_is_kept_once_taken_and_every_other_option_as_sent() {
    let reported = [
        option("model", OptionCategory::Model),
        option("effort", OptionCategory::ThoughtLevel),
        option("mode", OptionCategory::Mode),
        option("fast", OptionCategory::ModelConfig),
        option("brave", OptionCategory::Other),
    ];
    for (id, keep) in [
        ("model", KeepAt::Taken),
        ("effort", KeepAt::Sending),
        ("mode", KeepAt::Sending),
        ("fast", KeepAt::Sending),
        ("brave", KeepAt::Sending),
        ("gone", KeepAt::Sending),
    ] {
        assert_eq!(keep_at(&reported, &OptionId::new(id)), keep, "{id}");
    }
}

#[test]
fn a_kept_choice_of_the_same_option_is_replaced_where_it_stood() {
    let kept = [
        select("model", "sonnet"),
        select("effort", "low"),
        select("mode", "default"),
    ];
    assert_eq!(
        kept_with(Some(&kept), &select("effort", "high")),
        [
            select("model", "sonnet"),
            select("effort", "high"),
            select("mode", "default"),
        ]
    );
}

#[test]
fn a_new_option_is_added_after_the_kept_ones() {
    let kept = [select("model", "sonnet")];
    assert_eq!(
        kept_with(Some(&kept), &select("mode", "plan")),
        [select("model", "sonnet"), select("mode", "plan")]
    );
    assert_eq!(
        kept_with(None, &select("mode", "plan")),
        [select("mode", "plan")]
    );
}

#[test]
fn answers_settle_the_oldest_change_of_their_own_session() {
    let (one, two) = (SessionId::random(), SessionId::random());
    let mut flight = InFlight::default();
    flight.sent(change("laptop", one, select("model", "a")));
    flight.sent(change("laptop", two, select("model", "b")));
    flight.sent(change("laptop", one, select("effort", "c")));
    flight.sent(change("server", one, select("mode", "d")));

    assert_eq!(
        flight.answered("laptop", one).map(|change| change.choice),
        Some(select("model", "a"))
    );
    assert_eq!(
        flight.answered("laptop", one).map(|change| change.choice),
        Some(select("effort", "c"))
    );
    assert_eq!(flight.answered("laptop", one), None);
    assert_eq!(
        flight.len(),
        2,
        "another session's and another machine's wait"
    );
    assert_eq!(
        flight.answered("server", one).map(|change| change.choice),
        Some(select("mode", "d"))
    );
    assert_eq!(
        flight.answered("laptop", two).map(|change| change.choice),
        Some(select("model", "b"))
    );
    assert!(flight.is_empty());
}

#[test]
fn past_its_bound_the_oldest_change_is_forgotten() {
    let session = SessionId::random();
    let mut flight = InFlight::default();
    for index in 0..=InFlight::MAX {
        flight.sent(change(
            "laptop",
            session,
            select("model", &index.to_string()),
        ));
    }
    assert_eq!(flight.len(), InFlight::MAX);
    assert_eq!(
        flight
            .answered("laptop", session)
            .map(|change| change.choice),
        Some(select("model", "1")),
        "the first was forgotten"
    );
}

#[test]
fn every_code_has_its_refusal() {
    for (code, refusal) in [
        (None, Refusal::Other),
        (Some(ErrorCode::Conflict), Refusal::Busy),
        (Some(ErrorCode::InvalidRequest), Refusal::NotOffered),
        (Some(ErrorCode::Forbidden), Refusal::ReadOnly),
        (Some(ErrorCode::NotFound), Refusal::Gone),
        (Some(ErrorCode::Unsupported), Refusal::Unsupported),
        (Some(ErrorCode::Unavailable), Refusal::Unavailable),
        (Some(ErrorCode::Unauthorized), Refusal::Other),
        (Some(ErrorCode::ProtocolMismatch), Refusal::Other),
        (Some(ErrorCode::HarnessNotInstalled), Refusal::Other),
        (Some(ErrorCode::SpawnFailed), Refusal::Other),
        (Some(ErrorCode::Internal), Refusal::Other),
    ] {
        assert_eq!(Refusal::of(code), refusal, "{code:?}");
    }
}

#[test]
fn every_refusal_is_a_whole_sentence_in_both_languages() {
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    for (refusal, english, korean) in [
        (
            Refusal::Busy,
            "The agent can't make this change now.",
            "에이전트가 지금은 이 변경을 할 수 없습니다.",
        ),
        (
            Refusal::NotOffered,
            "The agent no longer offers this choice. Pick another one.",
            "에이전트가 더 이상 이 선택지를 제공하지 않습니다. 다른 것을 고르세요.",
        ),
        (
            Refusal::ReadOnly,
            "This device can only view this machine, so it can't change a session.",
            "이 기기는 이 머신을 보기만 할 수 있어 세션을 바꿀 수 없습니다.",
        ),
        (
            Refusal::Gone,
            "This session is no longer running on its machine.",
            "이 세션은 머신에서 더 이상 실행되고 있지 않습니다.",
        ),
        (
            Refusal::Unsupported,
            "This agent can't change its options while the session runs.",
            "이 에이전트는 세션이 실행되는 동안 옵션을 바꿀 수 없습니다.",
        ),
        (
            Refusal::Unavailable,
            "The change couldn't be sent now. Try again in a moment.",
            "지금은 변경을 보낼 수 없습니다. 잠시 후 다시 시도하세요.",
        ),
        (
            Refusal::Other,
            "The change didn't go through. Try again.",
            "변경이 적용되지 않았습니다. 다시 시도하세요.",
        ),
    ] {
        assert_eq!(refusal.words(en).unwrap(), english, "{refusal:?}");
        assert_eq!(refusal.words(ko).unwrap(), korean, "{refusal:?}");
    }
}
