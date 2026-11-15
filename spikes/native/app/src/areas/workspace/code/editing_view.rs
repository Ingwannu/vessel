//! A file being edited, drawn: Paseo's text file toolbar
//! (`file-panels/source/toolbar.tsx`: the size and the lines at the left,
//! the save state and the caret at the right, 36 tall on `surface0` with a
//! hairline under it), its conflict bar under it (`conflict-alert.tsx`),
//! the editor, and Reload's question when the buffer has changes.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::dialog::{DialogContent, DialogSize, dialog};
use vessel_app_kit::motion::{QUICK, STANDARD};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button, glyph};
use vessel_app_view::workspace::editor::{ConflictKind, EditorStatus};
use vessel_app_view::workspace::size::size_word;

use super::appear::appear;
use super::editing::{Editing, ReloadDecision};
use super::frame::note;
use crate::shell::Shell;

/// The unsaved mark: a dot six points across.
const DIRTY_DOT: f32 = 6.0;

/// The conflict bar's words never narrow below this, so they wrap under
/// the buttons rather than into a column one word wide.
const CONFLICT_WORDS_LEAST: f32 = 180.0;

/// The editing body under the file's head: toolbar, conflict, editor.
pub(super) fn body(
    shell: &Shell,
    editing: &Editing,
    key: &super::buffers::FileKey,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let labels = shell.words().labels();
    let Some(editor) = editing.editor.as_ref() else {
        if editing.model.status() == EditorStatus::Failed {
            let why = editing.model.error().unwrap_or_default();
            return super::viewer::failed(labels.read_failed, why, 16.0, tokens).into_any_element();
        }
        return note(labels.loading, tokens).into_any_element();
    };
    let save_key = key.clone();
    // The viewer's slot for the body is a plain block of a definite
    // height: the body takes all of it (`flex_1` there did nothing, and
    // the engine's editor, whose height is its parent's, fell back to two
    // rows).
    div()
        .size_full()
        .flex()
        .flex_col()
        .on_key_down(
            cx.listener(move |shell, event: &gpui::KeyDownEvent, _, cx| {
                let modifiers = event.keystroke.modifiers;
                let primary = if cfg!(target_os = "macos") {
                    modifiers.platform
                } else {
                    modifiers.control
                };
                if primary && !modifiers.alt && !modifiers.shift && event.keystroke.key == "s" {
                    shell.editor_save_for(&save_key, cx);
                    cx.stop_propagation();
                }
            }),
        )
        .child(toolbar(shell, editing, tokens))
        .child(
            button("source-open-find", labels.file_find, false, tokens).on_click({
                let editor = editor.clone();
                move |_, _, cx| editor.open_find(cx)
            }),
        )
        .children(editing.find.clone())
        .children(conflict_bar(shell, editing, key.clone(), tokens, cx))
        .children(write_recovery(shell, editing, key.clone(), tokens, cx))
        .child(div().flex_1().min_h_0().child(editor.element(tokens, cx)))
        .into_any_element()
}

/// Recovery reads are offered only after a terminal original-handler reply.
fn write_recovery(
    shell: &Shell,
    editing: &Editing,
    key: super::buffers::FileKey,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    use vessel_app_view::workspace::editor::WriteState;
    let labels = shell.words().labels();
    let state = editing.model.write_state()?;
    let content = match state {
        WriteState::Pending => return None,
        WriteState::Uncertain => div().child(labels.editor_write_unconfirmed),
        WriteState::Ended if editing.reading.is_some() => div().child(labels.loading),
        WriteState::Ended => div().child(
            button(
                "source-confirm-saved",
                labels.editor_confirm_saved,
                false,
                tokens,
            )
            .on_click(cx.listener(move |shell, _, _, cx| shell.editor_confirm_saved_for(&key, cx))),
        ),
    };
    Some(
        content
            .p(px(spacing::S2))
            .type_(Type::Meta)
            .text_color(tokens.neutral.foreground_muted),
    )
}

/// The size and lines; the save state; the caret and the selection's
/// button (a view of their own, `editor_status`).
fn toolbar(shell: &Shell, editing: &Editing, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    let words = shell.words();
    let labels = words.labels();
    let whisper = |text: String| {
        div()
            .flex_none()
            .type_(Type::Meta)
            .text_color(neutral.foreground_extra_muted)
            .child(text)
    };
    let size = editing.model.version().map_or(0, |version| version.size);
    let lines = editing.model.content().split('\n').count();
    // Each mark comes in once as its state begins (`appear`).
    let state: Option<AnyElement> = match editing.model.status() {
        EditorStatus::Dirty => Some(appear(
            "code-editor-dirty",
            QUICK,
            div()
                .size(px(DIRTY_DOT))
                .rounded_full()
                .bg(neutral.foreground_extra_muted),
        )),
        EditorStatus::Saving => Some(appear(
            "code-editor-saving-row",
            QUICK,
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S2))
                .child(vessel_app_kit::slider::spinner(
                    "code-editor-saving",
                    14.0,
                    neutral.foreground_muted,
                ))
                .child(
                    div()
                        .type_(Type::Meta)
                        .text_color(neutral.foreground_muted)
                        .child(labels.editor_saving),
                ),
        )),
        EditorStatus::Error => Some(appear(
            "code-editor-save-failed",
            QUICK,
            div()
                .type_(Type::Meta)
                .text_color(neutral.destructive)
                .child(labels.editor_save_failed),
        )),
        EditorStatus::Loading
        | EditorStatus::Failed
        | EditorStatus::Clean
        | EditorStatus::Conflict => None,
    };
    div()
        .flex_none()
        .h(px(vessel_app_kit::tabs::HEAD_HEIGHT))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S3))
        .px(px(spacing::S3))
        .bg(neutral.surface0)
        .border_b_1()
        .border_color(neutral.border)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S2))
                .child(whisper(size_word(size)))
                .child(whisper(words.editor_lines(lines))),
        )
        .children(state)
        .children(editing.status.clone())
}

