//! Patch7 control audit: the production readonly document profile, no app/node.
//! It has no repository authority. Actual C3 fake-node first-use remains separate.
use gpui::prelude::*;
use gpui::{Bounds, Context, Render, Window, WindowOptions, div, point, px, size};
use std::num::NonZeroU32;
use std::sync::Arc;
use vessel_app_kit::code_editor::{
    DiffDocumentEditor, DiffLine, DiffText, DiffTone, DiffWrapping, LineSyntax, Runs,
};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::{Assets, ButtonKind, ButtonState, Scheme, SchemeChoice, VirtualList};

const ROWS: u32 = 2500;
struct Plain;
impl LineSyntax for Plain {
    type State = ();
    fn language(&self) -> gpui::SharedString {
        "text".into()
    }
    fn line(&self, _: &str, (): ()) -> (Runs, ()) {
        (Vec::new(), ())
    }
}
struct Audit {
    editor: DiffDocumentEditor,
    list: VirtualList,
}
impl Audit {
    fn new(text: DiffText, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let list = VirtualList::new(false);
        list.reset(1);
        Self {
            editor: DiffDocumentEditor::new(
                text,
                Plain,
                DiffWrapping::Scroll,
                |owner: &mut Self, bounds, _, cx| {
                    owner.list.reveal_bounds(bounds, px(0.));
                    cx.notify();
                },
                window,
                cx,
            ),
            list,
        }
    }
    fn wrap(&mut self, wrapping: DiffWrapping, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.editor.set_wrapping(wrapping, window, cx);
        self.list.changed(0..1);
        cx.notify();
    }
}
fn fixture() -> Result<DiffText, vessel_app_kit::code_editor::DiffTextError> {
    let mut text = Vec::new();
    let mut lines = Vec::new();
    for row in 1..=ROWS {
        let (tone, content) = match row {
            1 => (
                DiffTone::Header,
                "@@ -17,2498 +42,2498 @@ mapped audit".to_owned(),
            ),
            2 => (
                DiffTone::Removed,
                "old source: removed text remains selectable".to_owned(),
            ),
            3 => (
                DiffTone::Added,
                "새 코드: 선택하고 복사해도 줄 번호는 들어가지 않습니다".to_owned(),
            ),
            4 => (DiffTone::Added, "long source / 한글 경로 / ".repeat(45)),
            _ => (
                DiffTone::Context,
                format!("source {row}: original buffer text"),
            ),
        };
        lines.push(DiffLine {
            number: if tone == DiffTone::Header {
                None
            } else {
                NonZeroU32::new(if row == 3 { 9_999_999 } else { row })
            },
            tone,
        });
        text.push(content);
    }
    DiffText::new(Arc::from(text.join("\n")), lines.into())
}

impl Render for Audit {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = vessel_app_kit::tokens(cx);
        let source = self.editor.clone();
        // The audit root has no horizontal inset or sibling pane. Its list
        // fills this viewport; do not borrow ListState inside its own renderer.
        let width = window.viewport_size().width;
        let body = self.list.element(move |_, window, cx| {
            source.element(&vessel_app_kit::tokens(cx), width, window, cx)
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(tokens.neutral.surface0)
            .child(
                div()
                    .flex()
                    .gap(px(spacing::S2))
                    .p(px(spacing::S2))
                    .child(
                        vessel_app_kit::button_with_kind(
                            "audit-scroll",
                            "Scroll",
                            ButtonKind::Secondary,
                            ButtonState::Enabled,
                            &tokens,
                        )
                        .on_click(cx.listener(|owner, _, window, cx| {
                            owner.wrap(DiffWrapping::Scroll, window, cx);
                        })),
                    )
                    .child(
                        vessel_app_kit::button_with_kind(
                            "audit-wrap",
                            "Wrap",
                            ButtonKind::Secondary,
                            ButtonState::Enabled,
                            &tokens,
                        )
                        .on_click(cx.listener(|owner, _, window, cx| {
                            owner.wrap(DiffWrapping::Wrap, window, cx);
                        })),
                    ),
            )
            .child(div().flex_1().min_h_0().child(body))
    }
}
fn main() {
    let text = match fixture() {
        Ok(text) => text,
        Err(error) => {
            tracing::error!(%error,"audit text admission failed");
            return;
        }
    };
    let scheme = if std::env::var("VESSEL_APP_THEME").as_deref() == Ok("light") {
        Scheme::Light
    } else {
        Scheme::Dark
    };
    let width = if std::env::var("VESSEL_APP_SIZE").as_deref() == Ok("390x844") {
        390.
    } else {
        1280.
    };
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx| {
            cx.set_reduce_motion(true);
            if let Err(error) = vessel_app_kit::init(cx, SchemeChoice::Fixed(scheme)) {
                tracing::error!(%error,"mapped diff audit could not initialize");
                cx.quit();
                return;
            }
            cx.on_window_closed(|cx, _| cx.quit()).detach();
            let options = WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(width), px(844.)),
                ))),
                ..WindowOptions::default()
            };
            if let Err(error) = cx.open_window(options, |window, cx| {
                cx.new(|cx| Audit::new(text, window, cx))
            }) {
                tracing::error!(%error,"mapped diff audit window could not open");
                cx.quit();
            }
            cx.activate(true);
        });
}
