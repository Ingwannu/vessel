//! Visual audit harness, not an app route. Original files are read only
//! from an explicitly supplied local fixture folder; production cannot
//! request or load these files. Capture instructions are in the track notes.
use std::path::PathBuf;
#[path = "agent_marks/picker.rs"]
mod picker;

use gpui::{
    AppContext as _, Bounds, Context, IntoElement, ParentElement as _, Render, Styled as _, Window,
    WindowOptions, div, img, px, size,
};
use vessel_app_kit::{
    Assets, Scheme, SchemeChoice, Tokens, Type, TypeExt as _,
    agent_mark::{self, Artwork, Size, Space},
};

#[derive(Debug)]
struct Preview {
    scheme: Scheme,
    originals: Option<PathBuf>,
    picker: vessel_app_kit::ComboboxState,
    selected: usize,
}
impl Render for Preview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let mut body = div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(16.0))
            .bg(tokens.neutral.surface0)
            .text_color(tokens.neutral.foreground)
            .type_(Type::Body)
            .child("Embedded artwork / official original · 공식 원본 비교");
        for requested in [Size::Sixteen, Size::Twenty] {
            let mut family = div()
                .flex()
                .items_center()
                .gap(px(16.0))
                .min_h(px(48.0))
                .child(format!("{} px", requested.points()));
            for art in [
                Some(agent_mark::CLAUDE),
                Some(agent_mark::CODEX),
                Some(agent_mark::GEMINI),
                Some(agent_mark::OPENCODE),
                Some(agent_mark::COPILOT),
                None,
            ] {
                family = family.child(agent_mark::agent_mark(
                    art,
                    requested,
                    Space::Lockup,
                    &tokens,
                ));
            }
            body = body.child(family);
        }
        body = body.children(self.comparison_rows(&tokens));
        body.child(picker::trigger(self, &tokens, cx))
            .children(picker::overlay(self, &tokens, window, cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.0))
                    .child("Brand minimum / 여백 포함")
                    .child(agent_mark::agent_mark(
                        Some(agent_mark::OPENCODE),
                        Size::Twenty,
                        Space::Row,
                        &tokens,
                    ))
                    .child(agent_mark::agent_mark(
                        Some(agent_mark::COPILOT),
                        Size::Twenty,
                        Space::Lockup,
                        &tokens,
                    )),
            )
    }
}
impl Preview {
    fn comparison_rows(&self, tokens: &Tokens) -> Vec<gpui::Div> {
        let mut rows = Vec::new();
        for (label, art, file) in [
            (
                "Claude Code",
                Some(agent_mark::CLAUDE),
                "anthropic/spark.svg",
            ),
            (
                "Codex · OpenAI",
                Some(agent_mark::CODEX),
                "openai/blossom.svg",
            ),
            (
                "Gemini CLI",
                Some(agent_mark::GEMINI),
                "google/gemini-cli-icon.png",
            ),
            ("OpenCode", Some(agent_mark::OPENCODE), "opencode/light.svg"),
            (
                "GitHub Copilot",
                Some(agent_mark::COPILOT),
                "github/GitHub_Invertocat_Black_Clearspace.svg",
            ),
            ("Unknown / 사용자 추가", None, ""),
        ] {
            let mut line = div()
                .flex()
                .items_center()
                .gap(px(16.0))
                .min_h(px(44.0))
                .child(div().w(px(155.0)).child(label));
            let original = self
                .originals
                .as_ref()
                .filter(|_| !file.is_empty())
                .map(|root| {
                    let file = match (file, self.scheme) {
                        ("opencode/light.svg", Scheme::Dark) => "opencode/dark.svg",
                        ("github/GitHub_Invertocat_Black_Clearspace.svg", Scheme::Dark) => {
                            "github/GitHub_Invertocat_White_Clearspace.svg"
                        }
                        _ => file,
                    };
                    root.join(file)
                });
            for requested in Size::ALL {
                line = line.child(cell(art, requested, tokens, original.clone()));
            }
            rows.push(line);
        }
        rows
    }
}
fn cell(art: Option<Artwork>, size: Size, tokens: &Tokens, original: Option<PathBuf>) -> gpui::Div {
    div()
        .w(px(100.0))
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(format!("{}", size.points()))
        .child(agent_mark::agent_mark(art, size, Space::Compact, tokens))
        .children(original.map(|path| {
            // The only located OpenAI source is white, so the genuine
            // original is compared on black even in the light-theme run.
            let background = if art == Some(agent_mark::CODEX) {
                gpui::rgb(0).into()
            } else {
                tokens.neutral.surface0
            };
            div()
                .size(px(size.points()))
                .bg(background)
                .child(img(path).size(px(size.points())))
        }))
}
fn main() {
    let scheme = if std::env::var("VESSEL_APP_THEME").as_deref() == Ok("light") {
        Scheme::Light
    } else {
        Scheme::Dark
    };
    let originals = std::env::var_os("VESSEL_MARK_ORIGINALS").map(PathBuf::from);
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx| {
            if let Err(error) = vessel_app_kit::init(cx, SchemeChoice::Fixed(scheme)) {
                tracing::error!(%error, "mark audit fonts could not start");
                cx.quit();
                return;
            }
            let options = WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(
                        px(
                            if std::env::var("VESSEL_APP_SIZE").as_deref() == Ok("390x844") {
                                390.0
                            } else {
                                900.0
                            },
                        ),
                        px(760.0),
                    ),
                    cx,
                ))),
                ..WindowOptions::default()
            };
            if let Err(error) = cx.open_window(options, |window, cx| {
                cx.new(|cx| Preview {
                    scheme,
                    originals,
                    picker: vessel_app_kit::ComboboxState::new(window, cx),
                    selected: 0,
                })
            }) {
                tracing::error!(%error, "mark audit window could not start");
                cx.quit();
            }
        });
}
