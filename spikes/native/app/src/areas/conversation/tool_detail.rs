//! Canonical DTOs choose existing kit output controls; no provider JSON decoding
//! or new file/network operation is introduced at the drawing boundary.

use super::entries::{Paint, marked_work_row};
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::output::{Line, Lines, Tone};
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, IntoElement as _, ParentElement as _, Styled as _, Window,
    div, px,
};
use vessel_app_kit::{Start, Type, TypeExt as _, lines_box, output_box, version_of};
use vessel_app_view::timeline::{
    DetailContents, MAX_LINES, Side, ToolDetailSource, ToolLabel, ToolSection, ToolSectionLabel,
    json_text, tool_display, tool_sections, unified_lines,
};
use vessel_proto::timeline::{DetailOffset, ToolCallDetail, ToolIcon};

pub(super) fn header(
    paint: &Paint<'_>,
    index: usize,
    source: &impl ToolDetailSource,
    detail: &ToolCallDetail,
    open: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let tool = source.tool();
    let display = tool_display(detail, &tool.title, tool.kind);
    let title = match display.label {
        ToolLabel::Kind(kind) => paint.catalog.timeline_tool().label(kind).to_owned(),
        ToolLabel::Provided(text) => text.into_owned(),
    };
    let mark = icon(display.icon);
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
    let sections = tool_sections(detail);
    if sections.is_empty() && source.failure().is_none() {
        return div().child(super::tool_sheet::empty(
            paint,
            index,
            source.tool().working(),
        ));
    }
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S1_5))
        .children(sections.iter().enumerate().map(|(part, section)| {
            let id = ElementId::NamedInteger(
                format!("canonical-tool-{index}").into(),
                u64::try_from(part).unwrap_or(u64::MAX),
            );
            section_body(paint, id, section, window, cx)
        }))
        .children(source.failure().map(|failure| {
            vessel_app_kit::alert::alert(
                vessel_app_kit::alert::Content {
                    title: Some(failure.message.clone().into()),
                    description: None,
                    action: None,
                },
                vessel_app_kit::alert::Variant::Error,
                vessel_app_kit::alert::Size::Small,
                paint.tokens,
            )
        }))
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
