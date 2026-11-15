//! Visual check of the permissions modes' marks, not an app route: the
//! kit's `Shield` beside the five marks drawn on its outline, at 14, 16 and
//! 20 points, each in a 40-point cell whose origin sits off the logical
//! pixel grid (x + 0.3, y + 0.6), so a capture shows how they land on
//! device pixels at 1, 1.25, 1.5 and 2 (`GPUI_X11_SCALE_FACTOR` on X11).
//! Rows top to bottom: `Shield`, `ShieldPlus`, `ShieldCheck`,
//! `ShieldOff`, `ShieldEllipsis`, `ShieldAlert`; columns: 14, 16, 20. `VESSEL_APP_THEME`:
//! `light`, default dark.
use gpui::{
    AppContext as _, Bounds, Context, IntoElement, ParentElement as _, Render, Styled as _, Window,
    WindowOptions, div, point, px, size,
};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::{Assets, Scheme, SchemeChoice, Tokens};

/// The first cell's logical origin: deliberately between pixels.
const ORIGIN: (f32, f32) = (10.3, 10.6);
/// Cell pitch and size, in logical points.
const PITCH: f32 = 48.0;
const CELL: f32 = 40.0;
const MARKS: [Glyph; 6] = [
    Glyph::Shield,
    Glyph::ShieldPlus,
    Glyph::ShieldCheck,
    Glyph::ShieldOff,
    Glyph::ShieldEllipsis,
    Glyph::ShieldAlert,
];
const SIZES: [f32; 3] = [14.0, 16.0, 20.0];

struct Sheet {
    scheme: Scheme,
}

impl Render for Sheet {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let ink = tokens.neutral.foreground;
        let cells = MARKS
            .into_iter()
            .zip([0.0_f32, 1.0, 2.0, 3.0, 4.0, 5.0])
            .flat_map(|(mark, row)| {
                SIZES
                    .into_iter()
                    .zip([0.0_f32, 1.0, 2.0])
                    .map(move |(size, column)| (row, column, mark, size))
            });
        div()
            .size_full()
            .relative()
            .bg(tokens.neutral.surface0)
            .children(cells.map(move |(row, column, mark, size)| {
                let (left, top) = ORIGIN;
                div()
                    .absolute()
                    .left(px(PITCH.mul_add(column, left)))
                    .top(px(PITCH.mul_add(row, top)))
                    .w(px(CELL))
                    .h(px(CELL))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(glyph(mark, size, ink))
            }))
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
                tracing::error!(%error, "shield marks fonts could not start");
                cx.quit();
                return;
            }
            let options = WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: size(px(160.0), px(300.0)),
                })),
                ..WindowOptions::default()
            };
            if let Err(error) =
                cx.open_window(options, |_window, cx| cx.new(|_cx| Sheet { scheme }))
            {
                tracing::error!(%error, "shield marks window could not start");
                cx.quit();
            }
        });
}
