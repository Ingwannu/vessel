//! A tool's text (a command's output, a call's input, an edit's diff) as
//! GOAT's command output panel draws it
//! (`apps/web/src/components/chat/CommandOutputPanel.tsx`): a box at most
//! 192 pixels high (`max-h-48`) that scrolls both ways, following the end,
//! with a copy button in its corner (`CopyRegion`, "Copy output"), the
//! text at 80% of the foreground. A diff's lines carry their number and a
//! bar in the added or removed colour at their left (the `bars` style of
//! GOAT's diff renderer, `@pierre/diffs`), on a wash of the same colour.
//!
//! [Decision Log]
//! - Purpose: a 2,400-line test run and a 75-line diff of 240-character
//!   lines read in place. The boxes before this cut the output at 240
//!   pixels with no way to scroll (everything past the twelfth line was
//!   unreachable) and drew the diff whole, its lines running past the
//!   card and the window's edge (the fleet's `shapes` session).
//! - Constraint: the box sits inside the timeline's own list, which
//!   scrolls by the same wheel; a long text must not be laid out whole on
//!   every frame (2,400 lines of 90 characters).
//! - Alternatives: one text element in a scrolling div (every line shaped
//!   each frame); the component layer's message scroller and diff view
//!   (their own look).
//! - Choice: GPUI's `uniform_list` (a mono line has one height), drawing
//!   only the lines in view, as wide as the widest line so the box
//!   scrolls sideways too; a short text gets a box of its own height. The
//!   lines are made once per version of the text (the caller's key) and
//!   kept in the element's state with the scroll handle. A vertical mask
//!   (the component layer's `ScrollableMask`) takes the wheel while the
//!   box can move and hands it to the timeline at either end, as a
//!   browser chains scrolling. An output opens at its end (GOAT's
//!   `followRef` starts true) and stays there while it grows, unless the
//!   person has scrolled up. A diff's sign is the bar's colour, not a `+`
//!   or `-` character (marks are drawn, never typed).
//! - The copy button's column: GOAT's text wraps inside `pe-7`, so its
//!   copy button has a column of its own. Here a line does not wrap but
//!   scrolls sideways, and a padding at the line's end kept clear of the
//!   button only once scrolled to that end: a long command showed on both
//!   sides of the button (the coordinator on build 0.0.0.0.8's
//!   captures). The box with a copy button keeps a column of
//!   [`GUTTER`] at its right that the lines' viewport never enters, so no
//!   text is under or beyond the button, wherever the box is used.
//! - Cost: a text is made into lines again whenever its version changes
//!   (a running command's output grows by chunks); that is linear in the
//!   text.

use std::ops::Range;
use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Axis, ElementId, HighlightStyle, Hsla, InteractiveElement as _,
    IntoElement as _, ListHorizontalSizingBehavior, ParentElement as _, ScrollStrategy,
    SharedString, Styled, StyledText, UniformListScrollHandle, Window, div, px, uniform_list,
};
use gpui_component::scroll::ScrollableMask;

use crate::chat_markdown::Look;
use crate::copy::copy_button;
use crate::fonts::Type;
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

/// The box's height by its number of lines, up to the cap (GOAT's
/// `max-h-48`, 192): 20 a line and 8 above and below.
const HEIGHTS: [f32; 10] = [
    16.0, 36.0, 56.0, 76.0, 96.0, 116.0, 136.0, 156.0, 176.0, 192.0,
];
const MAX_HEIGHT: f32 = 192.0;

/// A line's size and height.
const SIZE: f32 = 12.0;
const LINE: f32 = 20.0;

/// The box's inset.
const PAD: f32 = 8.0;

/// The column at the right of a box with a copy button that no line
/// enters (GOAT's `pe-7`): the button (22, 4 from the box's corner) and
/// 6 clear of the text.
const GUTTER: f32 = 32.0;

/// A diff's number column and its bar.
const NUMBER: f32 = 32.0;
const BAR: f32 = 2.0;

