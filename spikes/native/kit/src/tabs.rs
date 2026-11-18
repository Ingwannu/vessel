//! A panel's tabs, as Paseo's app draws its workspace tab row
//! (`screens/workspace/workspace-desktop-tabs-row.tsx`, `constants/
//! layout.ts`): a row 36 tall over a hairline, 4 in from each side; each
//! tab a chip 28 tall, 8 across, 6 corners, 4 apart, its label at 14 in
//! the muted ink; the chosen one filled with the second surface in the
//! full ink, a hovered one on the first. A count follows the label in the
//! muted ink at 12.
//!
//! [Decision Log]
//! - Purpose: the inspector beside a session switches what it shows
//!   (Summary, Changes, Files); track C's Summary and this track's code
//!   panel share the row.
//! - Constraint: the owner's rule (the brief's "The look"): Paseo's app is
//!   the model; the page's strip (icon squares named by tooltips) differs,
//!   and the kit has no icons yet.
//! - Choice: Paseo's labelled chips, which fit three in the narrowest
//!   inspector (360 points) and need no icon or tooltip; Paseo's close
//!   buttons and dragging are for its editor tabs and are not drawn here.
//!   Colours are Paseo's neutrals (`tokens.neutral`): the chosen fill
//!   `surface2`, a hovered one `surface1`, the row's hairline `border`.
//! - Cost: the page's corner badge for a count becomes a number after the
//!   label (the page is brought to the same, ADR 0036).

use gpui::prelude::*;
use gpui::{Div, ElementId, SharedString, Stateful, div, px};

use crate::fonts::Type;
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

/// The row's height (Paseo's bar, pointer-sized).
pub const HEAD_HEIGHT: f32 = 36.0;

/// The row the tabs sit in, with the panel's own actions at its right
/// (`end`).
#[must_use]
pub fn tab_head(tabs: Div, end: Option<Div>, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .gap(px(4.0))
        .h(px(HEAD_HEIGHT))
        .px(px(4.0))
        .border_b_1()
        .border_color(tokens.neutral.border)
        .child(
            tabs.flex()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .min_w(px(0.0)),
        )
        .children(end.map(|end| {
            end.flex()
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .ml_auto()
                .min_w(px(0.0))
        }))
}

/// A count as a tab writes it (past 99, "99+").
#[must_use]
pub fn count_text(count: u64) -> String {
    if count > 99 {
        "99+".to_owned()
    } else {
        count.to_string()
    }
}

/// One tab: its label, whether it is chosen, and a count (none at zero).
#[must_use]
pub fn tab(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    chosen: bool,
    count: Option<u64>,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = &tokens.neutral;
    let hover = neutral.surface1;
    div()
        .id(id)
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .gap(px(4.0))
        .h(px(28.0))
        .px(px(8.0))
        .rounded(px(6.0))
        .type_(Type::Body)
        .text_size(px(crate::fonts::ui(14.0)))
        .text_color(if chosen {
            neutral.foreground
        } else {
            neutral.foreground_muted
        })
        .when(chosen, |tab| tab.bg(neutral.surface2))
        .when(!chosen, |tab| {
            tab.cursor_pointer().hover(move |style| style.bg(hover))
        })
        // One line with a tail ellipsis, as Paseo's tab label: wrapped so it
        // can shrink below its words when a caller caps the tab's width (a
        // bare text child keeps its full width and was cut mid-letter).
        .child(
            div()
                .debug_selector(|| "tab-label".to_owned())
                .min_w(px(0.0))
                .truncate()
                .child(label.into()),
        )
        .children(count.filter(|count| *count > 0).map(|count| {
            div()
                .type_(Type::Meta)
                .text_color(neutral.foreground_extra_muted)
                .child(count_text(count))
        }))
}

#[cfg(test)]
mod tests {
    use super::{count_text, tab};
    use crate::{Scheme, SchemeChoice, Tokens};
    use gpui::{
        Context, InteractiveElement as _, IntoElement, ParentElement as _, Render, Styled as _,
        TestAppContext, Window, div, px,
    };

    struct Probe;
    impl Render for Probe {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
            let tokens = Tokens::of(Scheme::Dark);
            let long = "Review the shell's command palette with long session titles";
            div().child(
                tab("long", long, false, None, &tokens)
                    .debug_selector(|| "probe-tab".to_owned())
                    .max_w(px(128.0)),
            )
        }
    }

    #[gpui::test]
    fn a_long_label_ends_inside_its_capped_tab(cx: &mut TestAppContext) {
        cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
        let (_view, cx) = cx.add_window_view(|_window, _cx| Probe);
        cx.run_until_parked();
        let chip = cx.debug_bounds("probe-tab").unwrap();
        let label = cx.debug_bounds("tab-label").unwrap();
        assert_eq!(chip.size.width, px(128.0));
        // Inside the chip's 8 inset: the words end in an ellipsis there.
        assert!(label.right() <= chip.right(), "{label:?} in {chip:?}");
        assert!(label.left() >= chip.left(), "{label:?} in {chip:?}");
    }

    #[test]
    fn writes_a_count_as_a_tab_does() {
        assert_eq!(count_text(7), "7");
        assert_eq!(count_text(99), "99");
        assert_eq!(count_text(100), "99+");
    }
}
