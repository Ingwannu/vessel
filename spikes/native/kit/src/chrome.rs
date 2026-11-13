//! The window's own chrome, as Zed draws its: no system title bar on
//! Windows or Linux; a bar the app draws, which moves the window, maximises
//! it on a double click, and carries minimise, maximise and close drawn by
//! the app; on macOS the system's traffic lights, inset in the same bar.
//!
//! [Decision Log]
//! - Purpose: the owner's feedback on build 0.0.0.0.1: the title bar and
//!   its buttons are to be the app's own, not Windows' (whose bar took the
//!   system's accent colour above a dark app).
//! - What GPUI offers (read in gpui-pre 0.3.10): `TitlebarOptions` with
//!   `appears_transparent` hides the system bar (Windows, macOS) and
//!   `WindowDecorations::Client` asks a Linux compositor for none; on
//!   Windows an element marked with a `WindowControlArea` (`Drag`, `Min`,
//!   `Max`, `Close`) answers the system's hit test, so dragging, the snap
//!   layouts on the maximise button, the system menu and the buttons'
//!   clicks are Windows' own behaviour; on Linux the app moves the window
//!   (`start_window_move`), opens the window menu (`show_window_menu`) and
//!   handles the buttons' clicks itself; resizing at the edges is the
//!   app's there too (`start_window_resize`).
//! - Alternatives: `gpui-component`'s `TitleBar` (the same platform
//!   handling; its icons need its asset bundle and its colours its theme);
//!   Zed's title bar (GPL, read only).
//! - Choice: the same platform handling, drawn from the kit's tokens, with
//!   four small glyphs of the kit's own (`assets`), so nothing of the
//!   component layer's look reaches the bar.
//! - Not verified here: anything on a real Windows or macOS desktop (the
//!   owner tries each build). Under Xvfb with no window manager the bar is
//!   drawn and its buttons and drag call GPUI, but there is nothing to move
//!   or snap.

use gpui::prelude::*;
use gpui::{
    AnyElement, Bounds, Decorations, Div, MouseButton, Pixels, ResizeEdge, Stateful,
    TitlebarOptions, Window, WindowBounds, WindowControlArea, WindowDecorations, WindowOptions,
    div, point, px, svg,
};

use gpui_component::InteractiveElementExt as _;

use crate::assets::Glyph;
use crate::theme::Tokens;

/// The bar's height.
pub const HEIGHT: f32 = 36.0;

/// Room left for macOS's traffic lights.
const TRAFFIC_LIGHTS: f32 = 78.0;

/// The width of the strip at a client-decorated window's edge that resizes
/// it (Linux).
const RESIZE_EDGE: f32 = 6.0;

/// Windows 11's close button, hovered and pressed (its own colours, the one
/// red in the chrome: it is the platform's, not the page's danger).
const CLOSE_HOVER: u32 = 0xc42b_1cff;
const CLOSE_PRESSED: u32 = 0xc42b_1ccc;

/// Whether the system draws the title bar after all:
/// `VESSEL_APP_TITLEBAR=system`. The way out while the app's own bar is
/// new on a platform (it had never run on Windows when it was made): the
/// system's bar, buttons and frame, and the app draws none. Read once.
#[must_use]
pub fn system_titlebar() -> bool {
    static SYSTEM: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *SYSTEM.get_or_init(|| {
        std::env::var("VESSEL_APP_TITLEBAR").is_ok_and(|value| value.eq_ignore_ascii_case("system"))
    })
}

/// The window's options: for an app-drawn title bar, the system bar
/// hidden (Windows, macOS, with the traffic lights inset), client
/// decorations on Linux and the app owning the bar's drag; with
/// [`system_titlebar`], the system's own.
#[must_use]
pub fn window_options(title: String, bounds: Bounds<Pixels>) -> WindowOptions {
    let own = !system_titlebar();
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some(title.into()),
            appears_transparent: own,
            traffic_light_position: own.then(|| point(px(12.0), px(11.0))),
        }),
        window_decorations: Some(if own {
            WindowDecorations::Client
        } else {
            WindowDecorations::Server
        }),
        app_owns_titlebar_drag: own,
        icon: crate::assets::window_icon(128)
            .map_err(|error| tracing::warn!(%error, "no window icon"))
            .ok()
            .map(std::sync::Arc::new),
        ..WindowOptions::default()
    }
}

