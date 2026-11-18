//! Paseo's shared screen/compact header, with caller-owned navigation.
//!
//! [Decision Log]
//! - Purpose: one frame for schedules, board and settings, rather than
//!   an area-specific title size, spacing or phone navigation bar.
//! - Constraint: the app owns routes, sidebar state, window drag and chrome.
//! - Alternatives: area-local headers, or a kit control coupled to Shell.
//! - Choice: a typed frame with title/leading/actions slots; convenience
//!   entries delegate, and a compact menu calls only its supplied callback.
//! - Why: settings' back button and the shell's menu share one renderer
//!   without sharing navigation policy. Existing kit faces supply text metrics.
//! - Cost: no drag integration.
//! - Subtitle (TRACKER 789, the compact workspace header): Paseo's
//!   `CompactHeader` takes a line under its title, and its title then reads
//!   one step smaller (14 rather than 16). The caller supplies the line; the
//!   frame keeps the row, the islands and the title's ink and weight.

use crate::{Tokens, Type, TypeExt as _, assets::Glyph, fonts, picker, theme::spacing};
use gpui::prelude::*;
use gpui::{AnyElement, App, Div, FontWeight, Hsla, SharedString, Svg, Window, div, px, svg};

/// The reference's two screen layouts, using the app's compact breakpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Density {
    /// `ScreenHeader` / `ScreenTitle`.
    Desktop,
    /// `CompactHeader` with one-line title and action islands.
    Compact,
}

/// Presentation slots; their callbacks and navigation stay with the caller.
pub struct Content {
    /// Whole translated title; it is one line, as in the reference header.
    pub title: SharedString,
    /// Caller-owned back/toggle control, already wired.
    pub leading: Option<AnyElement>,
    /// Caller-owned, already wired actions.
    pub actions: Option<AnyElement>,
    /// A line under the title (Paseo's `subtitle`); the title is then 14.
    pub subtitle: Option<AnyElement>,
}
impl std::fmt::Debug for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Content")
            .field("title", &self.title)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Geometry {
    top: f32,
    row: f32,
    horizontal: f32,
    outer_gap: f32,
    title: f32,
    weight: FontWeight,
}
/// Paseo's `fontSize.base`: a title over a subtitle.
const SUBTITLED_TITLE: f32 = 14.0;

/// The title's size: one step smaller over a subtitle (Paseo's
/// `singleLineTitle` is only for a title alone).
const fn title_size(geometry: Geometry, subtitled: bool) -> f32 {
    if subtitled {
        SUBTITLED_TITLE
    } else {
        geometry.title
    }
}

