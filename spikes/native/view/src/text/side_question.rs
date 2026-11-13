//! Side panel words after GOAT's `SideChatSurface`, with explicit partial
//! context and native-export boundaries (ADR 0045). No wire error is shown.

mod failure;

use super::Lang;
use crate::side_question::{ParentStatus, SubmitProblem};

impl Lang {
    /// The side-question panel's words.
    #[must_use]
    pub const fn side_question(self) -> &'static SideWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One complete language for the side panel.
#[derive(Debug)]
pub struct SideWords {
    /// Language for enum-dependent sentences.
    pub lang: Lang,
    /// Short panel tab label.
    pub title: &'static str,
    /// Reset icon tooltip; discards this side conversation.
    pub start_over: &'static str,
    /// Navigation icon tooltip; opens the existing side session in full.
    pub open_full: &'static str,
    /// Neutral loading line while opening a side session.
    pub starting: &'static str,
    /// Retry button following a failed open.
    pub retry: &'static str,
    /// Empty question-field placeholder.
    pub placeholder: &'static str,
    /// Send button's accessible label.
    pub send: &'static str,
    /// Stop the side's turn, leaving the parent running.
    pub stop: &'static str,
    /// Explicitly resume an ended side; never automatic on panel open.
    pub resume: &'static str,
    /// Context-help text: no full history or unavailable goal is promised.
    pub partial_context: &'static str,
    /// Native export help; the provider may retain the generated context.
    pub native_export: &'static str,
}

impl SideWords {
    /// Notice beside a retained composer draft; no node or parser Display.
    #[must_use]
    pub const fn submit_problem(&self, problem: &SubmitProblem) -> &'static str {
        match (self.lang, problem) {
            (Lang::En, SubmitProblem::Loading) => {
                "The conversation is still loading. Try again once it is ready."
            }
            (Lang::Ko, SubmitProblem::Loading) => {
                "대화를 불러오는 중입니다. 준비가 끝나면 다시 시도하세요."
            }
            (Lang::En, SubmitProblem::Busy) => {
                "The side chat is still working. Wait for it to finish, or stop it before asking again."
            }
            (Lang::Ko, SubmitProblem::Busy) => {
                "곁대화에서 작업이 진행 중입니다. 작업이 끝날 때까지 기다리거나 중단한 뒤 다시 질문하세요."
            }
            (Lang::En, SubmitProblem::Resume) => {
                "Resume this side chat before asking another question."
            }
            (Lang::Ko, SubmitProblem::Resume) => "곁대화를 재개한 뒤 새 질문을 보내세요.",
            (Lang::En, SubmitProblem::Nested) => {
                "Open the original conversation to ask a new side question."
            }
            (Lang::Ko, SubmitProblem::Nested) => "원래 대화를 열어 새 곁질문을 보내세요.",
            (Lang::En, SubmitProblem::Terminal) => "Open a conversation to ask a side question.",
            (Lang::Ko, SubmitProblem::Terminal) => "대화를 열어 곁질문을 보내세요.",
            (Lang::En, SubmitProblem::NativeCommand) => {
                "Run this command in an ordinary conversation."
            }
            (Lang::Ko, SubmitProblem::NativeCommand) => "일반 대화에서 이 명령을 실행하세요.",
            (Lang::En, SubmitProblem::Question(_)) => {
                "Enter a short question using ordinary text, tabs or line breaks."
            }
            (Lang::Ko, SubmitProblem::Question(_)) => {
                "일반 글자, 탭, 줄바꿈으로 짧은 질문을 입력하세요."
            }
        }
    }

    /// Parent status beside its title; every label must stay short.
    #[must_use]
    pub const fn parent_status(&self, status: ParentStatus) -> &'static str {
        match (self.lang, status) {
            (Lang::En, ParentStatus::NeedsApproval) => "Needs approval",
            (Lang::Ko, ParentStatus::NeedsApproval) => "승인 필요",
            (Lang::En, ParentStatus::NeedsInput) => "Needs input",
            (Lang::Ko, ParentStatus::NeedsInput) => "답변 필요",
            (Lang::En, ParentStatus::Starting) => "Starting",
            (Lang::Ko, ParentStatus::Starting) => "시작 중",
            (Lang::En, ParentStatus::Working) => "Working",
            (Lang::Ko, ParentStatus::Working) => "작업 중",
            (Lang::En, ParentStatus::Interrupted) => "Interrupted",
            (Lang::Ko, ParentStatus::Interrupted) => "중단됨",
            (Lang::En, ParentStatus::Failed) => "Failed",
            (Lang::Ko, ParentStatus::Failed) => "실패함",
            (Lang::En, ParentStatus::Idle) => "Idle",
            (Lang::Ko, ParentStatus::Idle) => "대기 중",
        }
    }
}

