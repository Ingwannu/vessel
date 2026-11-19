//! Paseo's sidebar status marks (TRACKER 662/663): what a workspace row's
//! leading slot draws (`WorkspaceStatusIndicator`,
//! `sidebar-workspace-row-content.tsx`) and the badge on a project's mark
//! (`ProjectStatusBadge`, `project-leading-visual.tsx`), with the running
//! ring both share (`components/status-ring`).
//!
//! [Decision Log]
//! - Purpose: one drawing of each status, so a session's row and its
//!   project's badge never disagree (Paseo keeps the geometry in
//!   `status-indicator-geometry.ts` and `status-ring/geometry.ts` for that
//!   reason).
//! - Constraint: GPUI tints an SVG in one colour, and Paseo's needs-input
//!   alert is two (an amber disc, its outline and marks in `surface0`);
//!   its ring's track and moving quarter differ only in opacity, which a
//!   mask carries badly (`loading.rs`: GPUI gamma-corrects mask coverage).
//! - Choice: the alert as two round boxes (the `surface0` outline's outer
//!   edge, then the amber inside it) under the marks' glyph; the ring as
//!   two opaque glyphs (track, quarter) with their opacity on the element,
//!   the quarter turned by the kit's rotation every 900 ms. A badge's shell
//!   and the ring's frame are filled with the row's own fill, following its
//!   hover and press through the row's group, so the mark reads as a hole
//!   in the project's mark (Paseo's knockout).
//! - Difference, named: Paseo's rings share one clock across the app (a row
//!   that appears mid-turn is in step); here each ring starts when drawn.

use crate::assets::Glyph;
use crate::theme::SidebarSurfaces;
use crate::{Tokens, loading};
use gpui::prelude::*;
use gpui::{AnyElement, Div, ElementId, Hsla, SharedString, Stateful, Svg, div, px, svg};
use std::sync::Arc;
use std::time::Duration;

/// A row's status, in Paseo's buckets (`SidebarStateBucket`), most urgent
/// first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusMark {
    /// Someone must answer: the amber alert.
    NeedsInput,
    /// The agent failed: a red dot.
    Failed,
    /// The agent works: the ring.
    Running,
    /// Finished work not yet seen: a green dot.
    Attention,
    /// Nothing to report: a faint dot in a row's slot, nothing on a badge.
    Done,
}

/// The title's line: a slot this tall centres its mark on the title's
/// first line (Paseo's `LEADING_SLOT_HEIGHT`, `TITLE_LINE_HEIGHT`).
pub const SLOT_HEIGHT: f32 = 20.0;
/// A filled status dot (`STATUS_INDICATOR_FILLED_DOT_SIZE`).
pub const FILLED_DOT: f32 = 6.0;
/// The dot over a workspace's glyph, with its 1 outline
/// (`STATUS_INDICATOR_DOT_SIZE`).
const OVERLAY_DOT: f32 = 8.0;
/// The alert (`STATUS_INDICATOR_ALERT_SIZE`), Lucide's `CircleAlert`.
const ALERT: f32 = 12.0;
/// Its outline's outer edge: Lucide's circle of radius 10 and stroke 2 on
/// a 24 grid, at 12.
const ALERT_OUTLINE: f32 = 11.0;
/// The amber inside the outline.
const ALERT_DISC: f32 = 9.0;
/// The badge's shell (`STATUS_BADGE_SIZE`).
const BADGE: f32 = 12.0;
/// How far the shell overhangs the mark's corner (`STATUS_BADGE_OFFSET`).
const BADGE_OFFSET: f32 = -4.0;
/// The ring around the 6 dot: a gap of 1.5 and a stroke of 1.5 each side.
const RING: f32 = 12.0;
/// Its frame, one wider each side: the knockout margin.
const RING_FRAME: f32 = 14.0;
/// Where the ring sits in its frame.
const RING_INSET: f32 = 1.0;
/// The frame re-anchored on the shell's corner, growing around the same
/// centre (`getStatusRingOffset(-4, 12)`).
const RING_OFFSET: f32 = -5.0;
/// The track behind the quarter, and the quarter itself.
const TRACK_OPACITY: f32 = 0.3;
const HEAD_OPACITY: f32 = 0.9;
/// One turn (`STATUS_RING_PERIOD_MS`).
const PERIOD: Duration = Duration::from_millis(900);
/// A done row's dot, holding the rail without reporting.
const IDLE_OPACITY: f32 = 0.3;

