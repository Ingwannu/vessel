//! Which rows of a long list to draw: those within a margin of the view.
//! The others keep their space, from the heights measured so far and a
//! guess for the rest (the page's `data/window.ts`, which its timeline
//! draws with).
//!
//! [Decision Log]
//! - Purpose: a 20,000-entry session scrolls smoothly on every surface;
//!   the window, the follow-the-end rule and "go to this row" (find, the
//!   outline) behave alike on the page and in the app (ADR 0036).
//! - Constraint: this crate holds no UI toolkit's types and its lints deny
//!   float arithmetic, so pixels here are whole numbers. The app's list
//!   measures in fractional pixels and rounds once, where it hands a
//!   height over.
//! - Alternatives: the toolkit's own list alone (GPUI's `list` windows
//!   and measures by itself): it does, and the app's list uses it, but
//!   where a row goes when the window goes to it, and when the view
//!   follows the end, are rules of Vessel's, written once here with the
//!   page's cases so both surfaces answer the same.
//! - Cost: a guessed height is a whole-pixel mean, where the page's is
//!   fractional; a list of 20,000 guessed rows is then off by under a
//!   pixel each, which the drawn row's correction (`off_by`) takes up as
//!   on the page.

/// The rows to draw, `start` to `end` (exclusive), and the space the rows
/// before and after them take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The first row drawn.
    pub start: usize,
    /// One past the last row drawn.
    pub end: usize,
    /// Pixels above the first row drawn.
    pub before: u32,
    /// Pixels below the last row drawn.
    pub after: u32,
}

/// How far from the end a view still follows it, in pixels (the page's
/// 48).
pub const FOLLOW: u32 = 48;

/// How far past the view rows are drawn, each way, in pixels (the page's
/// window draws about 1,200 px around the view).
pub const MARGIN: u32 = 1200;

/// The gap left above a row put at the view's top, in pixels.
pub const INSET: u32 = 16;

/// The mean of the heights measured, or `fallback` before any.
#[must_use]
pub fn guess(measured: impl IntoIterator<Item = u32>, fallback: u32) -> u32 {
    let (sum, count) = measured
        .into_iter()
        .fold((0_u64, 0_u64), |(sum, count), height| {
            (
                sum.saturating_add(u64::from(height)),
                count.saturating_add(1),
            )
        });
    sum.checked_div(count)
        .and_then(|mean| u32::try_from(mean).ok())
        .unwrap_or(fallback)
}

/// Where each of `count` rows starts, and after the last, where the list
/// ends: `count + 1` offsets. `height(row)` is a row's measured height,
/// `None` before it was drawn; those are taken as `estimate`.
#[must_use]
pub fn offsets(count: usize, height: impl Fn(usize) -> Option<u32>, estimate: u32) -> Vec<u32> {
    let mut starts = Vec::with_capacity(count.saturating_add(1));
    let mut at = 0_u32;
    starts.push(at);
    for row in 0..count {
        at = at.saturating_add(height(row).unwrap_or(estimate));
        starts.push(at);
    }
    starts
}

/// The first row whose bottom is below `y`, by binary search over
/// `starts` (increasing, as `offsets` gives them).
fn first_ending_below(starts: &[u32], y: u32) -> usize {
    // Rows are `starts[i]..starts[i + 1]`; a row ends below `y` when its
    // end is past it.
    let ends = starts.get(1..).unwrap_or_default();
    ends.partition_point(|end| *end <= y)
}

/// The rows overlapping `top` to `top + height`, widened by `margin` each
/// way.
#[must_use]
pub fn span(starts: &[u32], top: u32, height: u32, margin: u32) -> Span {
    let count = starts.len().saturating_sub(1);
    let total = starts.last().copied().unwrap_or(0);
    if count == 0 {
        return Span {
            start: 0,
            end: 0,
            before: 0,
            after: 0,
        };
    }
    let last = count.saturating_sub(1);
    let start = first_ending_below(starts, top.saturating_sub(margin)).min(last);
    let bottom = top.saturating_add(height).saturating_add(margin);
    let mut end = start;
    while end < count && starts.get(end).copied().unwrap_or(total) < bottom {
        end = end.saturating_add(1);
    }
    let end = end.max(start.saturating_add(1));
    let before = starts.get(start).copied().unwrap_or(0);
    let after = total.saturating_sub(starts.get(end).copied().unwrap_or(total));
    Span {
        start,
        end,
        before,
        after,
    }
}

/// Where a row is put when the window goes to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// Its top just under the view's top (the outline's jump to a
    /// prompt).
    Start,
    /// Its middle in the view's middle (find's match).
    Center,
}

fn half(value: u32) -> u32 {
    value.checked_div(2).unwrap_or(0)
}

/// The scroll offset that puts row `index` at `align` in a view `view`
/// pixels high, from the offsets as estimated. The drawn row then says how
/// far off the estimate was (`off_by`).
#[must_use]
pub fn scroll_for(starts: &[u32], index: usize, view: u32, align: Align) -> u32 {
    let top = starts.get(index).copied().unwrap_or(0);
    let bottom = starts.get(index.saturating_add(1)).copied().unwrap_or(top);
    match align {
        Align::Start => top.saturating_sub(INSET),
        Align::Center => half(top.saturating_add(bottom)).saturating_sub(half(view)),
    }
}

/// How far the content must scroll (positive: down) for a box drawn `top`
/// pixels below the view's top edge (negative: above it), `height` high,
/// to sit at `align` in a view `view` pixels high. A box taller than the
/// view is put at its top, as `Center` would hide its start.
#[must_use]
pub fn off_by(top: i64, height: u32, view: u32, align: Align) -> i64 {
    let inset = i64::from(INSET);
    match align {
        Align::Center if height <= view => top
            .saturating_add(i64::from(half(height)))
            .saturating_sub(i64::from(half(view))),
        Align::Start | Align::Center => top.saturating_sub(inset),
    }
}

/// Whether the drawn rows (`drawn`, over `starts`) cover a view `height`
/// pixels high scrolled to `top`: when they do not, part of the view shows
/// empty space until the window is worked out again.
#[must_use]
pub fn covers(starts: &[u32], drawn: Span, top: u32, height: u32) -> bool {
    let from = starts.get(drawn.start).copied().unwrap_or(0);
    let to = starts.get(drawn.end).copied().unwrap_or(from);
    drawn.start == drawn.end || (from <= top && to >= top.saturating_add(height))
}

/// Whether a view scrolled to `top`, `view` pixels high, over content
/// `content` pixels high, is close enough to the end to follow it as the
/// log grows.
#[must_use]
pub const fn at_end(top: u32, view: u32, content: u32) -> bool {
    content.saturating_sub(top.saturating_add(view)) <= FOLLOW
}
