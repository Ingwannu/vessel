//! A chat started in a new worktree: the session's screen opens at once
//! with the prompt, and this card under it shows the worktree being made,
//! the commands' approval when one is needed, and setup's output, until
//! the session is created (GOAT prepares the environment in the thread;
//! Paseo's setup panel, `panels/setup-panel.tsx`, is the card's look: a
//! bordered item, its header on the raised surface with the state's icon,
//! the words and a chevron, the log under it on the base surface, 400 high
//! at most, in mono 20 apart).
//!
//! [Decision Log]
//! - Purpose: Send must not hold the person on the new-chat screen while
//!   a worktree is made and set up (the owner's complaint about the page).
//! - Constraint: the session exists only when the worktree is ready; the
//!   approval is a person's choice; the walk is the link's
//!   (`link::workspace::start`).
//! - Alternatives: a modal over the composer (the page's, rejected);
//!   Paseo's setup as a separate panel of the workspace (Vessel has no
//!   workspace screen before the session exists).
//! - Choice: the lead's pending screen draws this card where it says
//!   "starting"; the steps arrive as `Answer::Start`, the end as
//!   `Answer::StartEnded`.
//! - Cost: leaving the screen does not stop the start (the card is back on
//!   return while it runs); Stop drops the answers' channel, which ends the
//!   walk at its next step.
//! - Differences from Paseo: the node streams setup as one log, so there
//!   is one item, not one per command.

use tokio::sync::mpsc;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Start, Tokens, Type, TypeExt as _, button, output_box};
use vessel_app_link::workspace::{StartAnswer, StartEnded, StartKey};
use vessel_app_view::workspace::command_changes::Listed;
use vessel_app_view::workspace::setup_log::SetupLog;
use vessel_app_view::workspace::start::{ApprovalAnswer, FailedAnswer, Step};
use vessel_proto::workspace::SetupState;

use super::approval::{self, Actions, Panel};
use super::start_header::header;
use crate::shell::Shell;

/// A start in flight.
pub(crate) struct Starting {
    /// Which start the answers belong to.
    pub(crate) key: StartKey,
    /// Where.
    pub(crate) machine: String,
    /// The project's name.
    pub(crate) project: String,
    /// Whether a worktree is made first (the card shows only then).
    pub(crate) worktree: bool,
    /// The step shown.
    pub(crate) step: Step,
    /// Setup's output so far.
    pub(crate) log: SetupLog,
    /// The person's answers; `None` once stopped.
    pub(crate) answers: Option<mpsc::Sender<StartAnswer>>,
    /// The log folded away by the person (it shows while setup runs or
    /// failed, as Paseo's running command expands on its own).
    pub(crate) folded: bool,
}

impl Shell {
    /// A step of a start arrived.
    pub(super) fn start_step(&mut self, key: StartKey, step: Step) {
        let Some(start) = self.areas.workspace.start.as_mut() else {
            return;
        };
        if start.key != key {
            return;
        }
        match step {
            Step::Output(text) => start.log.append(&text),
            step @ (Step::Idle
            | Step::Making
            | Step::Approval { .. }
            | Step::Refused(_)
            | Step::Setup { .. }) => start.step = step,
        }
    }

    /// A start ended: open the session, or back to the draft with why.
    pub(super) fn start_ended(
        &mut self,
        key: StartKey,
        ended: StartEnded,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(start) = self.areas.workspace.start.take_if(|start| start.key == key) else {
            return;
        };
        match ended {
            StartEnded::Started(mut card) => {
                // Renamed in the book while it started: the session is the
                // machine's under its new name.
                if card.machine != start.machine {
                    card.machine.clone_from(&start.machine);
                }
                self.started(&card, cx);
                self.command(vessel_app_link::Command::Reload);
            }
            StartEnded::Stopped => {
                if let Step::Refused(refusal) = &start.step {
                    self.areas.draft.error = Some(self.words().start_refusal(refusal));
                }
                self.start_failed();
            }
            StartEnded::Failed(problem) => {
                self.areas.draft.error = Some(self.words().problem(&problem));
                self.start_failed();
            }
        }
    }

