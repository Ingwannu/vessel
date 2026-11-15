//! Canonical DTOs choose existing kit output controls; no provider JSON decoding
//! or new file/network operation is introduced at the drawing boundary.

use super::entries::{Paint, marked_work_row};
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::output::{Line, Lines, Tone};
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Start, Type, TypeExt as _, lines_box, output_box, version_of};
use vessel_app_view::timeline::{
    DetailContents, MAX_LINES, Side, ToolDetailSource, ToolLabel, ToolSection, ToolSectionLabel,
    agent_message_display, json_text, tool_display, tool_sections, unified_lines,
};
use vessel_proto::timeline::{
    AgentMessageProvenance, DetailOffset, ToolCallDetail, ToolFailure, ToolIcon,
};

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

/// Only empty-content loading presentation, never execution or approval outcome.
#[derive(Clone, Copy)]
pub(super) enum DetailContentState {
    Loading,
    /// Content is ready to present, including work that is merely proposed.
    Ready,
}

/// Shared borrowed body for canonical timeline and permission presentation DTOs.
/// The caller retains identity/disclosure/approval; no `ToolView` or log is needed.
#[derive(Clone, Copy)]
pub(super) struct BorrowedDetail<'a> {
    pub(super) detail: &'a ToolCallDetail,
    pub(super) state: DetailContentState,
    pub(super) failure: Option<&'a ToolFailure>,
}

pub(super) fn borrowed_body(
    paint: &Paint<'_>,
    id: &ElementId,
    source: BorrowedDetail<'_>,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let BorrowedDetail {
        detail,
        state,
        failure: reported,
    } = source;
    let sections = tool_sections(detail);
    if sections.is_empty() && reported.is_none() {
        return div().child(super::tool_sheet::empty_at(
            paint,
            child_id(id, "loading"),
            matches!(state, DetailContentState::Loading),
        ));
    }
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S1_5))
        .children(sections.iter().enumerate().map(|(part, section)| {
            section_body(
                paint,
                child_id(id, &format!("section-{part}")),
                section,
                window,
                cx,
            )
        }))
        .children(
            reported
                .map(|reported| failure_at(paint, child_id(id, "failure"), reported, window, cx)),
        )
}

fn child_id(parent: &ElementId, part: &str) -> ElementId {
    ElementId::NamedChild(std::sync::Arc::new(parent.clone()), part.to_owned().into())
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
    failure_at(
        paint,
        ElementId::NamedInteger("canonical-tool-error".into(), super::entries::id(index)),
        failure,
        window,
        cx,
    )
}

fn failure_at(
    paint: &Paint<'_>,
    id: ElementId,
    failure: &ToolFailure,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    vessel_app_kit::alert::alert(
        vessel_app_kit::alert::Content {
            title: None,
            description: Some(vessel_app_kit::plain_text(
                id,
                &failure.message,
                version_of(&failure.message),
                window,
                cx,
            )),
            action: None,
        },
        vessel_app_kit::alert::Variant::Error,
        vessel_app_kit::alert::Size::Small,
        paint.tokens,
    )
}

/// Paseo's scrollable plain detail caps its body at 300 logical points.
const PROSE_MAX_HEIGHT: f32 = 300.0;
/// The source's UI prose uses a 22-point line, independently of code rows.
const PROSE_LINE_HEIGHT: f32 = 22.0;

/// Literal reasoning and plain tool bodies share this selectable UI prose.
pub(super) fn prose(
    paint: &Paint<'_>,
    id: ElementId,
    text: &str,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    div()
        .id(id.clone())
        .max_h(px(PROSE_MAX_HEIGHT))
        .overflow_y_scroll()
        .type_(Type::Body)
        .line_height(px(vessel_app_kit::fonts::ui(PROSE_LINE_HEIGHT)))
        .text_color(paint.tokens.neutral.foreground)
        .child(vessel_app_kit::plain_text(
            id,
            text,
            version_of(text),
            window,
            cx,
        ))
        .into_any_element()
}

fn section_body(
    paint: &Paint<'_>,
    id: ElementId,
    section: &ToolSection<'_>,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    match section {
        ToolSection::Terminal { command, output } => {
            let text = output.filter(|text| !text.is_empty()).map_or_else(
                || format!("$ {command}"),
                |output| format!("$ {command}\n\n{output}"),
            );
            output_box(id, &text, Start::Top, paint.tokens, window, cx)
        }
        ToolSection::Code {
            text, first_line, ..
        } => code(paint, id, text, *first_line, window, cx),
        ToolSection::Patch { text, .. } => patch(paint, id, text, window, cx),
        ToolSection::Edit { old, new, .. } => {
            super::tool::diff(paint, id, *old, new, window, cx).into_any_element()
        }
        ToolSection::Text(text) => output_box(id, text, Start::Top, paint.tokens, window, cx),
        ToolSection::Prose(text) => prose(paint, id, text, window, cx),
        ToolSection::Opaque { label, raw } => div()
            .flex()
            .flex_col()
            .child(
                div()
                    .type_(Type::Meta)
                    .text_color(paint.look.secondary)
                    .child(match label {
                        ToolSectionLabel::Input => paint.words.input(),
                        ToolSectionLabel::Output => paint.words.output(),
                    }),
            )
            .child(output_box(
                id,
                &json_text(raw),
                Start::Top,
                paint.tokens,
                window,
                cx,
            ))
            .into_any_element(),
    }
}

fn code(
    paint: &Paint<'_>,
    id: ElementId,
    text: &str,
    first: Option<DetailOffset>,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let version = version_of(&(text, first));
    let lines = lines_box(
        id,
        version,
        || Lines {
            lines: text
                .lines()
                .take(MAX_LINES)
                .enumerate()
                .map(|(index, line)| Line {
                    text: line.to_owned().into(),
                    tone: Tone::Plain,
                    number: first
                        .and_then(|offset| usize::try_from(offset.get()).ok())
                        .and_then(|offset| offset.checked_add(index)),
                })
                .collect(),
            copy: Some(text.to_owned().into()),
        },
        Start::Top,
        paint.tokens,
        window,
        cx,
    );
    preview(paint, lines, text.lines().nth(MAX_LINES).is_some())
}

fn patch(
    paint: &Paint<'_>,
    id: ElementId,
    text: &str,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let lines = lines_box(
        id,
        version_of(text),
        || Lines {
            lines: unified_lines(text)
                .0
                .into_iter()
                .map(|line| Line {
                    text: line.text.to_owned().into(),
                    number: line.number,
                    tone: match line.side {
                        Side::Added => Tone::Added,
                        Side::Removed => Tone::Removed,
                        Side::Context | Side::Header => Tone::Plain,
                    },
                })
                .collect(),
            copy: Some(text.to_owned().into()),
        },
        Start::Top,
        paint.tokens,
        window,
        cx,
    );
    preview(paint, lines, unified_lines(text).1)
}

fn preview(paint: &Paint<'_>, lines: AnyElement, cut: bool) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .child(lines)
        .children(cut.then(|| {
            div()
                .type_(Type::Meta)
                .text_color(paint.look.secondary)
                .child(paint.catalog.timeline_tool().preview_cut())
        }))
        .into_any_element()
}

#[cfg(test)]
mod tests;