const EN: SideWords = SideWords {
    lang: Lang::En,
    title: "Side chat",
    start_over: "Start a new side chat",
    open_full: "Open as full thread",
    starting: "Starting side chat…",
    retry: "Try again",
    placeholder: "Ask a side question…",
    send: "Send side question",
    stop: "Stop side chat",
    resume: "Resume side chat",
    partial_context: "Sees the original session's current status, recorded plan and recent messages. Recent conversation is partial; goal information is unavailable. Nothing said here is added to the original session.",
    native_export: "A native export includes the provider's original conversation files. Those files may contain context from the original session sent with side questions.",
};

const KO: SideWords = SideWords {
    lang: Lang::Ko,
    title: "곁대화",
    start_over: "새 곁대화 시작",
    open_full: "전체 대화로 열기",
    starting: "곁대화를 시작하는 중…",
    retry: "다시 시도",
    placeholder: "곁질문을 입력하세요…",
    send: "곁질문 보내기",
    stop: "곁대화 중단",
    resume: "곁대화 재개",
    partial_context: "원래 세션의 현재 상태, 기록된 계획, 최근 메시지를 참고합니다. 최근 대화는 일부만 포함하며 목표 정보는 제공되지 않습니다. 여기서 나눈 대화는 원래 세션에 추가되지 않습니다.",
    native_export: "원본 내보내기에는 제공자의 대화 파일이 포함됩니다. 이 파일에는 곁질문과 함께 보낸 원래 세션의 참고 문맥이 남아 있을 수 있습니다.",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_status_has_the_reference_label_and_a_korean_label() {
        let cases = [
            (ParentStatus::NeedsApproval, "Needs approval", "승인 필요"),
            (ParentStatus::NeedsInput, "Needs input", "답변 필요"),
            (ParentStatus::Starting, "Starting", "시작 중"),
            (ParentStatus::Working, "Working", "작업 중"),
            (ParentStatus::Interrupted, "Interrupted", "중단됨"),
            (ParentStatus::Failed, "Failed", "실패함"),
            (ParentStatus::Idle, "Idle", "대기 중"),
        ];
        for (status, en, ko) in cases {
            assert_eq!(Lang::En.side_question().parent_status(status), en);
            assert_eq!(Lang::Ko.side_question().parent_status(status), ko);
        }
    }

    #[test]
    fn partial_context_and_native_export_boundaries_are_whole_sentences() {
        assert_eq!(
            EN.partial_context,
            "Sees the original session's current status, recorded plan and recent messages. Recent conversation is partial; goal information is unavailable. Nothing said here is added to the original session."
        );
        assert_eq!(
            KO.partial_context,
            "원래 세션의 현재 상태, 기록된 계획, 최근 메시지를 참고합니다. 최근 대화는 일부만 포함하며 목표 정보는 제공되지 않습니다. 여기서 나눈 대화는 원래 세션에 추가되지 않습니다."
        );
        assert_eq!(
            EN.native_export,
            "A native export includes the provider's original conversation files. Those files may contain context from the original session sent with side questions."
        );
        assert_eq!(
            KO.native_export,
            "원본 내보내기에는 제공자의 대화 파일이 포함됩니다. 이 파일에는 곁질문과 함께 보낸 원래 세션의 참고 문맥이 남아 있을 수 있습니다."
        );
    }
    #[test]
    fn composer_refusals_are_complete_localized_sentences() {
        let cases = [
            (
                SubmitProblem::Loading,
                "The conversation is still loading. Try again once it is ready.",
                "대화를 불러오는 중입니다. 준비가 끝나면 다시 시도하세요.",
            ),
            (
                SubmitProblem::Busy,
                "The side chat is still working. Wait for it to finish, or stop it before asking again.",
                "곁대화에서 작업이 진행 중입니다. 작업이 끝날 때까지 기다리거나 중단한 뒤 다시 질문하세요.",
            ),
            (
                SubmitProblem::Resume,
                "Resume this side chat before asking another question.",
                "곁대화를 재개한 뒤 새 질문을 보내세요.",
            ),
            (
                SubmitProblem::Nested,
                "Open the original conversation to ask a new side question.",
                "원래 대화를 열어 새 곁질문을 보내세요.",
            ),
            (
                SubmitProblem::Terminal,
                "Open a conversation to ask a side question.",
                "대화를 열어 곁질문을 보내세요.",
            ),
            (
                SubmitProblem::NativeCommand,
                "Run this command in an ordinary conversation.",
                "일반 대화에서 이 명령을 실행하세요.",
            ),
            (
                SubmitProblem::Question(
                    "".parse::<vessel_proto::automation::PromptText>()
                        .unwrap_err(),
                ),
                "Enter a short question using ordinary text, tabs or line breaks.",
                "일반 글자, 탭, 줄바꿈으로 짧은 질문을 입력하세요.",
            ),
        ];
        for (problem, en, ko) in cases {
            assert_eq!(EN.submit_problem(&problem), en);
            assert_eq!(KO.submit_problem(&problem), ko);
        }
    }
}
