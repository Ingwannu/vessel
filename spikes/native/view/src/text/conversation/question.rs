//! The words of a question's card: Paseo's (`message.question.*`, its
//! English and Korean), and why a value does not fit (the page's English,
//! put in Korean here).

use crate::Lang;
use crate::timeline::{Problem, Wrong};

use super::ConversationWords;

impl ConversationWords {
    /// The timeline's line for a question while its card waits over the
    /// composer.
    #[must_use]
    pub const fn question_waiting(&self) -> &'static str {
        self.either("Waiting on an answer", "답을 기다리는 중")
    }

    /// The question card's button on its last step.
    #[must_use]
    pub const fn question_submit(&self) -> &'static str {
        self.either("Submit", "제출")
    }

    /// The question card's button on any other step.
    #[must_use]
    pub const fn question_next(&self) -> &'static str {
        self.either("Next", "다음")
    }

    /// The question card's button that declines the question.
    #[must_use]
    pub const fn question_dismiss(&self) -> &'static str {
        self.either("Dismiss", "닫기")
    }

    /// The question card's text box, empty, on a free-text step.
    #[must_use]
    pub const fn question_answer_placeholder(&self) -> &'static str {
        self.either("Type your answer...", "답변을 입력하세요...")
    }

    /// The question card's "Other" box, empty, under a choice's options.
    #[must_use]
    pub const fn question_other_placeholder(&self) -> &'static str {
        self.either("Other...", "기타...")
    }

    /// A yes-or-no field's two option rows on the question card, and its
    /// answer in the timeline's settled question.
    #[must_use]
    pub const fn question_yes_no(&self) -> (&'static str, &'static str) {
        (self.either("Yes", "예"), self.either("No", "아니요"))
    }

    /// Under the question card when the node refused the answer; `why`
    /// is the node's own message.
    #[must_use]
    pub fn question_refused(&self, why: &str) -> String {
        match self.lang {
            Lang::En => format!("Could not send your response. Try again. ({why})"),
            Lang::Ko => format!("응답을 보내지 못했습니다. 다시 시도해 주세요. ({why})"),
        }
    }

    /// Under the question card: why its step cannot be left yet.
    #[must_use]
    pub fn question_problem(&self, problem: &Problem) -> String {
        match problem {
            Problem::Wrong(wrong) => self.question_wrong(wrong),
            Problem::ChooseAnAnswer => self
                .either("Choose an answer.", "답을 고르세요.")
                .to_owned(),
            Problem::Required => self
                .either("This field is required.", "꼭 채워야 하는 칸입니다.")
                .to_owned(),
        }
    }

    /// Under the question card: why a value does not fit its field.
    #[must_use]
    pub fn question_wrong(&self, wrong: &Wrong) -> String {
        match (self.lang, wrong) {
            (Lang::En, Wrong::Unanswerable) => "This can't be answered here".to_owned(),
            (Lang::Ko, Wrong::Unanswerable) => "여기서는 답할 수 없습니다".to_owned(),
            (Lang::En, Wrong::TooLong) => "Too long".to_owned(),
            (Lang::Ko, Wrong::TooLong) => "너무 깁니다".to_owned(),
            (Lang::En, Wrong::AtLeastChars(n)) => format!("At least {n} characters"),
            (Lang::Ko, Wrong::AtLeastChars(n)) => format!("{n}자 이상이어야 합니다"),
            (Lang::En, Wrong::AtMostChars(n)) => format!("At most {n} characters"),
            (Lang::Ko, Wrong::AtMostChars(n)) => format!("{n}자 이하여야 합니다"),
            (Lang::En, Wrong::NotANumber) => "Not a number".to_owned(),
            (Lang::Ko, Wrong::NotANumber) => "숫자가 아닙니다".to_owned(),
            (Lang::En, Wrong::WholeNumber) => "A whole number".to_owned(),
            (Lang::Ko, Wrong::WholeNumber) => "정수여야 합니다".to_owned(),
            (Lang::En, Wrong::AtLeast(least)) => format!("At least {least}"),
            (Lang::Ko, Wrong::AtLeast(least)) => format!("{least} 이상이어야 합니다"),
            (Lang::En, Wrong::AtMost(most)) => format!("At most {most}"),
            (Lang::Ko, Wrong::AtMost(most)) => format!("{most} 이하여야 합니다"),
            (Lang::En, Wrong::ChooseAtLeast(n)) => format!("Choose at least {n}"),
            (Lang::Ko, Wrong::ChooseAtLeast(n)) => format!("{n}개 이상 고르세요"),
            (Lang::En, Wrong::ChooseAtMost(n)) => format!("Choose at most {n}"),
            (Lang::Ko, Wrong::ChooseAtMost(n)) => format!("{n}개 이하로 고르세요"),
        }
    }
}
