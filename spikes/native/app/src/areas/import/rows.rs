//! A bounded virtual list keeps long agent histories responsive on a phone.

use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::spacing::{S1, S2, S3, S4};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window,
    div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::history::{Entry, Selection};

/// Leave room for scope, search and persistent import actions at every size.
const VIEWPORT_SHARE: f32 = 0.30;

pub(super) fn list(
    shell: &Shell,
    _tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let capture = super::intent::Capture::of(shell);
    let height = f32::from(window.viewport_size().height).mul_add(VIEWPORT_SHARE, 0.0);
    div()
        .h(px(height))
        .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
        .w_full()
        .min_w_0()
        .overflow_hidden()
        .child(shell.areas.import.list.element(cx.processor(
            move |shell: &mut Shell, index: usize, _window, cx| {
                if !capture.accepts(shell) {
                    return div().into_any_element();
                }
                let tokens = vessel_app_kit::tokens(cx);
                let area = &shell.areas.import;
                let Some(id) = area.visible.get(index) else {
                    return div().into_any_element();
                };
                let Some(entry) = area.model.entries().iter().find(|entry| &entry.id == id) else {
                    return div().into_any_element();
                };
                row(shell, entry, &tokens, cx)
            },
        )))
}

fn row(shell: &Shell, entry: &Entry, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> AnyElement {
    let area = &shell.areas.import;
    let words = shell.words.history();
    let selected = area.model.selection(&entry.id);
    let (icon, action) = match selected {
        Selection::Selected => (Glyph::SquareCheck, words.selected()),
        Selection::Available => (Glyph::Circle, words.select()),
        Selection::Existing(_) => (Glyph::ChevronRight, words.open()),
    };
    let listener = super::intent::listener(
        shell,
        super::intent::Action::Row {
            id: entry.id.clone(),
            selection: selected,
        },
    );
    div()
        .id(ElementId::Name(
            format!("history-row-{}", entry.id.as_str()).into(),
        ))
        .flex()
        .items_start()
        .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
        .gap(px(S3))
        .px(px(S2))
        .py(px(S3))
        .min_w_0()
        .rounded(px(vessel_app_kit::theme::radius::R2))
        .when(selected == Selection::Selected, |row| {
            row.bg(tokens.neutral.surface2)
        })
        .when(!area.model.importing(), |row| {
            row.cursor_pointer()
                .hover(|row| row.bg(tokens.neutral.surface1))
        })
        .child(vessel_app_kit::glyph(
            icon,
            S4,
            tokens.neutral.foreground_muted,
        ))
        .child(details(shell, entry, tokens))
        .child(
            div()
                .flex_shrink_0()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(action),
        )
        .on_click(cx.listener(move |shell, _, window, cx| listener(shell, window, cx)))
        .into_any_element()
}

fn details(shell: &Shell, entry: &Entry, tokens: &Tokens) -> Div {
    let words = shell.words.history();
    let title = entry
        .title
        .as_ref()
        .map_or_else(|| words.untitled().to_owned(), ToString::to_string);
    let activity = entry.updated_at.map_or_else(
        || words.unknown_time().to_owned(),
        |at| {
            shell
                .t
                .lang
                .machines()
                .ago(vessel_app_view::ago::Ago::of(at, chrono::Utc::now()))
        },
    );
    let failure = shell
        .areas
        .import
        .model
        .outcomes()
        .iter()
        .find(|outcome| outcome.id == entry.id)
        .and_then(|outcome| outcome.result.as_ref().err());
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(S1))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .truncate()
                .child(title),
        )
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .truncate()
                .child(entry.cwd.to_string_lossy().into_owned()),
        )
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_extra_muted)
                .child(activity),
        )
        .children(failure.map(|problem| {
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.destructive)
                .child(problem.words(shell.words).to_owned())
        }))
}
