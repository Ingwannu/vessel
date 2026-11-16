//! Whole migrated values and exhaustive delivery mapping preserve both languages.
use crate::localization::{ENGLISH, Locale, Translator};
use crate::prefs::TurnDelivery;

#[test]
fn en_whole_general_values() {
    let words = Translator::get(ENGLISH).unwrap().general();
    assert_eq!(words.title(), "General");
    assert_eq!(words.sending(), "Sending");
    assert_eq!(words.send_shortcut(), "Send shortcut");
    assert_eq!(words.default_send(), "Default send");
    assert_eq!(words.queue(), "Queue");
    assert_eq!(words.steer(), "Steer");
    assert_eq!(words.interrupt(), "Interrupt");
    assert_eq!(words.kept(), "Kept");
    assert_eq!(words.preferences(), "Preferences");
    assert_eq!(words.this_launch(), "This launch only");
    assert_eq!(words.delivery(TurnDelivery::Queue), "Queue");
    assert_eq!(
        words.delivery_note(TurnDelivery::Queue),
        "A message sent while the agent works waits, and goes when its turn ends; while a permission request waits, it stops the turn instead."
    );
    assert_eq!(words.delivery(TurnDelivery::Steer), "Steer");
    assert_eq!(
        words.delivery_note(TurnDelivery::Steer),
        "A message sent while the agent works joins its turn, where the agent can take one; otherwise it stops the turn and starts the next."
    );
    assert_eq!(words.delivery(TurnDelivery::Interrupt), "Interrupt");
    assert_eq!(
        words.delivery_note(TurnDelivery::Interrupt),
        "A message sent while the agent works stops its turn and starts the next."
    );
}

#[test]
fn ko_whole_general_values() {
    let words = Translator::get(Locale::from_tag("ko").unwrap())
        .unwrap()
        .general();
    assert_eq!(words.title(), "일반");
    assert_eq!(words.sending(), "전송");
    assert_eq!(words.send_shortcut(), "전송 단축키");
    assert_eq!(words.default_send(), "기본 전송");
    assert_eq!(words.queue(), "대기열");
    assert_eq!(words.steer(), "지시");
    assert_eq!(words.interrupt(), "중단");
    assert_eq!(words.kept(), "보관");
    assert_eq!(words.preferences(), "환경설정");
    assert_eq!(words.this_launch(), "이번 실행에만");
    assert_eq!(words.delivery(TurnDelivery::Queue), "대기열");
    assert_eq!(
        words.delivery_note(TurnDelivery::Queue),
        "에이전트가 일하는 동안 보낸 메시지는 기다렸다가, 차례가 끝나면 갑니다. 권한 요청이 기다리는 동안에는 지금 차례를 멈추고 갑니다."
    );
    assert_eq!(words.delivery(TurnDelivery::Steer), "지시");
    assert_eq!(
        words.delivery_note(TurnDelivery::Steer),
        "에이전트가 일하는 동안 보낸 메시지는, 에이전트가 받을 수 있으면 지금 차례에 합류하고 아니면 지금 차례를 멈추고 다음 차례를 시작합니다."
    );
    assert_eq!(words.delivery(TurnDelivery::Interrupt), "중단");
    assert_eq!(
        words.delivery_note(TurnDelivery::Interrupt),
        "에이전트가 일하는 동안 보낸 메시지는 지금 차례를 멈추고 다음 차례를 시작합니다."
    );
}
