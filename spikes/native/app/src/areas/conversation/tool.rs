//! A tool call: GOAT's work log row (its title, quiet, with what it
//! touched at the right and its state in a word), opened into the page's
//! card body: the input, the content (diffs line by line, numbered) and
//! the output (the page's `features/session/tool-card.tsx`). Open while
//! the call runs, closed once it has ended, unless the person turned it.

use vessel_app_kit::ui::{
    AnyElement, Context, Div, Hsla, IntoElement as _, ParentElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Type, TypeExt as _};
use vessel_app_view::timeline::{Side, ToolView, block_text, diff_lines, json_text};
use vessel_proto::{ToolCallContent, ToolCallStatus};

use super::entries::{Paint, work_row};
use crate::shell::Shell;

/// The tool call at `index`, open or closed.
pub(super) fn card(
    paint: &Paint<'_>,
    index: usize,
    tool: &ToolView,
    open: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = paint.words;
    let title = if tool.title.is_empty() {
        words.tool_call().to_owned()
    } else {
        tool.title.clone()
    };
    let state = words.tool_status(tool.status).to_owned();
    let color = match tool.status {
        ToolCallStatus::Pending | ToolCallStatus::InProgress => paint.tokens.st_building,
        ToolCallStatus::Completed => paint.look.secondary,
        ToolCallStatus::Failed => paint.tokens.neutral.destructive,
    };
    let detail = match tool.path() {
        Some(path) => format!("{} · {state}", tail(&path.to_string_lossy())),
        None => state,
    };
    let head = work_row(paint, index, title, Some((detail, color)), open, cx);
    let mut row = div().flex().flex_col().child(head);
    if open {
        row = row.child(body(paint, tool));
    }
    row.into_any_element()
}

/// A path's last two segments: what a narrow row has room for (the page
/// truncates from the left).
fn tail(path: &str) -> String {
    let parts: Vec<&str> = path
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .collect();
    match parts.as_slice() {
        [.., parent, name] if parts.len() > 2 => format!("…/{parent}/{name}"),
        _ => path.to_owned(),
    }
}

fn body(paint: &Paint<'_>, tool: &ToolView) -> Div {
    let words = paint.words;
    let mut body = div()
        .ml(px(28.0))
        .mt(px(4.0))
        .mb(px(6.0))
        .flex()
        .flex_col()
        .gap(px(6.0));
    if let Some(input) = &tool.input {
        body = body
            .child(label(paint, words.input()))
            .child(code(paint, json_text(input)));
    }
    for content in &tool.content {
        body = match content {
            ToolCallContent::Diff {
                path,
                old_text,
                new_text,
            } => body
                .child(label(paint, &words.diff_of(&path.to_string_lossy())))
                .child(diff(paint, old_text.as_deref(), new_text)),
            ToolCallContent::Content { content } => body.child(code(paint, block_text(content))),
        };
    }
    if let Some(output) = &tool.output {
        body = body
            .child(label(paint, words.output()))
            .child(code(paint, json_text(output)));
    }
    body
}

fn label(paint: &Paint<'_>, text: &str) -> Div {
    div()
        .type_(Type::Meta)
        .text_color(paint.look.secondary)
        .child(text.to_owned())
}

/// Text in the code surface (GOAT's `pre`: radius 8, a hairline, mono 12).
fn code(paint: &Paint<'_>, text: String) -> Div {
    div()
        .max_h(px(240.0))
        .overflow_hidden()
        .p(px(8.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(paint.look.border)
        .bg(paint.look.code)
        .type_(Type::Mono)
        .text_size(px(12.0))
        .text_color(paint.tokens.neutral.foreground)
        .child(text)
}

fn diff(paint: &Paint<'_>, old: Option<&str>, new: &str) -> Div {
    let (lines, cut) = diff_lines(old, new);
    let mut block = div()
        .flex()
        .flex_col()
        .rounded(px(8.0))
        .border_1()
        .border_color(paint.look.border)
        .bg(paint.look.code)
        .py(px(4.0))
        .type_(Type::Mono)
        .text_size(px(12.0));
    for line in lines {
        let (mark, ink, wash): (&str, Hsla, Hsla) = match line.side {
            Side::Removed => (
                "-",
                paint.look.removed,
                Hsla {
                    a: 0.08,
                    ..paint.look.removed
                },
            ),
            Side::Added => (
                "+",
                paint.look.added,
                Hsla {
                    a: 0.08,
                    ..paint.look.added
                },
            ),
        };
        block = block.child(
            div()
                .flex()
                .gap(px(8.0))
                .px(px(8.0))
                .bg(wash)
                .child(
                    div()
                        .w(px(32.0))
                        .flex_shrink_0()
                        .text_color(paint.look.secondary)
                        .child(line.number.to_string()),
                )
                .child(div().text_color(ink).child(format!("{mark} {}", line.text))),
        );
    }
    if cut {
        block = block.child(
            div()
                .px(px(8.0))
                .text_color(paint.look.secondary)
                .child("…"),
        );
    }
    block
}
