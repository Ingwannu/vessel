//! Canonical retained owners survive alias reuse and forget, independent from UI.
use super::*;
use gpui::TestAppContext;
use vessel_app_link::workspace::FileAnswer;
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
use vessel_app_view::workspace::save::Saved;
use vessel_proto::{NodeId, workspace::ReadFile};

#[gpui::test]
fn reused_alias_cannot_install_an_old_nodes_read_or_drop_its_draft(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, cx| {
        let root = io_tests::root();
        io_tests::target(shell, root);
        shell.open_file_at("same.txt".parse().unwrap(), None, cx);
        let a = shell.areas.workspace.code.file_key().unwrap();
        let old_scope = shell
            .areas
            .workspace
            .code
            .viewer_for(&a)
            .unwrap()
            .scope
            .clone();
        let ticket = shell.editing_for(&a).unwrap().reading.unwrap().0;
        let ask = ReadFile {
            root,
            path: a.path.clone(),
            offset: 0,
            limit: None,
        };
        shell.editor_read_answered("node", ticket, &ask, Ok(io_tests::text("baseline")), cx);
        let _step = shell.editing_for(&a).unwrap().model.edit("preserve me");
        shell.flush_file();
        shell.areas.workspace.code.machine_forgot("node");
        assert_eq!(shell.areas.workspace.code.file_key(), Some(a.clone()));
        assert_eq!(
            shell.editing_for(&a).unwrap().model.content(),
            "preserve me"
        );
        assert!(shell.park_file());
        io_tests::target(shell, root);
        let target = shell.areas.workspace.code.target.as_mut().unwrap();
        target.admission.as_mut().unwrap().context = ConnectionEpochs::default()
            .admit(MachineKey::from_book("node"), NodeId::random())
            .unwrap();
        shell.open_file_at("same.txt".parse().unwrap(), None, cx);
        let b = shell.areas.workspace.code.file_key().unwrap();
        assert_ne!(a.node, b.node);
        assert_ne!(a, b);
        shell.file_scoped_answer(
            &old_scope,
            FileAnswer::EditorRead {
                ask,
                ticket,
                result: Ok(io_tests::text("stale old node")),
            },
            false,
            true,
            cx,
        );
        assert_eq!(
            shell.editing_for(&a).unwrap().model.content(),
            "preserve me"
        );
        assert_eq!(shell.editing_for(&b).unwrap().model.content(), "");
        // A local queue refusal for an unrelated ticket cannot settle A's flush.
        let unrelated = shell.next_file_ticket().unwrap();
        shell.file_scoped_answer(
            &old_scope,
            FileAnswer::Saved {
                ticket: unrelated,
                root,
                path: a.path.clone(),
                saved: Saved::Failed(vessel_app_view::workspace::save::SaveFailure::NoGuard),
            },
            false,
            false,
            cx,
        );
        assert!(shell.editing_for(&a).unwrap().writing.is_some());
    });
}
