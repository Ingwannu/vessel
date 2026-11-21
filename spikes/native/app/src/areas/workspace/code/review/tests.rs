//! Review comments on the Changes diff through the real code panel (ADR
//! 0124, P6b-2a): the key follows the diff shown, the editor saves into
//! the drafts document, edits in place, and a delete takes its open edit.
use super::super::source_tests::{prepare, ready, root, setup};
use super::super::{ScopeKind, WorkspaceRoot};
use super::Editor;
use crate::shell::Shell;
use gpui::TestAppContext;
use std::num::NonZeroU32;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::workspace::review::{
    CommentId, DraftComment, ReviewKey, ReviewMode, ReviewSide, ReviewTarget,
};

fn target() -> ReviewTarget {
    ReviewTarget {
        path: "a.rs".parse().unwrap(),
        side: ReviewSide::New,
        line: NonZeroU32::new(2).unwrap(),
    }
}

fn editor(shell: &Shell) -> Option<&Editor> {
    shell.areas.workspace.code.changes.review.editor.as_ref()
}

fn kept(shell: &Shell, key: &ReviewKey) -> Vec<DraftComment> {
    shell.areas.composer.drafts.reviews().comments(key).to_vec()
}

/// Type `text` in the open editor and press Comment.
fn write(shell: &mut Shell, text: &str, window: &mut Window, cx: &mut Context<'_, Shell>) {
    editor(shell).unwrap().field.set_text(text, window, cx);
    shell.review_save(window, cx);
}

/// One comment saved on `target()`: its key and identity.
fn saved(
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> (ReviewKey, CommentId) {
    let key = shell.review_key().unwrap();
    shell.review_open(target(), None, 3, window, cx);
    write(shell, "Cap this.", window, cx);
    let id = kept(shell, &key)[0].id;
    (key, id)
}

#[gpui::test]
fn a_comment_is_saved_under_its_line_for_the_diff_shown(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, window, cx| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let key = shell.review_key().unwrap();
        assert_eq!(key.mode, ReviewMode::Uncommitted);
        assert_eq!(key.root, WorkspaceRoot::Project { project });
        assert!(!key.ignore_whitespace);
        // Blank: Comment saves nothing and the editor stays.
        shell.review_open(target(), None, 3, window, cx);
        shell.review_save(window, cx);
        assert!(editor(shell).is_some());
        assert!(kept(shell, &key).is_empty());
        write(shell, "Cap this.", window, cx);
        assert!(editor(shell).is_none());
        let comments = kept(shell, &key);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].body, "Cap this.");
        assert_eq!(comments[0].target, target());
        // Another comparison has its own comments.
        shell.areas.workspace.code.changes.scope = ScopeKind::Staged;
        let staged = shell.review_key().unwrap();
        assert!(kept(shell, &staged).is_empty());
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn a_comment_is_edited_in_place_and_a_delete_takes_its_open_edit(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, window, cx| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let (key, id) = saved(shell, window, cx);
        // Edit: the editor opens with the comment's words; Cancel keeps it.
        shell.review_open(target(), Some(id), 3, window, cx);
        assert_eq!(editor(shell).unwrap().field.text(cx), "Cap this.");
        assert_eq!(editor(shell).unwrap().comment, Some(id));
        shell.review_cancel(cx);
        assert!(editor(shell).is_none());
        assert_eq!(kept(shell, &key)[0].body, "Cap this.");
        shell.review_open(target(), Some(id), 3, window, cx);
        write(shell, "Cap this at 30 seconds.", window, cx);
        let comments = kept(shell, &key);
        assert_eq!(comments.len(), 1);
        assert_eq!(
            (comments[0].id, comments[0].body.as_str()),
            (id, "Cap this at 30 seconds.")
        );
        // A delete takes the comment and its open edit.
        shell.review_open(target(), Some(id), 3, window, cx);
        shell.review_delete(id, 3, cx);
        assert!(editor(shell).is_none());
        assert!(shell.areas.composer.drafts.reviews().is_empty());
        scene.close(shell.fixture_link()).unwrap();
    });
}
