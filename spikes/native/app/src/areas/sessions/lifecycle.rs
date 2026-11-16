//! Confirmed lifecycle mutations settle before navigation or fresh-draft creation.
mod draw;
#[cfg(test)]
mod tests;
use crate::{routes::Route, shell::Shell};
pub(crate) use draw::{error, overlay};
use vessel_app_kit::DialogState;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{
    Acted, Command,
    sessions::lifecycle::{Answer, Failure},
};
use vessel_app_view::machine_book::Name;
use vessel_app_view::sessions::{Lifecycle, LifecycleAction, LifecycleTarget, LifecycleTicket};
use vessel_proto::SessionInfo;

pub(super) struct State {
    running: Lifecycle,
    /// Independent monotonic tickets for UI decisions; never sent to the link.
    confirming: Lifecycle,
    asking: Option<Prepared>,
    dialog: DialogState,
    dialog_focus: DialogFocus,
    after: Option<Prepared>,
    ready: Option<Prepared>,
    error: Option<(LifecycleTarget, Problem)>,
}
enum DialogFocus {
    Settled,
    Return,
}
#[derive(Debug, Clone, Copy)]
pub(super) enum HistoryAction {
    Archive,
    Unarchive,
}
enum Problem {
    Busy,
    ExistingDraft,
    ProfileUnavailable,
    Refused(vessel_app_link::ActFailed),
    LostReply(vessel_app_link::ActFailed),
    Uncertain,
    Offline,
    WrongReply,
    DestinationChanged,
    IdentityUnavailable,
}
#[derive(Clone)]
struct Prepared {
    target: LifecycleTarget,
    action: LifecycleAction,
    seed: Option<Box<SessionInfo>>,
    choices: Vec<vessel_proto::OptionChoice>,
    directory: Option<vessel_app_view::session_marks::CapturedSessionTarget>,
}
impl State {
    pub(super) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            running: Lifecycle::default(),
            confirming: Lifecycle::default(),
            asking: None,
            dialog: DialogState::new(cx),
            dialog_focus: DialogFocus::Settled,
            after: None,
            ready: None,
            error: None,
        }
    }
    pub(super) fn navigation_changed(&mut self) {
        if let Some(ticket) = self.confirming.pending().map(|pending| pending.ticket) {
            let _cancelled = self.confirming.finish(ticket);
        }
        if self.asking.take().is_some() {
            self.dialog_focus = DialogFocus::Return;
        }
        self.after = None;
        self.ready = None;
    }
}
impl Shell {
    /// Current parentage, rather than immutable creator provenance, authorizes close.
    pub(crate) fn request_session_close(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        // The reviewed W2 parentage producer is not wired yet. Unknown must
        // never become a root archive or a hidden detached root by guessing.
        self.notice = Some(self.words.sessions().parentage_unavailable().to_owned());
        cx.notify();
    }

