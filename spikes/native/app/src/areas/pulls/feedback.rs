//! Whether the pull request's failing checks and reviews are sent to the
//! session (`pr.feedback`, ADR 0029's feedback): a switch in the facts,
//! drawn as Paseo's settings row (the words left, the switch right). The
//! node owns the state (`PullRequestView.feedback`); the switch shows the
//! node's answer, not a guess, and waits while a change is out.

use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_link::workspace::{Ask, PullAsk};
use vessel_app_view::toasts::ToastTone;
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::forge::{SessionPullRequest, SetFeedback};

use super::FactsRead;
use crate::shell::Shell;

impl Shell {
    /// The switch pressed: ask the node for the other state.
    fn pulls_toggle_feedback(&mut self, cx: &mut Context<'_, Self>) {
        let pulls = &self.areas.pulls;
        if pulls.feedback_sending.is_some() {
            return;
        }
        let Some(facts) = pulls.facts.as_ref() else {
            return;
        };
        let FactsRead::Ready(view) = &facts.read else {
            return;
        };
        let ask = SetFeedback {
            session: facts.subject.session,
            enabled: !view.feedback,
        };
        let machine = facts.subject.machine.clone();
        self.areas.pulls.feedback_sending = Some(ask.clone());
        self.ask(&machine, Ask::Pull(PullAsk::Feedback(ask)));
        cx.notify();
    }

    /// What `pr.feedback` answered: the facts take the node's state.
    pub(super) fn pulls_feedback_answered(
        &mut self,
        ask: &SetFeedback,
        result: Result<Box<SessionPullRequest>, Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        let pulls = &mut self.areas.pulls;
        if pulls.feedback_sending.as_ref() != Some(ask) {
            return;
        }
        pulls.feedback_sending = None;
        pulls.feedback_turn = pulls.feedback_turn.wrapping_add(1);
        match result {
            Ok(now) => {
                if let Some(FactsRead::Ready(view)) = pulls
                    .facts
                    .as_mut()
                    .filter(|facts| facts.subject.session == ask.session)
                    .map(|facts| &mut facts.read)
                {
                    view.feedback = now.feedback;
                }
            }
            Err(problem) => {
                let failure = self.t.lang.git_call_failure(&problem);
                self.toast(failure.title, ToastTone::Error, cx);
            }
        }
    }
}

/// The switch's row, for a pull request read whole.
pub(super) fn row(
    shell: &Shell,
    on: bool,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let pulls = &shell.areas.pulls;
    let enabled = pulls.feedback_sending.is_none();
    div()
        .id("pulls-feedback-row")
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(spacing::S3))
        .cursor_pointer()
        .child(
            div()
                .min_w_0()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .child(shell.words.pulls().feedback().to_owned()),
        )
        .child(vessel_app_kit::settings::switch(
            "pulls-feedback",
            on,
            enabled,
            touch,
            pulls.feedback_turn,
            tokens,
        ))
        .on_click(cx.listener(|shell, _event, _window, cx| shell.pulls_toggle_feedback(cx)))
        .into_any_element()
}