/// The icon box Paseo's `densityIconBox("md")` gives: 16 on a wide window,
/// 20 on a phone.
#[must_use]
pub const fn icon_box(compact: bool) -> f32 {
    if compact { 20.0 } else { 16.0 }
}

/// A workspace glyph's box (`DensityIcon size="sm"`): 14, 18 on a phone.
const fn glyph_box(compact: bool) -> f32 {
    if compact { 18.0 } else { 14.0 }
}

/// The row a badge sits on, so the badge's shell paints the row's own fill
/// in every state (Paseo's `SidebarSurfaceBackdrop`).
#[derive(Debug, Clone)]
pub struct Knockout {
    /// The row's group (its `group`), whose hover and press the shell
    /// follows.
    pub group: SharedString,
    /// Whether the row is selected: its fill holds on hover.
    pub selected: bool,
}

impl Knockout {
    /// `element` filled as the row behind it.
    fn fill(&self, element: Stateful<Div>, tokens: &Tokens) -> Stateful<Div> {
        let fills = SidebarSurfaces::of(tokens);
        let rest = if self.selected {
            fills.selected
        } else {
            fills.rest
        };
        let hover = if self.selected {
            fills.selected
        } else {
            fills.hover
        };
        element
            .bg(rest)
            .group_hover(self.group.clone(), move |style| style.bg(hover))
            .group_active(self.group.clone(), move |style| style.bg(fills.pressed))
    }
}

/// A workspace row's leading slot (Paseo's `WorkspaceStatusIndicator`):
/// the icon box wide and the title's line tall. `kind`: the workspace's
/// glyph a failed one shows under its red dot (Paseo draws it only then).
#[must_use]
pub fn status_slot(
    id: impl Into<ElementId>,
    mark: StatusMark,
    kind: Glyph,
    compact: bool,
    tokens: &Tokens,
) -> AnyElement {
    let id: ElementId = id.into();
    let status = tokens.status();
    let slot = div()
        .relative()
        .w(px(icon_box(compact)))
        .h(px(SLOT_HEIGHT))
        .flex_none()
        .flex()
        .items_center()
        .justify_center();
    match mark {
        StatusMark::Running => slot.child(ring(id, None, tokens)),
        StatusMark::NeedsInput => slot.child(alert(tokens)),
        StatusMark::Attention => slot.child(dot(FILLED_DOT, status.success_dot)),
        StatusMark::Done => slot.child(
            dot(FILLED_DOT, tokens.neutral.foreground_extra_muted).opacity(IDLE_OPACITY),
        ),
        StatusMark::Failed => slot
            .child(vector(
                kind,
                glyph_box(compact),
                tokens.neutral.foreground_muted,
            ))
            .child(
                dot(OVERLAY_DOT, status.danger_dot)
                    .absolute()
                    .right_0()
                    .bottom_0()
                    .border_1()
                    .border_color(tokens.neutral.surface0),
            ),
    }
    .into_any_element()
}

/// A project mark's leading slot (Paseo's `ProjectStatusIndicator`): the
/// icon box wide and the title's line tall, `mark` (the project's icon or
/// its lettered square, `icon_box` square) in the middle, and the project's
/// status badge on its corner when it has one to show.
#[must_use]
pub fn marked(mark: AnyElement, compact: bool, badge: Option<AnyElement>) -> Div {
    let side = px(icon_box(compact));
    div()
        .w(side)
        .h(px(SLOT_HEIGHT))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .relative()
                .size(side)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(mark)
                .children(badge),
        )
}

