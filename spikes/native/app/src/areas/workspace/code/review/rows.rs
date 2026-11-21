//! A diff line with Paseo's inline review (`review/surface.tsx`,
//! `review/geometry.ts`, `review/comment-sheet.tsx`): a "+" over the
//! line's number (on hover; always on a phone), then the line's thread
//! (its comments with edit and delete, and the editor) under it. On a
//! phone the editor is the kit's sheet, titled "Review comment".
//!
//! Paseo's values: the "+" 22 square, `accent`, 6 round, its plus 16 in
//! `accentForeground`, 10 past the number's right edge; the thread 8
//! above and below, 12 across, 6 between blocks, at least as high as
//! geometry.ts counts (a comment 72, the editor 132); a comment
//! `surface2`, a hairline of `borderAccent`, 8 round, 12 by 8 inside, its
//! words two lines at most, its buttons 26 round with marks of 14; the
//! editor the same card, 12 inside, 12 between its parts.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::dialog::{DialogContent, DialogInput, DialogSize, dialog};
use vessel_app_kit::theme::{radius, spacing};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    KeyDownEvent, ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled,
    Window, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, Type, TypeExt as _, breakpoint, glyph};
use vessel_app_view::workspace::diff_rows::DiffRow;
use vessel_app_view::workspace::review::{DraftComment, ReviewSide, can_save};
use vessel_proto::workspace::RelPath;

use super::{Editor, target_of};
use crate::shell::Shell;

/// geometry.ts: a comment's height, the editor's, the gap between blocks
/// and the thread's padding above and below.
const COMMENT: u16 = 72;
const EDITOR: u16 = 132;
const GAP: u16 = 6;
const PADDING: u16 = 8;
/// The "+": its side, and its left edge (the number's right edge, 34, plus
/// 10, less its side).
const ADD: f32 = 22.0;
const ADD_LEFT: f32 = 22.0;
/// Its top: (the line's 20 less its 22) / 2, rounded down.
const ADD_TOP: f32 = -1.0;
/// A comment's edit and delete buttons, round.
const ACTION: f32 = 26.0;

/// geometry.ts `getInlineReviewThreadState`: the thread's least height,
/// in whole points (counted in integers: the lints forbid float sums).
fn thread_height(comments: usize, editor: bool) -> f32 {
    let count = |n: usize| u16::try_from(n).unwrap_or(u16::MAX);
    let blocks = comments.saturating_add(usize::from(editor));
    let height = count(comments)
        .saturating_mul(COMMENT)
        .saturating_add(if editor { EDITOR } else { 0 })
        .saturating_add(count(blocks.saturating_sub(1)).saturating_mul(GAP))
        .saturating_add(PADDING.saturating_mul(2));
    f32::from(height)
}

/// Row `index`: diff line `line` of `path`, with its review.
pub(in super::super) fn line(
    shell: &Shell,
    index: usize,
    path: &RelPath,
    line: &DiffRow,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let drawn = super::super::change_rows::diff_row(line, tokens);
    let (Some(target), Some(key)) = (target_of(path, line), shell.review_key()) else {
        return drawn;
    };
    let neutral = tokens.neutral;
    let narrow = breakpoint::phone(window);
    let group = SharedString::from(format!("review-line-{index}"));
    let opened = target.clone();
    let add = div()
        .id(("review-add", index))
        .absolute()
        .top(px(ADD_TOP))
        .left(px(ADD_LEFT))
        .size(px(ADD))
        .rounded(px(radius::MD))
        .bg(neutral.accent)
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .role(gpui::Role::Button)
        .aria_label(SharedString::from(shell.words.review().add().to_owned()))
        .child(glyph(Glyph::Plus, 16.0, neutral.accent_foreground))
        .on_click(cx.listener(move |shell, _event, window, cx| {
            shell.review_open(opened.clone(), None, index, window, cx);
        }))
        .when(!narrow, |add| {
            add.invisible().group_hover(group.clone(), Styled::visible)
        });
    let row = div()
        .relative()
        .w_full()
        .group(group.clone())
        .child(drawn)
        .child(add);
    let comments: Vec<DraftComment> = shell
        .areas
        .composer
        .drafts
        .reviews()
        .on(&key, &target)
        .cloned()
        .collect();
    let editor = shell
        .areas
        .workspace
        .code
        .changes
        .review
        .editor
        .as_ref()
        .filter(|editor| !narrow && editor.key == key && editor.target == target);
    if comments.is_empty() && editor.is_none() {
        return row.into_any_element();
    }
    div()
        .w_full()
        .flex()
        .flex_col()
        .child(row)
        .child(thread(shell, &comments, editor, index, tokens, cx))
        .into_any_element()
}