/// How strongly a diff's line is washed in its colour.
const WASH: f32 = 0.08;

mod model;
pub use model::{AnnotatedLines, Line, Lines, Start, Tone};

/// What the box keeps between frames.
struct Kept {
    version: u64,
    lines: Rc<[Line]>,
    emphasis: Rc<[Vec<Range<usize>>]>,
    copy: Option<SharedString>,
    /// The widest line, which sets the box's scrolled width.
    widest: usize,
    numbered: bool,
    handle: UniformListScrollHandle,
    /// Whether the box has been placed at its start yet.
    placed: bool,
}

impl Kept {
    fn take(&mut self, version: u64, mut made: AnnotatedLines) {
        self.version = version;
        self.widest = widest(&made.lines.lines);
        self.numbered = made.lines.lines.iter().any(|line| line.number.is_some());
        self.emphasis = made
            .lines
            .lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                let ranges = made
                    .emphasis
                    .get_mut(index)
                    .map_or_else(Vec::new, std::mem::take);
                crate::text_ranges::valid(&line.text, ranges)
            })
            .collect::<Vec<_>>()
            .into();
        self.lines = made.lines.lines.into();
        self.copy = made.lines.copy;
    }
}

/// The colours of a box.
#[derive(Debug, Clone, Copy)]
struct Paint {
    look: Look,
    added: Hsla,
    removed: Hsla,
}

/// A box of lines. `id` keys its state and must stay the same for the
/// same box across frames; `version` changes whenever its text does, and
/// only then is `make` called.
pub fn lines_box(
    id: impl Into<ElementId>,
    version: u64,
    make: impl FnOnce() -> Lines,
    start: Start,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    annotated_lines_box(id, version, || make().into(), start, tokens, window, cx)
}

/// The same output control with optional changed byte ranges. The version must
/// change when text or ranges change; colors always follow current theme tokens.
pub fn annotated_lines_box(
    id: impl Into<ElementId>,
    version: u64,
    make: impl FnOnce() -> AnnotatedLines,
    start: Start,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let id: ElementId = id.into();
    let paint = Paint {
        look: Look::of(tokens),
        added: tokens.st_ready,
        removed: tokens.neutral.destructive,
    };
    let mut make = Some(make);
    let state = window.use_keyed_state(id.clone(), cx, |_, _| {
        let mut kept = Kept {
            version,
            lines: Rc::new([]),
            emphasis: Rc::new([]),
            copy: None,
            widest: 0,
            numbered: false,
            handle: UniformListScrollHandle::new(),
            placed: false,
        };
        if let Some(make) = make.take() {
            kept.take(version, make());
        }
        kept
    });
    let (lines, emphasis, copy, widest, numbered, handle, place) = state.update(cx, |kept, _| {
        if kept.version != version
            && let Some(make) = make.take()
        {
            kept.take(version, make());
        }
        // Following: opened at the end, and kept there while it grows
        // unless the person scrolled up.
        let place = match start {
            Start::Top => false,
            Start::End => !kept.placed || kept.handle.is_scrolled_to_end() != Some(false),
        };
        kept.placed = true;
        (
            Rc::clone(&kept.lines),
            Rc::clone(&kept.emphasis),
            kept.copy.clone(),
            kept.widest,
            kept.numbered,
            kept.handle.clone(),
            place,
        )
    });
    let count = lines.len();
    if place {
        handle.scroll_to_item(count.saturating_sub(1), ScrollStrategy::Bottom);
    }
    let list = uniform_list(
        id.clone(),
        count,
        move |range: Range<usize>, _: &mut Window, _: &mut App| {
            range
                .filter_map(|index| {
                    let line = lines.get(index)?;
                    Some(row(
                        line,
                        emphasis.get(index).map_or(&[], Vec::as_slice),
                        numbered,
                        paint,
                    ))
                })
                .collect::<Vec<_>>()
        },
    )
    .with_horizontal_sizing_behavior(ListHorizontalSizingBehavior::Unconstrained)
    .with_width_from_item(Some(widest))
    .track_scroll(&handle)
    .py(px(PAD))
    .h(px(HEIGHTS.get(count).copied().unwrap_or(MAX_HEIGHT)));
    let look = paint.look;
    let corner = copy.is_some();
    // The frame's id scopes its parts' (the list, the copy button), so
    // two boxes in one card do not share them.
    div()
        .id(id.clone())
        .relative()
        .flex()
        .flex_row()
        .rounded(px(8.0))
        .border_1()
        .border_color(look.edge)
        .bg(look.fill)
        .face(Type::Mono)
        .text_size(px(crate::fonts::code(SIZE)))
        .line_height(px(crate::fonts::code(LINE)))
        .text_color(look.text.opacity(0.8))
        .overflow_hidden()
        .child(list.flex_1().min_w(px(0.0)))
        .when(corner, |frame| frame.child(div().flex_none().w(px(GUTTER))))
        .child(ScrollableMask::new(Axis::Vertical, &handle).id(id))
        .children(copy.map(|copy| {
            div()
                .absolute()
                .top(px(4.0))
                .right(px(4.0))
                .rounded(px(6.0))
                .bg(look.fill)
                .child(copy_button("copy", copy, look.copy(), window, cx))
        }))
        .into_any_element()
}

