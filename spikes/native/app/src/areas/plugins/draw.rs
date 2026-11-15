use super::{Opener, SurfaceEvent, SurfaceView};
use gpui::StatefulInteractiveElement as _;
use std::{cell::Cell, rc::Rc, sync::Arc};
use vessel_app_kit::{
    ButtonConfig, ButtonContent, ButtonKind, ButtonState, ControlSizing, Tokens, Type,
    TypeExt as _,
    badge::{self, Size, Variant},
    theme::spacing,
    ui::{
        AnyElement, ClickEvent, Context, Div, ElementId, FluentBuilder as _,
        InteractiveElement as _, IntoElement, ParentElement as _, Styled as _, Window, div, px,
    },
};
use vessel_app_view::plugins::FieldValue;
use vessel_proto::plugins::{
    AttachmentUrl, Availability, ContributionId, Control, ControlContent, StatusTone, Surface,
    ToggleState,
};

pub(super) fn identity(view: &SurfaceView, control: &ContributionId) -> ElementId {
    // Includes kind, node and generation, so two hosts/surfaces never share a
    // Markdown/editor cache entry merely because local control IDs agree.
    ElementId::Name(
        format!(
            "plugin-{:?}-{:?}-{}",
            view.instance,
            view.state.address(),
            control.as_str()
        )
        .into(),
    )
}

pub(super) fn surface(
    view: &SurfaceView,
    surface: &Surface,
    tokens: &Tokens,
    live: bool,
    window: &mut Window,
    cx: &mut Context<'_, SurfaceView>,
) -> Div {
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(px(spacing::S3))
        .children(
            surface
                .controls
                .iter()
                .filter_map(|control| draw_control(view, control, tokens, live, window, cx)),
        )
}

fn draw_control(
    view: &SurfaceView,
    control: &Control,
    tokens: &Tokens,
    live: bool,
    window: &mut Window,
    cx: &mut Context<'_, SurfaceView>,
) -> Option<AnyElement> {
    let element = match &control.content {
        ControlContent::Text { text } => div()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground)
            .child(text.as_str().to_owned())
            .into_any_element(),
        ControlContent::Markdown { text } => markdown(view, &control.id, text),
        ControlContent::Stack { surface: nested } => {
            surface(view, nested, tokens, live, window, cx).into_any_element()
        }
        ControlContent::Row { surface: nested } => div()
            .flex()
            .flex_row()
            .flex_wrap()
            .min_w_0()
            .gap(px(spacing::S2))
            .children(
                nested
                    .controls
                    .iter()
                    .filter_map(|child| draw_control(view, child, tokens, live, window, cx)),
            )
            .into_any_element(),
        ControlContent::Section {
            title,
            surface: nested,
        } => vessel_app_kit::settings::section(
            title.as_str().to_owned(),
            surface(view, nested, tokens, live, window, cx),
            tokens,
        )
        .into_any_element(),
        ControlContent::TextField { .. } => {
            view.fields.get(&control.id)?.clone().into_any_element()
        }
        ControlContent::Choice { label, options, .. } => choice(
            view,
            &control.id,
            label,
            options,
            Look { tokens, live },
            window,
            cx,
        ),
        ControlContent::Toggle {
            label,
            state: declared,
            ..
        } => toggle(
            view,
            &control.id,
            label,
            *declared,
            Look { tokens, live },
            window,
            cx,
        ),
        ControlContent::Status { label, tone } => badge::badge(
            label.as_str().to_owned(),
            match tone {
                StatusTone::Neutral => Variant::Muted,
                StatusTone::Success => Variant::Success,
                StatusTone::Warning => Variant::Warning,
                StatusTone::Danger => Variant::Error,
            },
            Size::Small,
            None,
            tokens,
        )
        .into_any_element(),
        ControlContent::Button { button } => {
            draw_button(view, &control.id, button, Look { tokens, live }, window, cx)?
        }
    };
    Some(element)
}

fn config(kind: ButtonKind, enabled: bool, window: &Window) -> ButtonConfig {
    ButtonConfig::new(
        kind,
        if enabled {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        if narrow(window) {
            ControlSizing::Touch
        } else {
            ControlSizing::Pointer
        },
    )
}
pub(super) fn narrow(window: &Window) -> bool {
    window.viewport_size().width < px(vessel_app_kit::picker::COMPACT)
}

#[derive(Clone, Copy)]
struct Look<'a> {
    tokens: &'a Tokens,
    live: bool,
}

