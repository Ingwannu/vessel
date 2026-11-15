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

use crate::window_glyph::{WindowGlyph, window_glyph};
use gpui::prelude::*;
use gpui::{
    AnyElement, Bounds, Decorations, Div, MouseButton, Pixels, ResizeEdge, Stateful,
    TitlebarOptions, Window, WindowBounds, WindowControlArea, WindowDecorations, WindowOptions,
    div, point, px,
};

use gpui_component::InteractiveElementExt as _;

use crate::theme::Tokens;

/// The bar's height.
pub const HEIGHT: f32 = 36.0;

/// Room left for macOS's traffic lights.
const TRAFFIC_LIGHTS: f32 = 78.0;

/// The width of the strip at a client-decorated window's edge that resizes
/// it (Linux).
const RESIZE_EDGE: f32 = 6.0;

/// The close button, hovered and pressed: Paseo's `rgba(232,17,35,0.9)`
/// (Windows' own close red; the one red in the chrome, the platform's).
const CLOSE_HOVER: u32 = 0xe811_23e6;
const CLOSE_PRESSED: u32 = 0xe811_23b3;

/// A window control's width on Windows (Paseo's, the system's own).
const WINDOWS_CONTROL: f32 = 46.0;

/// A window control's width on Linux (Paseo's).
const LINUX_CONTROL: f32 = 36.0;

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
/// macOS), `trailing` at the right before the window's controls (Windows,
/// Linux), Zed's place for its update status. Nothing with
/// [`system_titlebar`]: the system's bar is the window's then.
///
/// `trailing` is the app's to click: it occludes the bar behind it, so
/// Windows' hit test does not read it as the bar's drag area (GPUI asks
/// which control area's hitbox is under the mouse, and an occluding
/// element hides the bar's) and a press on it does not move the window on
/// Linux.
pub fn title_bar(
    content: AnyElement,
    trailing: Option<AnyElement>,
    tokens: &Tokens,
    window: &mut Window,
) -> Stateful<Div> {
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
        .child(div().flex_1().h_full().flex().items_center().child(content))
        .children(trailing.map(|trailing| {
            div()
                .id("title-bar-trailing")
                .h_full()
                .flex()
                .items_center()
                .px(px(8.0))
                .occlude()
                .child(trailing)
        }));
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

/// One window control, as Paseo draws its own (`window-controls.tsx`,
/// `window-chrome-presentation.ts`): on Windows a 46-wide cell whose hover
/// is the interaction highlight, red under the close glyph (which turns
/// white); on Linux a 36-wide cell with a circle on hover.
///
/// Every control occludes the bar behind it. GPUI answers Windows' hit
/// test with the first window-control area under the mouse in paint
/// order, and the bar (the drag area) is painted before its children:
/// without the occlusion the bar is hit over a button, Windows is told
/// "caption", a single click is swallowed by the move loop and a double
/// click maximises (the owner's report on 0.0.0.0.3). Zed occludes its
/// caption buttons the same way. On Windows the control's area is all
/// there is: the system acts on the click, and nothing of the app's may
/// handle it (a handler would eat the first click). Linux has no hit
/// test: there the app acts on the click itself.
fn control(which: Control, tokens: &Tokens) -> Stateful<Div> {
    let (id, glyph, area) = match which {
        Control::Minimize => (
            "window-minimize",
            WindowGlyph::Minimize,
            WindowControlArea::Min,
        ),
        Control::Maximize => (
            "window-maximize",
            WindowGlyph::Maximize,
            WindowControlArea::Max,
        ),
        Control::Restore => (
            "window-restore",
            WindowGlyph::Restore,
            WindowControlArea::Max,
        ),
        Control::Close => ("window-close", WindowGlyph::Close, WindowControlArea::Close),
    };
    let close = which == Control::Close;
    let neutral = tokens.neutral;
    let (hover, pressed) = if close {
        (
            gpui::rgba(CLOSE_HOVER).into(),
            gpui::rgba(CLOSE_PRESSED).into(),
        )
    } else {
        (neutral.interaction_highlight, neutral.surface2)
    };
    let ink = neutral.foreground_muted;
    let linux = cfg!(any(target_os = "linux", target_os = "freebsd"));
    // The mark's line language at 9 points, its straight strokes on the
    // device grid (worker B's `window_glyph`; the owner's word 37). The
    // cell, its fills and its control area are as before, so Windows'
    // Snap Layouts over Maximise are unchanged.
    let icon = window_glyph(glyph, ink, close.then(gpui::white), id);
    let cell = div()
        .id(id)
        .group(id)
        .w(px(if linux {
            LINUX_CONTROL
        } else {
            WINDOWS_CONTROL
        }))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .occlude()
        .window_control_area(area);
    if !linux {
        return cell
            .hover(move |style| style.bg(hover))
            .active(move |style| style.bg(pressed))
            .child(icon);
    }
    // Linux: a circle under the glyph on hover, and the app acts on the
    // click.
    cell.child(
        div()
            .size(px(24.0))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .group_hover(id, move |style| style.bg(hover))
            .child(icon),
    )
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

// macOS draws its own traffic lights: there are no controls of the app's.
#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    //! The title bar's control areas, as Windows' hit test reads them.
    //!
    //! On Windows GPUI answers `WM_NCHITTEST` with the first control area,
    //! in paint order, whose hitbox is in the window's hit test
    //! (`on_hit_test_window_control` in gpui's window.rs). The bar (`Drag`)
    //! is painted before the buttons inside it, so over a button the bar must
    //! not be in the hit test at all, or Windows reads the button as the
    //! caption: one click is swallowed by the move loop and a double click
    //! maximises (build 0.0.0.0.3 on the owner's Windows 11). The probe is a
    //! scroll listener on the bar: it runs when the bar's hitbox is in the
    //! hit test (`Hitbox::should_handle_scroll`), the test the Windows hit
    //! test makes for each control area. That the system then minimises,
    //! maximises or closes on one click is Windows' own (seen under wine,
    //! docs/plans/backend-m12-release.md).

    use std::cell::Cell;
    use std::rc::Rc;

    use gpui::{Context, IntoElement, Render, ScrollEvent, TestAppContext, Window};

    use super::*;
    use crate::theme::{Scheme, Tokens};

    /// The test window's bar width, and the trailing element's.
    const WIDTH: f32 = 600.0;
    const TRAILING: f32 = 40.0;
    /// The bar's middle, top to bottom (`HEIGHT` / 2).
    const MIDDLE: f32 = 18.0;
    /// Right to left across the 600 px bar: close, maximise and minimise
    /// (`LINUX_CONTROL`, 36 px each), then the trailing element (40 px
    /// with 8 px either side); the middle of each.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    const OVER: [(&str, f32); 4] = [
        ("close", 582.0),
        ("maximise", 546.0),
        ("minimise", 510.0),
        ("trailing", 464.0),
    ];
    /// The same with `WINDOWS_CONTROL` (46 px each).
    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    const OVER: [(&str, f32); 4] = [
        ("close", 577.0),
        ("maximise", 531.0),
        ("minimise", 485.0),
        ("trailing", 434.0),
    ];

    /// The title bar, with the probe on the bar.
    struct Probe {
        reached: Rc<Cell<usize>>,
    }

    impl Render for Probe {
        fn render(&mut self, window: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
            let reached = Rc::clone(&self.reached);
            let tokens = Tokens::of(Scheme::Dark);
            let trailing = div().w(px(TRAILING)).h_full().into_any_element();
            div().w(px(WIDTH)).child(
                title_bar(div().into_any_element(), Some(trailing), &tokens, window)
                    .on_scroll(move |_, _, _| reached.set(reached.get().saturating_add(1))),
            )
        }
    }

    #[gpui::test]
    fn over_the_window_controls_the_bar_is_not_the_drag_area(cx: &mut TestAppContext) {
        let reached = Rc::new(Cell::new(0_usize));
        let (_, cx) = cx.add_window_view({
            let reached = Rc::clone(&reached);
            move |_, _| Probe { reached }
        });
        let mut bar_under = |x: f32| {
            let before = reached.get();
            cx.simulate_event(ScrollEvent {
                position: point(px(x), px(MIDDLE)),
                ..ScrollEvent::default()
            });
            reached.get() > before
        };
        // The probe sees the bar where nothing covers it.
        assert!(bar_under(100.0), "the probe never saw the bar");
        for (what, x) in OVER {
            assert!(
                !bar_under(x),
                "over {what} the bar's drag area is in the hit test: Windows reads it as the caption"
            );
        }
    }
}
