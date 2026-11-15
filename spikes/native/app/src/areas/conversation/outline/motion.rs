//! Magnification changes only a pill's paint box, never its stable slot.
//! Reuse the shared quick transition (150 ms rather than Paseo's CSS 140 ms).

use gpui::AnimationExt as _;
use vessel_app_kit::motion::{QUICK, transition};
use vessel_app_kit::ui::{AnyElement, ElementId, Hsla, IntoElement as _, Styled as _, div, px};

use super::{HIGH, LIT_WIDE, REST_HIGH, REST_WIDE, WIDE_FROM_LIT, WIDE_FROM_REST};

#[derive(Debug, Clone, Copy)]
pub(super) struct Size {
    width: f32,
    height: f32,
}

pub(super) fn size(index: usize, lit: bool, attention: Option<usize>, held: bool) -> Size {
    let (widths, rest) = if lit {
        (&WIDE_FROM_LIT, LIT_WIDE)
    } else {
        (&WIDE_FROM_REST, REST_WIDE)
    };
    let near = attention.filter(|_| !held).map(|at| at.abs_diff(index));
    let (width, height) = near
        .and_then(|near| Some((*widths.get(near)?, *HIGH.get(near)?)))
        .unwrap_or((rest, REST_HIGH));
    Size { width, height }
}

pub(super) fn pill(from: Size, to: Size, color: Hsla, turn: u64) -> AnyElement {
    let pill = div()
        .w(px(to.width))
        .h(px(to.height))
        .rounded_full()
        .bg(color);
    if turn == 0 {
        return pill.into_any_element();
    }
    pill.with_animation(
        ElementId::NamedInteger("outline-motion".into(), turn),
        transition(QUICK),
        move |pill, progress| {
            pill.w(px(interpolate(from.width, to.width, progress)))
                .h(px(interpolate(from.height, to.height, progress)))
        },
    )
    .into_any_element()
}

#[expect(
    clippy::float_arithmetic,
    reason = "interpolation of bounded logical pill dimensions at a 0..=1 animation progress"
)]
fn interpolate(from: f32, to: f32, progress: f32) -> f32 {
    progress.mul_add(to - from, from)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reduced_motion_preserves_resting_sizes_and_dock_magnification_is_bounded() {
        let rest = size(0, false, Some(0), true);
        assert_eq!(rest.width.total_cmp(&REST_WIDE), std::cmp::Ordering::Equal);
        assert_eq!(rest.height.total_cmp(&REST_HIGH), std::cmp::Ordering::Equal);
        for index in 0..6 {
            let pill = size(index, false, Some(0), false);
            assert!((REST_WIDE..=WIDE_FROM_REST[0]).contains(&pill.width));
            assert!((REST_HIGH..=HIGH[0]).contains(&pill.height));
        }
    }
}
