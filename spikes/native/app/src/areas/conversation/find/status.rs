//! Failure/retry and target lifetime; no second search or timeline owner.
use super::{FindExit, REVEAL_DEADLINE};
use crate::areas::conversation::ConversationArea;
use crate::shell::Shell;
use std::rc::Rc;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_view::timeline::{FindFailure, FindPhase};
use vessel_proto::EventSeq;

impl Shell {
    /// Call on the old resident before changing active panes, including
    /// Session → Session: its tasks must not be left attached to hidden input.
    pub(crate) fn conversation_deactivated(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.finish_find(FindExit::Deactivate, window, cx);
        self.areas.conversation.outline_pointer.leave();
        self.areas.conversation.clear_prompts();
        cx.notify();
    }

    pub(in crate::areas::conversation) fn find_failed(&mut self, failure: FindFailure) {
        self.areas.conversation.fail_find(failure);
    }

    pub(super) fn find_target(&mut self, at: EventSeq, cx: &mut Context<'_, Self>) {
        let Some(bar) = self.areas.conversation.find.as_mut() else {
            return;
        };
        bar.at = Some(at);
        bar.target = Rc::new(());
        let target = Rc::clone(&bar.target);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(REVEAL_DEADLINE).await;
            let _gone = this.update(cx, |shell, cx| {
                if shell
                    .areas
                    .conversation
                    .find
                    .as_ref()
                    .is_some_and(|bar| Rc::ptr_eq(&bar.target, &target) && bar.at == Some(at))
                {
                    shell.find_failed(FindFailure::Reveal);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(super) fn find_retry(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.areas.conversation.lost.is_some()
            && let Some(shown) = &self.areas.conversation.shown
        {
            self.command(Command::Open {
                machine: shown.machine.clone(),
                session: shown.session,
            });
        }
        self.start_search(false, cx);
        cx.notify();
    }
}

// The resource-local primitive also serves W12 retirement of a hidden resident.
impl ConversationArea {
    pub(in crate::areas::conversation) fn fail_find(&mut self, failure: FindFailure) {
        let Some(bar) = self.find.as_mut() else {
            return;
        };
        if bar.query.trim().is_empty() {
            return;
        }
        if bar.at.is_some() && self.reveal == bar.at {
            self.reveal = None;
        }
        bar.phase = FindPhase::Failed(failure);
        bar.finder = None;
        // A failed/retired projection must retain input, not stale row indices
        // or a queued refresh of those indices. Retry rebuilds them normally.
        bar.matches.clear();
        bar.index = 0;
        bar.refreshing = false;
        bar.again = false;
        bar.keep = None;
        bar.said = None;
        bar.generation = Rc::new(());
        bar.at = None;
        bar.target = Rc::new(());
    }
}
