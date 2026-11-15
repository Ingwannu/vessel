//! The full-window lightbox uses kit controls and the existing motion tokens.

use super::{Gesture, PictureState, decode, pointer, source};
use crate::shell::Shell;
use gpui::{
    AnimationExt as _, KeyDownEvent, MouseDownEvent, ObjectFit, StatefulInteractiveElement as _,
    StyledImage as _, anchored, deferred, img,
};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::{IMAGE_SCRIM, spacing};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    MouseButton, ParentElement as _, Styled as _, Window, div, point, px, rgba,
};
use vessel_app_kit::{CardState, PillParts, Tokens, Type, TypeExt as _, motion, pill_parts};
use vessel_app_view::composer::viewer::{
    Extent, MAX_SCALE, MIN_SCALE, Viewport, ZOOM_IN, ZOOM_OUT,
};

pub(crate) fn render(
    shell: &mut Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let source = source(shell)?;
    let size = window.viewport_size();
    let canvas = Extent::new(f32::from(size.width), f32::from(size.height))?;
    let content = content(shell, source, canvas, tokens, window, cx);
    let overlay = div()
        .id("attachment-lightbox")
        .track_focus(&shell.areas.composer.lightbox.focus)
        .w(size.width)
        .h(size.height)
        .relative()
        .overflow_hidden()
        .occlude()
        .on_key_down(cx.listener(|shell, event: &KeyDownEvent, window, cx| {
            match event.keystroke.key.as_str() {
                "escape" => shell.close_image(window, cx),
                "+" | "=" => {
                    if let Some(viewport) = &mut shell.areas.composer.lightbox.viewport {
                        viewport.zoom_center(ZOOM_IN);
                        cx.notify();
                    }
                }
                "-" => {
                    if let Some(viewport) = &mut shell.areas.composer.lightbox.viewport {
                        viewport.zoom_center(ZOOM_OUT);
                        cx.notify();
                    }
                }
                "0" => {
                    if let Some(viewport) = &mut shell.areas.composer.lightbox.viewport {
                        viewport.reset();
                        cx.notify();
                    }
                }
                _ => {}
            }
            // A modal owns its key scope; Tab cannot reach a background field.
            cx.stop_propagation();
        }))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|shell, event: &MouseDownEvent, _window, _cx| {
                shell.areas.composer.lightbox.gesture =
                    pointer(event.position).map(Gesture::Pressed);
            }),
        )
        .on_mouse_move(cx.listener(|shell, event, _window, cx| shell.image_moved(event, cx)))
        .on_mouse_up(MouseButton::Left, cx.listener(Shell::image_released))
        .on_scroll(cx.listener(|shell, event, _window, cx| shell.image_scrolled(event, cx)))
        .on_pinch(cx.listener(|shell, event, _window, cx| shell.image_pinched(event, cx)))
        .child(content)
        .child(toolbar(shell, tokens, window, cx))
        .children(shell.areas.composer.lightbox.tips.render(tokens))
        .with_animation(
            "attachment-lightbox-arrive",
            motion::transition(motion::STANDARD),
            |overlay, done| overlay.bg(rgba(IMAGE_SCRIM).opacity(done)),
        );
    Some(
        deferred(
            anchored()
                .position(point(gpui::Pixels::ZERO, gpui::Pixels::ZERO))
                .child(overlay),
        )
        .with_priority(3)
        .into_any_element(),
    )
}

fn content(
    shell: &mut Shell,
    source: PictureState,
    canvas: Extent,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let decoded = match source {
        PictureState::Pending => return status(shell.words.composer().image_loading(), tokens),
        PictureState::Failed => return status(shell.words.composer().image_failed(), tokens),
        PictureState::Ready(source) => decode::load(source, window, cx),
    };
    let Some(decoded) = decoded else {
        return status(shell.words.composer().image_loading(), tokens);
    };
    let Ok(image) = decoded else {
        return status(shell.words.composer().image_failed(), tokens);
    };
    let pixels = image.size(0);
    let width = f32::from(gpui::Pixels::from(u32::from(pixels.width)));
    let height = f32::from(gpui::Pixels::from(u32::from(pixels.height)));
    let Some(extent) = Extent::new(width, height) else {
        return status(shell.words.composer().image_failed(), tokens);
    };
    let viewport = shell
        .areas
        .composer
        .lightbox
        .viewport
        .get_or_insert_with(|| Viewport::new(extent, canvas));
    viewport.resize(canvas);
    img(image)
        .absolute()
        .left(px(viewport.left()))
        .top(px(viewport.top()))
        .w(px(viewport.size().width()))
        .h(px(viewport.size().height()))
        .object_fit(ObjectFit::Contain)
        .into_any_element()
}

