//! The Files tab's panels drawn (`panels.rs`): Paseo's Preview|Source
//! control in the file's head, and the Markdown preview: the front matter
//! table, then the body through the kit's Markdown, with relative links
//! opening files here. The image panel is `image_view.rs`.

use vessel_app_kit::chat_markdown::{Options, actions::Actions, chat_markdown_with};
use vessel_app_kit::segmented::{SegmentSize, segment, segmented};
use vessel_app_kit::theme::{radius, spacing};
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px, relative,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::workspace::front_matter::FrontMatterRow;
use vessel_app_view::workspace::markdown_paths::{self, Target};
use vessel_app_view::workspace::viewers::Side;

use super::frame::note;
use super::panels::PreviewKey;
use super::{Shown, Viewer};
use crate::shell::Shell;

/// The head's Preview|Source control, for a file with a Preview once its
/// text is read.
pub(super) fn side_control(
    shell: &Shell,
    viewer: &Viewer,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    // Read: shown read-only (`text`), or in the editor (a desktop's width
    // reads into the edit and leaves `text` loading).
    let read = matches!(viewer.text, Shown::Text { .. })
        || viewer
            .edit
            .as_ref()
            .is_some_and(|editing| editing.model.version().is_some());
    if !viewer.look.panel.has_preview() || !read {
        return None;
    }
    let words = shell.t.lang.viewers();
    let chosen = viewer.look.side;
    let one = |id: &'static str, side: Side, label: &'static str, cx: &mut Context<'_, Shell>| {
        segment(id, label, chosen == side, true, SegmentSize::Small, tokens)
            .role(gpui::Role::RadioButton)
            .aria_label(label)
            .on_click(cx.listener(move |shell, _event, _window, cx| shell.code_side(side, cx)))
    };
    Some(
        segmented(SegmentSize::Small)
            .id("code-side")
            .role(gpui::Role::RadioGroup)
            .aria_label(words.file_view)
            .child(one("code-side-preview", Side::Preview, words.preview, cx))
            .child(one("code-side-source", Side::Source, words.source, cx))
            .into_any_element(),
    )
}

/// The Markdown preview of the text shown (the edit's, while one is open),
/// or Paseo's words for a file past its display budget.
pub(super) fn markdown(
    shell: &Shell,
    viewer: &Viewer,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.viewers();
    // Derived once per text (`Look::preview`), not on every render.
    let preview = match (&viewer.edit, &viewer.text) {
        (Some(editing), _) if editing.model.version().is_some() => viewer
            .look
            .preview(PreviewKey::Edit(editing.model.revision()), || {
                editing.model.content().to_owned()
            }),
        // Read only in part (a phone's width reads a page at a time): the
        // Preview draws the whole document or says it cannot.
        (_, Shown::Text { more: true, .. }) => {
            return note(words.too_large, tokens).into_any_element();
        }
        (_, Shown::Text { lines, .. }) => {
            viewer.look.preview(PreviewKey::Read, || lines.join("\n"))
        }
        _ => return note(shell.words().labels().loading, tokens).into_any_element(),
    };
    let Some(document) = preview.document.as_ref() else {
        return note(words.too_large, tokens).into_any_element();
    };
    let from = viewer.path.clone();
    let folder = shell
        .areas
        .workspace
        .code
        .cwd()
        .map(|cwd| cwd.to_string_lossy().into_owned());
    let shell_handle = cx.entity().downgrade();
    let actions = Actions::new(move |href, _window, cx| {
        match markdown_paths::target(href.as_str(), &from, folder.as_deref()) {
            Target::Web(url) => cx.open_url(&url),
            Target::File(path) => {
                let _gone = shell_handle.update(cx, |shell, cx| shell.open_file(path, cx));
            }
            Target::Data(_) | Target::None => {}
        }
    })
    .with_images(viewer.look.pictures.resolver());
    let body = chat_markdown_with(
        "code-markdown-preview",
        &document.body,
        preview.version,
        Options {
            actions: Some(actions),
            ..Options::default()
        },
        tokens,
        window,
        cx,
    );
    cx.defer_in(window, Shell::preview_pictures);
    div()
        .id("code-markdown-scroll")
        .size_full()
        .overflow_y_scroll()
        .child(
            div()
                .max_w(px(MARKDOWN_COLUMN))
                .mx_auto()
                .px(px(spacing::S6))
                .py(px(spacing::S4))
                // The document's ink: the kit's Markdown draws in the
                // colour it is given (the conversation's bubbles set theirs).
                .text_color(tokens.neutral.foreground)
                .children(front_matter(&document.front_matter, tokens))
                .child(body),
        )
        .into_any_element()
}

/// Paseo's front matter table (`markdown-preview/index.tsx`): a bordered
/// box, radius 8, 24 above the body; each row a key cell (30% of the
/// width, at most 180, on the second surface, the key mono in the muted
/// ink) and a value cell, both 12 across and 8 down, 14/20 text alike; a
/// hairline between rows.
fn front_matter(rows: &[FrontMatterRow], tokens: &Tokens) -> Option<AnyElement> {
    if rows.is_empty() {
        return None;
    }
    let neutral = tokens.neutral;
    let table = div()
        .flex()
        .flex_col()
        .mb(px(spacing::S6))
        .border_1()
        .border_color(neutral.border)
        .rounded(px(radius::R2))
        .overflow_hidden()
        .children(rows.iter().enumerate().map(|(index, row)| {
            div()
                .flex()
                .flex_row()
                .items_stretch()
                .when(index > 0, |line| {
                    line.border_t_1().border_color(neutral.border)
                })
                .child(
                    div()
                        // A fixed share, as Paseo's basis: every key
                        // column the same width, a long key wrapping.
                        .w(relative(KEY_SHARE))
                        .max_w(px(KEY_MAX))
                        .flex_shrink_0()
                        .min_w(px(spacing::S0))
                        .px(px(spacing::S3))
                        .py(px(spacing::S2))
                        .bg(neutral.surface2)
                        .type_(Type::MonoBody)
                        .text_color(neutral.foreground_muted)
                        .child(row.key.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(spacing::S0))
                        .px(px(spacing::S3))
                        .py(px(spacing::S2))
                        .type_(Type::Label)
                        .font_weight(gpui::FontWeight::NORMAL)
                        .text_color(neutral.foreground)
                        .child(row.value.clone()),
                )
        }));
    Some(table.into_any_element())
}

/// The key cell's share of the row (Paseo's `flexBasis: "30%"`).
const KEY_SHARE: f32 = 0.3;
/// The key cell's widest (Paseo's `maxWidth: 180`).
const KEY_MAX: f32 = 180.0;

/// The reading column's width (the page's 78ch at its 14px body).
const MARKDOWN_COLUMN: f32 = 760.0;
