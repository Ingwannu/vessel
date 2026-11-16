//! The full-window lightbox uses kit controls and the existing motion tokens.

use super::{Gesture, PictureState, decode, pointer, source};
use crate::shell::Shell;
use gpui::{
    AnimationExt as _, KeyDownEvent, MouseDownEvent, ObjectFit, StatefulInteractiveElement as _,
    StyledImage as _, anchored, deferred, img,
};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::icon_chrome::Presence;
use vessel_app_kit::theme::{IMAGE_SCRIM, spacing};
use vessel_app_kit::toolbar_button::{ToolbarButton, ToolbarConfig};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, MouseButton,
    ParentElement as _, Styled as _, Window, div, point, px, rgba,
};
use vessel_app_kit::{ButtonState, ControlSizing, Tokens, Type, TypeExt as _, motion};
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
    let region = Region::of(size);
    let canvas = Extent::new(f32::from(size.width), f32::from(region.viewport_height))?;
    let content = content(shell, source, canvas, tokens, window, cx);
    let viewport = div()
        .id("attachment-lightbox")
        .absolute()
        .left_0()
        .top(region.top)
        .w(size.width)
        .h(region.viewport_height)
        .overflow_hidden()
        .occlude()
        .on_hover(cx.listener(|shell, hovered: &bool, _window, cx| {
            shell.areas.composer.lightbox.hovered = *hovered;
            cx.notify();
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
        .with_animation(
            "attachment-lightbox-arrive",
            motion::transition(motion::STANDARD),
            |viewport, done| viewport.bg(rgba(IMAGE_SCRIM).opacity(done)),
        );
    let overlay = div()
        .id("attachment-lightbox-window")
        .track_focus(&shell.areas.composer.lightbox.focus)
        .w(size.width)
        .h(size.height)
        .relative()
        .on_key_down(cx.listener(|shell, event: &KeyDownEvent, window, cx| {
            shell.image_key(event.keystroke.key.as_str(), window, cx);
            // A modal owns its key scope; Tab cannot reach a background field.
            cx.stop_propagation();
        }))
        .children(bar_shade(region, size.width, cx))
        .child(viewport);
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

/// Where the lightbox lies in the window. Paseo's modal is its own window
/// chrome root (`WindowChromeRootRegion`, attachment-lightbox.tsx:85), so the
/// window's controls stay usable: here the image viewport starts below the
/// app's own title bar, the bar beside the controls is shaded and pressing it
/// dismisses as Paseo's backdrop does, and the controls' corner is left
/// uncovered. With the system's title bar both are empty.
#[derive(Clone, Copy)]
struct Region {
    /// The viewport's top: the app's own bar's height.
    top: gpui::Pixels,
    /// The viewport's height below that bar.
    viewport_height: gpui::Pixels,
    /// The width of the window controls' corner, left uncovered.
    controls: gpui::Pixels,
}

impl Region {
    fn of(size: gpui::Size<gpui::Pixels>) -> Self {
        let height = f32::from(size.height);
        let top = f32::from(vessel_app_kit::chrome::bar_height()).min(height);
        // `mul_add`: the difference without an arithmetic operator (the lints).
        Self {
            top: px(top),
            viewport_height: px(top.mul_add(-1.0, height)),
            controls: vessel_app_kit::chrome::controls_corner().width,
        }
    }
}

fn bar_shade(
    region: Region,
    width: gpui::Pixels,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let shaded = f32::from(region.controls)
        .mul_add(-1.0, f32::from(width))
        .max(0.0);
    (region.top > gpui::Pixels::ZERO).then(|| {
        div()
            .id("attachment-lightbox-bar")
            .absolute()
            .left_0()
            .top_0()
            .w(px(shaded))
            .h(region.top)
            .occlude()
            .on_click(cx.listener(|shell, _event, window, cx| shell.close_image(window, cx)))
            .with_animation(
                "attachment-lightbox-bar-arrive",
                motion::transition(motion::STANDARD),
                |shade, done| shade.bg(rgba(IMAGE_SCRIM).opacity(done)),
            )
            .into_any_element()
    })
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

/// One toolbar action: its element id, glyph, effect, accessible name and
/// whether the viewport's scale admits it now.
struct Control {
    id: &'static str,
    glyph: Glyph,
    action: Action,
    label: String,
    enabled: bool,
}

/// Paseo's `ViewportToolbar` (zoomable-viewport/toolbar.tsx:39-71) and the
/// lightbox's close action (attachment-lightbox.tsx:59-69): the shared small
/// icon chrome in a top-right cluster inside the viewport, spacing 2 from its
/// edges and spacing 1 apart, shown on hover, focus or touch. No tooltips:
/// Paseo's buttons carry only their accessible names.
fn toolbar(shell: &Shell, tokens: &Tokens, window: &Window, cx: &mut Context<'_, Shell>) -> Div {
    let sizing = ControlSizing::from_compact(vessel_app_kit::breakpoint::phone(window));
    let lightbox = &shell.areas.composer.lightbox;
    let presence = lightbox.presence(sizing);
    let scale = lightbox.viewport.map(Viewport::scale);
    let words = shell.words.composer();
    let controls = [
        Control {
            id: "image-zoom-in",
            glyph: Glyph::ZoomIn,
            action: Action::In,
            label: words.image_zoom_in().to_owned(),
            enabled: scale.is_some_and(|v| v < MAX_SCALE),
        },
        Control {
            id: "image-zoom-out",
            glyph: Glyph::ZoomOut,
            action: Action::Out,
            label: words.image_zoom_out().to_owned(),
            enabled: scale.is_some_and(|v| v > MIN_SCALE),
        },
        Control {
            id: "image-reset",
            glyph: Glyph::Scan,
            action: Action::Reset,
            label: words.image_reset().to_owned(),
            enabled: scale.is_some_and(|v| v != MIN_SCALE),
        },
        Control {
            id: "image-close",
            glyph: Glyph::X,
            action: Action::Close,
            label: words.image_close().to_owned(),
            enabled: true,
        },
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
                .map(|control| action_slot(control, sizing, presence, tokens, cx)),
        )
}

/// A press on an action is not a press on the viewport: it neither starts a
/// drag nor dismisses the image. A hidden action takes no pointer events
/// (Paseo's `pointerEvents: "none"`), so a press there reaches the viewport.
fn action_slot(
    control: Control,
    sizing: ControlSizing,
    presence: Presence,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let availability = if control.enabled {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    };
    let config = ToolbarConfig::new(sizing, availability);
    let action = control.action;
    let button = ToolbarButton::viewport(
        control.id,
        control.label,
        control.glyph,
        config,
        presence,
        tokens,
    )
    .on_click(cx.listener(move |shell, _, window, cx| {
        shell.image_action(action, window, cx);
    }));
    let slot = div().flex_none().child(button);
    match presence {
        Presence::Hidden => slot,
        Presence::Shown => slot
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
    }
}

impl Shell {
    fn image_action(&mut self, action: Action, window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Action::Close = action {
            self.close_image(window, cx);
            return;
        }
        if let Some(viewport) = &mut self.areas.composer.lightbox.viewport {
            match action {
                Action::In => viewport.zoom_center(ZOOM_IN),
                Action::Out => viewport.zoom_center(ZOOM_OUT),
                Action::Reset => viewport.reset(),
                Action::Close => {}
            }
            cx.notify();
        }
    }

    /// Escape closes; `+`/`=`, `-` and `0` zoom and reset. Tab and those keys
    /// are the keyboard way to the toolbar (Paseo shows it while focus is within
    /// it), so they also show it until the image closes.
    fn image_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<'_, Self>) {
        let action = match key {
            "escape" => Some(Action::Close),
            "+" | "=" => Some(Action::In),
            "-" => Some(Action::Out),
            "0" => Some(Action::Reset),
            _ => None,
        };
        if matches!(key, "tab" | "+" | "=" | "-" | "0") {
            self.areas.composer.lightbox.keyed = true;
            cx.notify();
        }
        if let Some(action) = action {
            self.image_action(action, window, cx);
        }
    }
}
