//! The pending owner retains the final payload until its exact result arrives.
//! An unknown legacy RPC outcome stays locked; it cannot reupload or resend.
use crate::shell::Shell;
use std::sync::Arc;
use vessel_app_kit::ui::Context;
use vessel_app_link::{
    Command,
    attachments::{Attach, Files, SubmitFailure, WithFiles},
};
use vessel_app_view::composer::{context::SubmissionOutcome, drafts::DraftKey, tray::Readiness};
use vessel_proto::{Delivered, SessionId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Phase {
    Preparing,
    Submitting(Arc<WithFiles>),
    Retained {
        payload: Arc<WithFiles>,
        problem: SubmitFailure,
    },
}

impl Shell {
    pub(crate) fn composer_pump(&mut self, session: SessionId, cx: &mut Context<'_, Self>) {
        let composer = &mut self.areas.composer;
        if composer.capabilities.blocked(session) {
            cx.notify();
            return;
        }
        let Some(waiting) = composer.sends.get(&session).cloned() else {
            cx.notify();
            return;
        };
        if waiting.phase != Phase::Preparing {
            return;
        }
        let tray = composer.trays.tray(DraftKey::Session { session });
        tray.send();
        let mut commands: Vec<Command> = tray
            .starts()
            .into_iter()
            .map(|(item, resume)| {
                Command::Files(Files::Send(Attach {
                    machine: waiting.machine.clone(),
                    expected_node: waiting.expected_node,
                    session,
                    key: item.key,
                    name: item.name,
                    mime: item.mime,
                    size: item.size,
                    origin: item.origin,
                    resume,
                }))
            })
            .collect();
        if let Readiness::Ready(attachments) = tray.readiness() {
            super::sent::seed(
                &composer.sent,
                waiting.machine.as_str(),
                waiting.expected_node,
                session,
                tray,
            );
            let prompt = Arc::new(WithFiles {
                message: waiting.message,
                machine: waiting.machine,
                expected_node: waiting.expected_node,
                session,
                text: waiting.text,
                context: waiting.context,
                attachments,
                delivery: waiting.delivery,
            });
            if let Some(waiting) = composer.sends.get_mut(&session) {
                waiting.phase = Phase::Submitting(Arc::clone(&prompt));
            }
            commands.push(Command::Files(Files::Prompt(prompt)));
        }
        for command in commands {
            self.command(command);
        }
        cx.notify();
    }

    pub(super) fn composer_submitted(
        &mut self,
        prompt: Arc<WithFiles>,
        result: Result<Delivered, SubmitFailure>,
        cx: &mut Context<'_, Self>,
    ) {
        let session = prompt.session;
        let Some(waiting) = self.areas.composer.sends.get_mut(&session) else {
            return;
        };
        if !matches!(&waiting.phase, Phase::Submitting(current) if Arc::ptr_eq(current, &prompt)) {
            return;
        }
        match result {
            Ok(_delivered) => {
                if let Some(context) = &prompt.context
                    && let Some(tray) = self
                        .areas
                        .composer
                        .contexts
                        .get_mut(DraftKey::Session { session })
                {
                    let _settled = tray.settle(context, SubmissionOutcome::Sent);
                }
                if let Some(tray) = self
                    .areas
                    .composer
                    .trays
                    .get_mut(DraftKey::Session { session })
                {
                    tray.clear();
                }
                drop(self.areas.composer.sends.remove(&session));
            }
            Err(problem) => {
                waiting.phase = Phase::Retained {
                    payload: prompt,
                    problem,
                };
                self.toast(
                    self.words.composer().file_submission_retained(),
                    vessel_app_view::toasts::ToastTone::Error,
                    cx,
                );
            }
        }
        self.local_files_changed(cx);
        cx.notify();
    }
}

#[cfg(test)]
mod tests;

impl Shell {
    fn retry_attachment_submission(&mut self, session: SessionId, cx: &mut Context<'_, Self>) {
        let Some(waiting) = self.areas.composer.sends.get_mut(&session) else {
            return;
        };
        let Phase::Retained {
            payload,
            problem: SubmitFailure::NotSent | SubmitFailure::Target(_),
        } = &waiting.phase
        else {
            return;
        };
        let payload = Arc::clone(payload);
        waiting.phase = Phase::Submitting(Arc::clone(&payload));
        self.command(Command::Files(Files::Prompt(payload)));
        cx.notify();
    }
}

pub(super) fn notice(
    shell: &Shell,
    target: DraftKey,
    tokens: &vessel_app_kit::Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<vessel_app_kit::ui::AnyElement> {
    use vessel_app_kit::alert::{Content, Size, Variant, alert};
    use vessel_app_kit::ui::{
        IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, div,
    };
    let DraftKey::Session { session } = target else {
        return None;
    };
    let waiting = shell.areas.composer.sends.get(&session)?;
    let words = shell.words.composer();
    let (description, variant, retry) = match &waiting.phase {
        Phase::Preparing => (words.file_submission_preparing(), Variant::Info, false),
        Phase::Submitting(_) => (words.file_submission_sending(), Variant::Info, false),
        Phase::Retained { problem, .. } => match problem {
            SubmitFailure::NotSent | SubmitFailure::Target(_) => {
                (words.file_submission_not_sent(), Variant::Error, true)
            }
            SubmitFailure::TimedOut => (words.file_submission_retained(), Variant::Error, false),
            #[cfg(not(target_arch = "wasm32"))]
            SubmitFailure::Uncertain(_) => {
                (words.file_submission_retained(), Variant::Error, false)
            }
        },
    };
    let action = retry.then(|| {
        vessel_app_kit::button(
            "attachment-retry-payload",
            words.file_submission_retry(),
            true,
            tokens,
        )
        .on_click(cx.listener(move |shell, _event, _window, cx| {
            shell.retry_attachment_submission(session, cx);
        }))
        .into_any_element()
    });
    Some(
        alert(
            Content {
                title: None,
                description: Some(div().child(description.to_owned()).into_any_element()),
                action,
            },
            variant,
            Size::Small,
            tokens,
        )
        .into_any_element(),
    )
}
