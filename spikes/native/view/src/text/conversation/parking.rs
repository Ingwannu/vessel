//! The words of a parked session (protocol 12, ADR 0038), after Paseo's
//! "Release idle agents" ("Stops the agent's process while keeping its
//! conversation. Send a message to resume." / "대화를 유지하면서 에이전트
//! 프로세스를 중지합니다. 메시지를 보내면 다시 이어집니다."): the
//! timeline's lines, the bar's word and hint, the banner when a resume
//! failed.

use vessel_proto::parking::ParkReason;

use crate::Lang;
use crate::timeline::ParkingChange;

use super::ConversationWords;

impl ConversationWords {
    /// The timeline's line for a `parked` or `woken` record (the time
    /// follows it, as a status line's does).
    #[must_use]
    pub fn parking_line(&self, change: &ParkingChange) -> String {
        match change {
            ParkingChange::Parked(ParkReason::Idle) => self
                .either("Agent released while idle", "유휴 상태라 에이전트를 해제함")
                .to_owned(),
            ParkingChange::Parked(ParkReason::Asked | ParkReason::Unknown) => self
                .either("Agent released", "에이전트를 해제함")
                .to_owned(),
            ParkingChange::Parked(ParkReason::WakeFailed { message, .. }) => match self.lang {
                Lang::En => format!("Resuming the agent failed: {message}"),
                Lang::Ko => format!("에이전트를 다시 잇지 못함: {message}"),
            },
            ParkingChange::Woken => self
                .either("Agent resumed", "에이전트를 다시 이음")
                .to_owned(),
        }
    }

    /// The bar's word for a released agent (Paseo's).
    #[must_use]
    pub const fn parked_word(&self) -> &'static str {
        self.either("Released", "해제됨")
    }

    /// The bar's word when resuming a released agent failed.
    #[must_use]
    pub const fn resume_failed_word(&self) -> &'static str {
        self.either("Resume failed", "재개 실패")
    }

    /// Beside the bar's word for a released agent: what a send does
    /// (Paseo's sentence).
    #[must_use]
    pub const fn parked_hint(&self) -> &'static str {
        self.either(
            "The agent was released while idle. Send a message to resume.",
            "유휴 상태라 에이전트를 해제했습니다. 메시지를 보내면 다시 이어집니다.",
        )
    }
}
