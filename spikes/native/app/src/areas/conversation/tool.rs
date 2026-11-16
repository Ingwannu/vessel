//! A tool call: GOAT's work log row (its title, quiet, with what it
//! touched at the right and its state in a word), opened into the page's
//! card body: the input, the content (diffs line by line, numbered) and
//! the output (the page's `features/session/tool-card.tsx`). Ordinary calls
//! start closed as Paseo's `ToolCall` does; explicit choices survive updates.

use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, IntoElement as _, ParentElement as _, Styled as _, Window,
    div, px,
};
use vessel_app_kit::{Start, Type, TypeExt as _, output_box};
use vessel_app_view::timeline::{ToolDetailSource, ToolView, block_text, json_text};
use vessel_proto::{ToolCallContent, ToolCallStatus};

use super::entries::{Paint, work_row};
use crate::shell::Shell;

/// The tool call at `index`, open or closed.
pub(super) fn card(
    paint: &Paint<'_>,
    index: usize,
    source: &impl ToolDetailSource,
    open: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let tool = source.tool();
    let plan = vessel_app_view::timeline::plan_text(source);
    if let Some(text) = plan {
        return proposal(paint, index, (tool, &text), open, window, cx);
    }
    let head = super::tool_detail::header(paint, index, source, open, cx);
    let mut row = div().flex().flex_col().child(head);
    if open && super::tool_detail::can_open(source) {
        row = row.child(
            div()
                .ml(px(28.0))
                .child(body(paint, index, source, window, cx)),
        );
    }
    row.into_any_element()
}

pub(super) fn legacy_head(
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
    let color = state_color(paint, tool.status);
    let detail = match tool.path() {
        Some(path) => format!("{} · {state}", tail(&path.to_string_lossy())),
        None => state,
    };
    work_row(paint, index, title, Some((detail, color)), open, cx)
}

pub(super) fn state_color(paint: &Paint<'_>, status: ToolCallStatus) -> vessel_app_kit::ui::Hsla {
    match status {
        ToolCallStatus::Pending | ToolCallStatus::InProgress => paint.tokens.st_building,
        ToolCallStatus::Completed => paint.look.secondary,
        ToolCallStatus::Failed => paint.tokens.neutral.destructive,
    }
}

fn proposal(
    paint: &Paint<'_>,
    index: usize,
    (tool, text): (&ToolView, &str),
    open: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let seat = paint.seat.clone();
    super::plan_card::card(
        super::plan_card::PlanCard {
            id: ElementId::NamedInteger("stream-plan".into(), super::entries::id(index)),
            text,
            title: None,
            description: None,
            outcome: vessel_app_view::timeline::plan_outcome(paint.entries, &tool.id),
            expansion: if open {
                vessel_app_view::timeline::PlanExpansion::Expanded
            } else {
                vessel_app_view::timeline::PlanExpansion::Collapsed
            },
            footer: None,
        },
        move |shell, _window, cx| shell.toggle_entry(&seat, index, cx),
        paint.catalog.timeline_plan(),
        paint.tokens,
        window,
        cx,
    )
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

pub(super) fn body(
    paint: &Paint<'_>,
    index: usize,
    source: &impl ToolDetailSource,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    if let Some(message) = source.agent_message() {
        return super::tool_detail::message_body(paint, index, source, message, window, cx);
    }
    if let Some(detail) = source.detail() {
        return super::tool_detail::body(paint, index, source, detail, window, cx);
    }
    let tool = source.tool();
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
    if tool.input.is_none() && tool.output.is_none() && tool.content.is_empty() {
        return match source.failure() {
            Some(reported) => div().child(super::tool_detail::failure(
                paint, index, reported, window, cx,
            )),
            None => div().child(super::tool_sheet::empty(paint, index, tool.working())),
        };
    }
    let mut body = div().mt(px(4.0)).mb(px(6.0)).flex().flex_col().gap(px(6.0));
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
    body.children(
        source
            .failure()
            .map(|reported| super::tool_detail::failure(paint, index, reported, window, cx)),
    )
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
/// bounded preview; a word under it when it was cut.
pub(super) fn diff(
    paint: &Paint<'_>,
    id: ElementId,
    old: Option<&str>,
    new: &str,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    super::tool_body::computed_diff(
        &super::tool_body::BodyPaint::from(paint),
        id,
        old,
        new,
        window,
        cx,
    )
}
