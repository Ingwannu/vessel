//! Staged facade for the ONE shared borrowed detail kernel.
//! Remove the parent module cfg(test) only with the actual A8 product caller.
use super::entries::Paint;
use super::tool_body::{self, BodyPaint};
pub(super) use super::tool_body::{BorrowedDetail, DetailContentState};
use vessel_app_kit::ui::{App, Div, ElementId, Window};

impl<'a> From<&Paint<'a>> for BodyPaint<'a> {
    fn from(paint: &Paint<'a>) -> Self {
        Self {
            catalog: paint.catalog,
            words: paint.words,
            tokens: paint.tokens,
            secondary: paint.look.secondary,
        }
    }
}

/// Existing callers keep their drawing context; the body reads only its narrow projection.
pub(super) fn borrowed_body(
    paint: &Paint<'_>,
    id: &ElementId,
    detail: BorrowedDetail<'_>,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    tool_body::render(&BodyPaint::from(paint), id, detail, window, cx)
}
