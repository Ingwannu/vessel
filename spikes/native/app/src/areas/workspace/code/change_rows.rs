//! The Changes list's rows: a file's header (Paseo's file header: the
//! chevron, the status letter in its tone, the name, the folder muted or
//! where it was renamed from, the counts), its diff's rows as the kit
//! draws a diff's lines, and the notes in a file's place. A file's diff
//! not read yet is asked for as its row is drawn.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, Hsla, InteractiveElement as _, IntoElement as _,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    px,
};
use vessel_app_kit::{LineTone, OutputLine, Tokens, Type, TypeExt as _, diff_line, glyph};
use vessel_app_view::workspace::changes::{
    ChangeRow, FileNote, FileState, Tone, split_path, status_letter,
};
use vessel_app_view::workspace::diff_rows::{DiffBody, DiffRow};
use vessel_proto::workspace::LineKind;

use super::Summary;
use crate::shell::Shell;

/// The tone's colour.
fn tone_colour(tone: Tone, tokens: &Tokens) -> Hsla {
    match tone {
        Tone::Added => tokens.st_ready,
        Tone::Removed => tokens.neutral.destructive,
        Tone::Modified => tokens.st_building,
        Tone::Renamed => tokens.st_review,
        Tone::Other => tokens.neutral.foreground_muted,
    }
}

/// Row `index` of the list; a file's diff not read yet is asked for here,
/// as its row comes into view.
pub(super) fn row(
    shell: &mut Shell,
    index: usize,
    tokens: &Tokens,
    _window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let changes = &shell.areas.workspace.code.changes;
    let Summary::Ready(summary) = &changes.summary else {
        return div().into_any_element();
    };
    let Some(change_row) = changes.rows.get(index).cloned() else {
        return div().into_any_element();
    };
    let file_of = |file: usize| summary.files.get(file).cloned();
    match change_row {
        ChangeRow::File { file, open } => file_of(file).map_or_else(
            || div().into_any_element(),
            |change| file_header(shell, index, &change, open, tokens, cx),
        ),
        ChangeRow::Diff { file, row, .. } => {
            let line = file_of(file).and_then(|change| match changes.states.get(&change.path) {
                Some(
                    FileState::Read(DiffBody::Rows { rows, .. })
                    | FileState::Loading(Some(DiffBody::Rows { rows, .. })),
                ) => rows.get(row).cloned(),
                Some(
                    FileState::Read(_)
                    | FileState::Loading(_)
                    | FileState::Unread
                    | FileState::Failed(_),
                )
                | None => None,
            });
            line.map_or_else(|| div().into_any_element(), |line| diff_row(&line, tokens))
        }
        ChangeRow::Note {
            file,
            note: file_note,
        } => {
            let Some(change) = file_of(file) else {
                return div().into_any_element();
            };
            match &file_note {
                FileNote::Loading => {
                    if matches!(
                        changes.states.get(&change.path),
                        None | Some(FileState::Unread)
                    ) {
                        shell.read_diff(&change.path, None);
                    }
                    note_row(shell.words().file_note(&file_note), tokens).into_any_element()
                }
                FileNote::More => {
                    let more = match changes.states.get(&change.path) {
                        Some(FileState::Read(DiffBody::Rows { more, .. })) => *more,
                        Some(_) | None => None,
                    };
                    let path = change.path.clone();
                    note_row(shell.words().file_note(&file_note), tokens)
                        .id(("code-more", index))
                        .cursor_pointer()
                        .text_color(tokens.neutral.foreground)
                        .on_click(cx.listener(move |this, _event, _window, cx| {
                            this.read_diff(&path, more);
                            cx.notify();
                        }))
                        .into_any_element()
                }
                FileNote::Binary
                | FileNote::TooLarge { .. }
                | FileNote::Submodule { .. }
                | FileNote::Unchanged
                | FileNote::Unknown
                | FileNote::Failed(_) => {
                    note_row(shell.words().file_note(&file_note), tokens).into_any_element()
                }
            }
        }
    }
}