/// A command's output or a call's input: `text` in a box.
pub fn output_box(
    id: impl Into<ElementId>,
    text: &str,
    start: Start,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    lines_box(
        id,
        version_of(text),
        || Lines::text(text),
        start,
        tokens,
        window,
        cx,
    )
}

/// A version of `text` for [`lines_box`]: its hash, so that any change
/// splits it again. Its length alone was the version before, and two
/// texts of one length shown one after the other (two errors, two tool
/// outputs) left the first on screen. Hashing reads the text each frame:
/// about a tenth of a millisecond for 100 KB.
/// Typed formatting inputs may be included in a tuple: missing numbering and
/// an explicitly reported line zero must not reuse the same cached rows.
#[must_use]
pub fn version_of<T: std::hash::Hash + ?Sized>(text: &T) -> u64 {
    use std::hash::{DefaultHasher, Hasher as _};
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// One diff line drawn on its own, as a row of a long list (the code
/// panel's Changes, where a file's diff is rows among the files'): the
/// box's bar, number, wash and type, the text wrapped when `wrap` (a list
/// row cannot scroll sideways on its own).
#[must_use]
pub fn diff_line(line: &Line, wrap: bool, tokens: &Tokens) -> AnyElement {
    let paint = Paint {
        look: Look::of(tokens),
        added: tokens.st_ready,
        removed: tokens.neutral.destructive,
    };
    div()
        .w_full()
        .face(Type::Mono)
        .text_size(px(crate::fonts::code(SIZE)))
        .line_height(px(crate::fonts::code(LINE)))
        .text_color(paint.look.text.opacity(0.8))
        .child(row_of(line, &[], true, wrap, paint))
        .into_any_element()
}

/// One line: a diff's bar and number, then the text, unwrapped.
fn row(line: &Line, emphasis: &[Range<usize>], numbered: bool, paint: Paint) -> AnyElement {
    row_of(line, emphasis, numbered, false, paint)
}

fn row_of(
    line: &Line,
    emphasis: &[Range<usize>],
    numbered: bool,
    wrap: bool,
    paint: Paint,
) -> AnyElement {
    let ink = match line.tone {
        Tone::Plain => None,
        Tone::Added => Some(paint.added),
        Tone::Removed => Some(paint.removed),
    };
    let mut row = div()
        .flex()
        .flex_row()
        .pr(px(PAD))
        .when(!wrap, Styled::whitespace_nowrap);
    if let Some(ink) = ink {
        row = row.bg(ink.opacity(WASH));
    }
    if numbered {
        row = row
            .child(
                // Stretched to the row's height by the row (a percentage of
                // an unsized row would be nothing).
                div().w(px(BAR)).flex_none().when_some(ink, Styled::bg),
            )
            .child(
                div()
                    .w(px(NUMBER))
                    .flex_none()
                    .pr(px(PAD))
                    .text_right()
                    .text_color(paint.look.quiet)
                    .children(line.number.map(|number| number.to_string())),
            );
    } else {
        row = row.pl(px(PAD));
    }
    let mut text = StyledText::new(line.text.clone());
    if let Some(ink) = ink {
        let style = HighlightStyle {
            background_color: Some(ink.opacity(WASH)),
            ..HighlightStyle::default()
        };
        text = text.with_highlights(emphasis.iter().cloned().map(|range| (range, style)));
    }
    row.child(
        div()
            .when(wrap, |text| text.flex_1().min_w_0())
            .when_some(ink, Styled::text_color)
            .child(text),
    )
    .into_any_element()
}

/// Which of `lines` is the widest (in characters).
fn widest(lines: &[Line]) -> usize {
    lines
        .iter()
        .enumerate()
        .max_by_key(|(_, line)| line.text.chars().count())
        .map_or(0, |(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::{AnnotatedLines, Kept, Lines, Tone, version_of, widest};
    use gpui::UniformListScrollHandle;
    use std::rc::Rc;

    #[test]
    fn cache_validates_emphasis_against_each_line_without_changing_text_or_copy() {
        let mut kept = Kept {
            version: 0,
            lines: Rc::new([]),
            emphasis: Rc::new([]),
            copy: None,
            widest: 0,
            numbered: false,
            handle: UniformListScrollHandle::new(),
            placed: false,
        };
        kept.take(
            1,
            AnnotatedLines {
                lines: Lines::text("보드\nplain"),
                emphasis: vec![vec![1..3, 0..3, 0..99], vec![0..2, 1..4]],
            },
        );
        assert_eq!(kept.emphasis.get(0).unwrap(), &[0..3]);
        assert_eq!(kept.emphasis.get(1).unwrap(), &[0..4]);
        assert_eq!(kept.lines.get(0).unwrap().text.as_ref(), "보드");
        assert_eq!(kept.copy.as_deref(), Some("보드\nplain"));
        kept.take(2, Lines::text("바뀜").into());
        assert!(kept.emphasis.get(0).unwrap().is_empty());
        assert_eq!(kept.copy.as_deref(), Some("바뀜"));
    }

    // Two texts of one length, one after the other (two errors): each is
    // a version of its own, so the box splits the second again; the same
    // text again is the same version, so the box keeps what it split.
    #[test]
    fn texts_of_one_length_are_different_versions() {
        let first = "error: the branch is behind";
        let second = "error: the remote is behind";
        assert_eq!(first.len(), second.len());
        assert_ne!(version_of(first), version_of(second));
        assert_eq!(version_of(first), version_of(first));
    }

    #[test]
    fn a_text_keeps_its_blank_lines_and_copies_whole() {
        let lines = Lines::text("first\n\nthird");
        let texts: Vec<&str> = lines.lines.iter().map(|line| line.text.as_ref()).collect();
        assert_eq!(texts, ["first", "", "third"]);
        assert!(lines.lines.iter().all(|line| line.tone == Tone::Plain));
        assert_eq!(lines.copy.as_deref(), Some("first\n\nthird"));
    }

    // The box's scrolled width is the widest line's: counted in
    // characters, so a Korean line is not taken for wider than it looks.
    #[test]
    fn the_widest_line_is_counted_in_characters() {
        let lines = Lines::text("가나다라마바\nabcdefghij\nabc");
        assert_eq!(widest(&lines.lines), 1);
        assert_eq!(widest(&[]), 0);
    }

    #[test]
    fn typed_numbering_inputs_invalidate_equal_text() {
        let text = "same body";
        assert_ne!(
            version_of(&(text, None::<u64>)),
            version_of(&(text, Some(0_u64)))
        );
    }
}