fn choice(
    view: &SurfaceView,
    control_id: &ContributionId,
    label: &vessel_proto::plugins::Caption,
    options: &[vessel_proto::plugins::ChoiceOption],
    look: Look<'_>,
    window: &mut Window,
    cx: &mut Context<'_, SurfaceView>,
) -> AnyElement {
    let selected = match view.state.value(control_id) {
        Some(FieldValue::Choice(selected)) => selected.as_ref(),
        Some(FieldValue::Text(_) | FieldValue::Toggle(_)) | None => None,
    };
    let choices = div()
        .flex()
        .flex_wrap()
        .gap(px(spacing::S2))
        .children(options.iter().map(|option| {
            let field = control_id.clone();
            let option_id = option.id.clone();
            let kind = if selected == Some(&option.id) {
                ButtonKind::Primary
            } else {
                ButtonKind::Outline
            };
            vessel_app_kit::button_control(
                ElementId::Name(
                    format!("{:?}-{}", identity(view, control_id), option.id.as_str()).into(),
                ),
                ButtonContent::Label(option.title.as_str().to_owned().into()),
                config(kind, look.live, window),
                look.tokens,
            )
            .on_click(cx.listener(move |view, _event, _window, cx| {
                view.change(&field, FieldValue::Choice(Some(option_id.clone())), cx);
            }))
        }));
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(vessel_app_kit::kicker(
            label.as_str().to_owned(),
            look.tokens,
        ))
        .child(choices)
        .into_any_element()
}

fn toggle(
    view: &SurfaceView,
    control_id: &ContributionId,
    label: &vessel_proto::plugins::Caption,
    declared: ToggleState,
    look: Look<'_>,
    window: &mut Window,
    cx: &mut Context<'_, SurfaceView>,
) -> AnyElement {
    let state = match view.state.value(control_id) {
        Some(FieldValue::Toggle(state)) => *state,
        Some(FieldValue::Text(_) | FieldValue::Choice(_)) | None => declared,
    };
    let field = control_id.clone();
    let next = match state {
        ToggleState::On => ToggleState::Off,
        ToggleState::Off => ToggleState::On,
    };
    let switch = vessel_app_kit::settings::switch(
        identity(view, control_id),
        state == ToggleState::On,
        look.live,
        narrow(window),
        0,
        look.tokens,
    );
    let switch = switch.when(look.live, |switch| {
        switch.on_click(cx.listener(move |view, _event, _window, cx| {
            view.change(&field, FieldValue::Toggle(next), cx);
        }))
    });
    vessel_app_kit::settings::row(
        label.as_str().to_owned(),
        None,
        Some(switch.into_any_element()),
        true,
        look.tokens,
    )
    .into_any_element()
}

fn draw_button(
    view: &SurfaceView,
    control_id: &ContributionId,
    button: &vessel_proto::plugins::Button,
    look: Look<'_>,
    window: &mut Window,
    cx: &mut Context<'_, SurfaceView>,
) -> Option<AnyElement> {
    if button.availability == Availability::Hidden {
        return None;
    }
    let field = control_id.clone();
    let behavior = button.behavior.clone();
    let anchor = Rc::new(Cell::new(None));
    let measured = Rc::clone(&anchor);
    let available = look.live && button.availability == Availability::Enabled;
    let element = vessel_app_kit::button_control(
        identity(view, &field),
        ButtonContent::Label(button.title.as_str().to_owned().into()),
        config(ButtonKind::Secondary, available, window),
        look.tokens,
    )
    .relative()
    .child(vessel_app_kit::picker::anchor_probe(&anchor))
    .on_click(cx.listener(move |view, event: &ClickEvent, window, cx| {
        view.press(
            &field,
            &behavior,
            super::popovers::Trigger {
                at: event.position(),
                bounds: measured.get(),
            },
            window,
            cx,
        );
    }))
    .into_any_element();
    Some(
        div()
            .debug_selector(|| "plugin-surface-button".to_owned())
            .child(element)
            .into_any_element(),
    )
}

fn markdown(
    view: &SurfaceView,
    control_id: &ContributionId,
    text: &vessel_proto::plugins::ContentText,
) -> AnyElement {
    let markdown = vessel_app_kit::markdown_guarded(
        identity(view, control_id),
        text.as_str().to_owned(),
        vessel_app_kit::MarkdownGuards {
            images: vessel_app_kit::ImagePolicy::RefuseAll,
            on_link: Arc::clone(&view.markdown_link),
        },
    );
    div()
        .id(identity(view, control_id))
        .debug_selector(|| "plugin-surface-markdown".to_owned())
        .child(markdown)
        .into_any_element()
}

/// Install once so drawing and delayed delivery retain the exact same handler.
pub(super) fn link_handler(
    opener: Opener,
    cx: &Context<'_, SurfaceView>,
) -> vessel_app_kit::MarkdownLinkHandler {
    let owner = cx.weak_entity();
    Arc::new(
        move |link, _event, window, cx| match link.as_str().parse::<AttachmentUrl>() {
            Ok(url) => {
                drop(owner.update(cx, |view, cx| {
                    match view.with_registry(|registry| view.state.check_current(registry)) {
                        Ok(()) => opener(&url, window, cx),
                        Err(problem) => cx.emit(SurfaceEvent::Rejected(problem)),
                    }
                }));
            }
            Err(_untrusted) => {
                drop(owner.update(cx, |_view, cx| cx.emit(SurfaceEvent::LinkRejected)));
            }
        },
    )
}