/// Paseo's conflict bar: amber hairline under it, the triangle, the title
/// and what to do, Overwrite (a modified buffer over a changed file) and
/// Reload. It arrives as a surface (`STANDARD`), once per conflict: its
/// id does not change as Overwrite comes and goes.
fn conflict_bar(
    shell: &Shell,
    editing: &Editing,
    key: super::buffers::FileKey,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let (kind, overwrite) = editing.model.conflict()?;
    let neutral = tokens.neutral;
    // Paseo's `amber[500]`: the theme's amber status colour.
    let amber = tokens.st_review;
    let labels = shell.words().labels();
    let (title, description) = match (kind, overwrite) {
        (ConflictKind::Deleted, _) => (labels.editor_deleted_title, Some(labels.editor_preserved)),
        (ConflictKind::Changed, true) => (
            labels.editor_changed_on_disk,
            Some(labels.editor_conflict_description),
        ),
        (ConflictKind::Changed, false) => (labels.editor_changed_on_disk, None),
        (ConflictKind::Unverified, _) => (
            labels.editor_changed_on_disk,
            Some(labels.editor_conflict_unverified),
        ),
    };
    let actions = (kind != ConflictKind::Deleted).then(|| {
        div()
            .flex_none()
            .ml_auto()
            .flex()
            .flex_row()
            .gap(px(spacing::S2))
            .children(overwrite.then(|| {
                button(
                    "code-editor-overwrite",
                    labels.editor_overwrite,
                    false,
                    tokens,
                )
                .on_click({
                    let key = key.clone();
                    cx.listener(move |this, _, _, cx| {
                        this.editor_overwrite(&key, cx);
                    })
                })
            }))
            .child(
                button("code-editor-reload", labels.editor_reload, false, tokens).on_click({
                    let key = key.clone();
                    cx.listener(move |this, _, window, cx| {
                        this.editor_reload(&key, window, cx);
                    })
                }),
            )
    });
    let bar = div()
        .flex_none()
        .min_h(px(spacing::S12))
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(spacing::S3))
        .px(px(spacing::S3))
        .py(px(spacing::S2))
        .bg(neutral.surface1)
        .border_b_1()
        .border_color(amber)
        .child(glyph(Glyph::Alert, 14.0, amber))
        .child(
            div()
                .flex_1()
                .min_w(px(CONFLICT_WORDS_LEAST))
                .flex()
                .flex_col()
                .gap(px(spacing::S1))
                .child(div().type_(Type::Label).text_color(amber).child(title))
                .children(description.map(|text| {
                    div()
                        .type_(Type::Meta)
                        .text_color(neutral.foreground_muted)
                        .child(text)
                })),
        )
        .children(actions);
    Some(appear("code-editor-conflict", STANDARD, bar))
}

/// Reload's question while it is asked (Paseo's `confirmDialog`,
/// destructive).
pub(super) fn reload_dialog(
    shell: &Shell,
    key: &super::buffers::FileKey,
    tokens: &Tokens,
    window: &mut Window,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let editing = shell.areas.workspace.code.viewer_at(key)?.edit.as_ref()?;
    let prompt = editing.reload.as_ref()?;
    let state = &prompt.dialog;
    let ticket = prompt.ticket;
    let key = key.clone();
    let labels = shell.words().labels();
    let content = DialogContent {
        title: labels.editor_reload_title.into(),
        body: div()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(labels.editor_reload_message)
            .into_any_element(),
        footer: vec![
            button("code-reload-cancel", labels.cancel, false, tokens)
                .on_click({
                    let key = key.clone();
                    cx.listener(move |this, _, _, cx| {
                        this.editor_reload_answered(&key, ticket, ReloadDecision::Keep, cx);
                    })
                })
                .into_any_element(),
            vessel_app_kit::button_with_kind(
                "code-reload-confirm",
                labels.editor_reload,
                ButtonKind::Destructive,
                ButtonState::Enabled,
                tokens,
            )
            .on_click({
                let key = key.clone();
                cx.listener(move |this, _, _, cx| {
                    this.editor_reload_answered(&key, ticket, ReloadDecision::Reload, cx);
                })
            })
            .into_any_element(),
        ],
        size: DialogSize::Normal,
    };
    Some(dialog(
        state,
        content,
        tokens,
        window,
        narrow,
        move |shell: &mut Shell, _window, cx| {
            shell.editor_reload_answered(&key, ticket, ReloadDecision::Keep, cx);
        },
        cx,
    ))
}
