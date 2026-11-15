//! A failed update relaunch reports a visible refusal without starting a node or writer.

use gpui::{
    AppContext as _, Bounds, Context, IntoElement, ParentElement as _, Render, Styled as _, Window,
    div, px, size,
};
use vessel_app_kit::theme::spacing::{S4, S6};
use vessel_app_kit::{SchemeChoice, Type, TypeExt as _, outline_button};
use vessel_app_view::localization::{LanguagePreference, Translator};

const ERROR_WIDTH: f32 = 520.0;
const ERROR_HEIGHT: f32 = 260.0;

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Restart,
    Launch,
}

struct RestartFailure {
    kind: Kind,
    words: &'static Translator,
}

impl Render for RestartFailure {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        use gpui::StatefulInteractiveElement as _;
        let tokens = vessel_app_kit::tokens(cx);
        let words = self.words.desktop_shell();
        let (title, hint) = match self.kind {
            Kind::Restart => (words.restart_failed(), words.restart_hint()),
            Kind::Launch => (words.launch_failed(), words.launch_hint()),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .justify_center()
            .p(px(S6))
            .gap(px(S4))
            .bg(tokens.bg_shell)
            .text_color(tokens.neutral.foreground)
            .child(div().type_(Type::Title).child(title.to_owned()))
            .child(div().type_(Type::Body).child(hint.to_owned()))
            .child(
                outline_button(
                    "restart-error-close",
                    words.close_window().to_owned(),
                    true,
                    &tokens,
                )
                .on_click(cx.listener(|_this, _event, _window, cx| cx.quit())),
            )
    }
}

pub(super) fn show(kind: Kind, choice: SchemeChoice, application: fn() -> gpui::Application) {
    let language = crate::language::Language::new();
    let words = match language.prepare(LanguagePreference::default()) {
        Ok(words) => words,
        Err(error) => {
            tracing::error!(%error,"restart failure words could not start");
            return;
        }
    };
    application()
        .with_assets(vessel_app_kit::Assets)
        .run(move |cx| {
            if let Err(error) = vessel_app_kit::init(cx, choice) {
                tracing::error!(%error,"restart failure view could not start");
                cx.quit();
                return;
            }
            cx.set_quit_mode(gpui::QuitMode::LastWindowClosed);
            let bounds = Bounds::centered(None, size(px(ERROR_WIDTH), px(ERROR_HEIGHT)), cx);
            let options = gpui::WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Vessel".into()),
                    ..gpui::TitlebarOptions::default()
                }),
                ..gpui::WindowOptions::default()
            };
            match cx.open_window(options, |window, cx| {
                let failure = cx.new(|_cx| RestartFailure { kind, words });
                vessel_app_kit::root(failure, window, cx)
            }) {
                Ok(_window) => cx.activate(true),
                Err(error) => {
                    tracing::error!(%error,"restart failure window could not open");
                    cx.quit();
                }
            }
        });
}