/// A file's header: Paseo's file header (the chevron, the status letter in
/// its tone, the name, the folder muted, the counts at the end).
fn file_header(
    shell: &Shell,
    index: usize,
    change: &vessel_proto::workspace::FileChange,
    open: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let words = shell.words();
    let (letter, tone) = status_letter(change.status);
    let (folder, name) = split_path(change.path.as_str());
    let path = change.path.clone();
    let hover = neutral.surface2;
    let renamed = change
        .old_path
        .as_ref()
        .map(|old| format!("\u{2190} {}", old.as_str()));
    div()
        .id(("code-file", index))
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .min_h(px(32.0))
        .px(px(8.0))
        .border_t_1()
        .border_color(neutral.border)
        .bg(neutral.surface1)
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .child(glyph(
            if open {
                Glyph::ChevronDown
            } else {
                Glyph::ChevronRight
            },
            12.0,
            neutral.foreground_muted,
        ))
        .child(
            div()
                .w(px(12.0))
                .flex_none()
                .type_(Type::Mono)
                .text_color(tone_colour(tone, tokens))
                .child(letter.to_string()),
        )
        // The name keeps its room before the folder gives up its own (a
        // hundred files can share a long beginning).
        .child(
            div()
                .flex_shrink(1.0)
                .min_w_0()
                .truncate()
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .child(name.to_owned()),
        )
        .child(
            div()
                .flex_1()
                .flex_shrink(64.0)
                .min_w_0()
                .truncate()
                // A folder's end (the nearest folder) says the most.
                .text_ellipsis_start()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(renamed.unwrap_or_else(|| folder.to_owned())),
        )
        .when(change.binary, |row| {
            row.child(
                div()
                    .flex_none()
                    .type_(Type::Meta)
                    .text_color(neutral.foreground_muted)
                    .child(words.status_word(change.status)),
            )
        })
        .children(change.additions.map(|added| {
            div()
                .flex_none()
                .type_(Type::Meta)
                .text_color(tokens.st_ready)
                .child(format!("+{added}"))
        }))
        .children(change.deletions.map(|removed| {
            div()
                .flex_none()
                .type_(Type::Meta)
                .text_color(neutral.destructive)
                .child(format!("\u{2212}{removed}"))
        }))
        .on_click(cx.listener(move |this, _event, _window, cx| this.fold_file(path.clone(), cx)))
        .into_any_element()
}

/// A row of a file's diff: a hunk's header, or a line as the kit draws a
/// diff's line (its bar, its number, its wash), wrapped.
fn diff_row(line: &DiffRow, tokens: &Tokens) -> AnyElement {
    let neutral = tokens.neutral;
    match line {
        DiffRow::Hunk {
            header, section, ..
        } => div()
            .w_full()
            .flex()
            .flex_row()
            .gap(px(8.0))
            .px(px(8.0))
            .py(px(2.0))
            .bg(neutral.surface1)
            .type_(Type::Mono)
            .text_color(neutral.foreground_muted)
            .child(header.clone())
            .children(
                section
                    .clone()
                    .map(|section| div().truncate().child(section)),
            )
            .into_any_element(),
        DiffRow::Line {
            line,
            text,
            old,
            new,
            ..
        } => {
            let (tone, number) = match line {
                LineKind::Added => (LineTone::Added, *new),
                LineKind::Removed => (LineTone::Removed, *old),
                LineKind::Context | LineKind::NoNewline => (LineTone::Plain, new.or(*old)),
            };
            diff_line(
                &OutputLine {
                    text: SharedString::from(text.to_string()),
                    tone,
                    number: number.and_then(|number| usize::try_from(number).ok()),
                },
                true,
                tokens,
            )
        }
        DiffRow::Cut { .. } => note_row("\u{2026}".to_owned(), tokens).into_any_element(),
    }
}

/// A note in a file's place.
fn note_row(text: String, tokens: &Tokens) -> Div {
    div()
        .w_full()
        .px(px(28.0))
        .py(px(6.0))
        .type_(Type::Meta)
        .text_color(tokens.neutral.foreground_muted)
        .child(text)
}
