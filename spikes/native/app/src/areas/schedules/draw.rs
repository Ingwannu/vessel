//! `SchedulesScreen` frames and cached body states. Shared controls draw every
//! surface; no formatting, system lookup or node call runs during painting.
use crate::shell::Shell;
use std::sync::Arc;
use vessel_app_kit::{
    self as kit, ButtonKind, ButtonState, ControlSizing, Tokens, Type, TypeExt as _,
    theme::spacing,
    ui::{
        AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
        SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
    },
};
use vessel_app_view::schedules::{locale, page::Body, presentation::Notice};
const EMPTY_MAX_WIDTH: f32 = 420.0;
fn inset(touch: bool) -> kit::ui::Pixels {
    px(if touch { spacing::S3 } else { spacing::S6 })
}
fn centered() -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(spacing::S6))
        .p(px(spacing::S6))
}
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    if !shell.areas.schedules.ui.pending_focus.is_empty() || super::editor::needs_frame(shell) {
        cx.defer_in(window, Shell::schedules_overlay_frame);
    }
    let touch = kit::breakpoint::phone(window);
    let title = shell.t.lang.schedules().title;
    let header = if touch {
        let owner = cx.entity().downgrade();
        kit::compact_header(
            title,
            Vec::new(),
            move |window, cx| {
                let _closed = owner.update(cx, |shell, cx| shell.toggle_sidebar(window, cx));
            },
            tokens,
        )
    } else {
        kit::screen_header(title, Vec::new(), tokens, false)
    };
    let mut root = div()
        .relative()
        .size_full()
        .min_w_0()
        .flex()
        .flex_col()
        .bg(tokens.neutral.surface0)
        .text_color(tokens.neutral.foreground)
        .child(header);
    if !shell.areas.schedules.ui.editor.is_mounted()
        && let Some(notice) = shell.areas.schedules.ui.editor.opening_error()
    {
        root = root.child(row_notice(notice, tokens));
    }
    if shell.areas.schedules.data.page.filters_visible() {
        root = root.child(
            div()
                .px(inset(touch))
                .pt(px(spacing::S4))
                .child(super::filters::toolbar(shell, tokens, touch, cx)),
        );
    }
    root = root.child(body(shell, tokens, touch, cx));
    if let Some(popup) = super::filters::popup(shell, tokens, window, cx) {
        root = root.child(popup);
    }
    if let Some(menu) = super::overlays::menu(shell, tokens, touch, cx) {
        root = root.child(menu);
    }
    if let Some(dialog) = super::overlays::deletion(shell, tokens, window, touch, cx) {
        root = root.child(dialog);
    }
    if let Some(history) = super::history::render(shell, tokens, window, touch, cx) {
        root = root.child(history);
    }
    if let Some(editor) = super::editor::render(shell, tokens, window, touch, cx) {
        root = root.child(editor);
    }
    if let Some(picker) = super::editor::popup(shell, tokens, window, cx) {
        root = root.child(picker);
    }
    root.into_any_element()
}
fn body(shell: &Shell, tokens: &Tokens, touch: bool, cx: &mut Context<'_, Shell>) -> AnyElement {
    let data = &shell.areas.schedules.data;
    match data.body {
        Body::Loading => centered()
            .child(kit::loading::activity_indicator(
                "schedules-loading",
                kit::loading::Size::Large,
                tokens.neutral.foreground_muted,
            ))
            .into_any_element(),
        Body::Failed => failed(shell, tokens, touch, cx),
        Body::Empty if !data.page.filters_visible() => centered()
            .children(
                (!data.host_errors.is_empty())
                    .then(|| kit::alert::host_errors(data.host_errors.iter().cloned(), tokens)),
            )
            .child(empty(shell, tokens, touch, cx))
            .into_any_element(),
        Body::Empty | Body::Content => content(shell, tokens, touch, cx),
    }
}
fn failed(shell: &Shell, tokens: &Tokens, touch: bool, cx: &mut Context<'_, Shell>) -> AnyElement {
    let data = &shell.areas.schedules.data;
    let mut body = centered();
    if let Some(local) = &data.local {
        body = body.child(notice(local, tokens));
    } else if data.blocking.is_empty() {
        body = body.child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .text_center()
                .whitespace_normal()
                .child(shell.words.schedules().unable()),
        );
    } else {
        for message in &data.blocking {
            body = body.child(notice(message, tokens));
        }
    }
    if data.page.retry_available() {
        body = body.child(
            kit::button_with_kind_at(
                "schedules-retry",
                shell.words.schedules().retry(),
                ButtonKind::Ghost,
                ButtonState::Enabled,
                ControlSizing::from_compact(touch),
                tokens,
            )
            .on_click(cx.listener(|shell, _, _, cx| {
                shell.areas.schedules.data.page.retry_failed();
                shell.schedules_due();
                let _changed = shell.schedules_changed(cx);
                cx.notify();
            })),
        );
    }
    body.into_any_element()
}
fn content(shell: &Shell, tokens: &Tokens, touch: bool, cx: &mut Context<'_, Shell>) -> AnyElement {
    let data = &shell.areas.schedules.data;
    let mut content = div()
        .flex()
        .flex_col()
        .w_full()
        .flex_grow(1.0)
        .gap(px(spacing::S3))
        .pt(px(spacing::S4))
        .pb(px(spacing::S6));
    if !data.host_errors.is_empty() {
        content = content.child(div().px(inset(touch)).child(kit::alert::host_errors(
            data.host_errors.iter().cloned(),
            tokens,
        )));
    }
    if let Some(local) = &data.local {
        content = content.child(div().px(inset(touch)).child(notice(local, tokens)));
    }
    if data.body == Body::Content {
        let mut card = kit::settings::card(tokens);
        for (index, row) in data.rows.iter().enumerate() {
            card = card.child(super::row::render(shell, row, index, tokens, touch, cx));
        }
        content = content.child(div().px(inset(touch)).child(card));
    } else {
        content = content.child(centered().child(empty(shell, tokens, touch, cx)));
    }
    div()
        .id("schedules-scroll")
        .min_h_0()
        .flex_1()
        .flex()
        .flex_col()
        .overflow_y_scroll()
        .child(content)
        .into_any_element()
}
pub(super) fn notice(message: &Notice, tokens: &Tokens) -> Div {
    kit::alert::alert(
        kit::alert::Content {
            title: None,
            description: Some(
                div()
                    .child(SharedString::from(Arc::clone(&message.label)))
                    .into_any_element(),
            ),
            action: None,
        },
        crate::badge::alert_variant(message.variant),
        kit::alert::Size::ExtraSmall,
        tokens,
    )
}
fn empty(shell: &Shell, tokens: &Tokens, touch: bool, cx: &mut Context<'_, Shell>) -> Div {
    let bucket = shell.areas.schedules.data.page.bucket;
    let hint = locale::empty_hint(bucket, shell.words);
    let mut words = div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(spacing::S2))
        .child(
            div()
                .type_(Type::Body)
                .text_center()
                .whitespace_normal()
                .child(locale::empty_title(bucket, shell.words)),
        );
    if let Some(hint) = hint {
        words = words.child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .text_center()
                .whitespace_normal()
                .child(hint),
        );
    }
    let docs = if bucket == vessel_app_view::schedules::Bucket::Active {
        crate::docs_links::SCHEDULES.map(|destination| {
            kit::external_link::external_link(
                "schedules-docs",
                Some(shell.words.schedules().see_docs().into()),
                tokens,
            )
            .on_click(move |_, _, cx| cx.open_url(destination.url()))
        })
    } else {
        None
    };
    let words = words.children(docs);
    div()
        .w_full()
        .max_w(px(EMPTY_MAX_WIDTH))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(if hint.is_some() {
            spacing::S4
        } else {
            spacing::S3
        }))
        .child(kit::picker::glyph(
            kit::assets::Glyph::CalendarClock,
            kit::theme::icon::LG,
            tokens.neutral.foreground_muted,
        ))
        .child(words)
        .children(
            (bucket == vessel_app_view::schedules::Bucket::Active)
                .then(|| new_action(shell, "schedules-empty-new", tokens, touch, cx)),
        )
}

pub(super) fn row_notice(message: &Notice, tokens: &Tokens) -> Div {
    div().mt(px(spacing::S3)).child(notice(message, tokens))
}

pub(super) fn new_action(
    shell: &Shell,
    id: &'static str,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let expected = shell.areas.schedules.data.editor.visit();
    kit::button_control(
        id,
        kit::ButtonContent::IconAndLabel {
            icon: kit::ButtonIcon::new(kit::assets::Glyph::Plus),
            label: shell.words.schedules().form_new_title().into(),
        },
        kit::ButtonConfig::new(
            ButtonKind::Outline,
            if shell.areas.schedules.data.can_create
                && !shell.areas.schedules.ui.editor.is_mounted()
            {
                ButtonState::Enabled
            } else {
                ButtonState::Disabled
            },
            ControlSizing::from_compact(touch),
        ),
        tokens,
    )
    .on_click(
        cx.listener(move |shell, _, window, cx| {
            super::editor::open_new(shell, expected, window, cx)
        }),
    )
    .into_any_element()
}
