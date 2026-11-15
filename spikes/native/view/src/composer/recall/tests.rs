//! Paseo's cases (`composer/recall.test.ts`), driven as its composer drives
//! recall: each result becomes the composer's text, the caret at its end.

use super::*;

const SENT: [&str; 2] = ["add tests", "fix the auth bug"];

struct Composer {
    recall: Recall,
    text: String,
    sent: Vec<&'static str>,
}

impl Composer {
    fn new(sent: &[&'static str]) -> Self {
        Self {
            recall: Recall::default(),
            text: String::new(),
            sent: sent.to_vec(),
        }
    }

    fn press_at(&mut self, arrow: Arrow, caret: Option<usize>) -> Option<String> {
        let caret = caret.unwrap_or(self.text.len());
        let sent = self.sent.clone();
        let result = self.recall.press(
            arrow,
            At {
                text: &self.text,
                selection: (caret, caret),
            },
            || sent,
        );
        if let Some(text) = &result {
            self.text.clone_from(text);
        }
        result
    }

    fn press(&mut self, arrow: Arrow) -> Option<String> {
        self.press_at(arrow, None)
    }
}

#[test]
fn walks_back_through_sent_messages_and_forward_to_an_empty_composer() {
    let mut composer = Composer::new(&SENT);
    assert_eq!(composer.press(Arrow::Up).as_deref(), Some("add tests"));
    assert_eq!(
        composer.press(Arrow::Up).as_deref(),
        Some("fix the auth bug")
    );
    assert_eq!(composer.press(Arrow::Down).as_deref(), Some("add tests"));
    assert_eq!(composer.press(Arrow::Down).as_deref(), Some(""));
    assert!(!composer.recall.showing(&composer.text));
}

#[test]
fn leaves_the_caret_to_the_input_past_the_oldest_message() {
    let mut composer = Composer::new(&SENT);
    let _newest = composer.press(Arrow::Up);
    let _oldest = composer.press(Arrow::Up);
    assert_eq!(composer.press(Arrow::Up), None);
    assert_eq!(composer.text, "fix the auth bug");
}

#[test]
fn only_starts_from_an_empty_composer() {
    let mut composer = Composer::new(&SENT);
    composer.text = "draft".to_owned();
    assert_eq!(composer.press(Arrow::Up), None);
    assert_eq!(composer.press(Arrow::Down), None);
}

#[test]
fn ignores_down_from_an_empty_composer_and_up_with_no_history() {
    assert_eq!(Composer::new(&SENT).press(Arrow::Down), None);
    assert_eq!(Composer::new(&[]).press(Arrow::Up), None);
}

#[test]
fn hands_the_arrows_back_to_the_caret_once_the_recalled_message_is_edited() {
    let mut composer = Composer::new(&SENT);
    let _recalled = composer.press(Arrow::Up);
    composer.text = "add tests!".to_owned();
    assert!(!composer.recall.showing(&composer.text));
    assert_eq!(composer.press(Arrow::Up), None);
    assert_eq!(composer.press(Arrow::Down), None);
}

#[test]
fn moves_between_lines_of_a_multi_line_message_before_changing_message() {
    let mut composer = Composer::new(&["first line\nsecond line", "older"]);
    assert_eq!(
        composer.press(Arrow::Up).as_deref(),
        Some("first line\nsecond line")
    );
    assert_eq!(
        composer.press_at(Arrow::Up, Some("first line\nsec".len())),
        None
    );
    assert_eq!(
        composer.press_at(Arrow::Up, Some(3)).as_deref(),
        Some("older")
    );
    assert_eq!(
        composer.press(Arrow::Down).as_deref(),
        Some("first line\nsecond line")
    );
    assert_eq!(composer.press_at(Arrow::Down, Some(3)), None);
    assert_eq!(composer.press(Arrow::Down).as_deref(), Some(""));
}

#[test]
fn ignores_a_selection() {
    let mut composer = Composer::new(&SENT);
    let _recalled = composer.press(Arrow::Up);
    let sent = SENT.to_vec();
    assert_eq!(
        composer.recall.press(
            Arrow::Up,
            At {
                text: "add tests",
                selection: (0, 3),
            },
            || sent,
        ),
        None
    );
}

#[test]
fn sent_messages_are_newest_first_without_empty_ones_or_repeats_in_a_row() {
    let oldest_first = ["fix it", "  ", "fix it", "add tests", "add tests", "fix it"];
    assert_eq!(
        sent_newest_first(oldest_first.into_iter()),
        ["fix it", "add tests", "fix it"]
    );
}
