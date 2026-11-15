//! A file shown in the Files tab: Back and its path (and "Add lines to
//! chat" while lines are chosen), then its lines numbered and coloured by
//! the page's lexer; a click on a number chooses the line, Shift-click
//! extends the choice.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, ClickEvent, Context, Div, FluentBuilder as _, HighlightStyle,
    InteractiveElement as _, IntoElement as _, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, StyledText, Window, div, px,
};
use vessel_app_kit::{CodeTokens, Tokens, Type, TypeExt as _, glyph};
use vessel_app_view::workspace::syntax::{TokenKind, language_of, tokenize};

use super::frame::note;
use super::{Shown, Viewer, buffers::FileKey};
use crate::shell::Shell;

/// The file shown: Back and its path, then its lines, or the editor when
/// it is edited in place.
pub(super) fn viewer_element(
    shell: &Shell,
    viewer: &Viewer,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let key = FileKey::new(&viewer.scope, viewer.path.clone());
    if let Some(editing) = &viewer.edit
        && (viewer.goto.is_some()
            || editing.stale
            || (editing.editor.is_none() && editing.model.version().is_some()))
    {
        // The control is made, or a reload's text put in it, on the next
        // frame: an answer brings no window.
        let key = key.clone();
        cx.defer_in(window, move |shell, window, cx| {
            shell.editor_frame_for(&key, window, cx)
        });
    }
    let neutral = tokens.neutral;
    let labels = shell.words().labels();
    let hover = neutral.surface2;
    let head = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .min_h(px(36.0))
        .px(px(8.0))
        .border_b_1()
        .border_color(neutral.border)
        .child(
            div()
                .id("code-back")
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .h(px(28.0))
                .px(px(8.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .type_(Type::Body)
                .text_color(neutral.foreground_muted)
                .child(glyph(Glyph::ChevronLeft, 14.0, neutral.foreground_muted))
                .child(labels.back)
                .on_click({
                    let key = key.clone();
                    cx.listener(move |this, _event, _window, cx| this.leave_file_for(&key, cx))
                }),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_ellipsis_start()
                .type_(Type::Mono)
                .text_color(neutral.foreground_muted)
                .child(viewer.path.as_str().to_owned()),
        )
        .children(viewer.watch.paused().then(|| {
            div()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(labels.file_watch_paused)
        }))
        .children(viewer.chosen.map(|chosen| {
            vessel_app_kit::button(
                "code-add-to-chat",
                super::search::chosen_words(shell, chosen),
                true,
                tokens,
            )
            .on_click(cx.listener(|this, _event, window, cx| {
                this.add_lines_to_chat(window, cx);
            }))
        }));
    let head = head.children(super::markdown::toolbar(shell, viewer, tokens, cx));
    let body: AnyElement =
        if let Some(preview) = super::markdown::body(shell, viewer, tokens, window, cx) {
            preview
        } else if let Some(editing) = &viewer.edit {
            super::editing_view::body(shell, editing, &key, tokens, cx)
        } else {
            read_only_body(shell, viewer, tokens, cx)
        };
    let narrow = vessel_app_kit::breakpoint::phone(window);
    div()
        .flex_1()
        .min_h(px(0.0))
        .flex()
        .flex_col()
        .child(head)
        .children(super::intelligence_view::panel(shell, viewer, tokens, cx))
        .child(div().flex_1().min_h(px(0.0)).child(body))
        .children(super::editing_view::reload_dialog(
            shell, &key, tokens, window, narrow, cx,
        ))
        .into_any_element()
}

/// The lines of a file shown read-only (a phone's width, a binary, more
/// than the editable size).
fn read_only_body(
    shell: &Shell,
    viewer: &Viewer,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let labels = shell.words().labels();
    match &viewer.text {
        Shown::Loading => note(labels.loading, tokens).into_any_element(),
        Shown::Binary => note(labels.binary_file, tokens).into_any_element(),
        Shown::TooLarge => note(labels.too_large, tokens).into_any_element(),
        Shown::Failed(why) => failed(labels.read_failed, why, 16.0, tokens).into_any_element(),
        Shown::Text { .. } => viewer
            .list
            .element(cx.processor({
                let key = FileKey::new(&viewer.scope, viewer.path.clone());
                move |shell: &mut Shell, index: usize, _window, cx| {
                    let tokens = vessel_app_kit::tokens(cx);
                    code_line(shell, &key, index, &tokens, cx)
                }
            }))
            .into_any_element(),
    }
}

/// Line `index` of the file shown: its number, its text coloured.
fn code_line(
    shell: &Shell,
    key: &FileKey,
    index: usize,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let Some(Shown::Text {
        states,
        more,
        source,
    }) = shell
        .areas
        .workspace
        .code
        .viewer_at(key)
        .map(|viewer| &viewer.text)
    else {
        return div().into_any_element();
    };
    let neutral = tokens.neutral;
    let Some(text) = source.line(index) else {
        return if *more {
            note(shell.words().labels().file_continues, tokens).into_any_element()
        } else {
            div().into_any_element()
        };
    };
    let path = shell
        .areas
        .workspace
        .code
        .viewer_at(key)
        .map_or("", |viewer| viewer.path.as_str());
    let language = language_of(
        if source.size().0 <= vessel_app_view::workspace::source::MAX_HIGHLIGHTED_BYTES {
            path
        } else {
            ""
        },
    );
    let state = states.get(index).copied().unwrap_or_default();
    let (runs, _) = tokenize(text, language, state);
    let colours = CodeTokens::of(tokens.scheme);
    let highlights = runs.into_iter().filter_map(|token| {
        let colour = match token.kind? {
            TokenKind::Keyword | TokenKind::Tag => colours.keyword,
            TokenKind::String => colours.string,
            TokenKind::Comment => colours.comment,
            TokenKind::Number => colours.number,
            TokenKind::Constant => colours.constant,
            TokenKind::Function => colours.function,
        };
        Some((
            token.range,
            HighlightStyle {
                color: Some(colour),
                ..HighlightStyle::default()
            },
        ))
    });
    let styled = StyledText::new(SharedString::from(text.to_owned())).with_highlights(highlights);
    let chosen = shell
        .areas
        .workspace
        .code
        .viewer_at(key)
        .and_then(|viewer| viewer.chosen)
        .is_some_and(|(first, last)| (first..=last).contains(&index));
    line_frame(key.clone(), index, chosen, tokens, cx)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_color(neutral.foreground)
                .child(styled),
        )
        .into_any_element()
}

/// A line's frame: the number column (muted, right-aligned; a click
/// chooses the line, Shift extends the choice), mono 12 on 20, wrapped; a
/// chosen line on the second surface.
fn line_frame(
    key: FileKey,
    index: usize,
    chosen: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let neutral = tokens.neutral;
    div()
        .w_full()
        .flex()
        .flex_row()
        .gap(px(12.0))
        .px(px(8.0))
        .type_(Type::Mono)
        .when(chosen, |line| line.bg(neutral.surface2))
        .child(
            div()
                .id(("code-line", index))
                .w(px(36.0))
                .flex_none()
                .text_right()
                .cursor_pointer()
                .text_color(if chosen {
                    neutral.foreground
                } else {
                    neutral.foreground_extra_muted
                })
                .hover(move |style| style.text_color(neutral.foreground))
                .child(index.saturating_add(1).to_string())
                .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
                    this.choose_line_for(&key, index, event.modifiers().shift, cx);
                })),
        )
}

/// A read that failed: the panel's sentence, the node's reason under it.
pub(super) fn failed(sentence: &'static str, reason: &str, inset: f32, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .pl(px(inset))
        .pr(px(16.0))
        .py(px(12.0))
        .child(
            div()
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .child(sentence),
        )
        .child(
            div()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(reason.to_owned()),
        )
}

impl Shell {
    /// Choose line `index` of the viewed file; with Shift, from the line
    /// chosen first to this one; a second click on the only line chosen
    /// lets it go.
    fn choose_line_for(
        &mut self,
        key: &FileKey,
        index: usize,
        extend: bool,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(viewer) = self.areas.workspace.code.viewer_for(key) {
            viewer.chosen = match viewer.chosen {
                Some((first, _)) if extend => Some((first.min(index), first.max(index))),
                Some((first, last)) if first == index && last == index => None,
                Some(_) | None => Some((index, index)),
            };
        }
        cx.notify();
    }
}
