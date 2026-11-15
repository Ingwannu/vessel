//! Canonical header/provenance wrappers reuse the shared borrowed body.
use super::entries::{Paint, marked_work_row};
use super::tool_body::{self, BodyPaint};
pub(super) use super::tool_body::{BorrowedDetail, DetailContentState};
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, App, Context, Div, ElementId, IntoElement as _, ParentElement as _, Styled as _,
    Window, div, px,
};
use vessel_app_kit::{Type, TypeExt as _};
use vessel_app_view::timeline::{
    DetailContents, ToolDetailSource, ToolLabel, agent_message_display, tool_display,
};
use vessel_proto::timeline::{AgentMessageProvenance, ToolCallDetail, ToolFailure, ToolIcon};

impl<'a> From<&Paint<'a>> for BodyPaint<'a> {
    fn from(paint: &Paint<'a>) -> Self {
        Self {
            catalog: paint.catalog,
            words: paint.words,
            tokens: paint.tokens,
            secondary: paint.look.secondary,
        }
    }
}

/// Existing callers keep their drawing context; the body reads only its narrow projection.
pub(super) fn borrowed_body(
    paint: &Paint<'_>,
    id: &ElementId,
    detail: BorrowedDetail<'_>,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    tool_body::render(&BodyPaint::from(paint), id, detail, window, cx)
}

pub(super) fn prose(
    paint: &Paint<'_>,
    id: ElementId,
    text: &str,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    tool_body::prose(&BodyPaint::from(paint), id, text, window, cx)
}

pub(super) fn header(
    paint: &Paint<'_>,
    index: usize,
    source: &impl ToolDetailSource,
    open: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let tool = source.tool();
    let display = match source.agent_message() {
        Some(message) => agent_message_display(message),
        None => match source.detail() {
            Some(detail) => tool_display(detail, &tool.title, tool.kind),
            None => return super::tool::legacy_head(paint, index, tool, open, cx),
        },
    };
    let title = match display.label {
        ToolLabel::Kind(kind) => paint.catalog.timeline_tool().label(kind).to_owned(),
        ToolLabel::Provided(text) => text.into_owned(),
        ToolLabel::AgentMessage(event) => paint
            .catalog
            .timeline_tool()
            .agent_message(event)
            .to_owned(),
    };
    let mark = if source.agent_message().is_some() {
        Glyph::Mark
    } else {
        icon(display.icon)
    };
    let color = super::tool::state_color(paint, tool.status);
    let state = paint.words.tool_status(tool.status);
    let summary = display
        .summary
        .map_or_else(|| state.to_owned(), |text| format!("{text} · {state}"));
    if !can_open_contents(display.contents, source) {
        return div()
            .flex()
            .items_center()
            .gap(px(spacing::S1_5))
            .type_(Type::Meta)
            .text_color(paint.look.secondary)
            .child(glyph(mark, spacing::S3_5, paint.look.secondary))
            .child(title)
            .child(div().truncate().text_color(color).child(summary))
            .into_any_element();
    }
    marked_work_row(
        paint,
        index,
        (Some(mark), title),
        Some((summary, color)),
        open,
        cx,
    )
}

pub(super) fn can_open(source: &impl ToolDetailSource) -> bool {
    if source.agent_message().is_some() {
        return true;
    }
    source.detail().is_none_or(|detail| {
        let tool = source.tool();
        can_open_contents(
            tool_display(detail, &tool.title, tool.kind).contents,
            source,
        )
    })
}

fn can_open_contents(contents: DetailContents, source: &impl ToolDetailSource) -> bool {
    source.failure().is_some()
        || contents == DetailContents::Present
        || (contents == DetailContents::Empty && source.tool().working())
}

fn icon(icon: ToolIcon) -> Glyph {
    match icon {
        ToolIcon::Wrench => Glyph::Wrench,
        ToolIcon::SquareTerminal => Glyph::Terminal,
        ToolIcon::Eye => Glyph::Eye,
        ToolIcon::Pencil => Glyph::Pencil,
        ToolIcon::Search => Glyph::Search,
        ToolIcon::Bot => Glyph::Bot,
        ToolIcon::Sparkles => Glyph::Sparkles,
        ToolIcon::Brain => Glyph::Brain,
        ToolIcon::MicVocal => Glyph::MicVocal,
    }
}

pub(super) fn body(
    paint: &Paint<'_>,
    index: usize,
    source: &impl ToolDetailSource,
    detail: &ToolCallDetail,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    borrowed_body(
        paint,
        &ElementId::NamedInteger("canonical-tool".into(), super::entries::id(index)),
        BorrowedDetail {
            detail,
            state: if source.tool().working() {
                DetailContentState::Loading
            } else {
                DetailContentState::Ready
            },
            failure: source.failure(),
        },
        window,
        cx,
    )
}

/// Sender text is provenance only, kept literal through the shared prose body.
pub(super) fn message_body(
    paint: &Paint<'_>,
    index: usize,
    source: &impl ToolDetailSource,
    message: &AgentMessageProvenance,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let sender = paint
        .catalog
        .timeline_tool()
        .from_agent(message.sender.id.as_str())
        .unwrap_or_else(|_| message.sender.id.as_str().to_owned());
    let text = format!("{sender}\n\n{}", message.text);
    div()
        .child(prose(
            paint,
            ElementId::NamedInteger("agent-delivery-prose".into(), super::entries::id(index)),
            &text,
            window,
            cx,
        ))
        .children(
            source
                .failure()
                .map(|reported| failure(paint, index, reported, window, cx)),
        )
}

/// An explicit failure stays readable even without a canonical detail body.
pub(super) fn failure(
    paint: &Paint<'_>,
    index: usize,
    failure: &ToolFailure,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    super::tool_body::failure(
        &BodyPaint::from(paint),
        ElementId::NamedInteger("canonical-tool-error".into(), super::entries::id(index)),
        failure,
        window,
        cx,
    )
}

#[cfg(test)]
mod tests;
