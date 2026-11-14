//! Visual audit harness for the agents' marks at the display's scale, not
//! an app route: every mark at 14, 16 and 20 points, each in a 48-point
//! cell whose origin sits off the logical pixel grid (x + 0.3, y + 0.6),
//! so a capture shows how the mark lands on device pixels at 1, 1.25,
//! 1.5 and 2 (`Xft.dpi` 96, 120, 144, 192 on X11). A crop script reads
//! the cells by the constants below.
use gpui::{
    AppContext as _, Bounds, Context, IntoElement, ParentElement as _, Render, Styled as _, Window,
    WindowOptions, div, point, px, size,
};
use vessel_app_kit::{
    Assets, Scheme, SchemeChoice, Tokens,
    agent_mark::{self, Artwork, Size, Space},
};

/// The first cell's logical origin: deliberately between pixels.
const ORIGIN: (f32, f32) = (10.3, 10.6);
/// Cell pitch and size, in logical points.
const PITCH: f32 = 60.0;
const CELL: f32 = 48.0;
/// Rows, top to bottom, and columns, left to right.
const MARKS: [Artwork; 5] = [
    agent_mark::CLAUDE,
    agent_mark::CODEX,
    agent_mark::GEMINI,
    agent_mark::OPENCODE,
    agent_mark::GITHUB,
];
const SIZES: [Size; 3] = [Size::Fourteen, Size::Sixteen, Size::Twenty];

#[derive(Debug)]
struct Sheet {
    scheme: Scheme,
}
impl Render for Sheet {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        // Row and column numbers as floats, without a cast.
        let cells = MARKS
            .into_iter()
            .zip([0.0_f32, 1.0, 2.0, 3.0, 4.0])
            .flat_map(|(art, row)| {
                SIZES
                    .into_iter()
                    .zip([0.0_f32, 1.0, 2.0])
                    .map(move |(size, column)| (row, column, art, size))
            });
        div()
            .size_full()
            .relative()
            .bg(tokens.neutral.surface0)
            .children(cells.map(|(row, column, art, size)| {
                let (left, top) = ORIGIN;
                let (x, y) = (PITCH.mul_add(column, left), PITCH.mul_add(row, top));
                div()
                    .absolute()
                    .left(px(x))
                    .top(px(y))
                    .w(px(CELL))
                    .h(px(CELL))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(agent_mark::agent_mark(
                        Some(art),
                        size,
                        Space::Lockup,
                        &tokens,
                    ))
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
                tracing::error!(%error, "mark audit fonts could not start");
                cx.quit();
                return;
            }
            let options = WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: size(px(200.0), px(320.0)),
                })),
                ..WindowOptions::default()
            };
            if let Err(error) =
                cx.open_window(options, |_window, cx| cx.new(|_cx| Sheet { scheme }))
            {
                tracing::error!(%error, "mark audit window could not start");
                cx.quit();
            }
        });
}
