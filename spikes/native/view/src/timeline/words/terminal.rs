//! A terminal session's words: its frame, its banners, its toasts, the
//! link confirm (the page's `features/session/terminal-view.tsx`,
//! `data/models.ts`'s `usageLine`, `data/links.ts`).

use super::numbers::short_tokens;
use crate::terminal::Liveness;
use crate::timeline::ModelUsage;
use crate::{Lang, Text};

impl Text {
    /// A session's model calls in one line: `3 model calls · 1.2k in ·
    /// 300 out`, failures counted when there are any.
    #[must_use]
    pub fn usage_line(&self, usage: &ModelUsage) -> String {
        let mut parts = vec![match self.lang {
            Lang::En => format!(
                "{} model call{}",
                usage.calls,
                if usage.calls == 1 { "" } else { "s" }
            ),
            Lang::Ko => format!("모델 호출 {}회", usage.calls),
        }];
        if usage.failed > 0 {
            parts.push(match self.lang {
                Lang::En => format!("{} failed", usage.failed),
                Lang::Ko => format!("실패 {}", usage.failed),
            });
        }
        let input = short_tokens(usage.total_input());
        let output = short_tokens(usage.output);
        match self.lang {
            Lang::En => parts.extend([format!("{input} in"), format!("{output} out")]),
            Lang::Ko => parts.extend([format!("입력 {input}"), format!("출력 {output}")]),
        }
        parts.join(" · ")
    }

    /// The frame's corner: live, frozen or ended.
    #[must_use]
    pub const fn liveness(&self, liveness: Liveness) -> &'static str {
        match liveness {
            Liveness::Live => self.either("live", "실시간"),
            Liveness::Frozen => self.either("frozen", "멈춤"),
            Liveness::Ended => self.either("ended", "끝남"),
        }
    }

    /// The banner while the stream reconnects.
    #[must_use]
    pub fn terminal_reconnecting(&self, machine: &str, why: &str) -> String {
        match self.lang {
            Lang::En => format!("Reconnecting to {machine} · {why} · the terminal is frozen"),
            Lang::Ko => format!("{machine}에 다시 연결하는 중 · {why} · 터미널이 멈춰 있습니다"),
        }
    }

    /// The banner once the stream stopped.
    #[must_use]
    pub fn stream_stopped(&self, why: &str) -> String {
        match self.lang {
            Lang::En => format!("The stream stopped: {why}"),
            Lang::Ko => format!("스트림이 멈췄습니다: {why}"),
        }
    }

    /// The toast when keys overflow the sender.
    #[must_use]
    pub const fn keys_dropped(&self) -> &'static str {
        self.either(
            "Keys dropped: the session is not reading its input",
            "키 입력을 버렸습니다: 세션이 입력을 읽지 않습니다",
        )
    }

    /// The confirm before a link from the session opens.
    #[must_use]
    pub fn open_link(&self, href: &str) -> String {
        match self.lang {
            Lang::En => format!("Open this link from the session in your browser?\n\n{href}"),
            Lang::Ko => format!("세션에서 온 이 링크를 브라우저에서 열까요?\n\n{href}"),
        }
    }

    /// The terminal's label for a screen reader.
    #[must_use]
    pub const fn session_terminal(&self) -> &'static str {
        self.either("Session terminal", "세션 터미널")
    }
}
