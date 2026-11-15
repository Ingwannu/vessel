//! Only correlated side events reach this area's seat. Main records are separate.

use super::SIDE;
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::side_question::{Command as SideCommand, Event, Follow, ResultKind};
use vessel_app_link::{Act, Command};

impl Shell {
    pub(crate) fn side_event(&mut self, event: Event, cx: &mut Context<'_, Self>) {
        match event {
            Event::Answer(answer) => match answer.result {
                ResultKind::Opened(result) => {
                    let id = result.as_ref().map(|info| info.id).map_err(Clone::clone);
                    if !self.areas.side_question.state.opened(&answer.pending, id) {
                        return;
                    }
                    if let Ok(info) = result {
                        if self.areas.side_question.state.side() != Some(info.id) {
                            return;
                        }
                        let follow = Follow {
                            target: answer.pending.target,
                            side: info.id,
                            ticket: answer.pending.ticket,
                        };
                        let area = &mut self.areas.side_question;
                        area.conversation
                            .open(follow.target.machine.as_str(), follow.side);
                        area.conversation.info = Some(info);
                        if answer.pending.operation
                            == vessel_app_view::side_question::Operation::Reset
                        {
                            *area.acknowledged.borrow_mut() = Some(area.composer.text(cx));
                        }
                        area.follow = Some(follow.clone());
                        self.command(Command::SideQuestion(SideCommand::Watch(follow)));
                        self.act_at(SIDE, Act::Conversation);
                    }
                }
                ResultKind::Prompted(result) => {
                    let success = result.is_ok();
                    let area = &mut self.areas.side_question;
                    if !area.state.prompted(&answer.pending, result) {
                        return;
                    }
                    let sent = area.sent.take();
                    if success {
                        *area.acknowledged.borrow_mut() = sent;
                    }
                }
            },
            Event::Records { follow, records } => {
                let area = &mut self.areas.side_question;
                let _changed = ingest(
                    &mut area.conversation,
                    area.follow.as_ref(),
                    &follow,
                    &records,
                );
            }
            Event::Lost { follow, problem } => {
                let area = &mut self.areas.side_question;
                if area.follow.as_ref() != Some(&follow) {
                    return;
                }
                area.state.refused(problem);
            }
        }
        cx.notify();
    }
}

/// A stream may have queued records when its task was cancelled. Compare the
/// complete identity before mutating a transcript, including same-side reopen.
fn ingest(
    conversation: &mut super::super::conversation::ConversationArea,
    expected: Option<&Follow>,
    actual: &Follow,
    records: &[vessel_proto::EventRecord],
) -> bool {
    if expected != Some(actual) {
        return false;
    }
    conversation.take(&actual.side, records)
}

#[cfg(test)]
#[path = "intake_tests.rs"]
mod tests;
