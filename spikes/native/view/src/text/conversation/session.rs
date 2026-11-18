//! The session's words around its timeline: find, the context ring, the
//! title, the Session actions menu and its confirms, the banners (the
//! page's `data/find.ts`, `data/context-meter.ts`, `data/titles.ts`,
//! `data/marks.ts`, `features/session/session-actions.tsx`,
//! `use-mark-actions.tsx`, `ended-banner.tsx`).

use std::fmt::Write as _;

use vessel_proto::workspace::{GitOperation, KeptReason, RemoveWorktreeOutcome};
use vessel_proto::{InvalidTitle, QueueHold, SessionTitle};

use super::numbers::meter_tokens;
use super::{ConversationWords, plural};
use crate::Lang;
use crate::session::MarkAction;
use crate::timeline::{Fullness, Meter};

impl ConversationWords {
    /// The find bar's count: "3/12" (agent-orchestrator's and GOAT's), or
    /// "No results".
    #[must_use]
    pub fn count_text(&self, count: usize, index: usize) -> String {
        if count == 0 {
            return self.either("No results", "결과 없음").to_owned();
        }
        format!("{}/{count}", index.saturating_add(1))
    }

    /// The count as a screen reader says it: "3 of 12".
    #[must_use]
    pub fn count_spoken(&self, count: usize, index: usize) -> String {
        if count == 0 {
            return self.either("No results", "결과 없음").to_owned();
        }
        let at = index.saturating_add(1);
        match self.lang {
            Lang::En => format!("{at} of {count}"),
            Lang::Ko => format!("{count}개 중 {at}번째"),
        }
    }

