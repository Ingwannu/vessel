//! Prompt-index orchestration and jump lifetime. View owns pages; the existing
//! Follow supplies bodies and reveal. No second transcript or provider request.

use super::ConversationArea;
use crate::shell::Shell;
use std::{rc::Rc, time::Duration};
use vessel_app_kit::Reveal;
use vessel_app_kit::ui::Context;
use vessel_app_link::{Command, prompt_index::IndexAnswer};
use vessel_app_view::devices::MachineKey;
use vessel_app_view::prompt_index::{IndexAsk, IndexTarget, PromptIndex, PromptRows};
use vessel_proto::prompt_index::PromptIndexEpoch;
use vessel_proto::{AgentEvent, EventRecord, EventSeq, SessionEvent};

#[cfg(test)]
mod tests;

/// Paseo bounds loading a destination at five seconds.
const JUMP_DEADLINE: Duration = Duration::from_millis(5_000);

#[derive(Debug)]
struct Jump {
    seq: EventSeq,
    identity: Rc<()>,
}

/// State is retained in the existing conversation area; clear preserves ticket serials.
#[derive(Debug, Default)]
pub(super) struct Prompts {
    index: PromptIndex,
    jump: Option<Jump>,
    /// History chunks can extend a prompt without changing its first address.
    last_edit: Option<EventSeq>,
}

impl Prompts {
    fn clear(&mut self) {
        self.index.clear();
        self.jump = None;
        self.last_edit = None;
    }
}

impl ConversationArea {
    /// Cancelling navigation must not leave a delayed scroll owned by its jump.
    pub(super) fn clear_prompts(&mut self) {
        if self
            .prompts
            .jump
            .as_ref()
            .is_some_and(|jump| self.reveal == Some(jump.seq))
        {
            self.reveal = None;
        }
        self.prompts.clear();
    }

    /// Resident panes hold active UI/work. An inactive snapshot must still move
    /// the pure prompt/hover identity sources intact, never default their counters.
    #[cfg(test)]
    pub(crate) fn navigation_held(&self) -> bool {
        self.find.is_some()
            || self.prompts.jump.is_some()
            || matches!(
                self.prompts.index.phase(),
                vessel_app_view::prompt_index::IndexPhase::Reading(_)
            )
            || self.outline_pointer.settling()
    }

    pub(super) fn prompt_target(&self) -> Option<&IndexTarget> {
        self.prompts.index.target()
    }

    pub(super) fn prompt_epoch(&self) -> Option<PromptIndexEpoch> {
        self.prompts.index.epoch()
    }

    pub(super) fn prompt_rows(&self) -> PromptRows<'_> {
        PromptRows::new(self.prompts.index.entries(), &self.timeline, &self.outline)
    }
}

impl Shell {
    /// Pointer callbacks keep the painted session and epoch, not just its ordinal slot.
    pub(super) fn prompt_selected(
        &mut self,
        target: &IndexTarget,
        seq: EventSeq,
        epoch: Option<PromptIndexEpoch>,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &self.areas.conversation;
        if area.prompts.index.target() != Some(target)
            || (epoch.is_some() && area.prompts.index.epoch() != epoch)
            || !area.prompt_rows().iter().any(|(_, row)| row.seq == seq)
        {
            return;
        }
        self.prompt_jump(seq, cx);
    }

    pub(super) fn prompt_index_opened(&mut self, open: &crate::areas::session::Open) {
        self.areas.conversation.clear_prompts();
        if open.structured {
            self.prompt_index_refresh();
        }
    }

    pub(super) fn prompt_index_refresh(&mut self) {
        if !self.prefs.settings.appearance.chat_outline.on() {
            self.areas.conversation.clear_prompts();
            return;
        }
        let area = &self.areas.conversation;
        let Some(shown) = &area.shown else {
            return;
        };
        let machine = self
            .fleet
            .machines
            .iter()
            .find(|machine| machine.name == shown.machine);
        let target = IndexTarget {
            machine: MachineKey::from_book(&shown.machine),
            connection: machine.and_then(vessel_app_view::Machine::connection_context),
            session: shown.session,
        };
        let latest = area.prompts.last_edit;
        let ask = self
            .areas
            .conversation
            .prompts
            .index
            .refresh(target, latest);
        self.prompt_index_ask(ask);
    }

