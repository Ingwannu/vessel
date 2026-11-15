//! The words of the context meter beside the send button and its card:
//! Paseo's (`contextWindow.*`, its English and Korean), and GOAT's
//! "Compact context" with why it cannot run now.

use crate::Lang;
use crate::timeline::Meter;

use super::ConversationWords;
use super::numbers::meter_tokens;
use super::session::cost_label;

impl ConversationWords {
    /// The meter card's heading.
    #[must_use]
    pub const fn meter_title(&self) -> &'static str {
        self.either("Context window", "컨텍스트 윈도우")
    }

    /// The meter card's first line: how much of the window is used.
    #[must_use]
    pub fn meter_used(&self, meter: &Meter) -> String {
        let percent = meter.percent;
        match self.lang {
            Lang::En => format!("{percent}% used"),
            Lang::Ko => format!("{percent}% 사용됨"),
        }
    }

    /// The meter card's second line: the tokens used of the window's.
    #[must_use]
    pub fn meter_tokens_line(&self, meter: &Meter) -> String {
        let (used, size) = (meter_tokens(meter.used), meter_tokens(meter.size));
        match self.lang {
            Lang::En => format!("{used} / {size} tokens"),
            Lang::Ko => format!("{used} / {size} 토큰"),
        }
    }

    /// The meter card's line before the agent has said how full it is.
    #[must_use]
    pub const fn meter_no_data(&self) -> &'static str {
        self.either("No context data", "컨텍스트 데이터 없음")
    }

    /// The meter card's line for what the session has cost, when the agent
    /// says.
    #[must_use]
    pub fn meter_cost(&self, amount: f64, unit: &str) -> String {
        let cost = cost_label(amount, unit);
        match self.lang {
            Lang::En => format!("Session cost {cost}"),
            Lang::Ko => format!("세션 비용 {cost}"),
        }
    }

    /// The meter card's button that sends the agent's `/compact` (GOAT's).
    #[must_use]
    pub const fn meter_compact(&self) -> &'static str {
        self.either("Compact context", "컨텍스트 압축")
    }

    /// Under the meter card's compact button once the session has ended.
    #[must_use]
    pub const fn meter_compact_ended(&self) -> &'static str {
        self.either(
            "Compacting needs the session running.",
            "세션이 실행 중일 때만 압축할 수 있습니다.",
        )
    }

    /// Under the meter card's compact button while a turn runs.
    #[must_use]
    pub const fn meter_compact_busy(&self) -> &'static str {
        self.either(
            "Compacting waits until the turn ends.",
            "턴이 끝난 뒤에 압축할 수 있습니다.",
        )
    }
}
