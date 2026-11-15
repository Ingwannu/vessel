//! One borrowed canonical body for timelines and proposed permission details.
//! Identity, execution, approval, disclosure and I/O remain with the caller.
use super::entries::Paint;
use crate::shell::Shell;
use vessel_app_kit::output::{Line, Lines, Tone};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px, size,
};
use vessel_app_kit::{Start, Type, TypeExt as _, lines_box, output_box, skeleton, version_of};
use vessel_app_view::timeline::{
    MAX_LINES, Side, ToolSection, ToolSectionLabel, json_text, tool_sections, unified_lines,
};
use vessel_proto::timeline::{DetailOffset, ToolCallDetail, ToolFailure};

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
        return div().child(empty(
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
            reported.map(|reported| failure(paint, child_id(id, "failure"), reported, window, cx)),
        )
}

fn child_id(parent: &ElementId, part: &str) -> ElementId {
    ElementId::NamedChild(std::sync::Arc::new(parent.clone()), part.to_owned().into())
}

pub(super) fn failure(
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

/// A request supplies its own stable identity; it never fabricates a timeline row.
pub(super) fn empty(paint: &Paint<'_>, id: ElementId, working: bool) -> AnyElement {
    if !working {
        return div()
            .type_(Type::Body)
            .text_color(paint.look.secondary)
            .child(paint.catalog.timeline_tool().empty())
            .into_any_element();
    }
    let tokens = *paint.tokens;
    skeleton::pulse(id, move |opacity| {
        div().flex().flex_col().gap(px(spacing::S2)).children(
            [spacing::S24, spacing::S32, spacing::S16]
                .into_iter()
                .map(|width| {
                    skeleton::shape(
                        skeleton::Shape {
                            size: size(px(width), px(spacing::S2)),
                            rounding: skeleton::Rounding::Small,
                        },
                        opacity,
                        &tokens,
                    )
                }),
        )
    })
}

#[cfg(test)]
mod tests;
