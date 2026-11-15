//! Only the submission which admitted an upload may consume its late progress.
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::attachments::Step;
use vessel_app_view::composer::{drafts::DraftKey, tray::ItemKey};
use vessel_proto::SessionId;
impl Shell {
    /// A step of a file's upload to `session`: its tray follows (a tray
    /// dropped since, or a file removed since, takes nothing), and the
    /// next file, or the prompt, goes.
    pub(crate) fn composer_upload(
        &mut self,
        session: SessionId,
        message: &vessel_proto::timeline::ClientMessageId,
        key: ItemKey,
        step: Step,
        cx: &mut Context<'_, Self>,
    ) {
        if !self
            .areas
            .composer
            .sends
            .get(&session)
            .is_some_and(|waiting| {
                &waiting.message == message
                    && waiting.phase == super::super::transmission::Phase::Preparing
            })
        {
            return;
        }
        let Some(tray) = self
            .areas
            .composer
            .trays
            .get_mut(DraftKey::Session { session })
        else {
            return;
        };
        if !tray.items().iter().any(|item| {
            item.key == key
                && matches!(
                    item.state,
                    vessel_app_view::composer::tray::ItemState::Uploading(_)
                )
        }) {
            return;
        }
        match step {
            Step::Progress(received) => tray.progress(key, received),
            Step::Done(attachment) => tray.uploaded(key, attachment),
            Step::Failed { problem, again } => {
                tracing::info!(?problem, ?again, "a file's upload failed");
                tray.failed(key, problem, again);
            }
        }
        self.composer_pump(session, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        areas::composer::{attach::Waiting, transmission::Phase},
        shell::language_tests,
    };
    use gpui::TestAppContext;
    use vessel_app_view::{
        composer::tray::{Again, ItemState, Origin, Problem},
        machine_book::Name,
    };
    use vessel_proto::{Delivery, NodeId, timeline::ClientMessageId};
    #[gpui::test]
    fn an_old_submission_cannot_fail_a_reused_file_key_or_release_a_new_resume(
        cx: &mut TestAppContext,
    ) {
        language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update_in(cx, |shell, _window, cx| {
            let session = SessionId::random();
            let target = DraftKey::Session { session };
            let message = ClientMessageId::random();
            let old = ClientMessageId::random();
            drop(shell.areas.composer.sends.insert(
                session,
                Waiting {
                    message: message.clone(),
                    machine: Name::parse("fixture").unwrap(),
                    expected_node: NodeId::random(),
                    text: "new input".to_owned(),
                    context: None,
                    phase: Phase::Preparing,
                    local_files: Vec::new(),
                    delivery: Delivery::Start,
                    creation: None,
                },
            ));
            let tray = shell.areas.composer.trays.tray(target);
            let key = tray
                .add("file.txt", 1, Origin::Pasted(std::sync::Arc::from([1])))
                .unwrap();
            tray.target(Some(vessel_app_view::composer::tray::Target {
                abilities: Some(vessel_proto::AgentAbilities::default()),
                limits: vessel_proto::AttachmentLimits::DEFAULT,
            }));
            tray.send();
            assert_eq!(tray.starts().len(), 1);
            shell.composer_upload(
                session,
                &old,
                key,
                Step::Failed {
                    problem: Problem::Changed,
                    again: Again::Never,
                },
                cx,
            );
            assert!(matches!(
                shell.areas.composer.trays.get(target).unwrap().items()[0].state,
                ItemState::Uploading(_)
            ));
            shell.composer_answered(
                vessel_app_link::attachments::Answer::Woke {
                    message: old,
                    session,
                    outcome: vessel_app_link::attachments::WakeOutcome::Refused,
                },
                cx,
            );
            assert_eq!(shell.areas.composer.sends[&session].message, message);
            shell.composer_upload(
                session,
                &message,
                key,
                Step::Failed {
                    problem: Problem::Changed,
                    again: Again::Never,
                },
                cx,
            );
            assert!(matches!(
                shell.areas.composer.trays.get(target).unwrap().items()[0].state,
                ItemState::Failed { .. }
            ));
        });
    }
}
