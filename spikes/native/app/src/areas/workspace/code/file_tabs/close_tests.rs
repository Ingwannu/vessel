//! Synthetic layout/resource tests; no filesystem, transport or OS opener.
use super::close::Decision;
use super::*;
use crate::areas::workspace::{FileCloseRequest, code::io_tests};
use gpui::TestAppContext;
use vessel_app_view::workspace::editor::{Step, Written};

fn loaded(
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> (LayoutKey, TabId, FileKey) {
    let root = io_tests::root();
    let (origin, workspace) = tests::opening(shell, root);
    let tab = shell
        .open_workspace_file(
            &origin,
            "a.txt".parse().unwrap(),
            None,
            vessel_app_view::layout::Placement::Focused,
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
    (workspace, tab, file)
}

fn plan(
    shell: &mut Shell,
    workspace: &LayoutKey,
    tab: TabId,
    cx: &mut Context<'_, Shell>,
) -> vessel_app_view::layout::ClosePlan {
    shell
        .update_layout(workspace, |layout| layout.request_close(tab), cx)
        .unwrap()
}

#[gpui::test]
fn clean_owner_close_releases_only_after_actual_layout_ack(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let (workspace, tab, file) = loaded(shell, window, cx);
        let close = plan(shell, &workspace, tab, cx);
        assert_eq!(
            shell.request_layout_file_close(&workspace, &close, window, cx),
            FileCloseRequest::Handled
        );
        assert!(!shell.layout_tab_exists(&workspace, tab));
        assert!(!shell.areas.workspace.code.file_tabs.contains(&file));
        assert!(shell.areas.workspace.code.viewer_at(&file).is_none());
    });
}

#[gpui::test]
fn dirty_prompt_suspends_writes_cancel_keeps_and_discard_closes(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let (workspace, tab, file) = loaded(shell, window, cx);
        let _step = shell.editing_for(&file).unwrap().model.edit("draft");
        shell.publish_file_tabs(&file, cx);
        let close = plan(shell, &workspace, tab, cx);
        assert_eq!(
            shell.request_layout_file_close(&workspace, &close, window, cx),
            FileCloseRequest::Handled
        );
        let ticket = shell
            .areas
            .workspace
            .code
            .file_tabs
            .close
            .as_ref()
            .unwrap()
            .ticket;
        assert!(shell.areas.workspace.code.file_effect_held(&file));
        shell.flush_file();
        shell.editor_save_for(&file, cx);
        // An older Reload confirmation is also a new destructive intent.
        let reload = shell.next_file_ticket().unwrap();
        shell.editing_for(&file).unwrap().reload = Some(super::super::editing::ReloadPrompt {
            ticket: reload,
            dialog: vessel_app_kit::dialog::DialogState::new(cx),
        });
        shell.editor_reload_answered(
            &file,
            reload,
            super::super::editing::ReloadDecision::Reload,
            cx,
        );
        assert_eq!(shell.editing_for(&file).unwrap().model.content(), "draft");
        assert!(shell.editing_for(&file).unwrap().reading.is_none());
        shell.editor_reload_answered(
            &file,
            reload,
            super::super::editing::ReloadDecision::Keep,
            cx,
        );
        assert!(shell.editing_for(&file).unwrap().writing.is_none());
        assert!(shell.editing_for(&file).unwrap().model.writing().is_none());
        shell.file_close_answered(ticket, Decision::Keep, cx);
        assert!(shell.layout_tab_exists(&workspace, tab));
        assert!(shell.editing_for(&file).unwrap().model.modified());
        assert!(!shell.areas.workspace.code.file_effect_held(&file));
        shell.request_layout_file_close(&workspace, &close, window, cx);
        let current = shell
            .areas
            .workspace
            .code
            .file_tabs
            .close
            .as_ref()
            .unwrap()
            .ticket;
        assert_ne!(ticket, current);
        shell.file_close_answered(ticket, Decision::Discard, cx);
        assert!(
            shell.layout_tab_exists(&workspace, tab),
            "old dialog cannot answer a successor"
        );
        shell.file_close_answered(current, Decision::Discard, cx);
        assert!(!shell.layout_tab_exists(&workspace, tab));
        assert!(shell.areas.workspace.code.viewer_at(&file).is_none());
    });
}

#[gpui::test]
fn original_unknown_effect_cannot_be_discarded_even_after_undo(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let (workspace, tab, file) = loaded(shell, window, cx);
        let _step = shell.editing_for(&file).unwrap().model.edit("attempt");
        let Step::Write { sequence, .. } = shell.editing_for(&file).unwrap().model.save() else {
            panic!("original write");
        };
        let _step = shell
            .editing_for(&file)
            .unwrap()
            .model
            .saved(sequence, Written::Uncertain("lost original result".into()));
        let _step = shell.editing_for(&file).unwrap().model.edit("baseline");
        shell.publish_file_tabs(&file, cx);
        let close = plan(shell, &workspace, tab, cx);
        shell.request_layout_file_close(&workspace, &close, window, cx);
        assert!(shell.areas.workspace.code.file_tabs.close.is_none());
        assert!(shell.layout_tab_exists(&workspace, tab));
        assert!(shell.editing_for(&file).unwrap().model.writing().is_some());
        assert!(shell.areas.workspace.code.file_tabs.contains(&file));
    });
}

#[gpui::test]
fn changed_draft_after_prompt_is_preserved_and_refuses_confirmation(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let (workspace, tab, file) = loaded(shell, window, cx);
        let _step = shell.editing_for(&file).unwrap().model.edit("draft");
        shell.publish_file_tabs(&file, cx);
        let close = plan(shell, &workspace, tab, cx);
        shell.request_layout_file_close(&workspace, &close, window, cx);
        let ticket = shell
            .areas
            .workspace
            .code
            .file_tabs
            .close
            .as_ref()
            .unwrap()
            .ticket;
        let _step = shell
            .editing_for(&file)
            .unwrap()
            .model
            .edit("accepted before hold");
        shell.file_close_answered(ticket, Decision::Discard, cx);
        assert!(shell.layout_tab_exists(&workspace, tab));
        assert_eq!(
            shell.editing_for(&file).unwrap().model.content(),
            "accepted before hold"
        );
        assert!(shell.areas.workspace.code.file_tabs.contains(&file));
    });
}
