//! A compact footer reveals its timestamp on a tap; desktop hover and both
//! labels reserve the same width. A late expiry cannot hide a newer reveal.

use std::time::{Duration, Instant};

use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled, Window, div, px,
};
use vessel_app_kit::{breakpoint, version_of};

use super::{METADATA, Paint, Shell, id};

/// Paseo's compact `TIMESTAMP_REVEAL_MS`.
const TIMESTAMP_REVEAL: Duration = Duration::from_millis(3_000);
/// A footer label remains a touch target on a phone.
const TOUCH_HEIGHT: f32 = 44.0;

#[derive(Default)]
struct Revealed {
    at: Option<Instant>,
}

pub(super) fn draw(
    paint: &Paint<'_>,
    index: usize,
    primary: String,
    timestamp: String,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let narrow = breakpoint::phone(window);
    // This is presentation cache identity, not a session or permission id.
    let identity = id(index) ^ version_of(&timestamp).rotate_left(1) ^ version_of(&primary);
    let state = window.use_keyed_state(
        ElementId::NamedInteger("footer-time".into(), identity),
        cx,
        |_, _| Revealed::default(),
    );
    let showing = narrow
        && state
            .read(cx)
            .at
            .is_some_and(|at| at.elapsed() < TIMESTAMP_REVEAL);
    let group = format!("footer-time-{identity}");
    let sizer = if primary.chars().count() >= timestamp.chars().count() {
        primary.clone()
    } else {
        timestamp.clone()
    };
    let label = div()
        .id(ElementId::NamedInteger(
            "footer-time-label".into(),
            identity,
        ))
        .relative()
        .group(group.clone())
        .min_w_0()
        .text_size(px(METADATA))
        .text_color(paint.tokens.neutral.foreground_muted)
        .when(narrow, |label| {
            label.min_h(px(TOUCH_HEIGHT)).cursor_pointer()
        })
        .child(div().invisible().whitespace_nowrap().child(sizer))
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .whitespace_nowrap()
                .when(!narrow, |label| {
                    label.group_hover(group.clone(), Styled::invisible)
                })
                .child(if showing { timestamp.clone() } else { primary }),
        )
        .when(!narrow, |label| {
            label.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .invisible()
                    .whitespace_nowrap()
                    .group_hover(group, Styled::visible)
                    .child(timestamp),
            )
        })
        .when(narrow, |label| {
            label.on_click(cx.listener(move |_shell, _event, window, cx| {
                let at = state.update(cx, |state, _| {
                    state.at = if state.at.is_some_and(|at| at.elapsed() < TIMESTAMP_REVEAL) {
                        None
                    } else {
                        Some(Instant::now())
                    };
                    state.at
                });
                cx.notify();
                if let Some(at) = at {
                    let state = state.clone();
                    cx.spawn_in(window, async move |this, cx| {
                        cx.background_executor().timer(TIMESTAMP_REVEAL).await;
                        let _gone = this.update(cx, |_shell, cx| {
                            state.update(cx, |state, _| {
                                if state.at == Some(at) {
                                    state.at = None;
                                }
                            });
                            cx.notify();
                        });
                    })
                    .detach();
                }
            }))
        });
    label.into_any_element()
}
