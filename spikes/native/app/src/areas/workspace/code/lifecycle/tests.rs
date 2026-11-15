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
        assert_eq!(check_documents(shell), DocumentReadiness::Ready);
        let _step = shell.editing_for(&a).unwrap().model.edit("draft");
        assert_eq!(check_documents(shell), DocumentReadiness::Dirty);
        let Step::Write { sequence, .. } = shell.editing_for(&a).unwrap().model.save() else {
            panic!("write");
        };
        assert_eq!(check_documents(shell), DocumentReadiness::Pending);
        let _step = shell
            .editing_for(&a)
            .unwrap()
            .model
            .saved(sequence, Written::Uncertain("reply lost".into()));
        let _step = shell.editing_for(&a).unwrap().model.edit("baseline");
        assert_eq!(check_documents(shell), DocumentReadiness::Failed);
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
        assert_eq!(check_documents(shell), DocumentReadiness::Failed);
        assert!(shell.areas.workspace.code.files.retained.contains_key(&a));
    });
}
