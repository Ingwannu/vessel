//! The option row uses one Markdown engine and keeps reading separate from selection.
use super::*;
use std::sync::Arc;
use vessel_app_kit::field::FieldId;

/// Existing Paseo option geometry, kept while splitting the renderer.
const OPTION_LINE: f32 = 22.0;
const OPTION_DESCRIPTION: f32 = 13.0;
const CONTROL_EDGE: f32 = 18.0;

/// One option: Paseo's row, a radio or a checkbox at its left.
pub(super) fn option_row<P: Place>(
    (index, cursor, source): (usize, Cursor, FieldId),
    choice: Choice,
    on: bool,
    multiple: bool,
    busy: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let control = mark(on, multiple, tokens);
    let value = choice.value;
    div()
        .id(("question-option", index))
        .flex()
        .items_start()
        .gap(px(spacing::S2))
        .px(px(spacing::S3))
        .py(px(spacing::S2))
        .rounded(px(spacing::S1_5))
        .when(on, |row| row.bg(neutral.surface2))
        .hover(move |style| style.bg(neutral.surface2))
        .cursor_pointer()
        .when(!busy, |row| {
            row.on_click(cx.listener(move |shell, _event, window, cx| {
                if shell.question_cursor::<P>(cursor) {
                    shell.question_pick_at::<P>(cursor.id, &value, window, cx);
                }
            }))
        })
        .child(control)
        .child(
            div()
                .flex_1()
                .min_w(px(spacing::S0))
                .flex()
                .flex_col()
                .gap(px(spacing::S1))
                .child(
                    div()
                        .debug_selector(|| "question-option-title".to_owned())
                        .line_height(px(OPTION_LINE))
                        .text_color(if on {
                            neutral.foreground
                        } else {
                            neutral.foreground_muted
                        })
                        .child(markdown_label(
                            option_text_key(source, cursor, index, "question-option-title"),
                            choice.title,
                        )),
                )
                .children(choice.description.map(|description| {
                    div()
                        .id(option_text_key(source, cursor, index, "question-option-description-hit"))
                        .text_size(px(OPTION_DESCRIPTION))
                        .text_color(neutral.foreground_muted)
                        .debug_selector(|| "question-option-description".to_owned())
                        // Reading/selecting a description must not pick its option.
                        .on_click(|_event, _window, cx| cx.stop_propagation())
                        .child(markdown(
                            option_text_key(source, cursor, index, "question-option-description"),
                            description,
                        ))
                })),
        )
}

/// The real form owns rich text caches; positions only distinguish its parts.
fn option_text_key(
    source: FieldId,
    cursor: Cursor,
    index: usize,
    part: &'static str,
) -> vessel_app_kit::ui::ElementId {
    vessel_app_kit::ui::ElementId::NamedChild(
        Arc::new(source.element_id(part)),
        format!("{}-{index}", cursor.step).into(),
    )
}

/// An option's mark: a radio (one answer) or a checkbox (several),
/// filled in the accent when on.
fn mark(on: bool, multiple: bool, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .mt(px(spacing::S0_5))
        .size(px(CONTROL_EDGE))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(if on {
            neutral.accent
        } else {
            neutral.foreground_extra_muted
        })
        .map(|control| {
            if multiple {
                control.rounded(px(spacing::S1))
            } else {
                control.rounded_full()
            }
        })
        .when(on && multiple, |control| {
            control.bg(neutral.accent).child(glyph(
                Glyph::Check,
                spacing::S3,
                neutral.accent_foreground,
            ))
        })
        .when(on && !multiple, |control| {
            control.child(
                div()
                    .size(px(spacing::S2))
                    .rounded_full()
                    .bg(neutral.accent),
            )
        })
}
