//! The board's words beyond its lanes and phrases (those are `app.rs`'s):
//! its scope, its empty states, a card's origin and actions, and the
//! archive; in English and Korean. The English is the page's
//! (`features/board/*`). See `text/mod.rs` for the convention.

use vessel_proto::ErrorCode;

use super::Lang;

impl Lang {
    /// The board's words in this language.
    #[must_use]
    pub const fn board(self) -> &'static BoardWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's sentences.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the sentence it holds")]
pub struct BoardWords {
    pub lang: Lang,
    // The scope and the empty states.
    pub all_machines: &'static str,
    pub no_sessions: &'static str,
    pub empty_body: &'static str,
    pub new_session: &'static str,
    // A card.
    pub agent_of: &'static str,
    pub agent_of_after: &'static str,
    pub kill: &'static str,
    pub cancel: &'static str,
    pub archive_session: &'static str,
    pub restore_session: &'static str,
    pub unarchive: &'static str,
    pub unarchive_hint: &'static str,
    pub delete_permanently: &'static str,
    // The archive.
    pub archive: &'static str,
    pub archive_search: &'static str,
    pub no_archived: &'static str,
    pub no_results: &'static str,
}

impl BoardWords {
    /// The empty board of one machine.
    #[must_use]
    pub fn no_sessions_on(&self, machine: &str) -> String {
        match self.lang {
            Lang::En => format!("No sessions on {machine} yet"),
            Lang::Ko => format!("{machine} 머신에는 아직 세션이 없습니다"),
        }
    }

    /// A card's origin line for a session a paired device started.
    #[must_use]
    pub fn device(&self, id: &str) -> String {
        match self.lang {
            Lang::En => format!("device {id}"),
            Lang::Ko => format!("기기 {id}"),
        }
    }

    /// After "agent of {id}": the parent's machine, when it is another.
    #[must_use]
    pub fn elsewhere(&self, machine: &str) -> String {
        match self.lang {
            Lang::En => format!(" on {machine}"),
            Lang::Ko => format!(" ({machine})"),
        }
    }

    /// The kill confirm's title.
    #[must_use]
    pub fn kill_title(&self, id: &str) -> String {
        match self.lang {
            Lang::En => format!("Kill {id}?"),
            Lang::Ko => format!("{id} 세션을 종료할까요?"),
        }
    }

    /// The kill confirm's body.
    #[must_use]
    pub fn kill_body(&self, machine: &str) -> String {
        match self.lang {
            Lang::En => format!(
                "The agent on {machine} and everything it started stop now. The session's log stays."
            ),
            Lang::Ko => format!(
                "{machine} 머신의 에이전트와 그 에이전트가 시작한 모든 것이 지금 멈춥니다. 세션 기록은 남습니다."
            ),
        }
    }

    /// The toast after a kill.
    #[must_use]
    pub fn killed(&self, id: &str) -> String {
        match self.lang {
            Lang::En => format!("Killed {id}"),
            Lang::Ko => format!("{id} 세션을 종료했습니다"),
        }
    }

    /// Why a restore could not start a session again (the page's
    /// `againProblem`): the node's code in plain words, else its own
    /// (a conflict and a failed spawn say which).
    #[must_use]
    pub fn again_refused(&self, code: Option<ErrorCode>, message: &str) -> String {
        let (en, ko) = if code == Some(ErrorCode::Unsupported) {
            (
                "This agent cannot pick up where it left off.",
                "이 에이전트는 멈춘 곳에서 이어 가지 못합니다.",
            )
        } else if code == Some(ErrorCode::HarnessNotInstalled) {
            (
                "The agent is no longer installed on this machine.",
                "이 머신에 에이전트가 더 이상 설치되어 있지 않습니다.",
            )
        } else if code == Some(ErrorCode::Unavailable) {
            (
                "The machine is stopping; try again once it is back.",
                "머신이 멈추는 중입니다. 다시 돌아오면 시도하세요.",
            )
        } else if code == Some(ErrorCode::InvalidRequest) {
            return match self.lang {
                Lang::En => format!("Its profile is gone: {message}"),
                Lang::Ko => format!("프로필이 없어졌습니다: {message}"),
            };
        } else {
            return message.to_owned();
        };
        match self.lang {
            Lang::En => en.to_owned(),
            Lang::Ko => ko.to_owned(),
        }
    }

    /// The archive's "Show N more".
    #[must_use]
    pub fn show_more(&self, count: usize) -> String {
        match self.lang {
            Lang::En => format!("Show {count} more"),
            Lang::Ko => format!("{count}개 더 보기"),
        }
    }
}

const EN: BoardWords = BoardWords {
    lang: Lang::En,
    all_machines: "All machines",
    no_sessions: "No sessions yet",
    empty_body: "Start one and it appears here, in the lane of what it is doing.",
    new_session: "New session",
    agent_of: "agent of ",
    agent_of_after: "",
    kill: "Kill",
    cancel: "Cancel",
    archive_session: "Archive session",
    restore_session: "Restore session",
    unarchive: "Unarchive",
    unarchive_hint: "Unarchive: this agent cannot pick up where it left off",
    delete_permanently: "Delete permanently",
    archive: "Archive",
    archive_search: "Search archived sessions",
    no_archived: "No archived sessions",
    no_results: "No results.",
};

const KO: BoardWords = BoardWords {
    lang: Lang::Ko,
    all_machines: "모든 머신",
    no_sessions: "아직 세션이 없습니다",
    empty_body: "세션을 시작하면 지금 하는 일에 맞는 레인에 나타납니다.",
    new_session: "새 세션",
    agent_of: "",
    agent_of_after: "의 에이전트",
    kill: "종료",
    cancel: "취소",
    archive_session: "세션 보관",
    restore_session: "세션 복원",
    unarchive: "보관 해제",
    unarchive_hint: "보관 해제: 이 에이전트는 멈춘 곳에서 이어 가지 못합니다",
    delete_permanently: "영구 삭제",
    archive: "보관함",
    archive_search: "보관된 세션 검색",
    no_archived: "보관된 세션이 없습니다",
    no_results: "결과가 없습니다.",
};
