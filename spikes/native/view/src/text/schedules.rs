//! Schedule words, including every node state and run result, in both languages.

mod errors;
mod format;

use vessel_proto::automation::EndReason;

use super::Lang;
use crate::schedules::history::Outcome;
use crate::schedules::{Action, Status};

/// The schedule screen's fixed words. Keeping both tables the same type makes
/// an omitted translation a compile error.
#[derive(Debug)]
pub struct ScheduleWords {
    /// Screen and navigation label.
    pub title: &'static str,
    /// Primary create action.
    pub create: &'static str,
    /// Runnable filter and active badge.
    pub active: &'static str,
    /// Ended filter.
    pub ended: &'static str,
    /// Filter for states a newer machine has.
    pub unknown_filter: &'static str,
    /// Short state badge.
    pub unknown_badge: &'static str,
    /// Explanation when this app cannot understand a machine value.
    pub newer_machine: &'static str,
    /// Paused badge.
    pub paused: &'static str,
    /// Maximum-runs ended badge.
    pub finished: &'static str,
    /// Expiry ended badge.
    pub expired: &'static str,
    /// Device revocation ended badge.
    pub creator_gone: &'static str,
    /// Missing target ended badge.
    pub target_gone: &'static str,
    /// Edit action.
    pub edit: &'static str,
    /// Pause action.
    pub pause: &'static str,
    /// Resume action.
    pub resume: &'static str,
    /// Run-now action.
    pub run_now: &'static str,
    /// Delete action.
    pub delete: &'static str,
    /// Recorded run not yet made.
    pub making: &'static str,
    /// Run in progress.
    pub running: &'static str,
    /// Run waiting on permission.
    pub waiting: &'static str,
    /// Prompt started in its target session.
    pub prompted: &'static str,
    /// Prompt steered into an existing turn.
    pub steered: &'static str,
    /// Prompt waiting in its target session.
    pub queued: &'static str,
    /// Run skipped.
    pub skipped: &'static str,
    /// Run failed.
    pub failed: &'static str,
    /// Turn cancelled by a person.
    pub cancelled: &'static str,
    /// Turn reached a limit.
    pub limited: &'static str,
    /// Model refused.
    pub refused: &'static str,
    /// Session ended before its turn.
    pub session_ended: &'static str,
    /// Node/session interrupted.
    pub interrupted: &'static str,
    /// Ended schedule edit refusal.
    pub edit_ended: &'static str,
    /// Invalid automatic-session spec.
    pub structured_only: &'static str,
    /// Known insufficient authority or an actual Forbidden response.
    pub read_only: &'static str,
}

impl Lang {
    /// This language's schedule words.
    #[must_use]
    pub const fn schedules(self) -> &'static ScheduleWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

impl ScheduleWords {
    /// A state badge; the reason is never inferred from a local clock.
    #[must_use]
    pub const fn status(&self, status: Status) -> &'static str {
        match status {
            Status::Active => self.active,
            Status::Paused => self.paused,
            Status::Ended(reason) => match reason {
                EndReason::MaxRuns => self.finished,
                EndReason::Expired => self.expired,
                EndReason::CreatorGone => self.creator_gone,
                EndReason::TargetGone => self.target_gone,
                EndReason::Unknown => self.unknown_badge,
            },
            Status::Unknown => self.unknown_badge,
        }
    }

    /// An action's menu label.
    #[must_use]
    pub const fn action(&self, action: Action) -> &'static str {
        match action {
            Action::Edit => self.edit,
            Action::Pause => self.pause,
            Action::Resume => self.resume,
            Action::RunNow => self.run_now,
            Action::Delete => self.delete,
        }
    }

    /// A run's result; failure and interruption never borrow success copy.
    #[must_use]
    pub const fn outcome(&self, outcome: Outcome) -> &'static str {
        match outcome {
            Outcome::Making => self.making,
            Outcome::Running => self.running,
            Outcome::Waiting => self.waiting,
            Outcome::Finished => self.finished,
            Outcome::Prompted => self.prompted,
            Outcome::Steered => self.steered,
            Outcome::Queued => self.queued,
            Outcome::Skipped => self.skipped,
            Outcome::Failed => self.failed,
            Outcome::Cancelled => self.cancelled,
            Outcome::Limited => self.limited,
            Outcome::Refused => self.refused,
            Outcome::SessionEnded => self.session_ended,
            Outcome::Interrupted => self.interrupted,
            Outcome::Unknown => self.unknown_badge,
        }
    }
}

const EN: ScheduleWords = ScheduleWords {
    title: "Schedules",
    create: "New schedule",
    active: "Active",
    ended: "Ended",
    unknown_filter: "Other states",
    unknown_badge: "Unknown",
    newer_machine: "This machine uses a newer format. Update the app to view it.",
    paused: "Paused",
    finished: "Finished",
    expired: "Expired",
    creator_gone: "Creator no longer has access",
    target_gone: "Target gone",
    edit: "Edit schedule",
    pause: "Pause schedule",
    resume: "Resume schedule",
    run_now: "Run now",
    delete: "Delete schedule",
    making: "Preparing",
    running: "Running",
    waiting: "Needs permission",
    prompted: "Prompt sent",
    steered: "Added to the running turn",
    queued: "Queued",
    skipped: "Skipped",
    failed: "Failed",
    cancelled: "Cancelled",
    limited: "Limit reached",
    refused: "Request refused",
    session_ended: "Session ended",
    interrupted: "Interrupted",
    edit_ended: "An ended schedule cannot be edited",
    structured_only: "A schedule starts a new chat each time; it cannot continue an old one or open a terminal session",
    read_only: "This device can only view this machine's schedules.",
};

const KO: ScheduleWords = ScheduleWords {
    title: "예약",
    create: "새 예약",
    active: "활성",
    ended: "종료됨",
    unknown_filter: "기타 상태",
    unknown_badge: "알 수 없음",
    newer_machine: "기계에서 더 새로운 형식을 사용합니다. 앱을 업데이트하면 볼 수 있습니다.",
    paused: "일시 정지됨",
    finished: "완료됨",
    expired: "기간 만료됨",
    creator_gone: "작성자의 접근 권한이 사라짐",
    target_gone: "대상이 사라짐",
    edit: "예약 수정",
    pause: "예약 일시 정지",
    resume: "예약 재개",
    run_now: "지금 실행",
    delete: "예약 삭제",
    making: "준비 중",
    running: "실행 중",
    waiting: "권한 승인이 필요함",
    prompted: "프롬프트 보냄",
    steered: "진행 중인 턴에 추가됨",
    queued: "대기열에 추가됨",
    skipped: "건너뜀",
    failed: "실패함",
    cancelled: "취소됨",
    limited: "한도에 도달함",
    refused: "요청이 거절됨",
    session_ended: "세션이 끝남",
    interrupted: "중단됨",
    edit_ended: "끝난 예약은 수정할 수 없습니다",
    structured_only: "예약은 실행할 때마다 새 대화를 시작합니다. 이전 대화를 이어서 시작하거나 터미널 세션을 열 수는 없습니다.",
    read_only: "이 기계의 예약은 이 기기에서 조회만 할 수 있습니다.",
};
