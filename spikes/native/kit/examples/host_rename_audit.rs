//! Interactive audit of the real shared rename action. No node or app
//! route is involved. Rows show ordinary, disabled, neighbour and clipped
//! consumers; counters distinguish the action from its parent and neighbour.

use gpui::prelude::*;
use gpui::{Bounds, Context, MouseButton, Render, Window, WindowOptions, div, point, px, size};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::{
    Assets, ButtonState, Scheme, SchemeChoice, Tokens, Type, TypeExt as _, host_rename_button,
    outline_button,
};

/// These fixed coordinates are an audit grid, not control geometry.
const LEFT: f32 = 160.0;
const FIRST: f32 = 80.0;
const PITCH: f32 = 90.0;
const FRAME: f32 = 22.0;

#[derive(Clone, Copy, PartialEq)]
enum Case {
    Ordinary,
    Disabled,
    Neighbour,
    Clipped,
}

impl Case {
    const ALL: [Self; 4] = [
        Self::Ordinary,
        Self::Disabled,
        Self::Neighbour,
        Self::Clipped,
    ];

    const fn index(self) -> usize {
        match self {
            Self::Ordinary => 0,
            Self::Disabled => 1,
            Self::Neighbour => 2,
            Self::Clipped => 3,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Ordinary => "Ordinary",
            Self::Disabled => "Disabled",
            Self::Neighbour => "Neighbour",
            Self::Clipped => "Clipped parent",
        }
    }
}

/// Saturating counters keep repeated manual input from wrapping.
#[derive(Default)]
struct Presses {
    action: u32,
    parent: u32,
    parent_up: u32,
    neighbour: u32,
}

struct Sheet {
    scheme: Scheme,
    presses: [Presses; 4],
}

impl Sheet {
    fn parent(
        case: Case,
        action: gpui::Stateful<gpui::Div>,
        tokens: &Tokens,
        cx: &mut Context<'_, Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let index = case.index();
        div()
            .id(("audit-parent", index))
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .w(px(FRAME))
            .h(px(FRAME))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |sheet, _event, _window, cx| {
                    if let Some(presses) = sheet.presses.get_mut(index) {
                        presses.parent = presses.parent.saturating_add(1);
                    }
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |sheet, _event, _window, cx| {
                    if let Some(presses) = sheet.presses.get_mut(index) {
                        presses.parent_up = presses.parent_up.saturating_add(1);
                    }
                    cx.notify();
                }),
            )
            .when(case == Case::Clipped, |parent| {
                parent.overflow_hidden().bg(tokens.neutral.surface2)
            })
            .child(action)
    }

    fn group(case: Case, tokens: &Tokens, cx: &mut Context<'_, Self>) -> gpui::Div {
        let index = case.index();
        let state = if case == Case::Disabled {
            ButtonState::Disabled
        } else {
            ButtonState::Enabled
        };
        let action = host_rename_button(("audit-rename", index), state, tokens).when(
            state == ButtonState::Enabled,
            |action| {
                action.on_click(cx.listener(move |sheet, _event, _window, cx| {
                    if let Some(presses) = sheet.presses.get_mut(index) {
                        presses.action = presses.action.saturating_add(1);
                    }
                    cx.notify();
                }))
            },
        );
        let parent = Self::parent(case, action, tokens, cx);
        div()
            .relative()
            .flex()
            .flex_row()
            .gap(px(spacing::S1))
            .child(parent)
            .when(case == Case::Neighbour, |group| {
                group.child(
                    outline_button("audit-neighbour", "Next", true, tokens).on_click(cx.listener(
                        move |sheet, _event, _window, cx| {
                            if let Some(presses) = sheet.presses.get_mut(index) {
                                presses.neighbour = presses.neighbour.saturating_add(1);
                            }
                            cx.notify();
                        },
                    )),
                )
            })
    }

    fn row(&self, case: Case, tokens: &Tokens, cx: &mut Context<'_, Self>) -> gpui::Div {
        let index = case.index();
        let words = self.presses.get(index).map_or_else(String::new, |presses| {
            format!(
                "action={} parent={} up={} neighbour={}",
                presses.action, presses.parent, presses.parent_up, presses.neighbour
            )
        });
        div()
            .relative()
            .w_full()
            .h(px(PITCH))
            .child(
                div()
                    .absolute()
                    .left(px(spacing::S4))
                    .top(px(0.0))
                    .child(case.label()),
            )
            .child(
                div()
                    .absolute()
                    .left(px(LEFT))
                    .top(px(0.0))
                    .child(Self::group(case, tokens, cx)),
            )
            .child(
                div()
                    .absolute()
                    .left(px(spacing::S4))
                    .top(px(spacing::S8))
                    .child(words),
            )
    }
}

impl Render for Sheet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        div()
            .size_full()
            .relative()
            .bg(tokens.neutral.surface0)
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground)
            .child(div().p(px(spacing::S4)).child("Shared rename input audit"))
            .child(
                div()
                    .absolute()
                    .top(px(FIRST))
                    .left(px(0.0))
                    .right(px(0.0))
                    .flex()
                    .flex_col()
                    .children(Case::ALL.map(|case| self.row(case, &tokens, cx))),
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
                tracing::error!(%error, "rename audit fonts could not start");
                cx.quit();
                return;
            }
            let options = WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: size(px(520.0), px(480.0)),
                })),
                ..WindowOptions::default()
            };
            if let Err(error) = cx.open_window(options, |_window, cx| {
                cx.new(|_cx| Sheet {
                    scheme,
                    presses: std::array::from_fn(|_| Presses::default()),
                })
            }) {
                tracing::error!(%error, "rename audit window could not start");
                cx.quit();
            }
        });
}
