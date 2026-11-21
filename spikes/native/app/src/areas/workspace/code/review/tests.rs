//! Review comments on the Changes diff through the real code panel (ADR
//! 0124, P6b-2a): the key follows the diff shown, the editor saves into
//! the drafts document, edits in place, and a delete takes its open edit;
//! the list keeps room under "Send feedback" (P6b-2b; the send itself in
//! `send`).
use super::super::source_tests::{prepare, ready, root, setup};
use super::super::{ChangeRow, ScopeKind, WorkspaceRoot};
use super::Editor;
use crate::shell::Shell;
use gpui::TestAppContext;
use std::num::NonZeroU32;
use std::sync::Arc;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::workspace::changes::FileState;
use vessel_app_view::workspace::diff_rows::{DiffBody, DiffKey, DiffRow};
use vessel_app_view::workspace::review::{
    CommentId, DraftComment, ReviewKey, ReviewMode, ReviewSide, ReviewTarget,
};
use vessel_proto::workspace::LineKind;

mod send;

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

/// The fixture's `a.rs` read: a hunk, a context line (1) and an added
/// line (new 2), listed again.
fn read_rows(shell: &mut Shell) {
    let line = |line: u32, kind, old, new, text: &str| DiffRow::Line {
        key: DiffKey::Line { hunk: 0, line },
        line: kind,
        text: Arc::from(text),
        cut: false,
        old,
        new,
    };
    let rows: Arc<[DiffRow]> = Arc::from(vec![
        DiffRow::Hunk {
            key: DiffKey::Hunk(0),
            header: "@@ -1,1 +1,2 @@".to_owned(),
            section: None,
        },
        line(0, LineKind::Context, Some(1), Some(1), "let a = 1;"),
        line(1, LineKind::Added, None, Some(2), "let b = 2;"),
    ]);
    let changes = &mut shell.areas.workspace.code.changes;
    drop(changes.states.insert(
        "a.rs".parse().unwrap(),
        FileState::Read(DiffBody::Rows { rows, more: None }),
    ));
    changes.relist();
}

/// Frames until the window has nothing left to draw: an update of the
/// shell from a test draws nothing by itself.
fn redraw(shell: &gpui::Entity<Shell>, cx: &mut gpui::VisualTestContext) {
    shell.update(cx, |_, cx| cx.notify());
    for _ in 0..3 {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
    }
}

/// What the accessibility tree says of each control.
fn arias(shell: &gpui::Entity<Shell>, cx: &mut gpui::VisualTestContext) -> Vec<String> {
    let handle = cx.update(|window, _| window.window_handle());
    assert!(cx.simulate_accessibility_activation(handle));
    cx.run_until_parked();
    shell.update(cx, |_, cx| cx.notify());
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let json = cx.update(|window, _| window.debug_a11y_tree_json().unwrap());
    json.split("\"aria\":")
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(aria, _)| aria.to_owned()))
        .collect()
}

#[gpui::test]
fn the_add_button_opens_the_editor_from_an_assistive_click_with_no_hover(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    cx.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(800.0)));
    shell.update_in(cx, |shell, _window, _cx| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        read_rows(shell);
    });
    cx.run_until_parked();
    let said = arias(&shell, cx);
    let adds: Vec<&String> = said
        .iter()
        .filter(|aria| aria.contains(r#""label": "Add review comment""#))
        .collect();
    assert_eq!(adds.len(), 2, "one per reviewable line");
    assert!(
        adds.iter()
            .all(|aria| aria.contains(r#""Button""#) && aria.contains(r#""Click""#))
    );
    // The added line's "+", pressed at its centre with no hover first (an
    // assistive click is a down and an up there, nothing more).
    let index = (0..16)
        .find_map(|index| {
            shell.update(cx, |shell, _| {
                let rows = &shell.areas.workspace.code.changes.rows;
                matches!(rows.get(index), Some(ChangeRow::Diff { row: 2, .. })).then_some(index)
            })
        })
        .unwrap();
    let selector: &'static str = Box::leak(format!("review-add-{index}").into_boxed_str());
    let add = cx.debug_bounds(selector).unwrap();
    cx.simulate_click(add.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    shell.update(cx, |shell, _| {
        let editor = editor(shell).unwrap();
        assert_eq!(editor.target, target());
        assert_eq!(editor.comment, None);
    });
    shell.update(cx, |shell, _| scene.close(shell.fixture_link()).unwrap());
}

#[gpui::test]
fn the_list_keeps_room_under_send_feedback_while_comments_are_kept(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    cx.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(800.0)));
    shell.update_in(cx, |shell, _window, _cx| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let changes = &shell.areas.workspace.code.changes;
        // `relist` counts one row past the rows: the room's.
        assert_eq!(changes.list.rows(), changes.rows.len().saturating_add(1));
    });
    redraw(&shell, cx);
    let room =
        |cx: &mut gpui::VisualTestContext| cx.debug_bounds("review-room").unwrap().size.height;
    assert_eq!(room(cx), gpui::px(0.0));
    let (_key, id) = shell.update_in(cx, saved);
    redraw(&shell, cx);
    // The md button, its 16 and 44: Paseo's clearance on a pointer's width.
    assert_eq!(room(cx), gpui::px(92.0));
    // A phone's width measures the row again: the floating button's 56,
    // its 16 and 44.
    cx.simulate_resize(gpui::size(gpui::px(390.0), gpui::px(800.0)));
    redraw(&shell, cx);
    assert_eq!(room(cx), gpui::px(116.0));
    // The last comment deleted: no button, no room.
    shell.update(cx, |shell, cx| shell.review_delete(id, 3, cx));
    redraw(&shell, cx);
    assert_eq!(room(cx), gpui::px(0.0));
    shell.update(cx, |shell, _| scene.close(shell.fixture_link()).unwrap());
}
