//! Actual retained buffers must keep original uncertain writes protected.
use super::*;
use crate::{areas::workspace::code::io_tests, shell::Shell};
use gpui::TestAppContext;
use vessel_app_view::workspace::editor::{Step, Written};

#[gpui::test]
fn undo_and_a_clean_active_file_cannot_hide_a_retained_unknown_write(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let root = io_tests::root();
        io_tests::target(shell, root);
        shell.open_file_at("a.txt".parse().unwrap(), None, cx);
        let a = shell.areas.workspace.code.file_key().unwrap();
        let ticket = shell.editing_for(&a).unwrap().reading.unwrap().0;
        shell.editor_read_answered_for(
            &a,
            ticket,
            &super::super::editing::whole_read(root, a.path.clone()),
            Ok(io_tests::text("baseline")),
            cx,
        );
        assert_eq!(check_documents(shell, cx), DocumentReadiness::Ready);
        let _step = shell.editing_for(&a).unwrap().model.edit("draft");
        assert_eq!(check_documents(shell, cx), DocumentReadiness::Dirty);
        let Step::Write { sequence, .. } = shell.editing_for(&a).unwrap().model.save() else {
            panic!("write");
        };
        assert_eq!(check_documents(shell, cx), DocumentReadiness::Pending);
        let _step = shell
            .editing_for(&a)
            .unwrap()
            .model
            .saved(sequence, Written::Uncertain("reply lost".into()));
        let _step = shell.editing_for(&a).unwrap().model.edit("baseline");
        assert_eq!(check_documents(shell, cx), DocumentReadiness::Failed);
        assert!(
            !shell
                .areas
                .workspace
                .code
                .viewer_at(&a)
                .unwrap()
                .can_evict()
        );
        assert!(shell.park_file());
        shell.show_file("node", root, "b.txt".parse().unwrap(), None);
        assert_eq!(check_documents(shell, cx), DocumentReadiness::Failed);
        assert!(shell.areas.workspace.code.files.retained.contains_key(&a));
    });
}

#[gpui::test]
fn node_readiness_does_not_confuse_reused_alias_or_ignore_parked_effects(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let root = io_tests::root();
        io_tests::target(shell, root);
        shell.open_file_at("a.txt".parse().unwrap(), None, cx);
        let a = shell.areas.workspace.code.file_key().unwrap();
        let ticket = shell.editing_for(&a).unwrap().reading.unwrap().0;
        shell.editor_read_answered_for(
            &a,
            ticket,
            &super::super::editing::whole_read(root, a.path.clone()),
            Ok(io_tests::text("baseline")),
            cx,
        );
        let _step = shell.editing_for(&a).unwrap().model.edit("draft");
        assert!(shell.park_file());
        let other = vessel_proto::NodeId::random();
        let replacement = vessel_app_view::connection::ConnectionEpochs::default()
            .admit(
                vessel_app_view::connection::MachineKey::from_book("node"),
                other,
            )
            .unwrap();
        shell
            .areas
            .workspace
            .code
            .target
            .as_mut()
            .unwrap()
            .admission
            .as_mut()
            .unwrap()
            .context = replacement;
        shell.show_file("node", root, "a.txt".parse().unwrap(), None);
        let b = shell.areas.workspace.code.file_key().unwrap();
        assert_ne!(a.node, b.node);
        assert_eq!(a.path, b.path);
        assert_eq!(
            shell.file_documents_for_node(a.node, cx),
            DocumentReadiness::Dirty
        );
        assert_eq!(
            shell.file_documents_for_node(b.node, cx),
            DocumentReadiness::Ready
        );
        let Step::Write { sequence, .. } = shell.editing_for(&a).unwrap().model.save() else {
            panic!("write")
        };
        assert_eq!(
            shell.file_documents_for_node(a.node, cx),
            DocumentReadiness::Pending
        );
        let _step = shell
            .editing_for(&a)
            .unwrap()
            .model
            .saved(sequence, Written::Uncertain("original reply lost".into()));
        let _step = shell.editing_for(&a).unwrap().model.edit("baseline");
        assert_eq!(
            shell.file_documents_for_node(a.node, cx),
            DocumentReadiness::Failed
        );
        assert_eq!(
            shell.file_documents_for_node(b.node, cx),
            DocumentReadiness::Ready
        );
        assert_eq!(check_documents(shell, cx), DocumentReadiness::Failed);
    });
}
