use super::*;
use crate::{areas::composer::attach::Waiting, shell::language_tests};
use gpui::TestAppContext;
use vessel_app_view::{
    composer::context::{
        ContextAttachment, ContextId, ContextKind, ContextText, ContextTitle, Payload,
    },
    machine_book::Name,
};
use vessel_proto::{Delivery, NodeId};

#[gpui::test]
fn uncertainty_keeps_exact_payload_and_only_matching_completion_can_consume_it(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, cx| {
        let session = SessionId::random();
        let key = DraftKey::Session { session };
        let item = ContextAttachment::Snapshot {
            kind: ContextKind::Review,
            payload: Payload {
                id: ContextId::parse("review").unwrap(),
                title: ContextTitle::parse("Review").unwrap(),
                text: ContextText::parse("review context").unwrap(),
                url: None,
            },
        };
        let tray = shell.areas.composer.contexts.tray(key);
        tray.add(item).unwrap();
        let context = tray.prepare().unwrap();
        let _previous = shell.areas.composer.sends.insert(
            session,
            Waiting {
                message: vessel_proto::timeline::ClientMessageId::random(),
                machine: Name::parse("fixture").unwrap(),
                expected_node: NodeId::random(),
                text: String::new(),
                context,
                delivery: Delivery::Start,
                creation: None,
                local_files: Vec::new(),
                phase: Phase::Preparing,
            },
        );
        shell.composer_pump(session, cx);
        let Phase::Submitting(payload) = &shell.areas.composer.sends[&session].phase else {
            panic!("context-only payload must dispatch exactly once");
        };
        let payload = Arc::clone(payload);
        assert_eq!(payload.context.as_ref().unwrap().attachments().count(), 1);
        shell.composer_submitted(Arc::new((*payload).clone()), Ok(Delivered::Started), cx);
        assert!(
            shell.areas.composer.sends.contains_key(&session),
            "equal values are not this operation's ownership handle"
        );
        shell.composer_submitted(Arc::clone(&payload), Err(SubmitFailure::TimedOut), cx);
        shell.composer_pump(session, cx);
        let Phase::Retained {
            payload: retained, ..
        } = &shell.areas.composer.sends[&session].phase
        else {
            panic!("uncertainty must retain the immutable payload");
        };
        assert!(Arc::ptr_eq(retained, &payload));
        assert!(
            shell
                .areas
                .composer
                .contexts
                .get(key)
                .unwrap()
                .has_retained()
        );
        shell.composer_submitted(payload, Ok(Delivered::Started), cx);
        assert!(
            shell.areas.composer.sends.contains_key(&session),
            "a late duplicate must not unlock unknown delivery"
        );
    });
}
