//! Visual audit harness for the window controls' glyphs and the panel's
//! two-line edge at the display's scale, not an app route. Rows: the three
//! controls (windowed) and the restore glyph, in the bar's four states —
//! at rest, hovered, pressed, focused (focus draws as hover: no ring) —
//! each cell the title bar's 46 × 36 with today's fills (the close cell's
//! Windows red). Below them, a panel with `frame::panel_edges` two lines.
//! The first cell's origin sits off the logical grid (x + 0.3, y + 0.6),
//! so a capture at 1, 1.25, 1.5 and 2 shows how each lands on device
//! pixels. A crop script reads the cells by the constants below.
#![expect(clippy::float_arithmetic, reason = "laying out an audit sheet")]

use gpui::{
    AppContext as _, Bounds, Context, Hsla, IntoElement, ParentElement as _, Render, Styled as _,
    Window, WindowOptions, div, point, px, rgba, size,
};
use vessel_app_kit::frame::{self, Lines};
use vessel_app_kit::window_glyph::{WindowGlyph, window_glyph};
use vessel_app_kit::{Assets, Scheme, SchemeChoice, Tokens};

/// The first cell's logical origin: deliberately between pixels.
const ORIGIN: (f32, f32) = (10.3, 10.6);
/// A cell, as the title bar's on Windows.
const CELL: (f32, f32) = (46.0, 36.0);
/// The air between cells.
const AIR: f32 = 8.0;
/// Paseo's close red, hovered and pressed (the kit's chrome).
const CLOSE_HOVER: u32 = 0xe811_23e6;
const CLOSE_PRESSED: u32 = 0xe811_23b3;

#[derive(Debug, Clone, Copy)]
enum State {
    Rest,
    Hover,
    Pressed,
    Focus,
}

#[derive(Debug)]
struct Sheet {
    scheme: Scheme,
}

fn cell(glyph: WindowGlyph, state: State, tokens: &Tokens) -> gpui::Div {
    let neutral = tokens.neutral;
    let close = glyph == WindowGlyph::Close;
    let (fill, ink): (Option<Hsla>, Hsla) = match (state, close) {
        (State::Rest, _) => (None, neutral.foreground_muted),
        (State::Hover | State::Focus, false) => (
            Some(neutral.interaction_highlight),
            neutral.foreground_muted,
        ),
        (State::Pressed, false) => (Some(neutral.surface2), neutral.foreground_muted),
        (State::Hover | State::Focus, true) => (Some(rgba(CLOSE_HOVER).into()), gpui::white()),
        (State::Pressed, true) => (Some(rgba(CLOSE_PRESSED).into()), gpui::white()),
    };
    let base = div()
        .w(px(CELL.0))
        .h(px(CELL.1))
        .flex()
        .items_center()
        .justify_center()
        .child(window_glyph(glyph, ink, None, "audit"));
    match fill {
        Some(fill) => base.bg(fill),
        None => base,
    }
}

impl Render for Sheet {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let glyphs = [
            WindowGlyph::Minimize,
            WindowGlyph::Maximize,
            WindowGlyph::Restore,
            WindowGlyph::Close,
        ];
        let states = [State::Rest, State::Hover, State::Pressed, State::Focus];
        let (left, top) = ORIGIN;
        let rows = glyphs
            .into_iter()
            .zip([0.0_f32, 1.0, 2.0, 3.0])
            .map(|(glyph, row)| {
                div()
                    .absolute()
                    .left(px(left))
                    .top(px((CELL.1 + AIR).mul_add(row, top)))
                    .flex()
                    .flex_row()
                    .gap(px(AIR))
                    .children(states.map(|state| cell(glyph, state, &tokens)))
            });
        let panel_top = (CELL.1 + AIR).mul_add(4.0, top + 12.0);
        div()
            .size_full()
            .relative()
            .bg(tokens.bg_shell)
            .children(rows)
            .child(
                div()
                    .absolute()
                    .left(px(left + 8.0))
                    .top(px(panel_top))
                    .w(px(4.0_f32.mul_add(CELL.0 + AIR, -AIR - 16.0)))
                    .h(px(60.0))
                    .child(
                        div()
                            .size_full()
                            .bg(tokens.neutral.surface0)
                            .rounded(px(vessel_app_kit::theme::radius::R5)),
                    )
                    .child(frame::panel_edges(
                        vessel_app_kit::theme::radius::R5,
                        Lines::Two,
                        &tokens,
                    )),
            )
    }
}

fn main() {
    let scheme = if std::env::var("VESSEL_APP_THEME").as_deref() == Ok("light") {
        Scheme::Light
    } else {
        Scheme::Dark
    };
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx| {
            if let Err(error) = vessel_app_kit::init(cx, SchemeChoice::Fixed(scheme)) {
                tracing::error!(%error, "window glyph audit fonts could not start");
                cx.quit();
                return;
            }
            let options = WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: size(px(240.0), px(270.0)),
                })),
                ..WindowOptions::default()
            };
            if let Err(error) =
                cx.open_window(options, |_window, cx| cx.new(|_cx| Sheet { scheme }))
            {
                tracing::error!(%error, "window glyph audit window could not start");
                cx.quit();
            }
        });
}
