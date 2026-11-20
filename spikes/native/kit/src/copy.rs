//! A copy button: the copy glyph, a check for a moment once its text is
//! on the clipboard (GOAT's `MorphIcon` between `Copy` and `Check`, an
//! `icon-xs` ghost button). A code block's corner, a tool's output.
//!
//! Which text was copied last is one app-wide value: copying a second
//! thing takes the check from the first, as only one thing is on the
//! clipboard anyway, and no screen keeps state for it.

use std::hash::{DefaultHasher, Hash as _, Hasher as _};
use std::time::{Duration, Instant};

use gpui::{
    App, ClipboardItem, Div, ElementId, Global, Hsla, InteractiveElement as _, ParentElement as _,
    SharedString, Stateful, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};

use crate::assets::Glyph;
use crate::picker::glyph;

/// How long a copied thing shows its check.
const COPIED_FOR: Duration = Duration::from_millis(1500);

/// The button's side and its glyph's (GOAT's `icon-xs` and `size-3`).
const SIDE: f32 = 22.0;
const GLYPH: f32 = 12.0;

/// The text a person copied last, and when.
#[derive(Debug, Default)]
struct Copied(Option<(u64, Instant)>);

impl Global for Copied {}

/// The button's colours.
#[derive(Debug, Clone, Copy)]
pub struct CopyLook {
    /// The glyph.
    pub glyph: Hsla,
    /// The button under the pointer.
    pub hover: Hsla,
}

/// A button that puts `text` on the clipboard.
#[must_use]
pub fn copy_button(
    id: impl Into<ElementId>,
    text: SharedString,
    look: CopyLook,
    // Kept for its callers: the click reads the window it lands in.
    _window: &Window,
    cx: &App,
) -> Stateful<Div> {
    let copied = copied_within(&text, COPIED_FOR, cx);
    div()
        .id(id)
        .size(px(SIDE))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(move |style| style.bg(look.hover))
        .child(glyph(
            if copied { Glyph::Check } else { Glyph::Copy },
            GLYPH,
            look.glyph,
        ))
        .on_click(move |_, window, cx| {
            cx.stop_propagation();
            copy_for(&text, COPIED_FOR, window, cx);
        })
}

/// Whether `text` is what was copied last, less than `moment` ago (a
/// copy control's check).
#[must_use]
pub fn copied_within(text: &str, moment: Duration, cx: &App) -> bool {
    let key = key_of(text);
    cx.try_global::<Copied>()
        .and_then(|copied| copied.0)
        .is_some_and(|(copied, at)| copied == key && at.elapsed() < moment)
}

/// Put `text` on the clipboard as the thing copied last, the window drawn
/// again once `moment` is over (Paseo's hover card's copy rows keep their
/// check 2 s, this button's 1.5 s).
pub fn copy_for(text: &str, moment: Duration, window: &Window, cx: &mut App) {
    cx.write_to_clipboard(ClipboardItem::new_string(text.to_owned()));
    cx.set_global(Copied(Some((key_of(text), Instant::now()))));
    // The check goes back to the copy glyph once its moment is over;
    // nothing else would draw the window again.
    let handle = window.window_handle();
    cx.spawn(async move |cx| {
        cx.background_executor().timer(moment).await;
        let _closed = handle.update(cx, |_, window, _| window.refresh());
    })
    .detach();
}

/// A text's key, to know what was copied.
fn key_of(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}