/// The badge on a project's mark (Paseo's `ProjectStatusBadge`): the alert,
/// a red or green dot in a 12 shell, or the ring anchored to the same
/// corner; nothing when done. `label`: the bucket's name, said as a status.
#[must_use]
pub fn status_badge(
    id: impl Into<ElementId>,
    mark: StatusMark,
    label: impl Into<SharedString>,
    knockout: &Knockout,
    tokens: &Tokens,
) -> Option<AnyElement> {
    let id: ElementId = id.into();
    let status = tokens.status();
    let inside = match mark {
        StatusMark::Done => return None,
        StatusMark::Running => {
            // The ring is wider than the shell and carries its own knockout:
            // drawn in the shell it would be clipped against the very margin
            // that separates it from the mark.
            return Some(
                div()
                    .id(ElementId::NamedChild(Arc::new(id.clone()), "anchor".into()))
                    .absolute()
                    .right(px(RING_OFFSET))
                    .bottom(px(RING_OFFSET))
                    .role(gpui::Role::Status)
                    .aria_label(label)
                    .child(ring(id, Some(knockout), tokens))
                    .into_any_element(),
            );
        }
        StatusMark::NeedsInput => alert(tokens),
        StatusMark::Failed => dot(FILLED_DOT, status.danger_dot).into_any_element(),
        StatusMark::Attention => dot(FILLED_DOT, status.success_dot).into_any_element(),
    };
    let shell = div()
        .id(id)
        .absolute()
        .right(px(BADGE_OFFSET))
        .bottom(px(BADGE_OFFSET))
        .size(px(BADGE))
        .rounded_full()
        .overflow_hidden()
        .flex()
        .items_center()
        .justify_center()
        .role(gpui::Role::Status)
        .aria_label(label);
    Some(knockout.fill(shell, tokens).child(inside).into_any_element())
}

/// Paseo's running ring: the frame (filled as the row behind it when it
/// knocks out of a mark), the track, the turning quarter, the dot.
fn ring(id: ElementId, knockout: Option<&Knockout>, tokens: &Tokens) -> AnyElement {
    let colour = tokens.status().running_dot;
    let frame = div()
        .id(ElementId::NamedChild(Arc::new(id.clone()), "ring".into()))
        .relative()
        .size(px(RING_FRAME))
        .flex_none()
        .rounded_full()
        .flex()
        .items_center()
        .justify_center();
    let frame = match knockout {
        Some(knockout) => knockout.fill(frame, tokens),
        None => frame,
    };
    let placed = |glyph| {
        vector(glyph, RING, colour)
            .absolute()
            .top(px(RING_INSET))
            .left(px(RING_INSET))
    };
    frame
        .child(placed(Glyph::StatusRingTrack).opacity(TRACK_OPACITY))
        .child(loading::rotate(
            ElementId::NamedChild(Arc::new(id), "turn".into()),
            placed(Glyph::StatusRingHead).opacity(HEAD_OPACITY),
            PERIOD,
        ))
        .child(dot(FILLED_DOT, colour))
        .into_any_element()
}

/// Lucide's `CircleAlert` filled amber, its outline and marks in
/// `surface0` (Paseo's `needsInputColorMapping`).
fn alert(tokens: &Tokens) -> AnyElement {
    let knockout = tokens.neutral.surface0;
    div()
        .relative()
        .size(px(ALERT))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .size(px(ALERT_OUTLINE))
                .rounded_full()
                .bg(knockout)
                .flex()
                .items_center()
                .justify_center()
                .child(dot(ALERT_DISC, tokens.status().warning_dot)),
        )
        .child(
            vector(Glyph::StatusAlertMarks, ALERT, knockout)
                .absolute()
                .top_0()
                .left_0(),
        )
        .into_any_element()
}

fn dot(size: f32, colour: Hsla) -> Div {
    div().size(px(size)).flex_none().rounded_full().bg(colour)
}

fn vector(glyph: Glyph, size: f32, colour: Hsla) -> Svg {
    svg()
        .path(glyph.path())
        .size(px(size))
        .flex_none()
        .text_color(colour)
}

#[cfg(test)]
#[path = "status_mark_tests.rs"]
mod tests;