    /// Composer built-ins delegate here so they cannot close before archive succeeds.
    #[expect(
        dead_code,
        reason = "composer command dispatcher consumes this compiled lifecycle seam"
    )]
    pub(crate) fn composer_exit(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.prepare_session_lifecycle(LifecycleAction::Exit, window, cx);
    }
    /// Preserve the current launch config while starting a separate conversation.
    #[expect(
        dead_code,
        reason = "composer command dispatcher consumes this compiled lifecycle seam"
    )]
    pub(crate) fn composer_fresh(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.prepare_session_lifecycle(LifecycleAction::Fresh, window, cx);
    }
    pub(crate) fn sessions_lifecycle_busy(&self) -> bool {
        let state = &self.areas.sessions.lifecycle;
        state.running.pending().is_some()
            || state.confirming.pending().is_some()
            || self.areas.session_marks.is_pending()
    }
    pub(crate) fn prepare_session_lifecycle(
        &mut self,
        action: LifecycleAction,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.settle_lifecycle_dialog(window, cx);
        if self.sessions_lifecycle_busy() {
            return;
        }
        let Some(prepared) = self.prepare_current_lifecycle(action, cx) else {
            return;
        };
        match action {
            LifecycleAction::Archive | LifecycleAction::Delete => {
                self.ask_session_lifecycle(prepared, window, cx);
            }
            LifecycleAction::Unarchive
            | LifecycleAction::Resume
            | LifecycleAction::Exit
            | LifecycleAction::Fresh => {
                self.send_session_lifecycle(prepared, cx);
            }
        }
        cx.notify();
    }
    /// History callbacks retain the rendered node, never a newer alias binding.
    pub(super) fn history_lifecycle(
        &mut self,
        captured: vessel_app_view::session_marks::CapturedSessionTarget,
        action: HistoryAction,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.settle_lifecycle_dialog(window, cx);
        if self.sessions_lifecycle_busy() {
            return;
        }
        let Ok(host) = Name::parse(captured.context().machine().as_str()) else {
            self.notice = Some(self.words.session_marks().unavailable().to_owned());
            cx.notify();
            return;
        };
        let target = LifecycleTarget {
            host,
            session: captured.session(),
            node: Some(captured.context().node()),
        };
        let effect = match action {
            HistoryAction::Archive => LifecycleAction::Archive,
            HistoryAction::Unarchive => LifecycleAction::Unarchive,
        };
        let prepared = Prepared {
            target,
            action: effect,
            seed: None,
            choices: Vec::new(),
            directory: Some(captured),
        };
        match action {
            HistoryAction::Archive => self.ask_session_lifecycle(prepared, window, cx),
            HistoryAction::Unarchive => self.send_session_lifecycle(prepared, cx),
        }
        cx.notify();
    }
    fn ask_session_lifecycle(
        &mut self,
        prepared: Prepared,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.sessions.lifecycle;
        match state
            .confirming
            .begin(prepared.target.clone(), prepared.action)
        {
            Ok(_) => {
                state.asking = Some(prepared);
                state.dialog.focus(window, cx);
            }
            Err(_) => state.error = Some((prepared.target, Problem::Busy)),
        }
    }
    fn prepare_current_lifecycle(
        &mut self,
        action: LifecycleAction,
        cx: &mut Context<'_, Self>,
    ) -> Option<Prepared> {
        let open = self.areas.session.open.as_ref()?;
        let Ok(host) = Name::parse(&open.machine) else {
            self.notice = Some(self.words.sessions().navigation_refused().to_owned());
            cx.notify();
            return None;
        };
        let Some(address) = self.opened_session_address(&host, open.session) else {
            self.areas.sessions.lifecycle.error = Some((
                LifecycleTarget {
                    host,
                    session: open.session,
                    node: None,
                },
                Problem::IdentityUnavailable,
            ));
            cx.notify();
            return None;
        };
        let node = Some(address.node);
        let seed = self.areas.conversation.info.clone();
        if let Some(problem) = self.fresh_lifecycle_problem(action, seed.as_deref(), cx) {
            self.areas.sessions.lifecycle.error = Some((
                LifecycleTarget {
                    host,
                    session: open.session,
                    node,
                },
                problem,
            ));
            cx.notify();
            return None;
        }
        let choices = self.lifecycle_choices(seed.as_deref());
        Some(Prepared {
            target: LifecycleTarget {
                host,
                session: open.session,
                node,
            },
            action,
            seed,
            choices,
            directory: None,
        })
    }
    fn fresh_lifecycle_problem(
        &self,
        action: LifecycleAction,
        seed: Option<&SessionInfo>,
        cx: &Context<'_, Self>,
    ) -> Option<Problem> {
        if action != LifecycleAction::Fresh {
            return None;
        }
        if !self.areas.draft.composer.text(cx).trim().is_empty() {
            return Some(Problem::ExistingDraft);
        }
        let Some(seed) = seed else {
            return Some(Problem::Busy);
        };
        seed.profile
            .is_some()
            .then_some(Problem::ProfileUnavailable)
    }
    fn lifecycle_choices(&self, seed: Option<&SessionInfo>) -> Vec<vessel_proto::OptionChoice> {
        self.areas.conversation.timeline.options().map_or_else(
            || {
                seed.map(|info| info.spec.mode.options().to_vec())
                    .unwrap_or_default()
            },
            |options| {
                options
                    .iter()
                    .filter_map(vessel_proto::SessionOption::choice)
                    .collect()
            },
        )
    }
    fn send_session_lifecycle(&mut self, prepared: Prepared, cx: &mut Context<'_, Self>) {
        if let Some(target) = prepared.directory.as_ref() {
            let mark = match prepared.action {
                LifecycleAction::Archive => vessel_proto::Mark::Archive,
                LifecycleAction::Unarchive => vessel_proto::Mark::Unarchive,
                LifecycleAction::Delete
                | LifecycleAction::Resume
                | LifecycleAction::Exit
                | LifecycleAction::Fresh => return,
            };
            self.mark_session_target(target.clone(), mark, cx);
            return;
        }
        let area = &mut self.areas.sessions.lifecycle;
        match area.running.begin(prepared.target.clone(), prepared.action) {
            Ok(request) => {
                area.error = None;
                area.after = Some(prepared);
                self.command(Command::SessionLifecycle(request));
            }
            Err(_) => area.error = Some((prepared.target, Problem::Busy)),
        }
        cx.notify();
    }
    pub(crate) fn sessions_lifecycle_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        let Some(request) = self.areas.sessions.lifecycle.running.finish(answer.ticket) else {
            return;
        };
        let after = self.areas.sessions.lifecycle.after.take();
        match self.apply_lifecycle_result(&request, answer.result) {
            Ok(()) => self.areas.sessions.lifecycle.ready = after,
            Err(problem) => {
                if self.route == Route::Sessions {
                    self.notice = Some(draw::problem_words(self, &problem));
                }
                self.areas.sessions.lifecycle.error = Some((request.target, problem));
                self.reload();
            }
        }
        cx.notify();
    }
    fn apply_lifecycle_result(
        &mut self,
        request: &vessel_app_view::sessions::PendingLifecycle,
        result: Result<Acted, Failure>,
    ) -> Result<(), Problem> {
        if !self.lifecycle_target_current(&request.target) {
            return Err(Problem::DestinationChanged);
        }
        match result {
            Ok(Acted::Session(info))
                if info.id == request.target.session
                    && vessel_app_view::sessions::satisfies_lifecycle(request.action, &info) =>
            {
                self.fleet.put_session(request.target.host.as_str(), &info);
                if self.areas.conversation.shown.as_ref().is_some_and(|shown| {
                    shown.machine == request.target.host.as_str()
                        && shown.session == request.target.session
                }) {
                    self.areas.conversation.info = Some(info);
                }
                Ok(())
            }
            Ok(Acted::Deleted(deleted)) if request.action == LifecycleAction::Delete => {
                self.notice = Some(self.lifecycle_deleted_words(&deleted));
                self.fleet
                    .drop_session(request.target.host.as_str(), request.target.session);
                Ok(())
            }
            Ok(
                Acted::Session(_)
                | Acted::Deleted(_)
                | Acted::Forked(_)
                | Acted::Log(_)
                | Acted::Done
                | Acted::Conversation(_),
            ) => Err(Problem::WrongReply),
            Err(failure) => Err(match failure {
                Failure::Refused(failed) => Problem::Refused(failed),
                Failure::LostReply(failed) => Problem::LostReply(failed),
                Failure::Uncertain => Problem::Uncertain,
                Failure::Unsent => Problem::Offline,
                Failure::IdentityChanged => Problem::DestinationChanged,
            }),
        }
    }
    fn lifecycle_target_current(&self, target: &LifecycleTarget) -> bool {
        let node = self
            .fleet
            .machines
            .iter()
            .find(|machine| machine.name == target.host.as_str())
            .and_then(|machine| machine.node);
        target.matches_node(node)
    }
    fn lifecycle_deleted_words(
        &self,
        deleted: &vessel_proto::conversation::SessionDeleted,
    ) -> String {
        let words = self.t.lang.conversation();
        deleted.worktree.as_ref().map_or_else(
            || words.session_deleted().to_owned(),
            |worktree| {
                format!(
                    "{}. {}",
                    words.session_deleted(),
                    words.deleted_worktree(&worktree.outcome)
                )
            },
        )
    }
    pub(crate) fn settle_session_lifecycle(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.settle_lifecycle_dialog(window, cx);
        let Some(ready) = self.areas.sessions.lifecycle.ready.take() else {
            return;
        };
        if self.route != Route::Session
            || self.areas.session.open.as_ref().is_none_or(|open| {
                open.machine != ready.target.host.as_str() || open.session != ready.target.session
            })
        {
            return;
        }
        match ready.action {
            LifecycleAction::Exit | LifecycleAction::Delete => self.go(Route::Draft, window, cx),
            LifecycleAction::Fresh => {
                if let Some(seed) = ready.seed {
                    self.go(Route::Draft, window, cx);
                    self.choose_machine(ready.target.host.as_str().to_owned(), window, cx);
                    self.choose_agent(seed.spec.harness.clone());
                    self.choose_folder(
                        seed.spec
                            .cwd
                            .as_ref()
                            .map(|path| path.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    );
                    for choice in ready.choices {
                        self.choose_option(choice);
                    }
                    self.areas.draft.composer.clear(window, cx);
                }
            }
            LifecycleAction::Resume => {
                if let Some(card) = self
                    .fleet
                    .machines
                    .iter()
                    .flat_map(|machine| &machine.sessions)
                    .find(|card| {
                        card.machine == ready.target.host.as_str()
                            && card.session == ready.target.session
                    })
                {
                    self.open_session(crate::areas::session::Open::of(card), window, cx);
                }
            }
            LifecycleAction::Archive | LifecycleAction::Unarchive => {}
        }
        cx.notify();
    }
    fn settle_lifecycle_dialog(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.sessions.lifecycle;
        if matches!(state.dialog_focus, DialogFocus::Return) {
            state.dialog.dismissed(window, cx);
            state.dialog_focus = DialogFocus::Settled;
        }
    }
    fn dismiss_session_lifecycle(
        &mut self,
        ticket: LifecycleTicket,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.sessions.lifecycle;
        if state.confirming.finish(ticket).is_none() {
            return;
        }
        state.asking = None;
        state.dialog.dismissed(window, cx);
        cx.notify();
    }
    fn confirm_session_lifecycle(
        &mut self,
        ticket: LifecycleTicket,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self
            .areas
            .sessions
            .lifecycle
            .confirming
            .finish(ticket)
            .is_none()
        {
            return;
        }
        if let Some(prepared) = self.areas.sessions.lifecycle.asking.take() {
            self.areas.sessions.lifecycle.dialog.dismissed(window, cx);
            self.send_session_lifecycle(prepared, cx);
        }
    }
}