    /// Send the person's answer to the walk.
    fn start_answer(&mut self, answer: StartAnswer, cx: &mut Context<'_, Self>) {
        if let Some(start) = self.areas.workspace.start.as_mut() {
            if let (StartAnswer::Approval(ApprovalAnswer::Approve), Step::Approval { busy, .. }) =
                (answer, &mut start.step)
            {
                *busy = true;
            }
            if let Some(answers) = &start.answers
                && answers.try_send(answer).is_err()
            {
                tracing::warn!("the start no longer takes answers");
            }
        }
        cx.notify();
    }

    /// Stop the start: nothing more is asked and no session is started.
    pub(super) fn stop_start(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(start) = self.areas.workspace.start.as_mut() {
            start.answers = None;
        }
        cx.notify();
    }
}

/// The card, when a start that makes a worktree is in flight.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let start = shell.areas.workspace.start.as_ref()?;
    if !start.worktree {
        return None;
    }
    let neutral = tokens.neutral;
    let mut card = div()
        .w_full()
        .flex()
        .flex_col()
        .rounded(px(8.0))
        .border_1()
        .border_color(neutral.border)
        .overflow_hidden()
        .child(header(shell, start, tokens, cx));
    if let Step::Approval {
        commands,
        compared,
        busy,
    } = &start.step
        && let Some(asked) = Listed::of(commands)
    {
        let place = shell
            .words()
            .approval_place(&start.project, &start.machine, false);
        card = card.child(
            div()
                .p(px(12.0))
                .bg(neutral.surface0)
                .child(approval::render(
                    shell,
                    &Panel {
                        id: "start-approval",
                        asked: &asked,
                        compared,
                        place: &place,
                        busy: *busy,
                    },
                    Actions {
                        approve: Box::new(|this, _window, cx| {
                            this.start_answer(StartAnswer::Approval(ApprovalAnswer::Approve), cx);
                        }),
                        skip: Some(Box::new(|this, _window, cx| {
                            this.start_answer(StartAnswer::Approval(ApprovalAnswer::Skip), cx);
                        })),
                        cancel: Box::new(|this, _window, cx| {
                            this.start_answer(StartAnswer::Approval(ApprovalAnswer::Cancel), cx);
                        }),
                    },
                    tokens,
                    cx,
                )),
        );
    }
    if shows_log(start) {
        let shown = start.log.shown();
        let body: AnyElement = if shown.is_empty() {
            div()
                .p(px(12.0))
                .type_(Type::Body)
                .italic()
                .text_color(neutral.foreground_muted)
                .child(shell.words().labels().no_output)
                .into_any_element()
        } else {
            div()
                .max_h(px(400.0))
                .child(output_box(
                    "start-log",
                    &shown.join("\n"),
                    Start::End,
                    tokens,
                    window,
                    cx,
                ))
                .into_any_element()
        };
        card = card.child(
            div()
                .border_t_1()
                .border_color(neutral.border)
                .bg(neutral.surface0)
                .child(body),
        );
    }
    card = card.children(failed_actions(shell, start, tokens, cx));
    Some(card.into_any_element())
}

/// Whether the log is out: while setup runs or after it failed, unless
/// folded; never before setup.
fn shows_log(start: &Starting) -> bool {
    let Step::Setup { setup, .. } = &start.step else {
        return false;
    };
    !start.folded
        && match setup {
            SetupState::Running { .. } | SetupState::Failed { .. } => true,
            SetupState::None
            | SetupState::Skipped
            | SetupState::Succeeded { .. }
            | SetupState::Unknown => false,
        }
}

/// After a failed setup: try again, start anyway, or stop.
fn failed_actions(
    shell: &Shell,
    start: &Starting,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    let Step::Setup {
        setup: SetupState::Failed { .. },
        ..
    } = &start.step
    else {
        return None;
    };
    let labels = shell.words().labels();
    let answer = |answer: FailedAnswer| {
        cx.listener(
            move |this: &mut Shell, _event: &_, _window: &mut Window, cx| {
                this.start_answer(StartAnswer::Failed(answer), cx);
            },
        )
    };
    Some(
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .justify_end()
            .gap(px(8.0))
            .p(px(12.0))
            .border_t_1()
            .border_color(tokens.neutral.border)
            .child(
                button("start-stop-failed", labels.stop, false, tokens)
                    .on_click(answer(FailedAnswer::Stop)),
            )
            .child(
                button("start-anyway", labels.start_anyway, false, tokens)
                    .on_click(answer(FailedAnswer::Start)),
            )
            .child(
                button("start-retry", labels.try_again, true, tokens)
                    .on_click(answer(FailedAnswer::Retry)),
            ),
    )
}