fn status(text: &str, tokens: &Tokens) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p(px(spacing::S4))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.to_owned())
        .into_any_element()
}

#[derive(Clone, Copy)]
enum Action {
    In,
    Out,
    Reset,
    Close,
}

/// Paseo lightbox pointer control is20px; the shared spacing scale has no20px step.
const POINTER_CONTROL: f32 = 20.0;

fn toolbar(shell: &Shell, tokens: &Tokens, window: &Window, cx: &mut Context<'_, Shell>) -> Div {
    let side = px(if vessel_app_kit::breakpoint::phone(window) {
        vessel_app_kit::breakpoint::TOUCH_TARGET
    } else {
        POINTER_CONTROL
    });
    let scale = shell.areas.composer.lightbox.viewport.map(Viewport::scale);
    let words = shell.words.composer();
    let controls = [
        (
            "image-zoom-in",
            Glyph::Plus,
            Action::In,
            words.image_zoom_in(),
            scale.is_some_and(|v| v < MAX_SCALE),
        ),
        (
            "image-zoom-out",
            Glyph::Minus,
            Action::Out,
            words.image_zoom_out(),
            scale.is_some_and(|v| v > MIN_SCALE),
        ),
        (
            "image-reset",
            Glyph::Maximize,
            Action::Reset,
            words.image_reset(),
            scale.is_some_and(|v| v != MIN_SCALE),
        ),
        (
            "image-close",
            Glyph::Close,
            Action::Close,
            words.image_close(),
            true,
        ),
    ];
    div()
        .absolute()
        .top(px(spacing::S2))
        .right(px(spacing::S2))
        .flex()
        .gap(px(spacing::S1))
        .children(
            controls
                .into_iter()
                .zip(&shell.areas.composer.lightbox.places)
                .map(|((id, mark, action, label, enabled), place)| {
                    let state = if enabled {
                        CardState::Ready
                    } else {
                        CardState::Faded
                    };
                    let button = action_button(id, mark, action, state, side, tokens, cx);
                    shell.areas.composer.lightbox.tips.on(
                        button,
                        place,
                        label.to_owned(),
                        vessel_app_kit::breakpoint::phone(window),
                    )
                }),
        )
}

fn action_button(
    id: &'static str,
    mark: Glyph,
    action: Action,
    state: CardState,
    side: gpui::Pixels,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> vessel_app_kit::ui::Stateful<Div> {
    pill_parts(
        id,
        PillParts {
            leading: Some(mark),
            ..PillParts::default()
        },
        false,
        side == px(vessel_app_kit::breakpoint::TOUCH_TARGET),
        tokens,
    )
    .size(side)
    .rounded(px(vessel_app_kit::theme::radius::MD))
    .bg(tokens.neutral.surface2)
    .when(state != CardState::Ready, |button| button.opacity(0.5))
    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
    .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
    .on_click(cx.listener(move |shell, _, window, cx| {
        cx.stop_propagation();
        if state != CardState::Ready {
            return;
        }
        if let Action::Close = action {
            shell.close_image(window, cx);
            return;
        }
        if let Some(viewport) = &mut shell.areas.composer.lightbox.viewport {
            match action {
                Action::In => viewport.zoom_center(ZOOM_IN),
                Action::Out => viewport.zoom_center(ZOOM_OUT),
                Action::Reset => viewport.reset(),
                Action::Close => {}
            }
            cx.notify();
        }
    }))
}
