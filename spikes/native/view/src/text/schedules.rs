//! Schedule words, including every node state and run result, in both languages.

use vessel_proto::automation::{Cadence, EndReason};

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
    /// Unknown filter and fallback for newer values.
    pub unknown: &'static str,
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
    /// Invalid interval input.
    pub interval_number: &'static str,
    /// Ended schedule edit refusal.
    pub edit_ended: &'static str,
    /// Invalid automatic-session spec.
    pub structured_only: &'static str,
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

    /// Exact elapsed interval or the original typed cron and its chosen zone.
    /// No second cron parser and no rounding an elapsed interval into minutes.
    #[must_use]
    pub fn schedule_cadence(self, cadence: &Cadence) -> String {
        match cadence {
            Cadence::Every { seconds, .. } => match self {
                Self::En => format!("Every {} seconds", seconds.get()),
                Self::Ko => format!("{}초마다", seconds.get()),
            },
            Cadence::Cron { expr, zone } => match self {
                Self::En => format!("At scheduled times: {} ({})", expr.as_str(), zone.as_str()),
                Self::Ko => format!("정해진 시각: {} ({})", expr.as_str(), zone.as_str()),
            },
            Cadence::Unknown => self.schedules().unknown.to_owned(),
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
                EndReason::Unknown => self.unknown,
            },
            Status::Unknown => self.unknown,
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
            Outcome::Unknown => self.unknown,
        }
    }
}

const EN: ScheduleWords = ScheduleWords {
    title: "Schedules",
    create: "New schedule",
    active: "Active",
    ended: "Ended",
    unknown: "Unknown — update the app",
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
    interval_number: "Enter a whole number of seconds",
    edit_ended: "An ended schedule cannot be edited",
    structured_only: "Choose a structured session without resume",
};

const KO: ScheduleWords = ScheduleWords {
    title: "일정",
    create: "새 일정",
    active: "활성",
    ended: "종료됨",
    unknown: "알 수 없음 — 앱을 업데이트하세요",
    paused: "일시 정지됨",
    finished: "완료됨",
    expired: "기간 만료됨",
    creator_gone: "작성자의 접근 권한이 사라짐",
    target_gone: "대상이 사라짐",
    edit: "일정 수정",
    pause: "일정 일시 정지",
    resume: "일정 재개",
    run_now: "지금 실행",
    delete: "일정 삭제",
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
    interval_number: "초 단위의 정수를 입력하세요",
    edit_ended: "끝난 일정은 수정할 수 없습니다",
    structured_only: "이어서 시작하지 않는 구조화 세션을 선택하세요",
};
