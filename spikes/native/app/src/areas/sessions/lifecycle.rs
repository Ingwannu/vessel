//! Confirmed lifecycle mutations settle before navigation or fresh-draft creation.
use crate::{routes::Route, shell::Shell};
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, DialogContent, DialogSize, DialogState, Tokens, Type, TypeExt as _,
    breakpoint, button, button_with_kind, dialog,
};
use vessel_app_link::{
    Acted, Command,
    sessions::lifecycle::{Answer, Failure},
};
use vessel_app_view::machine_book::Name;
use vessel_app_view::sessions::{Lifecycle, LifecycleAction, LifecycleTarget};
use vessel_proto::SessionInfo;

pub(super) struct State {
    running: Lifecycle,
    asking: Option<Prepared>,
    dialog: DialogState,
    after: Option<Prepared>,
    ready: Option<Prepared>,
    error: Option<(LifecycleTarget, Problem)>,
}
enum Problem {
    Busy,
    ExistingDraft,
    ProfileUnavailable,
    Refused(vessel_app_link::ActFailed),
    Uncertain,
    Offline,
    WrongReply,
}
#[derive(Clone)]
struct Prepared {
    target: LifecycleTarget,
    action: LifecycleAction,
    seed: Option<Box<SessionInfo>>,
    choices: Vec<vessel_proto::OptionChoice>,
}
impl State {
    pub(super) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            running: Lifecycle::default(),
            asking: None,
            dialog: DialogState::new(cx),
            after: None,
            ready: None,
            error: None,
        }
    }
    pub(super) fn navigation_changed(&mut self) {
        self.asking = None;
        self.after = None;
        self.ready = None;
    }
}
impl Shell {
    /// A child closes in local layout; a root archives before closing.
    pub(crate) fn request_session_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        use vessel_app_view::sessions::{ClosePolicy, close_policy};
        let policy = self
            .areas
            .conversation
            .info
            .as_deref()
            .map(|info| close_policy(&info.created_by));
        match policy {
            Some(ClosePolicy::LayoutOnly) => self.go(Route::Draft, window, cx),
            Some(ClosePolicy::Archive) => {
                self.prepare_session_lifecycle(LifecycleAction::Exit, window, cx);
            }
            None => {
                self.notice = Some(self.words.sessions().navigation_busy().to_owned());
                cx.notify();
            }
        }
    }

    /// Composer built-ins delegate here so they cannot close before archive succeeds.
    pub(crate) fn composer_exit(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.prepare_session_lifecycle(LifecycleAction::Exit, window, cx);
    }
    /// Preserve the current launch config while starting a separate conversation.
    pub(crate) fn composer_fresh(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.prepare_session_lifecycle(LifecycleAction::Fresh, window, cx);
    }
    pub(crate) fn sessions_lifecycle_busy(&self) -> bool {
        self.areas.sessions.lifecycle.running.pending().is_some()
    }
    pub(crate) fn prepare_session_lifecycle(
        &mut self,
        action: LifecycleAction,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.sessions_lifecycle_busy() {
            return;
        }
        let Some(prepared) = self.prepare_current_lifecycle(action, cx) else {
            return;
        };
        match action {
            LifecycleAction::Archive | LifecycleAction::Delete => {
                self.areas.sessions.lifecycle.asking = Some(prepared);
                self.areas.sessions.lifecycle.dialog.focus(window, cx);
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
        let seed = self.areas.conversation.info.clone();
        if let Some(problem) = self.fresh_lifecycle_problem(action, seed.as_deref(), cx) {
            self.areas.sessions.lifecycle.error = Some((
                LifecycleTarget {
                    host,
                    session: open.session,
                    node: open.binding.as_ref().map(|binding| binding.address.node),
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
                node: open.binding.as_ref().map(|binding| binding.address.node),
            },
            action,
            seed,
            choices,
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
                let words = self.t.lang.conversation();
                self.notice = Some(deleted.worktree.as_ref().map_or_else(
                    || words.session_deleted().to_owned(),
                    |worktree| {
                        format!(
                            "{}. {}",
                            words.session_deleted(),
                            words.deleted_worktree(&worktree.outcome)
                        )
                    },
                ));
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
                Failure::Uncertain => Problem::Uncertain,
                Failure::Unsent => Problem::Offline,
                Failure::IdentityChanged => Problem::WrongReply,
            }),
        }
    }
    pub(crate) fn settle_session_lifecycle(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
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
    fn dismiss_session_lifecycle(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.sessions.lifecycle.asking = None;
        cx.notify();
    }
    fn confirm_session_lifecycle(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(prepared) = self.areas.sessions.lifecycle.asking.take() {
            self.send_session_lifecycle(prepared, cx);
        }
    }
}
pub(crate) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.sessions.lifecycle;
    let asking = state.asking.as_ref()?;
    let words = shell.t.lang.conversation();
    let destructive = asking.action == LifecycleAction::Delete;
    let title = if destructive {
        words.delete_title()
    } else {
        words.archive_title()
    };
    let body = if destructive {
        words.delete_words(asking.target.host.as_str())
    } else {
        words.archive_words(
            shell
                .areas
                .session
                .open
                .as_ref()
                .is_some_and(|open| open.live),
            asking.target.host.as_str(),
            shell
                .fleet
                .protocol_of(asking.target.host.as_str())
                .is_some_and(|protocol| protocol >= vessel_app_view::session::ENDS_BESIDE_PROTOCOL),
        )
    };
    Some(dialog(
        &state.dialog,
        DialogContent {
            title: title.into(),
            body: div().type_(Type::Body).child(body).into_any_element(),
            footer: footer(shell, asking.action, tokens, cx),
            size: DialogSize::Normal,
        },
        tokens,
        window,
        breakpoint::phone(window),
        Shell::dismiss_session_lifecycle,
        cx,
    ))
}

/// Only the session whose operation failed shows the retained refusal.
pub(crate) fn error(shell: &Shell, tokens: &Tokens) -> Option<AnyElement> {
    let (target, problem) = shell.areas.sessions.lifecycle.error.as_ref()?;
    let open = shell.areas.session.open.as_ref()?;
    if open.machine != target.host.as_str() || open.session != target.session {
        return None;
    }
    let words = shell.words.sessions();
    let message = match problem {
        Problem::Busy => words.navigation_busy(),
        Problem::ExistingDraft => words.existing_draft(),
        Problem::ProfileUnavailable => words.profile_unavailable(),
        Problem::Refused(failed) => &failed.message,
        Problem::Uncertain => words.outcome_uncertain(),
        Problem::Offline => words.navigation_offline(),
        Problem::WrongReply => words.navigation_refused(),
    };
    Some(
        div()
            .type_(Type::Body)
            .text_color(tokens.neutral.destructive)
            .child(message.to_owned())
            .into_any_element(),
    )
}

fn footer(
    shell: &Shell,
    action: LifecycleAction,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let words = shell.t.lang.conversation();
    vec![
        button("sessions-lifecycle-cancel", words.cancel(), false, tokens)
            .min_h(vessel_app_kit::ui::px(breakpoint::TOUCH_TARGET))
            .on_click(
                cx.listener(|shell, _, window, cx| shell.dismiss_session_lifecycle(window, cx)),
            )
            .into_any_element(),
        button_with_kind(
            "sessions-lifecycle-confirm",
            words.confirm(),
            if action == LifecycleAction::Delete {
                ButtonKind::Destructive
            } else {
                ButtonKind::Primary
            },
            ButtonState::Enabled,
            tokens,
        )
        .min_h(vessel_app_kit::ui::px(breakpoint::TOUCH_TARGET))
        .on_click(cx.listener(|shell, _, window, cx| shell.confirm_session_lifecycle(window, cx)))
        .into_any_element(),
    ]
}