/// The line's comments, the one being edited replaced by the editor, and
/// a new comment's editor after them.
fn thread(
    shell: &Shell,
    comments: &[DraftComment],
    editor: Option<&Editor>,
    index: usize,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let editing = editor
        .and_then(|editor| editor.comment)
        .filter(|id| comments.iter().any(|comment| comment.id == *id));
    let shown = comments
        .len()
        .saturating_sub(usize::from(editing.is_some()));
    let mut open = editor.map(|editor| editor_block(shell, editor, tokens, cx));
    let mut blocks: Vec<AnyElement> = Vec::with_capacity(comments.len().saturating_add(1));
    for comment in comments {
        if Some(comment.id) == editing {
            blocks.extend(open.take());
        } else {
            blocks.push(comment_block(shell, comment, index, tokens, cx));
        }
    }
    blocks.extend(open);
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(f32::from(GAP)))
        .py(px(f32::from(PADDING)))
        .px(px(spacing::S3))
        .min_h(px(thread_height(shown, editor.is_some())))
        .children(blocks)
}

/// The card a comment and the editor share.
fn card(tokens: &Tokens) -> Div {
    div()
        .bg(tokens.neutral.surface2)
        .border_1()
        .border_color(tokens.neutral.border_accent)
        .rounded(px(radius::R2))
}

/// A saved comment: its words, then edit and delete.
fn comment_block(
    shell: &Shell,
    comment: &DraftComment,
    index: usize,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let words = shell.words.review();
    let (id, target) = (comment.id, comment.target.clone());
    let action = |name: &'static str, mark: Glyph, colour, label: &str| {
        div()
            .id((name, index))
            .size(px(ACTION))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(neutral.surface3))
            .role(gpui::Role::Button)
            .aria_label(SharedString::from(label.to_owned()))
            .child(glyph(mark, 14.0, colour))
    };
    card(tokens)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .px(px(spacing::S3))
        .py(px(spacing::S2))
        .child(
            div()
                .flex_1()
                .min_w(px(spacing::S0))
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .line_clamp(2)
                .text_ellipsis()
                .child(comment.body.clone()),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S1))
                .flex_none()
                .child(
                    action(
                        "review-edit",
                        Glyph::Pencil,
                        neutral.foreground_muted,
                        words.edit(),
                    )
                    .on_click(cx.listener(move |shell, _event, window, cx| {
                        shell.review_open(target.clone(), Some(id), index, window, cx);
                    })),
                )
                .child(
                    action(
                        "review-delete",
                        Glyph::Trash2,
                        neutral.destructive,
                        words.delete(),
                    )
                    .on_click(cx.listener(
                        move |shell, _event, _window, cx| {
                            shell.review_delete(id, index, cx);
                        },
                    )),
                ),
        )
        .into_any_element()
}

/// Under the field when the words are past their bound: why Comment is off.
fn too_long(shell: &Shell, editor: &Editor, tokens: &Tokens) -> Option<Div> {
    let body = editor.body.trim();
    (!body.is_empty() && !can_save(body)).then(|| {
        div()
            .type_(Type::Meta)
            .text_color(tokens.neutral.destructive)
            .child(shell.words.review().too_long().to_owned())
    })
}

