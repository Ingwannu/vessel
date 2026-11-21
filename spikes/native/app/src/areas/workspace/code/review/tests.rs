//! Review comments on the Changes diff through the real code panel (ADR
//! 0124, P6b-2a): the key follows the diff shown, the editor saves into
//! the drafts document, edits in place, and a delete takes its open edit.
use super::super::source_tests::{prepare, ready, root, setup};
use super::super::{ScopeKind, WorkspaceRoot};
use gpui::TestAppContext;
use std::num::NonZeroU32;
use vessel_app_view::workspace::review::{ReviewMode, ReviewSide, ReviewTarget};

fn target() -> ReviewTarget {
    ReviewTarget {
        path: "a.rs".parse().unwrap(),
        side: ReviewSide::New,
        line: NonZeroU32::new(2).unwrap(),
    }
}

#[gpui::test]
fn a_comment_is_written_under_its_line_edited_in_place_and_deleted(cx: &mut TestAppContext) {
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
        assert!(shell.areas.workspace.code.changes.review.editor.is_some());
        assert!(
            shell
                .areas
                .composer
                .drafts
                .reviews()
                .comments(&key)
                .is_empty()
        );

        let editor = shell.areas.workspace.code.changes.review.editor.as_ref().unwrap();
        editor.field.set_text("Cap this.", window, cx);
        shell.review_save(window, cx);
        assert!(shell.areas.workspace.code.changes.review.editor.is_none());
        let kept = shell
            .areas
            .composer
            .drafts
            .reviews()
            .comments(&key)
            .to_vec();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].body, "Cap this.");
        assert_eq!(kept[0].target, target());
        let id = kept[0].id;

        // Edit: the editor opens with the comment's words; Cancel keeps it.
        shell.review_open(target(), Some(id), 3, window, cx);
        let editor = shell.areas.workspace.code.changes.review.editor.as_ref().unwrap();
        assert_eq!(editor.field.text(cx), "Cap this.");
        assert_eq!(editor.comment, Some(id));
        shell.review_cancel(cx);
        assert!(shell.areas.workspace.code.changes.review.editor.is_none());
        assert_eq!(
            shell.areas.composer.drafts.reviews().comments(&key)[0].body,
            "Cap this."
        );

        shell.review_open(target(), Some(id), 3, window, cx);
        let editor = shell.areas.workspace.code.changes.review.editor.as_ref().unwrap();
        editor.field.set_text("Cap this at 30 seconds.", window, cx);
        shell.review_save(window, cx);
        let kept = shell
            .areas
            .composer
            .drafts
            .reviews()
            .comments(&key)
            .to_vec();
        assert_eq!(kept.len(), 1);
        assert_eq!(
            (kept[0].id, kept[0].body.as_str()),
            (id, "Cap this at 30 seconds.")
        );

        // Another comparison has its own comments.
        shell.areas.workspace.code.changes.scope = ScopeKind::Staged;
        let staged = shell.review_key().unwrap();
        assert!(
            shell
                .areas
                .composer
                .drafts
                .reviews()
                .comments(&staged)
                .is_empty()
        );
        shell.areas.workspace.code.changes.scope = ScopeKind::Uncommitted;

        // A delete takes the comment and its open edit.
        shell.review_open(target(), Some(id), 3, window, cx);
        shell.review_delete(id, 3, cx);
        assert!(shell.areas.workspace.code.changes.review.editor.is_none());
        assert!(shell.areas.composer.drafts.reviews().is_empty());
        scene.close(shell.fixture_link()).unwrap();
    });
}
