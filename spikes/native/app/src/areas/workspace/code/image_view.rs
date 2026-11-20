//! The Files tab's image panel drawn (`panels.rs`, F-F40): Loading, the
//! error's words, or the image fitted to the panel and zoomable as the
//! composer's lightbox is (wheel at the cursor, pinch, drag once zoomed in,
//! a double click fits again), with Zoom in, Zoom out and Reset view in its
//! top right corner while the pointer is over it.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    MouseMoveEvent, ObjectFit, PinchEvent, ScrollDelta, ScrollEvent, StyledImage as _, img,
};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::icon_chrome::Presence;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::toolbar_button::{ToolbarButton, ToolbarConfig};
use vessel_app_kit::ui::{
    AnyElement, Bounds, Context, InteractiveElement as _, IntoElement as _, MouseButton,
    ParentElement as _, Pixels, StatefulInteractiveElement as _, Styled as _, Window, canvas, div,
    px,
};
use vessel_app_kit::{ButtonState, ControlSizing, Tokens};
use vessel_app_view::composer::viewer::{
    Extent, Fit, MAX_SCALE, MIN_SCALE, Position, Viewport, ZOOM_IN, ZOOM_OUT,
};

use super::frame::note;
use super::panels::{ImagePanel, ImageState};
use crate::shell::Shell;

/// The image panel: Loading, the error's words, or the image zoomable.
pub(super) fn image(
    shell: &Shell,
    image: &ImagePanel,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.viewers();
    let picture = match &image.state {
        ImageState::Loading => return note(words.loading_file, tokens).into_any_element(),
        ImageState::Failed(why) => {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .p(px(spacing::S4))
                .text_color(tokens.danger)
                .child(why.clone())
                .into_any_element();
        }
        ImageState::Ready(picture) => picture,
    };
    let probe = Rc::clone(&image.canvas);
    let drawn = probe
        .get()
        .and_then(|bounds| decoded(picture, bounds, window, cx));
    let mut element = div()
        .id("code-image")
        .size_full()
        .relative()
        .overflow_hidden()
        .role(gpui::Role::Image)
        .child(
            canvas(
                move |bounds, window, _cx| {
                    if probe.get() != Some(bounds) {
                        probe.set(Some(bounds));
                        window.refresh();
                    }
                },
                |_, (), _, _| {},
            )
            .absolute()
            .size_full(),
        );
    if let Some((content, canvas_size, decoded)) = drawn {
        let viewport = image
            .viewport
            .filter(|viewport| viewport.canvas() == canvas_size)
            .unwrap_or_else(|| Viewport::fitted(content, canvas_size, Fit::Panel));
        element = element.child(
            img(decoded)
                .absolute()
                .left(px(viewport.left()))
                .top(px(viewport.top()))
                .w(px(viewport.size().width()))
                .h(px(viewport.size().height()))
                .object_fit(ObjectFit::Contain),
        );
        cx.defer_in(window, move |shell, _window, _cx| {
            shell.image_fitted(viewport);
        });
    }
    with_input(element, cx)
        .child(toolbar(shell, image, tokens, window, cx))
        .into_any_element()
}

/// The image's decoded pixels and size, and the panel's, once both are
/// known: GPUI decodes asynchronously, its asset cache keyed by the image.
fn decoded(
    picture: &std::sync::Arc<gpui::Image>,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<(Extent, Extent, std::sync::Arc<gpui::RenderImage>)> {
    let canvas = Extent::new(f32::from(bounds.size.width), f32::from(bounds.size.height))?;
    let decoded = crate::areas::composer::decode_image(
        gpui::ImageSource::Image(std::sync::Arc::clone(picture)),
        window,
        cx,
    )?
    .ok()?;
    let pixels = decoded.size(0);
    let content = Extent::new(
        f32::from(Pixels::from(u32::from(pixels.width))),
        f32::from(Pixels::from(u32::from(pixels.height))),
    )?;
    Some((content, canvas, decoded))
}

/// The pointer's input over the image: hover shows the toolbar, a drag
/// pans, a double click fits again, the wheel and a pinch zoom.
fn with_input(
    element: gpui::Stateful<gpui::Div>,
    cx: &mut Context<'_, Shell>,
) -> gpui::Stateful<gpui::Div> {
    element
        .on_hover(cx.listener(|shell, hovered: &bool, _window, cx| {
            if let Some(image) = shell.shown_image() {
                image.hovered = *hovered;
                cx.notify();
            }
        }))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|shell, event: &gpui::MouseDownEvent, _window, cx| {
                if event.click_count >= 2 {
                    shell.image_zoom(Zoom::Reset, cx);
                } else if let Some(image) = shell.shown_image() {
                    image.dragged = at(&image.canvas, event.position);
                }
            }),
        )
        .on_mouse_move(cx.listener(|shell, event: &MouseMoveEvent, _window, cx| {
            shell.image_dragged(event, cx);
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|shell, _event, _window, _cx| {
                if let Some(image) = shell.shown_image() {
                    image.dragged = None;
                }
            }),
        )
        .on_scroll(cx.listener(|shell, event: &ScrollEvent, _window, cx| {
            shell.image_scrolled_here(event, cx);
        }))
        .on_pinch(cx.listener(|shell, event: &PinchEvent, _window, cx| {
            shell.image_pinched_here(event, cx);
        }))
}

