//! An agent message's avatar: the brand's star, relaxed into a circle,
//! holding the agent's own mark (the owner, 2026-11-14 ~05:55 UTC).
#![expect(
    dead_code,
    reason = "worker C mounts `avatar` at the head of each agent message; this expectation goes with that line"
)]

use vessel_app_kit::Tokens;
use vessel_app_kit::agent_mark::{Size, Space};
use vessel_app_kit::ui::{App, Div, ParentElement as _, Styled as _, div, px};
use vessel_app_kit::vector::{self, Drawing};
use vessel_app_view::brand::motion::{self, Journey};
use vessel_app_view::brand::{self as view, Pt};
use vessel_proto::HarnessRef;

use super::{AVATAR, Brand, QUIET_DARK, kit_shapes};

/// The avatar's ring and settled disc: the foreground at 22 %.
const AVATAR_INK: f32 = 0.22;

impl Brand {
    /// The journey now, for the avatar: done when none runs.
    pub(crate) fn journey(&self, still: bool) -> Journey {
        self.shared.journey.get().map_or(Journey::DONE, |started| {
            motion::journey(super::slow(started.at.elapsed().as_secs_f32()), still)
        })
    }
}

/// An agent message's avatar: the star relaxing into a circle and the
/// circle into a ring with the agent's own mark inside while the first
/// message's journey runs; the ring and the mark at rest. For worker C to
/// place at the head of each agent message.
pub(crate) fn avatar(brand: &Brand, agent: &HarnessRef, tokens: &Tokens, cx: &App) -> Div {
    let journey = brand.journey(vessel_app_kit::motion::held(cx));
    #[expect(clippy::float_arithmetic, reason = "the avatar's centre")]
    let half = AVATAR / 2.0;
    let shapes = view::avatar(Pt { x: half, y: half }, AVATAR, journey);
    let (disc, ring): (Vec<_>, Vec<_>) = shapes.into_iter().partition(|s| s.stroke.is_none());
    #[expect(
        clippy::float_arithmetic,
        reason = "the disc's ink grows from the drawing's to the ring's"
    )]
    let disc_alpha = (AVATAR_INK - QUIET_DARK).mul_add(journey.morphed, QUIET_DARK);
    let size = gpui::size(px(AVATAR), px(AVATAR));
    let layer = |shapes: Vec<view::Shape>, alpha: f32| {
        div().absolute().top_0().left_0().child(vector::inline(
            std::sync::Arc::new(Drawing {
                shapes: kit_shapes(&shapes),
                ink: tokens.neutral.foreground.opacity(alpha),
            }),
            size,
        ))
    };
    div()
        .relative()
        .flex_none()
        .size(px(AVATAR))
        .child(layer(disc, disc_alpha))
        .child(layer(ring, AVATAR_INK))
        .child(
            div()
                .absolute()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .opacity(journey.marked)
                .child(crate::agent_mark::mark(
                    Some(agent),
                    Size::Fourteen,
                    Space::Compact,
                    tokens,
                )),
        )
}
