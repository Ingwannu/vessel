//! A session's conversation and its frame: the timeline, the title bar
//! with its Session actions, find, the outline, the inspector's Summary,
//! and a terminal session's view (the page's `features/session`, less the
//! composer and its docks, which are the session area's).
//!
//! What it means is `view::timeline`, `view::session` and
//! `view::terminal`; what it asks of the node is `link::Act`; its words
//! are `Text`'s.

use vessel_app_kit::ui::Context;
use vessel_app_link::{ActAnswer, ActKind, Acted};
use vessel_app_view::Card;
use vessel_proto::SessionInfo;

use crate::shell::Shell;

/// The conversation area's state.
#[derive(Default)]
pub(crate) struct ConversationArea {
    /// The open session as the node last described it (marks, abilities,
    /// title); `None` until read.
    pub(crate) info: Option<Box<SessionInfo>>,
    /// The last act the node refused, and its words: shown beside the
    /// control it is about.
    pub(crate) refused: Option<(ActKind, String)>,
}

impl Shell {
    /// A session act answered.
    pub(crate) fn acted(&mut self, answer: ActAnswer, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.conversation;
        match answer.result {
            Ok(Acted::Session(info)) => {
                area.refused = None;
                let card = Card::from_session(&answer.machine, &info);
                if let Some(machine) = self
                    .fleet
                    .machines
                    .iter_mut()
                    .find(|machine| machine.name == answer.machine)
                    && let Some(slot) = machine
                        .sessions
                        .iter_mut()
                        .find(|slot| slot.session == card.session)
                {
                    *slot = card;
                }
                area.info = Some(info);
            }
            Ok(Acted::Deleted(deleted)) => {
                area.refused = None;
                area.info = None;
                if let Some(machine) = self
                    .fleet
                    .machines
                    .iter_mut()
                    .find(|machine| machine.name == answer.machine)
                {
                    machine
                        .sessions
                        .retain(|card| card.session != answer.session);
                }
                let words = self.t;
                let kept = deleted
                    .worktree
                    .as_ref()
                    .map(|worktree| words.deleted_worktree(&worktree.outcome));
                self.notice = Some(kept.map_or_else(
                    || words.session_deleted().to_owned(),
                    |kept| format!("{}. {kept}", words.session_deleted()),
                ));
            }
            Ok(Acted::Done) => area.refused = None,
            Err(failed) => area.refused = Some((answer.kind, failed.message)),
        }
        cx.notify();
    }
}
