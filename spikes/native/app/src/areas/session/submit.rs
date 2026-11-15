//! One session's send entry point; attachment preparation precedes text dispatch.
use super::shows_at_once;
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_view::composer::drafts::DraftKey;

impl Shell {
    pub(crate) fn submit_prompt(
        &mut self,
        text: String,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        // The legacy field has no resident context: at least require the same
        // presented destination before allocating an ID or touching input.
        let Some(target) = self
            .areas
            .session
            .open
            .as_ref()
            .map(|open| DraftKey::Session {
                session: open.session,
            })
        else {
            return;
        };
        if self.composer_destination() != Some(target) {
            return;
        }
        if self.composer_client_command(&text, window, cx) {
            return;
        }
        let Some(open) = &self.areas.session.open else {
            return;
        };
        if !self.session_may_send(&text)
            || self.areas.session.resuming.is_some()
            || self.conversation_cannot_resume()
        {
            return;
        }
        let message = vessel_proto::timeline::ClientMessageId::random();
        if !open.live && self.session_has_files() {
            self.conversation_sending(&text, cx);
            self.resume_attachment_prompt(message, &text, window, cx);
            return;
        }
        if open.live {
            let delivery = self.delivery();
            // Pending messages show at once; queued ones appear in their queue.
            if shows_at_once(delivery, self.areas.session.conversation.turn_running()) {
                self.conversation_sending(&text, cx);
            }
            if self.session_sends_files(message, &text, delivery, window, cx) {
                return;
            }
            let Some(open) = &self.areas.session.open else {
                return;
            };
            self.command(Command::Prompt {
                machine: open.machine.clone(),
                session: open.session,
                text,
                delivery,
            });
        } else {
            // Resume preserves history; the pending message stays visible.
            self.conversation_sending(&text, cx);
            let Some(open) = &self.areas.session.open else {
                return;
            };
            self.command(Command::Resume {
                machine: open.machine.clone(),
                session: open.session,
                text: text.clone(),
            });
            self.areas.session.resuming = Some(text);
            self.areas.session.resume_error = None;
        }
        self.session_composer_sent(window, cx);
        cx.notify();
    }
}

#[cfg(test)]
mod tests;