/// A window point as a point of the canvas measured in `probe`.
fn at(probe: &Rc<Cell<Option<Bounds<Pixels>>>>, point: gpui::Point<Pixels>) -> Option<Position> {
    let bounds = probe.get()?;
    // `mul_add`: the difference without an arithmetic operator (the lints).
    Position::new(
        f32::from(bounds.origin.x).mul_add(-1.0, f32::from(point.x)),
        f32::from(bounds.origin.y).mul_add(-1.0, f32::from(point.y)),
    )
}

/// A toolbar action.
#[derive(Debug, Clone, Copy)]
pub(super) enum Zoom {
    In,
    Out,
    Reset,
}

/// Paseo's viewport toolbar (`zoomable-viewport/toolbar.tsx`), as the
/// lightbox draws it, without its Close.
fn toolbar(
    shell: &Shell,
    image: &ImagePanel,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.viewers();
    let sizing = ControlSizing::from_compact(vessel_app_kit::breakpoint::phone(window));
    let presence = if image.hovered || sizing == ControlSizing::Touch {
        Presence::Shown
    } else {
        Presence::Hidden
    };
    let scale = image.viewport.map(Viewport::scale);
    let actions = [
        (
            "code-image-zoom-in",
            Glyph::ZoomIn,
            Zoom::In,
            words.zoom_in,
            scale.is_some_and(|v| v < MAX_SCALE),
        ),
        (
            "code-image-zoom-out",
            Glyph::ZoomOut,
            Zoom::Out,
            words.zoom_out,
            scale.is_some_and(|v| v > MIN_SCALE),
        ),
        (
            "code-image-reset",
            Glyph::Scan,
            Zoom::Reset,
            words.reset_view,
            scale.is_some_and(|v| v != MIN_SCALE),
        ),
    ];
    div()
        .absolute()
        .top(px(spacing::S2))
        .right(px(spacing::S2))
        .flex()
        .gap(px(spacing::S1))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .children(
            actions
                .into_iter()
                .map(|(id, glyph, zoom, label, enabled)| {
                    let availability = if enabled {
                        ButtonState::Enabled
                    } else {
                        ButtonState::Disabled
                    };
                    ToolbarButton::viewport(
                        id,
                        label,
                        glyph,
                        ToolbarConfig::new(sizing, availability),
                        presence,
                        tokens,
                    )
                    .on_click(cx.listener(move |shell, _, _window, cx| shell.image_zoom(zoom, cx)))
                }),
        )
        .into_any_element()
}

impl Shell {
    /// The image panel of the file shown, if it has one.
    fn shown_image(&mut self) -> Option<&mut ImagePanel> {
        self.areas
            .workspace
            .code
            .files
            .viewer
            .as_mut()?
            .look
            .image
            .as_mut()
    }

    /// The viewport drawn this frame, kept for the next input.
    fn image_fitted(&mut self, viewport: Viewport) {
        if let Some(image) = self.shown_image()
            && image.viewport != Some(viewport)
        {
            image.viewport = Some(viewport);
        }
    }

    fn image_zoom(&mut self, zoom: Zoom, cx: &mut Context<'_, Self>) {
        if let Some(viewport) = self.shown_image().and_then(|image| image.viewport.as_mut()) {
            match zoom {
                Zoom::In => viewport.zoom_center(ZOOM_IN),
                Zoom::Out => viewport.zoom_center(ZOOM_OUT),
                Zoom::Reset => viewport.reset(),
            }
            cx.notify();
        }
    }

    fn image_dragged(&mut self, event: &MouseMoveEvent, cx: &mut Context<'_, Self>) {
        let Some(image) = self.shown_image() else {
            return;
        };
        let (Some(from), Some(to)) = (image.dragged, at(&image.canvas, event.position)) else {
            return;
        };
        if let Some(viewport) = image.viewport.as_mut() {
            viewport.drag(from, to);
            image.dragged = Some(to);
            cx.notify();
        }
    }

    fn image_scrolled_here(&mut self, event: &ScrollEvent, cx: &mut Context<'_, Self>) {
        let Some(image) = self.shown_image() else {
            return;
        };
        let focal = at(&image.canvas, event.position);
        if let (Some(focal), Some(viewport)) = (focal, image.viewport.as_mut()) {
            let delta = match event.delta {
                ScrollDelta::Pixels(moved) => f32::from(moved.y),
                ScrollDelta::Lines(moved) => moved.y.mul_add(spacing::S4, 0.0),
            };
            viewport.scroll(delta, focal);
            cx.notify();
        }
        cx.stop_propagation();
    }

    fn image_pinched_here(&mut self, event: &PinchEvent, cx: &mut Context<'_, Self>) {
        let Some(image) = self.shown_image() else {
            return;
        };
        let focal = at(&image.canvas, event.position);
        if let (Some(focal), Some(viewport)) = (focal, image.viewport.as_mut()) {
            viewport.zoom(event.delta.mul_add(1.0, 1.0), focal);
            cx.notify();
        }
        cx.stop_propagation();
    }
}