    fn prompt_index_ask(&self, ask: Option<IndexAsk>) {
        if let Some(ask) = ask {
            self.command(Command::PromptIndex(ask));
        }
    }

    pub(crate) fn prompt_index_answered(
        &mut self,
        answer: IndexAnswer,
        cx: &mut Context<'_, Self>,
    ) {
        let ask = self
            .areas
            .conversation
            .prompts
            .index
            .answered(&answer.ask, answer.result);
        self.prompt_index_ask(ask);
        cx.notify();
    }

    pub(super) fn prompt_index_records(&mut self, records: &[EventRecord]) {
        if self.route != crate::routes::Route::Session
            || self
                .areas
                .session
                .open
                .as_ref()
                .is_some_and(|open| !open.structured)
        {
            return;
        }
        if let Some(record) = records.iter().rev().find(|record| {
            matches!(
                record.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnStarted { .. }
                        | AgentEvent::Steered { .. }
                        | AgentEvent::UserMessageChunk { .. }
                }
            )
        }) {
            self.areas.conversation.prompts.last_edit = Some(record.seq);
        }
        if self.areas.conversation.prompts.jump.is_some()
            && self.areas.conversation.reveal.is_none()
        {
            self.areas.conversation.prompts.jump = None;
        }
        if let Some(record) = records
            .iter()
            .rev()
            .find(|record| matches!(record.event, SessionEvent::Rewound { .. }))
        {
            let ask = self
                .areas
                .conversation
                .prompts
                .index
                .history_changed(record.seq);
            self.prompt_index_ask(ask);
        }
        self.prompt_index_refresh();
    }

    /// Readmission invalidates pages even if the book name/session UUID match.
    pub(crate) fn prompt_index_fleet_changed(&mut self) {
        if self.route == crate::routes::Route::Session
            && self
                .areas
                .session
                .open
                .as_ref()
                .is_some_and(|open| open.structured)
        {
            self.prompt_index_refresh();
        }
    }

    /// A loaded prompt reveals immediately; an unloaded address waits on Follow.
    pub(super) fn prompt_jump(&mut self, seq: EventSeq, cx: &mut Context<'_, Self>) {
        let area = &self.areas.conversation;
        let loaded = area
            .timeline
            .entries()
            .iter()
            .position(|entry| entry.seq == seq);
        if let Some(entry) = loaded {
            let area = &self.areas.conversation;
            if !area.timeline.rewound(entry)
                && let Some(row) = area.rows.row_index(entry, area.folds.as_ref(), &area.logs)
            {
                area.list.reveal(area.at(row), Reveal::Top);
                self.areas.conversation.prompts.jump = None;
                self.areas.conversation.reveal = None;
                cx.notify();
                return;
            }
            self.notice = Some(self.words.chat_find().reveal().to_owned());
            cx.notify();
            return;
        }
        if area.timeline.next() > seq {
            self.notice = Some(self.words.chat_find().reveal().to_owned());
            cx.notify();
            return;
        }
        let identity = Rc::new(());
        self.areas.conversation.reveal(seq);
        self.areas.conversation.prompts.jump = Some(Jump {
            seq,
            identity: Rc::clone(&identity),
        });
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(JUMP_DEADLINE).await;
            let _gone =
                this.update(cx, |shell, cx| {
                    let area = &mut shell.areas.conversation;
                    if area.prompts.jump.as_ref().is_some_and(|jump| {
                        jump.seq == seq && Rc::ptr_eq(&jump.identity, &identity)
                    }) {
                        area.prompts.jump = None;
                        if area.reveal == Some(seq) {
                            area.reveal = None;
                            shell.notice = Some(shell.words.chat_find().reveal().to_owned());
                            cx.notify();
                        }
                    }
                });
        })
        .detach();
        cx.notify();
    }
}