/// The title bar: `content` on the left (after the traffic lights on
/// macOS), the window's controls on the right (Windows, Linux). Nothing
/// with [`system_titlebar`]: the system's bar is the window's then.
pub fn title_bar(content: AnyElement, tokens: &Tokens, window: &mut Window) -> Stateful<Div> {
    if system_titlebar() {
        return div().id("title-bar");
    }
    let macos = cfg!(target_os = "macos");
    let linux = cfg!(any(target_os = "linux", target_os = "freebsd"));
    let maximized = window.is_maximized();
    let bar = div()
        .id("title-bar")
        .h(px(HEIGHT))
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .flex_shrink_0()
        .bg(tokens.bg_shell)
        .window_control_area(WindowControlArea::Drag)
        .when(macos, |bar| {
            bar.pl(px(TRAFFIC_LIGHTS))
                .on_double_click(|_, window, _| window.titlebar_double_click())
        })
        .when(!macos, |bar| bar.pl(px(12.0)))
        .when(linux, |bar| {
            bar.on_double_click(|_, window, _| window.zoom_window())
                .on_mouse_down(MouseButton::Left, |event, window, _| {
                    // A press on the bar itself (not a control) moves the
                    // window; the controls stop the event first.
                    if event.click_count == 1 {
                        window.start_window_move();
                    }
                })
                .on_mouse_down(MouseButton::Right, |event, window, _| {
                    window.show_window_menu(event.position);
                })
        })
        .child(div().flex_1().h_full().flex().items_center().child(content));
    if macos {
        bar
    } else {
        bar.child(control(Control::Minimize, tokens))
            .child(control(
                if maximized {
                    Control::Restore
                } else {
                    Control::Maximize
                },
                tokens,
            ))
            .child(control(Control::Close, tokens))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Control {
    Minimize,
    Maximize,
    Restore,
    Close,
}

fn control(which: Control, tokens: &Tokens) -> Stateful<Div> {
    let (id, glyph, area) = match which {
        Control::Minimize => ("window-minimize", Glyph::Minimize, WindowControlArea::Min),
        Control::Maximize => ("window-maximize", Glyph::Maximize, WindowControlArea::Max),
        Control::Restore => ("window-restore", Glyph::Restore, WindowControlArea::Max),
        Control::Close => ("window-close", Glyph::Close, WindowControlArea::Close),
    };
    let close = which == Control::Close;
    let (hover, pressed) = if close {
        (
            gpui::rgba(CLOSE_HOVER).into(),
            gpui::rgba(CLOSE_PRESSED).into(),
        )
    } else {
        (tokens.bg_hover, tokens.bg_active)
    };
    let ink = tokens.text_secondary;
    let linux = cfg!(any(target_os = "linux", target_os = "freebsd"));
    div()
        .id(id)
        .group(id)
        .w(px(46.0))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .window_control_area(area)
        .hover(move |style| style.bg(hover))
        .active(move |style| style.bg(pressed))
        .child(
            svg()
                .path(glyph.path())
                .size(px(10.0))
                .text_color(ink)
                .when(close, |icon| {
                    icon.group_hover(id, |style| style.text_color(gpui::white()))
                }),
        )
        .when(linux, |button| {
            // Linux has no hit test: the app acts on the click.
            button
                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    match which {
                        Control::Minimize => window.minimize_window(),
                        Control::Maximize | Control::Restore => window.zoom_window(),
                        Control::Close => window.remove_window(),
                    }
                })
        })
}

/// The window's content inside resize edges, where the app decorates the
/// window itself (Linux with client decorations); the content alone
/// elsewhere (the system resizes there).
pub fn resizable(content: AnyElement, window: &mut Window) -> Div {
    let Decorations::Client { tiling } = window.window_decorations() else {
        return div().size_full().child(content);
    };
    let edge = px(RESIZE_EDGE);
    let grip = |id: &'static str, which: ResizeEdge| {
        div()
            .id(id)
            .absolute()
            .on_mouse_down(MouseButton::Left, move |_, window, _| {
                window.start_window_resize(which);
            })
    };
    let mut frame = div().size_full().relative().child(content);
    if !tiling.top {
        frame = frame.child(
            grip("resize-top", ResizeEdge::Top)
                .top_0()
                .left(edge)
                .right(edge)
                .h(edge),
        );
    }
    if !tiling.bottom {
        frame = frame.child(
            grip("resize-bottom", ResizeEdge::Bottom)
                .bottom_0()
                .left(edge)
                .right(edge)
                .h(edge),
        );
    }
    if !tiling.left {
        frame = frame.child(
            grip("resize-left", ResizeEdge::Left)
                .left_0()
                .top(edge)
                .bottom(edge)
                .w(edge),
        );
    }
    if !tiling.right {
        frame = frame.child(
            grip("resize-right", ResizeEdge::Right)
                .right_0()
                .top(edge)
                .bottom(edge)
                .w(edge),
        );
    }
    frame
        .child(
            grip("resize-top-left", ResizeEdge::TopLeft)
                .top_0()
                .left_0()
                .size(edge),
        )
        .child(
            grip("resize-top-right", ResizeEdge::TopRight)
                .top_0()
                .right_0()
                .size(edge),
        )
        .child(
            grip("resize-bottom-left", ResizeEdge::BottomLeft)
                .bottom_0()
                .left_0()
                .size(edge),
        )
        .child(
            grip("resize-bottom-right", ResizeEdge::BottomRight)
                .bottom_0()
                .right_0()
                .size(edge),
        )
}
