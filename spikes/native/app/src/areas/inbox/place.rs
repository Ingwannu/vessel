//! The inbox's request cards share the conversation's drawing and validation,
//! while its exact-source send path handles stale requests and confirmations.
use crate::areas::conversation::{Answer, ConversationArea, Place, Seat};
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::inbox::Input;
use vessel_app_view::timeline::ToolView;
use vessel_proto::ToolCallId;

pub(super) struct InboxPlace;
impl Place for InboxPlace {
    const SEAT: Seat = Seat {
        get: |shell| Some(&shell.areas.inbox.conversation),
        get_mut: |shell| Some(&mut shell.areas.inbox.conversation),
    };
    fn answer(shell: &mut Shell, answer: Answer, cx: &mut Context<'_, Shell>) {
        let input = match answer {
            Answer::Permit(id, option) => Input::Permission(id, option),
            Answer::Question(id, answer) => Input::Question(id, answer),
        };
        shell.inbox_send_answer(input);
        cx.notify();
    }
    fn busy(shell: &Shell) -> bool {
        shell.areas.inbox.detail.pending().is_some()
            || shell
                .areas
                .inbox
                .detail
                .key()
                .is_none_or(|key| !shell.areas.inbox.store.may_answer(key))
    }
    fn tool<'a>(shell: &'a Shell, id: &ToolCallId) -> Option<&'a ToolView> {
        shell
            .areas
            .inbox
            .tool
            .as_ref()
            .filter(|tool| tool.id == *id)
    }
}

pub(super) fn clear_cards(area: &mut ConversationArea) {
    area.live = vessel_app_view::timeline::Live::default();
    area.permit_sending = None;
    area.refused = None;
    area.answering = None;
}
