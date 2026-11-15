//! Creation callbacks stay correlated after leaving the draft's destination.

use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_link::creation::Answer;
use vessel_app_view::creation::{Completion, Transition};
use vessel_app_view::{Picks, agent_name, choices_for};

use crate::routes::Route;
use crate::shell::Shell;

impl Shell {
    /// Shared event registration forwards here; another draft cannot be settled.
    pub(crate) fn creation_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        match answer {
            Answer::Registered {
                ticket,
                machine,
                session,
            } => {
                if self.areas.draft.creation.ticket() == Some(ticket) {
                    self.areas.draft.registered = Some(vessel_app_view::Card::from_session(
                        machine.as_str(),
                        &session,
                    ));
                }
            }
            Answer::Progress { ticket, stage } => {
                let _transition = self.areas.draft.creation.progress(ticket, stage);
            }
            Answer::Complete(ticket) => {
                let completion = self.areas.draft.creation.complete(ticket);
                match completion {
                    Completion::Open => {
                        if let Some(card) = self.areas.draft.registered.take() {
                            self.started_with_files(&card, cx);
                        }
                    }
                    Completion::Background => {
                        self.areas.draft.registered = None;
                        self.draft_started();
                    }
                    Completion::Ignored => {}
                }
            }
            Answer::Failed {
                ticket, failure, ..
            } => {
                if self.areas.draft.creation.fail(ticket, failure) == Transition::Applied {
                    // Keep the committed resource and the original payload. The
                    // recovery UI decides between observing and first-input retry.
                    self.areas.draft.error = None;
                }
            }
        }
        cx.notify();
    }
}

impl Shell {
    /// Send the draft: create the session with its first prompt and the
    /// choices that change something, remember them, and open the
    /// conversation with the prompt shown.
    pub(crate) fn send_draft(
        &mut self,
        text: String,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if (text.trim().is_empty() && !self.draft_has_files()) || self.areas.draft.sending.is_some()
        {
            return;
        }
        self.settle_draft();
        let (Some(machine), Some(harness)) = (
            self.areas.draft.machine.clone(),
            self.areas.draft.agent.clone(),
        ) else {
            self.areas.draft.error = Some(self.t.choose_first.to_owned());
            cx.notify();
            return;
        };
        // A project chosen in the tray that cannot be started in yet
        // (track F's `workspace::place`).
        if let Some(why) = self.place_blocked() {
            self.areas.draft.error = Some(why);
            cx.notify();
            return;
        }
        let options = self
            .offer_options(&machine, &harness)
            .map(|offered| choices_for(offered, &self.effective_picks()));
        if let Some(options) = &options {
            self.prefs.keep_choices(&machine, &harness, options);
        }
        let options = options.unwrap_or_default();
        let folder = self.areas.draft.folder.trim().to_owned();
        self.prefs.machine = Some(machine.clone());
        self.prefs.agent = Some(harness.clone());
        if folder.is_empty() {
            let _forgotten = self.prefs.folders.remove(&machine);
        } else {
            let _earlier = self.prefs.folders.insert(machine.clone(), folder.clone());
        }
        self.command(Command::SavePrefs(self.prefs.clone()));
        // In a project, track F's start (a new worktree made first, its
        // steps on the session's screen); else in the folder.
        if !self.start_in_place(&machine, &harness, &options, &text) {
            self.command(self.with_draft_files(Command::Start {
                machine: machine.clone(),
                harness: harness.clone(),
                folder: (!folder.is_empty()).then_some(folder),
                prompt: text.clone(),
                options,
            }));
        }
        let draft = &mut self.areas.draft;
        draft.error = None;
        draft.sending = Some(text.clone());
        // The next draft begins from what this one started with.
        draft.picks = Picks::default();
        draft.chosen = false;
        draft.composer.clear(window, cx);
        self.begin_pending(machine, agent_name(&harness), text, cx);
        self.brand.sent();
        self.route = Route::Session;
        cx.notify();
    }
}

impl Shell {
    /// The session started: the draft is spent.
    pub(crate) fn draft_started(&mut self) {
        self.areas.draft.sending = None;
    }

    /// The start failed: back to the draft, its text restored.
    pub(crate) fn start_failed(&mut self) {
        if self.areas.draft.sending.is_some() {
            self.route_to(Route::Draft);
            let text = self.areas.draft.sending.take();
            *self.areas.draft.restore.borrow_mut() = text;
        }
    }
}
