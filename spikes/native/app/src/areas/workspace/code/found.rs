//! The Files tab's search results in the tree's place: a line saying how
//! many were found (and that there are more when the machine stopped),
//! then each file found by name (its glyph, its name, its folder muted
//! under it) or each line found inside a file (the same, the line's
//! number at the right, the line in mono under it); a click opens the
//! file at the line. What made a result match is marked as Paseo marks
//! it (`components/ui/highlighted-text.tsx`: the surface3 tint, the text
//! in the foreground): the name's matched characters, the words on the
//! line.

use std::collections::HashSet;
use std::ops::Range;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, HighlightStyle, InteractiveElement as _, IntoElement as _,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, StyledText,
    div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, glyph};
use vessel_app_view::workspace::changes::split_path;
use vessel_app_view::workspace::marks::{indent_of, marks_at, span_marks};
use vessel_proto::workspace::{GrepMatch, RelPath};

use super::frame::note;
use super::search::{FIND_LIMIT, Results, Search};
use crate::shell::Shell;

/// The results in the tree's place, `None` when nothing is searched.
pub(super) fn results(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let search = &shell.areas.workspace.code.files.search;
    let words = shell.words();
    let labels = words.labels();
    Some(match &search.results {
        Results::Idle => return None,
        Results::Loading => note(labels.loading, tokens).into_any_element(),
        Results::Failed(why) => {
            super::viewer::failed(labels.search_failed, why, 16.0, tokens).into_any_element()
        }
        Results::Refused => note(labels.search_refused, tokens).into_any_element(),
        // A search over part of the folder that found nothing still says
        // how much it looked at (the count line below).
        Results::Found(found) if found.matches.is_empty() && found.complete => {
            note(labels.nothing_found, tokens).into_any_element()
        }
        Results::Grepped(grepped) if grepped.matches.is_empty() => {
            note(labels.nothing_found, tokens).into_any_element()
        }
        Results::Found(found) => {
            let capped = u32::try_from(found.matches.len()).is_ok_and(|count| count >= FIND_LIMIT);
            let line =
                words.found_count(found.matches.len(), found.indexed, found.complete, capped);
            with_count(line, list(search, cx), tokens)
        }
        Results::Grepped(grepped) => {
            let files = files_in(&grepped.matches);
            let line = words.grepped_count(grepped.matches.len(), files, grepped.more);
            with_count(line, list(search, cx), tokens)
        }
    })
}

/// How many files the lines found are in.
fn files_in(matches: &[GrepMatch]) -> usize {
    matches
        .iter()
        .map(|hit| hit.path.as_str())
        .collect::<HashSet<_>>()
        .len()
}

fn list(search: &Search, cx: &mut Context<'_, Shell>) -> AnyElement {
    search
        .list
        .element(
            cx.processor(|shell: &mut Shell, index: usize, _window, cx| {
                let tokens = vessel_app_kit::tokens(cx);
                result_row(shell, index, &tokens, cx)
            }),
        )
        .into_any_element()
}

/// The count line over the rows.
fn with_count(line: String, rows: AnyElement, tokens: &Tokens) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(
            div()
                .flex_none()
                .px(px(8.0))
                .pt(px(2.0))
                .pb(px(6.0))
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(line),
        )
        .child(div().flex_1().min_h(px(0.0)).child(rows))
        .into_any_element()
}

/// `text` with `marks` on Paseo's search tint.
fn marked(text: &str, marks: Vec<Range<usize>>, tokens: &Tokens) -> StyledText {
    let style = HighlightStyle {
        background_color: Some(tokens.neutral.surface3),
        color: Some(tokens.neutral.foreground),
        ..HighlightStyle::default()
    };
    StyledText::new(SharedString::from(text.to_owned()))
        .with_highlights(marks.into_iter().map(|range| (range, style)))
}

/// One result as a row draws it.
struct Hit<'a> {
    path: &'a RelPath,
    /// A line found: its number, its text, where the match starts on it
    /// (from 1).
    line: Option<(u32, &'a str, u32)>,
    /// A file found by name: the path's characters that matched.
    positions: &'a [u32],
}

fn hit_at(results: &Results, index: usize) -> Option<Hit<'_>> {
    match results {
        Results::Found(found) => found.matches.get(index).map(|found| Hit {
            path: &found.path,
            line: None,
            positions: &found.positions,
        }),
        Results::Grepped(grepped) => grepped.matches.get(index).map(|hit| Hit {
            path: &hit.path,
            line: Some((hit.line, hit.text.as_str(), hit.column)),
            positions: &[],
        }),
        Results::Idle | Results::Loading | Results::Failed(_) | Results::Refused => None,
    }
}

fn result_row(
    shell: &Shell,
    index: usize,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let hover = neutral.surface2;
    let search = &shell.areas.workspace.code.files.search;
    let Some(hit) = hit_at(&search.results, index) else {
        return div().into_any_element();
    };
    let (folder, name) = split_path(hit.path.as_str());
    // The node counts a name's matched characters over the whole path.
    let name_from = hit
        .path
        .as_str()
        .chars()
        .count()
        .saturating_sub(name.chars().count());
    let name_from = u32::try_from(name_from).unwrap_or(u32::MAX);
    let name_marks = marks_at(name, name_from, hit.positions.iter().copied());
    let folder_marks = marks_at(folder, 0, hit.positions.iter().copied());
    // The name first and whole where it fits (a hundred files can share
    // a long beginning); the line's number at the right, never cut; the
    // folder on the line under it.
    let head = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .child(glyph(Glyph::File, 14.0, neutral.foreground_muted))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .child(marked(name, name_marks, tokens)),
        )
        .children(hit.line.map(|(line, _, _)| {
            div()
                .flex_none()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(line.to_string())
        }));
    let place = (!folder.is_empty()).then(|| {
        div()
            .pl(px(20.0))
            .truncate()
            // A folder's end (the nearest folder) says the most.
            .text_ellipsis_start()
            .type_(Type::Meta)
            .text_color(neutral.foreground_muted)
            .child(marked(folder, folder_marks, tokens))
    });
    // The line is shown without its indent; its mark moves with it.
    let text = hit.line.map(|(_, text, column)| {
        let shown = text.trim();
        let first = column.saturating_sub(1).saturating_sub(indent_of(text));
        let marks = span_marks(shown, first, search.width);
        div()
            .pl(px(20.0))
            .truncate()
            .type_(Type::Mono)
            .text_color(neutral.foreground_muted)
            .child(marked(shown, marks, tokens))
    });
    let path = hit.path.clone();
    let line = hit.line.map(|(line, _, _)| line);
    div()
        .id(("code-found", index))
        .w_full()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .px(px(8.0))
        .py(px(6.0))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .child(head)
        .children(place)
        .children(text)
        .on_click(cx.listener(move |this, _event, _window, cx| {
            this.open_file_at(path.clone(), line, cx);
        }))
        .into_any_element()
}
