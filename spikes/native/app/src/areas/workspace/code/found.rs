//! The Files tab's search results in the tree's place: a file found by
//! name (its glyph, its name, its folder muted), or a line found inside a
//! file (the same, the line's number after the folder, the line in mono
//! under it); a click opens the file at the line.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, glyph};
use vessel_app_view::workspace::changes::split_path;
use vessel_proto::workspace::RelPath;

use super::frame::note;
use super::search::Results;
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
        Results::Found(found) if found.matches.is_empty() => {
            note(labels.nothing_found, tokens).into_any_element()
        }
        Results::Grepped(grepped) if grepped.matches.is_empty() => {
            note(labels.nothing_found, tokens).into_any_element()
        }
        Results::Found(_) | Results::Grepped(_) => search
            .list
            .element(
                cx.processor(|shell: &mut Shell, index: usize, _window, cx| {
                    let tokens = vessel_app_kit::tokens(cx);
                    result_row(shell, index, &tokens, cx)
                }),
            )
            .into_any_element(),
    })
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
    let (path, line, text): (RelPath, Option<u32>, Option<String>) = match &search.results {
        Results::Found(found) => match found.matches.get(index) {
            Some(found) => (found.path.clone(), None, None),
            None => return div().into_any_element(),
        },
        Results::Grepped(grepped) => match grepped.matches.get(index) {
            Some(hit) => (hit.path.clone(), Some(hit.line), Some(hit.text.clone())),
            None => return div().into_any_element(),
        },
        Results::Idle | Results::Loading | Results::Failed(_) | Results::Refused => {
            return div().into_any_element();
        }
    };
    let (folder, name) = split_path(path.as_str());
    let head = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .child(glyph(Glyph::File, 14.0, neutral.foreground_muted))
        .child(
            div()
                .flex_none()
                .max_w(px(240.0))
                .truncate()
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .child(name.to_owned()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(match line {
                    Some(line) => format!("{folder}:{line}"),
                    None => folder.to_owned(),
                }),
        );
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
        .children(text.map(|text| {
            div()
                .pl(px(20.0))
                .truncate()
                .type_(Type::Mono)
                .text_color(neutral.foreground_muted)
                .child(text.trim().to_owned())
        }))
        .on_click(cx.listener(move |this, _event, _window, cx| {
            this.open_file_at(path.clone(), line, cx);
        }))
        .into_any_element()
}
