//! Actual shared layout ids bind retained owners; synthetic roots never grant node authority.
use super::*;
use crate::areas::workspace::code::{file_open::FileOpenRefusal, io_tests};
use gpui::TestAppContext;
use vessel_app_view::layout::{HostAlias, LayoutKey, Placement};
use vessel_app_view::workspace::editor::{Step, Written};

pub(super) fn opening(
    shell: &mut Shell,
    scene: &mut vessel_app_link::testing::Scenario,
    root: vessel_proto::workspace::WorkspaceRoot,
    cx: &mut Context<'_, Shell>,
) -> (FileOpenContext, LayoutKey) {
    let opening = super::super::root_fixture::acquire(shell, scene, root, cx);
    let session = shell.areas.workspace.code.target.as_ref().unwrap().session;
    let key = LayoutKey::Session {
        machine: HostAlias::parse(opening.scope().connection().machine().as_str()).unwrap(),
        session,
    };
    (opening, key)
}

#[gpui::test]
fn original_receipt_creates_one_actual_tab_and_stale_open_allocates_nothing(
    cx: &mut TestAppContext,
) {
    let (shell, cx, mut scene) = super::super::root_fixture::setup(cx, "node");
    cx.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(900.0)));
    shell.update_in(cx, |shell, window, cx| {
        let root = io_tests::root();
        let (origin, workspace) = opening(shell, &mut scene, root, cx);
        let path = "a.txt".parse().unwrap();
        let first = shell
            .open_workspace_file(&origin, path, None, Placement::Focused, window, cx)
            .unwrap();
        let file = shell.areas.workspace.code.file_key().unwrap();
        let ticket = shell.editing_for(&file).unwrap().reading.unwrap().0;
        let second = shell
            .open_workspace_file(
                &origin,
                file.path.clone(),
                None,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(shell.editing_for(&file).unwrap().reading.unwrap().0, ticket);
        assert!(shell.layout_tab_exists(&workspace, first));
        assert!(shell.areas.workspace.code.file_tabs.contains(&file));
        let count = shell.areas.workspace.code.file_tabs.bindings.len();
        shell.areas.session.open.as_mut().unwrap().session = vessel_proto::SessionId::random();
        assert_eq!(
            shell.open_workspace_file(
                &origin,
                "other.txt".parse().unwrap(),
                None,
                Placement::Focused,
                window,
                cx
            ),
            Err(FileOpenRefusal::Changed)
        );
        assert_eq!(shell.areas.workspace.code.file_tabs.bindings.len(), count);
        assert_eq!(shell.areas.workspace.code.file_key().as_ref(), Some(&file));
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn rejected_placement_restores_original_active_owner_without_new_binding(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = super::super::root_fixture::setup(cx, "node");
    cx.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(900.0)));
    shell.update_in(cx, |shell, window, cx| {
        let root = io_tests::root();
        let (origin, workspace) = opening(shell, &mut scene, root, cx);
        let first = shell
            .open_workspace_file(
                &origin,
                "a.txt".parse().unwrap(),
                None,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        let file = shell.areas.workspace.code.file_key().unwrap();
        // File supports Explorer in the canonical layout; create and retire an
        // actual pane to exercise a genuine rejected placement instead.
        let missing = shell
            .update_layout(
                &workspace,
                |layout| {
                    let pane = layout.split(
                        vessel_app_view::layout::PaneId::MAIN,
                        vessel_app_view::layout::Position::Right,
                        vessel_app_view::layout::LayoutMode::SplitPanes,
                    )?;
                    layout.close_empty_pane(pane)?;
                    Ok(pane)
                },
                cx,
            )
            .unwrap();
        let result = shell.open_workspace_file(
            &origin,
            "b.txt".parse().unwrap(),
            None,
            Placement::Pane(missing),
            window,
            cx,
        );
        assert_eq!(
            result,
            Err(FileOpenRefusal::Layout(
                vessel_app_view::layout::LayoutError::MissingPane
            ))
        );
        assert_eq!(shell.areas.workspace.code.file_key().as_ref(), Some(&file));
        assert_eq!(shell.areas.workspace.code.file_tabs.bindings.len(), 1);
        assert_eq!(
            shell
                .areas
                .workspace
                .code
                .file_tabs
                .bindings
                .keys()
                .next()
                .unwrap()
                .1,
            first
        );
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn unknown_original_write_keeps_the_same_layout_tab_modified_after_undo(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = super::super::root_fixture::setup(cx, "node");
    cx.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(900.0)));
    shell.update_in(cx, |shell, window, cx| {
        let root = io_tests::root();
        let (origin, workspace) = opening(shell, &mut scene, root, cx);
        let tab = shell
            .open_workspace_file(
                &origin,
                "a.txt".parse().unwrap(),
                None,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        let file = shell.areas.workspace.code.file_key().unwrap();
        let ticket = shell.editing_for(&file).unwrap().reading.unwrap().0;
        shell.editor_read_answered_for(
            &file,
            ticket,
            &super::super::editing::whole_read(root, file.path.clone()),
            Ok(io_tests::text("baseline")),
            cx,
        );
        let _step = shell.editing_for(&file).unwrap().model.edit("draft");
        let Step::Write { sequence, .. } = shell.editing_for(&file).unwrap().model.save() else {
            panic!("write")
        };
        shell.publish_file_tabs(&file, cx);
        let modified = shell
            .update_layout(
                &workspace,
                |layout| Ok(layout.tab(tab).unwrap().modified()),
                cx,
            )
            .unwrap();
        assert_eq!(modified, Modified::Dirty);
        let _step = shell
            .editing_for(&file)
            .unwrap()
            .model
            .saved(sequence, Written::Uncertain("reply lost".into()));
        let _step = shell.editing_for(&file).unwrap().model.edit("baseline");
        shell.publish_file_tabs(&file, cx);
        let modified = shell
            .update_layout(
                &workspace,
                |layout| Ok(layout.tab(tab).unwrap().modified()),
                cx,
            )
            .unwrap();
        assert_eq!(modified, Modified::Dirty);
        assert!(shell.areas.workspace.code.file_tabs.contains(&file));
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn binding_reference_survives_layout_absence_until_owner_ack_then_becomes_disposable(
    cx: &mut TestAppContext,
) {
    let (shell, cx, mut scene) = super::super::root_fixture::setup(cx, "node");
    cx.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(900.0)));
    shell.update_in(cx, |shell, window, cx| {
        let root = io_tests::root();
        let (origin, workspace) = opening(shell, &mut scene, root, cx);
        shell.areas.workspace.code.files.narrow.set(true);
        let tab = shell
            .open_workspace_file(
                &origin,
                "a.txt".parse().unwrap(),
                None,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        let a = shell.areas.workspace.code.file_key().unwrap();
        let ticket = shell
            .areas
            .workspace
            .code
            .viewer_at(&a)
            .unwrap()
            .reading
            .unwrap();
        let ask = super::super::editing::whole_read(root, a.path.clone());
        shell.file_read_answered_for(&a, &ask, ticket, Ok(io_tests::text("abc")), cx);
        let _b_tab = shell
            .open_workspace_file(
                &origin,
                "b.txt".parse().unwrap(),
                None,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        let b = shell.areas.workspace.code.file_key().unwrap();
        shell.begin_file_mutation_for(
            origin.scope().clone(),
            vessel_app_link::workspace::PathChange::Delete(vessel_proto::workspace::DeletePath {
                root,
                path: a.path.clone(),
                recursive: false,
            }),
            cx,
        );
        assert!(
            shell.areas.workspace.code.mutation.is_none(),
            "bound paths cannot be orphaned before guarded retarget joins"
        );
        assert!(!shell.reserve_readonly_source(&b, super::super::buffers::MAX_READONLY_SOURCE));
        let plan = shell
            .update_layout(&workspace, |layout| layout.request_close(tab), cx)
            .unwrap();
        shell.layout_close_settled(&workspace, &plan, Ok(()), cx);
        assert!(!shell.layout_tab_exists(&workspace, tab));
        assert!(
            !shell.reserve_readonly_source(&b, super::super::buffers::MAX_READONLY_SOURCE),
            "layout absence alone is not owner release"
        );
        // Synthetic owner acknowledgement follows actual clean layout closure;
        // this does not claim the still-missing live close request dispatcher.
        let _released = shell
            .areas
            .workspace
            .code
            .file_tabs
            .bindings
            .remove(&(workspace, tab));
        assert!(shell.reserve_readonly_source(&b, super::super::buffers::MAX_READONLY_SOURCE));
        assert!(!shell.areas.workspace.code.files.retained.contains_key(&a));
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn reused_alias_session_and_wire_root_cannot_rebind_the_original_nodes_tab(
    cx: &mut TestAppContext,
) {
    let (shell, cx, mut scene) = super::super::root_fixture::setup(cx, "node");
    cx.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(900.0)));
    shell.update_in(cx, |shell, window, cx| {
        let root = io_tests::root();
        let (origin, workspace) = opening(shell, &mut scene, root, cx);
        let tab = shell
            .open_workspace_file(
                &origin,
                "a.txt".parse().unwrap(),
                None,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        let original = shell.areas.workspace.code.file_key().unwrap();
        let ticket = shell.editing_for(&original).unwrap().reading.unwrap().0;
        shell.editor_read_answered_for(
            &original,
            ticket,
            &super::super::editing::whole_read(root, original.path.clone()),
            Ok(io_tests::text("baseline A")),
            cx,
        );
        let _step = shell
            .editing_for(&original)
            .unwrap()
            .model
            .edit("unsaved A");
        shell.publish_file_tabs(&original, cx);
        let _replacement = scene
            .reconnect(
                shell.fixture_link(),
                vessel_app_link::testing::Reconnect::DifferentNode,
            )
            .unwrap();
        let replacement = super::super::root_fixture::refresh(shell, &mut scene, root, cx);
        assert_eq!(
            shell.open_workspace_file(
                &replacement,
                original.path.clone(),
                None,
                Placement::Focused,
                window,
                cx
            ),
            Err(FileOpenRefusal::DifferentNode)
        );
        assert_eq!(
            shell.areas.workspace.code.file_key().as_ref(),
            Some(&original)
        );
        assert_eq!(
            shell.editing_for(&original).unwrap().model.content(),
            "unsaved A"
        );
        assert!(shell.layout_tab_exists(&workspace, tab));
        assert_eq!(shell.areas.workspace.code.file_tabs.bindings.len(), 1);
        scene.close(shell.fixture_link()).unwrap();
    });
}
