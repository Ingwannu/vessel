//! A running session's model, effort, permissions and other options,
//! changed from its composer with the draft's pills, as Paseo's
//! `AgentControls` changes a running agent's
//! (`packages/app/src/composer/agent-controls/index.tsx`).
//!
//! A choice goes to the node at once as `session.set_option`, during a
//! turn too; the pills keep showing what the session reports, and the new
//! value shows when the session's `options_changed` record arrives. The
//! choice is also kept as the agent's default for a new chat, at the moment
//! Paseo keeps it (`view::composer::running::KeepAt`). A refusal is an
//! error toast in the app's words for the machine's answer.

use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{ActAnswer, ActKind, Command};
use vessel_app_view::composer::running::{Change, KeepAt, Refusal, keep_at, kept_with};
use vessel_app_view::toasts::ToastTone;
use vessel_proto::{HarnessRef, OptionChoice, SessionId};

use crate::routes::Route;
use crate::shell::Shell;

/// Whose options the composer's controls show and change.
///
/// [Decision Log]
/// - Purpose: a running session's model, effort and permissions change
///   from its composer with the draft's pills (Paseo's `AgentControls` on a
///   running agent; the coordinator's row a).
/// - Constraint: the pills, their lists, the effort card and the density
///   measure are the draft's state; the draft and a session are never on
///   screen together.
/// - Alternatives: a copy of the controls for the session (two drawings to
///   keep alike); moving every piece of state into a shared struct first
///   (a large move for one difference).
/// - Choice: the same state and drawing; the target, read from the route
///   (a live structured session on screen), decides where the options come
///   from and what a choice does.
/// - Cost: the popovers' state is shared, so a list open on the draft is
///   closed when the route changes (as it was).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ControlsTarget {
    /// The draft: its agent's probed options with the person's picks.
    Draft,
    /// A running structured session: its reported options; a choice is a
    /// `session.set_option` act.
    Session {
        machine: String,
        session: SessionId,
        agent: HarnessRef,
    },
}

impl Shell {
    /// Whose options the composer's controls show and change: the running
    /// structured session on screen, else the draft.
    pub(crate) fn controls_target(&self) -> ControlsTarget {
        let session = (self.route == Route::Session)
            .then_some(())
            .and(self.areas.session.open.as_ref())
            .filter(|open| open.structured && open.live)
            .and_then(|open| {
                let agent = self
                    .fleet
                    .machines
                    .iter()
                    .find(|machine| machine.name == open.machine)?
                    .sessions
                    .iter()
                    .find(|card| card.session == open.session)?
                    .detail
                    .harness
                    .clone()?;
                Some(ControlsTarget::Session {
                    machine: open.machine.clone(),
                    session: open.session,
                    agent,
                })
            });
        session.unwrap_or(ControlsTarget::Draft)
    }

    /// The agent the controls are for (the session's, or the draft's).
    pub(crate) fn controls_agent(&self) -> Option<HarnessRef> {
        match self.controls_target() {
            ControlsTarget::Draft => self.areas.draft.agent.clone(),
            ControlsTarget::Session { agent, .. } => Some(agent),
        }
    }

    /// The machine the controls are for (the session's, or the draft's).
    pub(crate) fn controls_machine(&self) -> Option<String> {
        match self.controls_target() {
            ControlsTarget::Draft => self.areas.draft.machine.clone(),
            ControlsTarget::Session { machine, .. } => Some(machine),
        }
    }

    /// Give the keys back to the composer the controls belong to (the
    /// session's or the draft's) after a list or the card closes.
    pub(crate) fn focus_controls_composer(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        match self.controls_target() {
            ControlsTarget::Session { .. } => self.areas.session.composer.focus(window, cx),
            ControlsTarget::Draft => self.areas.draft.composer.focus(window, cx),
        }
    }

    /// Change one of a running session's options: sent at once, during a
    /// turn too (Paseo blocks nothing; the agent says whether it takes it),
    /// and kept as the agent's default now or once taken.
    pub(crate) fn change_running_option(
        &mut self,
        machine: String,
        session: SessionId,
        agent: HarnessRef,
        choice: OptionChoice,
    ) {
        let reported = self.areas.conversation.timeline.options().unwrap_or(&[]);
        let keep = keep_at(reported, &choice.id);
        if keep == KeepAt::Sending {
            self.keep_running_choice(&machine, &agent, &choice);
        }
        self.areas.composer.changes.sent(Change {
            machine: machine.clone(),
            session,
            agent,
            choice: choice.clone(),
            keep,
        });
        self.command(Command::Act {
            machine,
            session,
            act: vessel_app_link::Act::SetOption(choice),
        });
    }

    /// Take the answer to an option change; any other act's answer is given
    /// back. A refusal is an error toast wherever the person is now (Paseo's
    /// `toast.error`), not the conversation's bar: the change was the
    /// composer's, and the person may have left the session.
    pub(crate) fn option_acted(
        &mut self,
        answer: ActAnswer,
        cx: &mut Context<'_, Self>,
    ) -> Option<ActAnswer> {
        if answer.kind != ActKind::SetOption {
            return Some(answer);
        }
        let change = self
            .areas
            .composer
            .changes
            .answered(&answer.machine, answer.session);
        match answer.result {
            Ok(_done) => {
                if let Some(change) = change.filter(|change| change.keep == KeepAt::Taken) {
                    self.keep_running_choice(&change.machine, &change.agent, &change.choice);
                }
            }
            Err(failed) => {
                // The machine's own words stay in the log (the toast says
                // the app's sentence; a tooltip or a copy of the original
                // comes with the shared mapping of codes).
                tracing::debug!(code = ?failed.code, said = %failed.message, "an option change was refused");
                match Refusal::of(failed.code).words(self.words) {
                    Ok(text) => self.toast(text, ToastTone::Error, cx),
                    Err(error) => {
                        tracing::error!(%error, "an option change's refusal could not be worded");
                    }
                }
            }
        }
        None
    }

    /// Keep `choice` among `agent`'s defaults for a new chat on `machine`
    /// (Paseo's `mergeProviderPreferences`), and save the app's choices.
    fn keep_running_choice(&mut self, machine: &str, agent: &HarnessRef, choice: &OptionChoice) {
        let kept = kept_with(self.prefs.choices(machine, agent), choice);
        self.prefs.keep_choices(machine, agent, &kept);
        self.command(Command::SavePrefs(self.prefs.clone()));
    }
}
