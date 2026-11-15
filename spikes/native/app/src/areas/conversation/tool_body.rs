//! One borrowed canonical body for timelines and proposed permission details.
//! Identity, execution, approval, disclosure and I/O remain with the caller.
use vessel_app_kit::output::{AnnotatedLines, Line, Lines, Tone, annotated_lines_box};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, App, Div, ElementId, FluentBuilder as _, Hsla, InteractiveElement as _,
    IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window,
    div, px, size,
};
use vessel_app_kit::{Start, Type, TypeExt as _, lines_box, output_box, skeleton, version_of};
use vessel_app_view::timeline::{
    MAX_LINES, Side, ToolSection, ToolSectionLabel, diff_lines, json_text, tool_sections,
    unified_lines, word_highlights,
};
use vessel_proto::timeline::{DetailOffset, ToolCallDetail, ToolFailure};

/// Borrowed drawing inputs only; no Entry, Seat, session or I/O capability.
pub(super) struct BodyPaint<'a> {
    pub(super) catalog: &'static vessel_app_view::localization::Translator,
    pub(super) words: &'static vessel_app_view::ConversationWords,
    pub(super) tokens: &'a vessel_app_kit::Tokens,
    pub(super) secondary: Hsla,
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

pub(super) fn render(
    paint: &BodyPaint<'_>,
    id: &ElementId,
    source: BorrowedDetail<'_>,
    window: &mut Window,
    cx: &mut App,
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
    paint: &BodyPaint<'_>,
    id: ElementId,
    failure: &ToolFailure,
    window: &mut Window,
    cx: &mut App,
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
    paint: &BodyPaint<'_>,
    id: ElementId,
    text: &str,
    window: &mut Window,
    cx: &mut App,
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
    paint: &BodyPaint<'_>,
    id: ElementId,
    section: &ToolSection<'_>,
    window: &mut Window,
    cx: &mut App,
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
            computed_diff(paint, id, *old, new, window, cx).into_any_element()
        }
        ToolSection::Text(text) => output_box(id, text, Start::Top, paint.tokens, window, cx),
        ToolSection::Prose(text) => prose(paint, id, text, window, cx),
        ToolSection::Opaque { label, raw } => div()
            .flex()
            .flex_col()
            .child(
                div()
                    .type_(Type::Meta)
                    .text_color(paint.secondary)
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
    paint: &BodyPaint<'_>,
    id: ElementId,
    text: &str,
    first: Option<DetailOffset>,
    window: &mut Window,
    cx: &mut App,
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
    paint: &BodyPaint<'_>,
    id: ElementId,
    text: &str,
    window: &mut Window,
    cx: &mut App,
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

fn preview(paint: &BodyPaint<'_>, lines: AnyElement, cut: bool) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .child(lines)
        .children(cut.then(|| {
            div()
                .type_(Type::Meta)
                .text_color(paint.secondary)
                .child(paint.catalog.timeline_tool().preview_cut())
        }))
        .into_any_element()
}

/// A request supplies its own stable identity; it never fabricates a timeline row.
pub(super) fn empty(paint: &BodyPaint<'_>, id: ElementId, working: bool) -> AnyElement {
    if !working {
        return div()
            .type_(Type::Body)
            .text_color(paint.secondary)
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

/// One computed diff body shared by canonical and legacy callers.
pub(super) fn computed_diff(
    paint: &BodyPaint<'_>,
    id: ElementId,
    old: Option<&str>,
    new: &str,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    // Both texts' versions (`kit::version_of`, their hashes): a change to
    // either splits the diff again (their lengths alone kept a first diff
    // on screen after a second of the same lengths).
    let version = version_of(old.unwrap_or("")).rotate_left(1) ^ version_of(new);
    let lines = annotated_lines_box(
        id,
        version,
        || {
            let (lines, _) = diff_lines(old, new);
            let highlights = word_highlights(&lines);
            let emphasis = lines
                .iter()
                .enumerate()
                .map(|(index, _)| highlights.for_line(index).to_vec())
                .collect();
            AnnotatedLines {
                lines: Lines {
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
                },
                emphasis,
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
        .gap(px(spacing::S1))
        .child(lines)
        .when(cut, |block| {
            block.child(
                div()
                    .type_(Type::Meta)
                    .text_color(paint.secondary)
                    .child(paint.words.diff_cut(MAX_LINES)),
            )
        })
}

#[cfg(test)]
mod tests;