const fn geometry(density: Density) -> Geometry {
    match density {
        Density::Desktop => Geometry {
            top: spacing::S0,
            row: 36.0,
            horizontal: spacing::S3,
            outer_gap: spacing::S0,
            title: 14.0,
            weight: FontWeight::LIGHT,
        },
        Density::Compact => Geometry {
            top: spacing::S2,
            row: 56.0,
            horizontal: spacing::S3,
            outer_gap: spacing::S2,
            title: 16.0,
            weight: FontWeight::MEDIUM,
        },
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct Paint {
    background: Hsla,
    border: Option<Hsla>,
    title: Hsla,
}
fn paint(density: Density, tokens: &Tokens) -> Paint {
    Paint {
        background: match density {
            Density::Desktop => tokens.neutral.surface0,
            Density::Compact => gpui::transparent_black(),
        },
        border: match density {
            Density::Desktop => Some(tokens.neutral.border),
            Density::Compact => None,
        },
        title: tokens.neutral.foreground,
    }
}

/// One renderer for a menu header or a settings back/detail header.
#[must_use]
pub fn frame(content: Content, density: Density, tokens: &Tokens) -> Div {
    let geometry = geometry(density);
    let paint = paint(density, tokens);
    let Content {
        title,
        leading,
        actions,
        subtitle,
    } = content;
    let size = title_size(geometry, subtitle.is_some());
    let title = div()
        .min_w_0()
        .flex_shrink(1.0)
        .type_(Type::Body)
        .text_size(px(fonts::ui(size)))
        .font_weight(geometry.weight)
        .text_color(paint.title)
        .truncate()
        .child(picker::one_line(&title));
    let title = div()
        .flex()
        .flex_col()
        .min_w_0()
        .flex_shrink(1.0)
        .child(title)
        .children(subtitle);
    let (left, leading) = match density {
        Density::Desktop => (
            div()
                .flex()
                .items_center()
                .gap(px(spacing::S2))
                .min_w_0()
                .flex_1()
                .children(leading)
                .child(title),
            None,
        ),
        Density::Compact => (
            div().min_w_0().flex_1().child(title),
            leading_for_compact(leading),
        ),
    };
    let mut row = div()
        .flex()
        .items_center()
        .justify_between()
        .h(px(geometry.row))
        .gap(px(geometry.outer_gap))
        .children(leading);
    row = row.child(left).children(actions.map(|actions| {
        div()
            .flex()
            .items_center()
            .flex_none()
            .gap(px(spacing::S2))
            .child(actions)
    }));
    if let Some(border) = paint.border {
        row = row.border_b_1().border_color(border);
    }
    match density {
        // ScreenHeader's padding and border belong to the SAME full-width
        // row. An outer padded wrapper would shorten the bottom rule.
        Density::Desktop => row
            .flex_none()
            .w_full()
            .px(px(geometry.horizontal))
            .bg(paint.background),
        Density::Compact => div()
            .flex_none()
            .w_full()
            .pt(px(geometry.top))
            .px(px(geometry.horizontal))
            .bg(paint.background)
            .child(row),
    }
}

fn leading_for_compact(leading: Option<AnyElement>) -> Option<Div> {
    leading.map(|leading| div().flex().items_center().flex_none().child(leading))
}

/// A title/actions header. Native chrome remains the caller's responsibility.
#[must_use]
pub fn screen_header(
    title: impl Into<SharedString>,
    actions: Vec<AnyElement>,
    tokens: &Tokens,
    narrow: bool,
) -> Div {
    frame(
        Content {
            title: title.into(),
            leading: None,
            actions: action_slot(actions),
            subtitle: None,
        },
        if narrow {
            Density::Compact
        } else {
            Density::Desktop
        },
        tokens,
    )
}

/// Phone menu header; the callback alone decides what opening the menu means.
#[must_use]
pub fn compact_header(
    title: impl Into<SharedString>,
    actions: Vec<AnyElement>,
    on_menu: impl Fn(&mut Window, &mut App) + 'static,
    tokens: &Tokens,
) -> Div {
    let wash = tokens.neutral.interaction_highlight;
    let menu = div()
        .id("compact-header-menu")
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(crate::breakpoint::TOUCH_TARGET))
        .rounded_full()
        .cursor_pointer()
        .occlude()
        .hover(move |style| style.bg(wash))
        .active(move |style| style.bg(wash))
        .child(menu_icon(tokens))
        .on_click(move |_event, window, cx| on_menu(window, cx));
    frame(
        Content {
            title: title.into(),
            leading: Some(menu.into_any_element()),
            actions: action_slot(actions),
            subtitle: None,
        },
        Density::Compact,
        tokens,
    )
}
fn action_slot(actions: Vec<AnyElement>) -> Option<AnyElement> {
    if actions.is_empty() {
        None
    } else {
        Some(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::S2))
                .children(actions)
                .into_any_element(),
        )
    }
}
fn menu_icon(tokens: &Tokens) -> Svg {
    // Svg requires its own ink on this pin; parent text colour is insufficient.
    svg()
        .path(Glyph::MenuCompact.path())
        .w(px(16.0))
        .h(px(12.0))
        .text_color(tokens.neutral.foreground_muted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scheme;

    #[test]
    fn header_geometry_matches_the_reference_at_both_layouts() {
        assert_eq!(
            geometry(Density::Desktop),
            Geometry {
                top: 0.0,
                row: 36.0,
                horizontal: 12.0,
                outer_gap: 0.0,
                title: 14.0,
                weight: FontWeight::LIGHT
            }
        );
        assert_eq!(
            geometry(Density::Compact),
            Geometry {
                top: 8.0,
                row: 56.0,
                horizontal: 12.0,
                outer_gap: 8.0,
                title: 16.0,
                weight: FontWeight::MEDIUM
            }
        );
        assert_eq!(
            crate::breakpoint::TOUCH_TARGET.to_bits(),
            44.0_f32.to_bits()
        );
    }
    #[test]
    fn a_subtitle_steps_the_compact_title_down_to_paseos_base() {
        let compact = geometry(Density::Compact);
        assert_eq!(title_size(compact, false).to_bits(), 16.0_f32.to_bits());
        assert_eq!(title_size(compact, true).to_bits(), 14.0_f32.to_bits());
    }
    #[test]
    fn frame_and_actual_menu_vector_have_literal_reference_colours() {
        for (scheme, background, border, foreground, muted) in [
            (
                Scheme::Light,
                0x00ff_ffff,
                0x00e4_e4e7,
                0x001a_1a1e,
                0x0071_717a,
            ),
            (
                Scheme::Dark,
                0x0018_1b1a,
                0x0025_2b2a,
                0x00fa_fafa,
                0x00a1_a5a4,
            ),
        ] {
            let tokens = Tokens::of(scheme);
            assert_eq!(
                paint(Density::Desktop, &tokens),
                Paint {
                    background: gpui::rgb(background).into(),
                    border: Some(gpui::rgb(border).into()),
                    title: gpui::rgb(foreground).into()
                }
            );
            assert_eq!(
                paint(Density::Compact, &tokens),
                Paint {
                    background: gpui::transparent_black(),
                    border: None,
                    title: gpui::rgb(foreground).into()
                }
            );
            let mut icon = menu_icon(&tokens);
            assert_eq!(icon.style().text.color, Some(gpui::rgb(muted).into()));
        }
    }
}

#[cfg(test)]
mod frame_regression {
    use super::*;
    use crate::Scheme;

    #[test]
    fn desktop_returned_frame_owns_full_width_border_and_total_height() {
        let mut header = frame(
            Content {
                title: "Schedules".into(),
                leading: None,
                actions: None,
                subtitle: None,
            },
            Density::Desktop,
            &Tokens::of(Scheme::Light),
        );
        let style = header.style();
        assert_eq!(style.size.width, Some(gpui::relative(1.0).into()));
        assert_eq!(style.size.height, Some(px(36.0).into()));
        assert_eq!(style.border_widths.bottom, Some(px(1.0).into()));
        assert_eq!(style.padding.left, Some(px(12.0).into()));
        assert_eq!(style.padding.right, Some(px(12.0).into()));
    }
}
