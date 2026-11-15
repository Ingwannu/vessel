//! Source row-background precedence shared by frame and trailing scrim.
use super::{DragState, Selection};
use crate::{Scheme, Tokens};
use gpui::Hsla;
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Paints {
    pub(super) rest: Hsla,
    pub(super) hover: Hsla,
    pub(super) pressed: Hsla,
}
impl Paints {
    pub(super) fn new(selection: Selection, drag: DragState, tokens: &Tokens) -> Self {
        let (hover, selected) = match tokens.scheme {
            Scheme::Light => (tokens.neutral.surface3, tokens.neutral.surface4),
            Scheme::Dark => (tokens.neutral.surface1, tokens.neutral.surface2),
        };
        let rest = if drag == DragState::Dragging {
            tokens.neutral.surface2
        } else if selection == Selection::Active {
            selected
        } else {
            tokens.neutral.surface_sidebar
        };
        let hover = if drag == DragState::Dragging {
            tokens.neutral.surface2
        } else if selection == Selection::Active {
            selected
        } else {
            hover
        };
        Self {
            rest,
            hover,
            pressed: tokens.neutral.surface2,
        }
    }
}
