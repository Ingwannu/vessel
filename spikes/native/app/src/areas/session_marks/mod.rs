//! Directory-only metadata never acknowledges a resident stop or mutates input.
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::{
    Command,
    session_marks::{Answer, Failure},
};
use vessel_app_view::session_marks::{BeginError, CapturedSessionTarget};
use vessel_proto::Mark;

impl Shell {
    /// S2 keeps this SAME captured target from render through confirmation.
    pub(crate) fn mark_session_target(
        &mut self,
        target: CapturedSessionTarget,
        mark: Mark,
        cx: &mut Context<'_, Self>,
    ) {
        let ask = self.areas.session_marks.begin(target, mark);
        match ask {
            Ok(ask) => self.command(Command::SessionMark(ask)),
            Err(BeginError::Busy) => {
                self.notice = Some(self.words.session_marks().busy().to_owned())
            }
            Err(BeginError::Exhausted | BeginError::ReadBoundary) => {
                self.notice = Some(self.words.session_marks().unavailable().to_owned());
            }
        }
        cx.notify();
    }

    pub(crate) fn session_mark_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        // No await, alias recapture or local callback between guard and apply.
        let (ask, result) = answer.into_reply().into_parts();
        if !self.areas.session_marks.finish(&ask) {
            return;
        }
        let original = ask.target().context();
        let current = self
            .fleet
            .machines
            .iter()
            .any(|machine| machine.connection_context().as_ref() == Some(original));
        match result {
            Ok(info) if current && info.id == ask.target().session() => {
                // The ACK settles directory work only. The canonical Fleet read
                // owns projection installation; do not race an older queued Fleet
                // by optimistically putting this unary snapshot into its rows.
                self.notice = None;
            }
            Ok(_) => self.notice = Some(self.words.session_marks().changed().to_owned()),
            Err(failure) => {
                self.notice = Some(
                    match failure {
                        Failure::Changed => self.words.session_marks().changed(),
                        Failure::Unsent | Failure::Unsupported => {
                            self.words.session_marks().unavailable()
                        }
                        Failure::Uncertain | Failure::InvalidReply | Failure::Refused(_) => {
                            self.words.session_marks().uncertain()
                        }
                    }
                    .to_owned(),
                );
            }
        }
        // An explicit fresh projection reconciles a dispatched uncertain mutation.
        // This never repeats the command or installs data into conversation state.
        self.reload();
        cx.notify();
    }
}

/// A local command rejection is certain no-send, without a fake delivery guard.
pub(crate) fn unsent(ask: vessel_app_link::session_marks::Ask) -> vessel_app_link::Event {
    vessel_app_link::Event::SessionMarked(Answer::refused(ask, Failure::Unsent))
}

#[cfg(test)]
mod tests;
