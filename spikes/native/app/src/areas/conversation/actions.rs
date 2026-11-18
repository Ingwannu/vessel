//! The Session actions (the tab menu's rows on the open agent's tab,
//! TRACKER 797) and their confirms (archive, delete, kill), the agent's log
//! shown from them, and the acts they send to the node for the session
//! shown.

use vessel_app_kit::ui::Context;
use vessel_app_link::{Act, Command};
use vessel_app_view::session::MarkAction;
use vessel_proto::Mark;

use super::Seat;
use crate::shell::Shell;

/// A confirm the area has open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Confirm {
    /// Kill it.
    Kill,
}

/// One row of the Session actions menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuAction {
    /// A mark or a life action.
    Mark(MarkAction),
    /// Copy the session's id.
    CopyId,
    /// Copy a credential-free link through the shared desktop route format.
    CopyLink,
    /// Close using the root/child lifecycle policy.
    Close,
    /// Kill a live session.
    Kill,
    /// Show the agent's own log (its lines are not in the conversation).
    AgentLog,
}

/// Whether the agent's own log is shown in its dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentLog {
    /// Not shown.
    Hidden,
    /// Shown over the conversation.
    Shown,
}

impl Shell {
    /// Ask the node to act on the session shown.
    pub(crate) fn act(&self, act: Act) {
        self.act_at(&Seat::SESSION, act);
    }

    /// Ask the node to act on the session the conversation at `seat`
    /// shows, through the existing legacy transport only. Admitted operations
    /// require W12's guarded owner path and cannot fall back to an alias.
    pub(crate) fn act_at(&self, seat: &Seat, act: Act) {
        if let Some(shown) = seat.legacy_target(self) {
            self.command(Command::Act {
                machine: shown.machine.clone(),
                session: shown.session,
                act,
            });
        }
    }

    pub(crate) fn menu_chose(
        &mut self,
        action: MenuAction,
        window: &mut vessel_app_kit::ui::Window,
        cx: &mut Context<'_, Self>,
    ) {
        let confirm = match action {
            MenuAction::Mark(MarkAction::Archive) => {
                self.prepare_session_lifecycle(
                    vessel_app_view::sessions::LifecycleAction::Archive,
                    window,
                    cx,
                );
                None
            }
            MenuAction::Mark(MarkAction::Delete) => {
                self.prepare_session_lifecycle(
                    vessel_app_view::sessions::LifecycleAction::Delete,
                    window,
                    cx,
                );
                None
            }
            MenuAction::Kill => Some(Confirm::Kill),
            MenuAction::Mark(MarkAction::StartAgain) => {
                self.conversation_start_again(cx);
                None
            }
            MenuAction::Mark(mark) => {
                if mark == MarkAction::MarkUnread
                    && let Some(shown) = self.areas.conversation.shown.clone()
                {
                    self.conversation_marked_unread(&shown.machine, shown.session);
                }
                self.mark(mark);
                None
            }
            MenuAction::AgentLog => {
                self.areas.conversation.agent_log = AgentLog::Shown;
                self.areas.conversation.dialog.focus(window, cx);
                cx.notify();
                None
            }
            MenuAction::CopyLink => {
                self.copy_session_link(cx);
                None
            }
            MenuAction::Close => {
                self.request_session_close(window, cx);
                None
            }
            MenuAction::CopyId => {
                if let Some(shown) = &self.areas.conversation.shown {
                    cx.write_to_clipboard(vessel_app_kit::ui::ClipboardItem::new_string(
                        shown.session.to_string(),
                    ));
                    // Paseo's agent id copy (workspace-screen.tsx 2090): a
                    // Vessel session is Paseo's agent.
                    self.toast_copied(self.words.layout().copied_agent_id(), cx);
                }
                None
            }
        };
        if let Some(confirm) = confirm {
            self.areas.conversation.confirm = Some(confirm);
            self.areas.conversation.dialog.focus(window, cx);
        }
        cx.notify();
    }

    /// A mark or a life action, without a confirm.
    fn mark(&self, action: MarkAction) {
        let act = match action {
            MarkAction::Pin => Act::Mark(Mark::Pin),
            MarkAction::Unpin => Act::Mark(Mark::Unpin),
            MarkAction::MarkUnread => Act::Mark(Mark::Unread),
            MarkAction::MarkRead => Act::Mark(Mark::Read {
                through: self.areas.conversation.timeline.next(),
            }),
            MarkAction::Archive => Act::Mark(Mark::Archive),
            MarkAction::Unarchive => Act::Mark(Mark::Unarchive),
            MarkAction::StartAgain => Act::StartAgain,
            MarkAction::Delete => Act::Delete,
        };
        self.act(act);
    }

    /// The confirm open was accepted.
    pub(crate) fn confirmed(&mut self, cx: &mut Context<'_, Self>) {
        let Some(confirm) = self.areas.conversation.confirm.take() else {
            return;
        };
        match confirm {
            Confirm::Kill => self.act(Act::Kill),
        }
        cx.notify();
    }

    /// The agent's log was closed.
    pub(crate) fn dismiss_log(
        &mut self,
        _window: &mut vessel_app_kit::ui::Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.conversation.agent_log = AgentLog::Hidden;
        cx.notify();
    }

    /// The confirm open was dismissed.
    pub(crate) fn dismiss_confirm(
        &mut self,
        _window: &mut vessel_app_kit::ui::Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.conversation.confirm = None;
        cx.notify();
    }
}