/// The inline editor: the field, then Cancel and Comment.
fn editor_block(
    shell: &Shell,
    editor: &Editor,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.review();
    let state = if can_save(&editor.body) {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    };
    let save = vessel_app_kit::button_with_kind(
        "review-save",
        words.save().to_owned(),
        ButtonKind::Primary,
        state,
        tokens,
    )
    .aria_label(SharedString::from(words.save_accessibility().to_owned()))
    .when(state == ButtonState::Enabled, |save| {
        save.on_click(cx.listener(|shell, _event, window, cx| shell.review_save(window, cx)))
    });
    let cancel = vessel_app_kit::button_with_kind(
        "review-cancel",
        words.cancel().to_owned(),
        ButtonKind::Ghost,
        ButtonState::Enabled,
        tokens,
    )
    .aria_label(SharedString::from(words.cancel_accessibility().to_owned()))
    .on_click(cx.listener(|shell, _event, _window, cx| shell.review_cancel(cx)));
    card(tokens)
        .min_h(px(f32::from(EDITOR)))
        .p(px(spacing::S3))
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .on_key_down(cx.listener(|shell, event: &KeyDownEvent, _window, cx| {
            if event.keystroke.key == "escape" {
                shell.review_cancel(cx);
                cx.stop_propagation();
            }
        }))
        .child(editor.field.element(tokens, false, cx))
        .children(too_long(shell, editor, tokens))
        .child(
            div()
                .flex()
                .flex_row()
                .justify_end()
                .gap(px(spacing::S2))
                .child(cancel)
                .child(save),
        )
        .into_any_element()
}

/// On a phone, the editor as the kit's sheet: "Review comment", the file
/// and the line under it, the field, Cancel and Save.
pub(in super::super) fn sheet(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if !breakpoint::phone(window) {
        return None;
    }
    let changes = &shell.areas.workspace.code.changes;
    let editor = changes.review.editor.as_ref()?;
    if shell.review_key().as_ref() != Some(&editor.key) {
        return None;
    }
    let words = shell.words.review();
    let target = &editor.target;
    let sign = match target.side {
        ReviewSide::Old => '\u{2212}',
        ReviewSide::New => '+',
    };
    let state = if can_save(&editor.body) {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    };
    let save = vessel_app_kit::button_with_kind(
        "review-sheet-save",
        words.sheet_save().to_owned(),
        ButtonKind::Primary,
        state,
        tokens,
    )
    .aria_label(SharedString::from(words.save_accessibility().to_owned()))
    .when(state == ButtonState::Enabled, |save| {
        save.on_click(cx.listener(|shell, _event, window, cx| shell.review_save(window, cx)))
    });
    let cancel = vessel_app_kit::button_with_kind(
        "review-sheet-cancel",
        words.cancel().to_owned(),
        ButtonKind::Secondary,
        ButtonState::Enabled,
        tokens,
    )
    .aria_label(SharedString::from(words.cancel_accessibility().to_owned()))
    .on_click(cx.listener(|shell, _event, _window, cx| shell.review_cancel(cx)));
    let content = DialogContent {
        title: words.label().to_owned().into(),
        body: div()
            .flex()
            .flex_col()
            .gap(px(spacing::S2))
            .child(editor.field.element(tokens, true, cx))
            .children(too_long(shell, editor, tokens))
            .into_any_element(),
        footer: vec![cancel.into_any_element(), save.into_any_element()],
        size: DialogSize::Normal,
    };
    let content = DialogInput::from(content).with_header_subtitle(
        div()
            .type_(Type::Meta)
            .text_color(tokens.neutral.foreground_muted)
            .child(format!(
                "{} \u{b7} {sign}{}",
                target.path.as_str(),
                target.line
            )),
    );
    Some(dialog(
        &changes.review.dialog,
        content,
        tokens,
        window,
        true,
        |shell: &mut Shell, _window, cx| shell.review_cancel(cx),
        cx,
    ))
}
