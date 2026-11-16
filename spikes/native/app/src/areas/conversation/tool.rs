//! A tool call: GOAT's work log row (its title, quiet, with what it
//! touched at the right and its state in a word), opened into the page's
//! card body: the input, the content (diffs line by line, numbered) and
//! the output (the page's `features/session/tool-card.tsx`). Open while
//! the call runs, closed once it has ended, unless the person turned it.

use vessel_app_kit::output::{Line, Lines, Tone};
use vessel_app_kit::ui::FluentBuilder as _;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, IntoElement as _, ParentElement as _, Styled as _, Window,
    div, px,
};
use vessel_app_kit::{Start, Type, TypeExt as _, lines_box, output_box, version_of};
use vessel_app_view::timeline::{MAX_LINES, Side, ToolView, block_text, diff_lines, json_text};
use vessel_proto::{ToolCallContent, ToolCallStatus};

use super::entries::{Paint, work_row};
use crate::shell::Shell;

/// The tool call at `index`, open or closed.
pub(super) fn card(
    paint: &Paint<'_>,
    index: usize,
    tool: &ToolView,
    open: bool,
    window: &mut Window,
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
        row = row.child(body(paint, index, tool, window, cx));
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

fn body(
    paint: &Paint<'_>,
    index: usize,
    tool: &ToolView,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    // Each box's id: the entry and the box's place in the card, so a box
    // keeps its scroll while the card grows.
    let key = |part: usize| {
        ElementId::NamedInteger(
            "tool-text".into(),
            u64::try_from(index)
                .unwrap_or(u64::MAX)
                .saturating_mul(64)
                .saturating_add(u64::try_from(part).unwrap_or(63)),
        )
    };
    let words = paint.words;
    let mut body = div()
        .ml(px(28.0))
        .mt(px(4.0))
        .mb(px(6.0))
        .flex()
        .flex_col()
        .gap(px(6.0));
    if let Some(input) = &tool.input {
        body = body.child(label(paint, words.input())).child(output_box(
            key(0),
            &json_text(input),
            Start::Top,
            paint.tokens,
            window,
            cx,
        ));
    }
    for (part, content) in (1..).zip(&tool.content) {
        body = match content {
            ToolCallContent::Diff {
                path,
                old_text,
                new_text,
                ..
            } => body
                .child(label(paint, &words.diff_of(&path.to_string_lossy())))
                .child(diff(
                    paint,
                    key(part),
                    old_text.as_deref(),
                    new_text,
                    window,
                    cx,
                )),
            ToolCallContent::Content { content, .. } => body.child(output_box(
                key(part),
                &block_text(content),
                Start::End,
                paint.tokens,
                window,
                cx,
            )),
        };
    }
    if let Some(output) = &tool.output {
        body = body.child(label(paint, words.output())).child(output_box(
            key(63),
            &json_text(output),
            Start::Top,
            paint.tokens,
            window,
            cx,
        ));
    }
    body
}

/// A tool call's diff, as its opened row shows it: what a permission card
/// shows for an edit (Paseo's `EditDetailSection`); `None` without one.
pub(super) fn diff_detail(
    paint: &Paint<'_>,
    id: &ElementId,
    tool: &ToolView,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    tool.content.iter().find_map(|content| match content {
        ToolCallContent::Diff {
            old_text, new_text, ..
        } => Some(
            diff(paint, id.clone(), old_text.as_deref(), new_text, window, cx).into_any_element(),
        ),
        ToolCallContent::Content { .. } => None,
    })
}

fn label(paint: &Paint<'_>, text: &str) -> Div {
    div()
        .type_(Type::Meta)
        .text_color(paint.look.secondary)
        .child(text.to_owned())
}

/// A diff in a box of numbered lines, at most the view's
/// [`MAX_LINES`]; a word under it when it was cut.
fn diff(
    paint: &Paint<'_>,
    id: ElementId,
    old: Option<&str>,
    new: &str,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    // Both texts' versions (`kit::version_of`, their hashes): a change to
    // either splits the diff again (their lengths alone kept a first diff
    // on screen after a second of the same lengths).
    let version = version_of(old.unwrap_or("")).rotate_left(1) ^ version_of(new);
    let lines = lines_box(
        id,
        version,
        || {
            let (lines, _) = diff_lines(old, new);
            Lines {
                lines: lines
                    .into_iter()
                    .map(|line| Line {
                        text: line.text.to_owned().into(),
                        tone: match line.side {
                            Side::Removed => Tone::Removed,
                            Side::Added => Tone::Added,
                            Side::Context | Side::Header => Tone::Plain,
                        },
                        number: line.number,
                    })
                    .collect(),
                copy: None,
            }
        },
        Start::Top,
        paint.tokens,
        window,
        cx,
    );
    // Whether it was cut, asked again each frame: the lines borrow the
    // texts and cost no copy, unlike the box's own.
    let (_, cut) = diff_lines(old, new);
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(lines)
        .when(cut, |block| {
            block.child(label(paint, &paint.words.diff_cut(MAX_LINES)))
        })
}
