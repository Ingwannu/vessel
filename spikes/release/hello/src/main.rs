//! The smallest GPUI window, built for each system before the app runs
//! there (docs/plans/backend-m12-release.md, step 1).
//!
//! It exists to separate two questions: whether GPUI at the desktop
//! track's pin builds, links and draws for a target at all (shaders,
//! resources, the renderer, the libraries the binary imports), and
//! whether the app does. When a target fails, this binary says which of
//! the two failed. It draws text in Latin and in Hangul because the
//! second is where a missing font or a wrong fallback shows first.
//!
//! It must stay free of anything the app adds (the node client, tokio,
//! fonts of our own), so that a failure here is GPUI's or the target's.

// No console window behind the app on Windows: a GUI binary is what the
// owner launches from the start menu.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::io::Write as _;

use gpui::{
    App, Bounds, Context, SharedString, Window, WindowBounds, WindowOptions, div, prelude::*, px,
    rgb, size,
};

/// The build's display label (`0.0.0.0.1`, ...), set by the release
/// script; not the crate's version, which stays `0.0.0` in a spike.
const LABEL: &str = match option_env!("VESSEL_APP_LABEL") {
    Some(label) => label,
    None => "dev",
};

/// Set to draw shapes only. Wine's DirectWrite (9.0 and 11.0) cannot
/// find a font the way GPUI asks (`IDWriteFontSet::GetMatchingFonts` is
/// a stub there), so any text ends the process; without text, a run under
/// wine still shows the renderer and the build's shaders drawing.
const SHAPES_ONLY: &str = "VESSEL_HELLO_SHAPES_ONLY";

/// What the window shows: enough to tell from a capture which build and
/// which target drew it.
struct Hello {
    title: SharedString,
    target: SharedString,
    shapes_only: bool,
}

impl Render for Hello {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let swatch = |color: u32| div().size_16().rounded_lg().bg(rgb(color));
        let swatches = div()
            .flex()
            .gap_3()
            .child(swatch(0x3b_82_f6))
            .child(swatch(0x22_c5_5e))
            .child(swatch(0xf5_9e_0b))
            .child(
                div()
                    .size_16()
                    .rounded_full()
                    .border_4()
                    .border_color(rgb(0xe8_ea_ed)),
            );
        let page = div()
            .flex()
            .flex_col()
            .gap_3()
            .size_full()
            .justify_center()
            .items_center()
            .bg(rgb(0x0f_11_15))
            .text_color(rgb(0xe8_ea_ed))
            .child(swatches);
        if self.shapes_only {
            return page;
        }
        page.text_xl()
            .child(self.title.clone())
            .child(self.target.clone())
            .child("한글 입력과 표시: 안녕하세요")
    }
}

fn main() {
    let shapes_only = std::env::var_os(SHAPES_ONLY).is_some();
    gpui_platform::application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(560.0), px(320.0)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..WindowOptions::default()
        };
        let opened = cx.open_window(options, |_, cx| {
            cx.new(|_| Hello {
                title: SharedString::from(format!("Vessel {LABEL}")),
                target: SharedString::from(format!(
                    "{} {}",
                    std::env::consts::OS,
                    std::env::consts::ARCH
                )),
                shapes_only,
            })
        });
        if let Err(error) = opened {
            // A GUI binary on Windows has no console; the message still
            // reaches a terminal or wine's log that started it.
            let mut stderr = std::io::stderr().lock();
            let _ignored = writeln!(stderr, "vessel-hello: no window: {error:#}");
            cx.quit();
            return;
        }
        cx.activate(true);
    });
}