    /// What a step that went round an end says.
    #[must_use]
    pub const fn wrapped(&self, forward: bool) -> &'static str {
        if forward {
            self.either(
                "Wrapped to the first match.",
                "첫 번째 결과로 돌아갔습니다.",
            )
        } else {
            self.either("Wrapped to the last match.", "마지막 결과로 돌아갔습니다.")
        }
    }

    /// The queue's heading over the composer: how many prompts wait
    /// (GOAT's "Queued").
    #[must_use]
    pub fn queued(&self, count: usize) -> String {
        let count = u64::try_from(count).unwrap_or(u64::MAX);
        match self.lang {
            Lang::En => format!("Queued · {}", plural(count, "message", "messages")),
            Lang::Ko => format!("대기 중 · {count}개"),
        }
    }

    /// The queue's heading over the composer while it is held: how many
    /// prompts wait, and why the queue does not move on its own.
    #[must_use]
    pub fn queued_held(&self, count: usize, why: QueueHold) -> String {
        let count = u64::try_from(count).unwrap_or(u64::MAX);
        let messages = plural(count, "message", "messages");
        match (self.lang, why) {
            (Lang::En, QueueHold::Cancelled) => {
                format!("Queued · {messages} · Paused: the turn was stopped")
            }
            (Lang::En, QueueHold::Failed) => {
                format!("Queued · {messages} · Paused: the turn failed")
            }
            (Lang::En, QueueHold::Ended) => {
                format!("Queued · {messages} · Paused: the session ended")
            }
            (Lang::En, QueueHold::Asked) => format!("Queued · {messages} · Paused"),
            (Lang::Ko, QueueHold::Cancelled) => {
                format!("대기 중 · {count}개 · 멈춤: 턴을 중지했습니다")
            }
            (Lang::Ko, QueueHold::Failed) => {
                format!("대기 중 · {count}개 · 멈춤: 턴이 실패했습니다")
            }
            (Lang::Ko, QueueHold::Ended) => {
                format!("대기 중 · {count}개 · 멈춤: 세션이 끝났습니다")
            }
            (Lang::Ko, QueueHold::Asked) => format!("대기 중 · {count}개 · 멈춤"),
        }
    }

    /// Let a held queue move again.
    #[must_use]
    pub const fn queue_resume(&self) -> &'static str {
        self.either("Resume", "다시 진행")
    }

    /// Empty the queue.
    #[must_use]
    pub const fn queue_clear(&self) -> &'static str {
        self.either("Clear", "모두 비우기")
    }

    /// A queued message's buttons, as a screen reader names them.
    #[must_use]
    pub const fn queue_edit(&self) -> &'static str {
        self.either("Edit queued message", "대기 메시지 편집")
    }

    /// Send a queued message into the running turn.
    #[must_use]
    pub const fn queue_steer(&self) -> &'static str {
        self.either("Steer", "진행 중인 턴에 보내기")
    }

    /// Send a queued message now: interrupting the running turn when one
    /// runs, else starting one.
    #[must_use]
    pub const fn queue_send_now(&self, interrupts: bool) -> &'static str {
        if interrupts {
            self.either("Interrupt and send", "중단하고 보내기")
        } else {
            self.either("Send now", "지금 보내기")
        }
    }

    /// Take a message out of the queue.
    #[must_use]
    pub const fn queue_remove(&self) -> &'static str {
        self.either("Remove from queue", "대기열에서 빼기")
    }

    /// The find bar's count while the search has not reached the end:
    /// the first match, of a count not known yet (GOAT's "1/…").
    #[must_use]
    pub const fn count_pending(&self) -> &'static str {
        "1/…"
    }

    /// The find bar's previous, as a screen reader names it.
    #[must_use]
    pub const fn previous_match(&self) -> &'static str {
        self.either("Previous match", "이전 결과")
    }

    /// The find bar's next.
    #[must_use]
    pub const fn next_match(&self) -> &'static str {
        self.either("Next match", "다음 결과")
    }

    /// The find bar's close.
    #[must_use]
    pub const fn close_find(&self) -> &'static str {
        self.either("Close find", "찾기 닫기")
    }

    /// The find bar's placeholder and label.
    #[must_use]
    pub const fn find_in_session(&self) -> &'static str {
        self.either("Find in session", "세션에서 찾기")
    }

    /// The context ring's hover: its heading, then its lines.
    #[must_use]
    pub fn meter_lines(&self, meter: &Meter, cost: Option<(f64, &str)>) -> Vec<String> {
        let mut lines = vec![
            self.either("Context window", "컨텍스트 창").to_owned(),
            match self.lang {
                Lang::En => format!(
                    "{} / {} tokens ({}%)",
                    meter_tokens(meter.used),
                    meter_tokens(meter.size),
                    meter.percent
                ),
                Lang::Ko => format!(
                    "토큰 {} / {} ({}%)",
                    meter_tokens(meter.used),
                    meter_tokens(meter.size),
                    meter.percent
                ),
            },
        ];
        if let Some((amount, unit)) = cost {
            lines.push(format!(
                "{} {}",
                self.either("Session cost", "세션 비용"),
                cost_label(amount, unit)
            ));
        }
        match meter.fullness {
            Fullness::Room => {}
            Fullness::Low => lines.push(
                self.either(
                    "Room is running low. A long task may not fit.",
                    "남은 공간이 적습니다. 긴 작업은 들어가지 않을 수 있습니다.",
                )
                .to_owned(),
            ),
            Fullness::Full => lines.push(
                self.either(
                    "The next turn may not fit. Compacting or starting a new conversation will reclaim room.",
                    "다음 턴이 들어가지 않을 수 있습니다. 압축하거나 새 대화를 시작하면 공간이 생깁니다.",
                )
                .to_owned(),
            ),
        }
        lines
    }

    /// The title's field, while it is renamed in place (its placeholder).
    #[must_use]
    pub const fn title_field(&self) -> &'static str {
        self.either("Title", "제목")
    }

    /// What an untitled session with no folder is called.
    #[must_use]
    pub const fn untitled(&self) -> &'static str {
        self.either("Untitled session", "제목 없는 세션")
    }

    /// Why `text` cannot be a title, as the field says it.
    #[must_use]
    pub fn title_refused(&self, why: InvalidTitle) -> String {
        match why {
            InvalidTitle::Empty => self
                .either("A title needs some text.", "제목을 입력하세요.")
                .to_owned(),
            InvalidTitle::ControlCharacter => self
                .either("A title is one line of text.", "제목은 한 줄이어야 합니다.")
                .to_owned(),
            InvalidTitle::TooLong { .. } => match self.lang {
                Lang::En => format!("A title is at most {} characters.", SessionTitle::MAX_CHARS),
                Lang::Ko => format!("제목은 {}자까지입니다.", SessionTitle::MAX_CHARS),
            },
        }
    }

    /// A Session actions item.
    #[must_use]
    pub const fn mark_action(&self, action: MarkAction) -> &'static str {
        match action {
            MarkAction::Pin => self.either("Pin session", "세션 고정"),
            MarkAction::Unpin => self.either("Unpin session", "고정 해제"),
            MarkAction::MarkUnread => self.either("Mark as unread", "읽지 않음으로 표시"),
            MarkAction::MarkRead => self.either("Mark as read", "읽음으로 표시"),
            MarkAction::StartAgain => self.either("Start again", "다시 시작"),
            MarkAction::Archive => self.either("Archive session", "세션 보관"),
            MarkAction::Unarchive => self.either("Unarchive", "보관 해제"),
            MarkAction::Delete => self.either("Delete permanently", "영구 삭제"),
        }
    }

    /// What the archive's confirmation says it does: stop the agent first
    /// on `machine` when the session is live, end what runs beside it on
    /// a node whose protocol has terminals (11), and that it can be
    /// restored.
    #[must_use]
    pub fn archive_words(&self, live: bool, machine: &str, ends_beside: bool) -> String {
        let mut words = String::new();
        if live {
            let _infallible = match self.lang {
                Lang::En => write!(words, "Archiving stops the agent on {machine} first. "),
                Lang::Ko => write!(words, "보관하면 먼저 {machine}의 에이전트를 멈춥니다. "),
            };
        }
        if ends_beside {
            words.push_str(self.either(
                "The terminals and services running beside it end too; their output is kept. ",
                "함께 실행 중인 터미널과 서비스도 끝납니다. 출력은 남습니다. ",
            ));
        }
        words.push_str(self.either(
            "You can always restore it from the Archive section later.",
            "나중에 언제든 아카이브에서 복원할 수 있습니다.",
        ));
        words
    }

    /// Why a worktree was kept.
    #[must_use]
    pub fn kept_reason(&self, reason: &KeptReason) -> String {
        match (reason, self.lang) {
            (KeptReason::Uncommitted { files }, Lang::En) => format!(
                "{files} uncommitted {}",
                if *files == 1 { "file" } else { "files" }
            ),
            (KeptReason::Uncommitted { files }, Lang::Ko) => {
                format!("커밋하지 않은 파일 {files}개")
            }
            (KeptReason::UniqueCommits { commits }, Lang::En) => format!(
                "{commits} {} no other branch has",
                if *commits == 1 { "commit" } else { "commits" }
            ),
            (KeptReason::UniqueCommits { commits }, Lang::Ko) => {
                format!("다른 브랜치에 없는 커밋 {commits}개")
            }
            (KeptReason::Operation { operation }, Lang::En) => {
                format!("a {} in progress", git_operation(*operation))
            }
            (KeptReason::Operation { operation }, Lang::Ko) => {
                format!("진행 중인 {}", git_operation(*operation))
            }
            (KeptReason::Submodules, _) => self.either("submodules", "서브모듈").to_owned(),
            (KeptReason::FlaggedFiles, _) => self
                .either("files git was told to keep", "git이 보존하도록 지정된 파일")
                .to_owned(),
            (KeptReason::WorktreeRefs, _) => {
                self.either("refs of its own", "고유한 ref").to_owned()
            }
            (KeptReason::InUse, _) => self
                .either(
                    "another live session works in it",
                    "다른 실행 중인 세션이 사용 중",
                )
                .to_owned(),
            (KeptReason::InspectionFailed, _) => self
                .either(
                    "git could not say what it holds",
                    "git이 내용을 확인하지 못함",
                )
                .to_owned(),
            (KeptReason::Unknown, _) => self
                .either("a reason this page does not know", "이 앱이 모르는 이유")
                .to_owned(),
        }
    }

    /// What a delete's toast says became of the session's worktree.
    #[must_use]
    pub fn deleted_worktree(&self, outcome: &RemoveWorktreeOutcome) -> String {
        match outcome {
            RemoveWorktreeOutcome::Removed { .. } => self
                .either("Its worktree was removed.", "워크트리를 지웠습니다.")
                .to_owned(),
            RemoveWorktreeOutcome::Kept { reasons, .. } => {
                let reasons: Vec<String> = reasons
                    .iter()
                    .map(|reason| self.kept_reason(reason))
                    .collect();
                match self.lang {
                    Lang::En => format!("Its worktree was kept: {}.", reasons.join(", ")),
                    Lang::Ko => format!("워크트리를 남겼습니다: {}.", reasons.join(", ")),
                }
            }
        }
    }

    /// The toast after a delete.
    #[must_use]
    pub const fn session_deleted(&self) -> &'static str {
        self.either("Session deleted", "세션을 삭제했습니다")
    }

    /// The ended banner's heading.
    #[must_use]
    pub const fn ended_heading(&self, archived: bool) -> &'static str {
        if archived {
            self.either("Session archived", "보관된 세션")
        } else {
            self.either("Session ended", "끝난 세션")
        }
    }

    /// The ended banner's line.
    #[must_use]
    pub const fn ended_line(&self, can_start_again: bool) -> &'static str {
        if can_start_again {
            self.either(
                "History is kept. Start the session again to continue.",
                "기록은 남아 있습니다. 이어 가려면 세션을 다시 시작하세요.",
            )
        } else {
            self.either(
                "History is kept. This agent cannot pick up where it left off.",
                "기록은 남아 있습니다. 이 에이전트는 멈춘 곳에서 이어 갈 수 없습니다.",
            )
        }
    }

    /// The Session actions menu's trigger.
    #[must_use]
    pub const fn session_actions(&self) -> &'static str {
        self.either("Session actions", "세션 작업")
    }

    /// The menu's row that copies the session's id: Paseo's agent id row
    /// (`workspace.tabs.menu.copyAgentId`; a Vessel session is its agent).
    #[must_use]
    pub const fn copy_agent_id(&self) -> &'static str {
        self.either("Copy agent id", "에이전트 ID 복사")
    }

    /// The menu's row that kills a live session, and the kill confirm's
    /// button.
    #[must_use]
    pub const fn kill(&self) -> &'static str {
        self.either("Kill", "강제 종료")
    }

    /// The kill confirm's title: "Kill 3f2a?".
    #[must_use]
    pub fn kill_title(&self, short_id: &str) -> String {
        match self.lang {
            Lang::En => format!("Kill {short_id}?"),
            Lang::Ko => format!("{short_id}을(를) 강제 종료할까요?"),
        }
    }

    /// What the kill confirm says.
    #[must_use]
    pub fn kill_words(&self, machine: &str) -> String {
        match self.lang {
            Lang::En => format!(
                "The agent on {machine} and everything it started stop now. The session's log stays."
            ),
            Lang::Ko => format!(
                "{machine}의 에이전트와 그것이 시작한 모든 것이 지금 멈춥니다. 세션 기록은 남습니다."
            ),
        }
    }

    /// The archive confirm's title.
    #[must_use]
    pub const fn archive_title(&self) -> &'static str {
        self.either(
            "Are you sure you want to archive this session?",
            "이 세션을 보관할까요?",
        )
    }

    /// The delete confirm's title.
    #[must_use]
    pub const fn delete_title(&self) -> &'static str {
        self.either(
            "Delete this session permanently?",
            "이 세션을 영구 삭제할까요?",
        )
    }

    /// What the delete confirm says.
    #[must_use]
    pub fn delete_words(&self, machine: &str) -> String {
        match self.lang {
            Lang::En => format!(
                "Its conversation and attachments are deleted for good. Its worktree is removed only if nothing in it would be lost; the agent's own history on {machine} is kept."
            ),
            Lang::Ko => format!(
                "대화와 첨부 파일이 영구히 삭제됩니다. 워크트리는 잃을 것이 없을 때만 지우고, {machine}에 있는 에이전트 자체 기록은 남습니다."
            ),
        }
    }

    /// A confirm's buttons: dismiss, accept, delete.
    #[must_use]
    pub const fn no(&self) -> &'static str {
        self.either("No", "아니요")
    }

    /// Accept a confirm.
    #[must_use]
    pub const fn confirm(&self) -> &'static str {
        self.either("Confirm", "확인")
    }

    /// Dismiss a confirm.
    #[must_use]
    pub const fn cancel(&self) -> &'static str {
        self.either("Cancel", "취소")
    }

    /// Delete, the delete confirm's button.
    #[must_use]
    pub const fn delete(&self) -> &'static str {
        self.either("Delete", "삭제")
    }

    /// The close button's label.
    #[must_use]
    pub const fn close(&self) -> &'static str {
        self.either("Close", "닫기")
    }

    /// An empty conversation.
    #[must_use]
    pub const fn send_to_start(&self) -> &'static str {
        self.either(
            "Send a message to start the conversation.",
            "메시지를 보내 대화를 시작하세요.",
        )
    }

    /// A tool card's sections.
    #[must_use]
    pub const fn input(&self) -> &'static str {
        self.either("Input", "입력")
    }

    /// A tool card's diff, by its file.
    #[must_use]
    pub fn diff_of(&self, path: &str) -> String {
        match self.lang {
            Lang::En => format!("Diff · {path}"),
            Lang::Ko => format!("변경 · {path}"),
        }
    }

    /// A tool card's output.
    #[must_use]
    pub const fn output(&self) -> &'static str {
        self.either("Output", "출력")
    }

    /// Under a diff cut at its first lines.
    #[must_use]
    pub fn diff_cut(&self, shown: usize) -> String {
        match self.lang {
            Lang::En => format!("Only the first {shown} lines are shown"),
            Lang::Ko => format!("처음 {shown}줄만 보입니다"),
        }
    }
}

fn git_operation(operation: GitOperation) -> &'static str {
    match operation {
        GitOperation::Merge => "merge",
        GitOperation::Rebase => "rebase",
        GitOperation::CherryPick => "cherry pick",
        GitOperation::Revert => "revert",
        GitOperation::Bisect => "bisect",
    }
}

/// An amount in its unit: "$0.50" for dollars, "12.5 credits" else.
pub(super) fn cost_label(amount: f64, unit: &str) -> String {
    if unit.eq_ignore_ascii_case("usd") {
        format!("${amount:.2}")
    } else {
        // Up to four decimals, without trailing zeros (the page's
        // `Number(amount.toFixed(4))`).
        let fixed = format!("{amount:.4}");
        let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
        format!("{trimmed} {unit}")
    }
}
