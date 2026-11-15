//! A file attached to a message (Paseo's `components/attachment-pill.tsx`,
//! value for value): an image as a 48 by 48 picture, any other file as a
//! two-line label (its name over its kind) beside its icon; in a composer
//! with a round remove button at its top left, in a sent message without.
//!
//! [Decision Log]
//! - Purpose: the composer's attached files (track P's row c) and a sent
//!   or queued message's files (track C) drawn alike, as Paseo draws both
//!   from one pill.
//! - Followed: Paseo's `AttachmentPill`, `AttachmentFrame`,
//!   `AttachmentLabel` and `AttachmentThumbnail`: every body 48 high so a
//!   tray of images and labels lines up; a label at most 260 wide, 12
//!   across and 8 between its icon column (18) and its text, on `surface1`;
//!   a one-point `borderAccent` frame at radius `md` (6); the remove button
//!   24 across, 8 outside the top left corner, on `surface2` with a
//!   `border` edge and a 12-point X, shown on hover on a desktop and always
//!   on touch; the tray a wrapping row 8 apart.
//! - Vessel's own: a file on its way to the node shows its progress as a
//!   2-point accent bar along the body's foot, and a file refused or
//!   failed has a `destructive` frame (Paseo sends a file with its message
//!   in one call, so it has neither); the screen says why in the label's
//!   second line.
//! - Cost: the remove button is only faded out on a desktop (GPUI's group
//!   hover sets styles, not hit testing), so a click on the pill's corner
//!   while the pointer is elsewhere cannot happen: the pointer is there.

use gpui::prelude::*;
use gpui::{
    AnyElement, ClickEvent, Context, Div, ElementId, ImageSource, ObjectFit, SharedString, Window,
    div, img, px, relative,
};

use crate::assets::Glyph;
use crate::fonts::Type;
use crate::picker::glyph;
use crate::popup::{ROW_RADIUS, Surfaces};
use crate::theme::{Tokens, spacing};
use crate::widgets::TypeExt as _;

/// Every body's height, so a tray of images and labels lines up (Paseo's
/// `ATTACHMENT_CONTENT_HEIGHT`; the spacing scale's 48).
const BODY: f32 = spacing::S12;

/// The widest a label is (Paseo's `labelBody.maxWidth`; no token is 260).
const LABEL_MAX: f32 = 260.0;

/// The label's icon column (Paseo's `labelIcon.width`; no token is 18).
const ICON_COLUMN: f32 = 18.0;

/// The label's icon (Paseo's `ICON_SIZE.sm`).
const ICON: f32 = 14.0;

/// The remove button, across (Paseo's `closeButton`; the spacing scale's 24).
const REMOVE: f32 = spacing::S6;

/// How far the remove button sits outside the top left corner (Paseo's
/// `top: -8, left: -8`; a negative, which no token is).
const REMOVE_OUTSET: f32 = -8.0;

/// The remove button's X (Paseo's `X size={12}`).
const REMOVE_GLYPH: f32 = 12.0;

/// The progress bar's height along the body's foot (Vessel's own).
const PROGRESS: f32 = 2.0;

/// What a pill shows (not `Debug`: GPUI's `ImageSource` is not).
#[derive(Clone)]
pub enum Body {
    /// An image: its picture, cropped to fill the square; `None` while it
    /// is not read yet (Paseo's placeholder, `surface1`).
    Image(Option<ImageSource>),
    /// Any other file: its icon, its name and a second line (its kind, or
    /// why it cannot go).
    Label {
        /// The icon.
        icon: Glyph,
        /// The file's name.
        title: SharedString,
        /// Its kind, or what is wrong.
        subtitle: SharedString,
    },
}

impl std::fmt::Debug for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Image(source) => f
                .debug_struct("Image")
                .field("read", &source.is_some())
                .finish(),
            Self::Label {
                icon,
                title,
                subtitle,
            } => f
                .debug_struct("Label")
                .field("icon", icon)
                .field("title", title)
                .field("subtitle", subtitle)
                .finish(),
        }
    }
}

/// Where a file is, as its pill shows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Standing {
    /// Attached, or sent.
    Quiet,
    /// On its way: how much is in, from 0 to 1.
    Sending(f32),
    /// Refused or failed (the screen says why).
    Trouble,
}

/// What a click on a pill does to its screen, given the pill's index in
/// its tray.
pub type PillClick<V> = fn(&mut V, usize, &mut Window, &mut Context<'_, V>);

/// What a pill in a composer does: a click on it (`None`: nothing, as
/// Paseo's file pill), a click on its remove button.
pub struct PillHandlers<V> {
    /// A click on the pill.
    pub on_open: Option<PillClick<V>>,
    /// A click on its remove button.
    pub on_remove: PillClick<V>,
}

impl<V> std::fmt::Debug for PillHandlers<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PillHandlers")
            .field("opens", &self.on_open.is_some())
            .finish_non_exhaustive()
    }
}

/// The tray the pills sit in: a row 8 apart that wraps (Paseo's
/// `attachmentTray`).
#[must_use]
pub fn attachment_tray() -> Div {
    div().flex().flex_row().flex_wrap().gap(px(spacing::S2))
}

/// A composer's pill: the body in its frame, the remove button at its top
/// left (always on `touch`, on hover otherwise).
#[expect(
    clippy::too_many_arguments,
    reason = "a pill is drawn from its place, its body, where it stands, what clicks do and how it shows"
)]
pub fn attachment_pill<V: 'static>(
    id: impl Into<SharedString>,
    index: usize,
    body: &Body,
    standing: Standing,
    handlers: &PillHandlers<V>,
    touch: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    attachment_pill_scoped(
        id,
        index,
        body,
        standing,
        handlers.on_open.map(|callback| {
            let callback: ScopedPillClick<V> = std::rc::Rc::new(callback);
            callback
        }),
        std::rc::Rc::new(handlers.on_remove),
        touch,
        tokens,
        cx,
    )
}

/// A retained callback captures its exact pane or draft address; it never reads
/// whichever composer happens to be selected when a delayed click arrives.
pub type ScopedPillClick<V> = std::rc::Rc<dyn Fn(&mut V, usize, &mut Window, &mut Context<'_, V>)>;

/// Same pill and renderer, with explicitly scoped callback ownership.
#[expect(
    clippy::too_many_arguments,
    reason = "same visual inputs as attachment_pill plus its two callbacks"
)]
pub fn attachment_pill_scoped<V: 'static>(
    id: impl Into<SharedString>,
    index: usize,
    body: &Body,
    standing: Standing,
    open: Option<ScopedPillClick<V>>,
    remove: ScopedPillClick<V>,
    touch: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let id: SharedString = id.into();
    let number = u64::try_from(index).unwrap_or(u64::MAX);
    let group: SharedString = format!("{id}-{number}").into();
    let frame = frame(body, standing, tokens)
        .id(ElementId::NamedInteger(id.clone(), number))
        .when_some(open, |frame, open| {
            frame.cursor_pointer().on_click(cx.listener(
                move |view, _event: &ClickEvent, window, cx| open(view, index, window, cx),
            ))
        });
    let surfaces = Surfaces::of(tokens);
    let button = div()
        .id(ElementId::NamedInteger(
            format!("{id}-remove").into(),
            number,
        ))
        .absolute()
        .top(px(REMOVE_OUTSET))
        .left(px(REMOVE_OUTSET))
        .size(px(REMOVE))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(surfaces.surface2)
        .border_1()
        .border_color(tokens.neutral.border)
        .cursor_pointer()
        .child(glyph(
            Glyph::Remove,
            REMOVE_GLYPH,
            tokens.neutral.foreground_muted,
        ))
        .on_click(cx.listener(move |view, _event: &ClickEvent, window, cx| {
            cx.stop_propagation();
            remove(view, index, window, cx);
        }));
    let button = if touch {
        button
    } else {
        button
            .opacity(0.0)
            .group_hover(group.clone(), |style| style.opacity(1.0))
    };
    div()
        .group(group)
        .relative()
        .child(frame)
        .child(button)
        .into_any_element()
}

/// A sent or queued message's pill: the body in its frame, nothing to
/// remove (Paseo's `AttachmentFrame`).
#[must_use]
pub fn attachment_frame(body: &Body, tokens: &Tokens) -> Div {
    frame(body, Standing::Quiet, tokens)
}

/// The body in its frame, with where it stands.
fn frame(body: &Body, standing: Standing, tokens: &Tokens) -> Div {
    let surfaces = Surfaces::of(tokens);
    let edge = match standing {
        Standing::Trouble => tokens.neutral.destructive,
        Standing::Quiet | Standing::Sending(_) => surfaces.border_accent,
    };
    let progress = match standing {
        Standing::Sending(done) => Some(done.clamp(0.0, 1.0)),
        Standing::Quiet | Standing::Trouble => None,
    };
    div()
        .relative()
        .rounded(px(ROW_RADIUS))
        .border_1()
        .border_color(edge)
        .overflow_hidden()
        .child(body_of(body, tokens))
        .children(progress.map(|done| {
            div()
                .absolute()
                .left_0()
                .bottom_0()
                .h(px(PROGRESS))
                .w(relative(done))
                .bg(tokens.neutral.accent)
        }))
}

fn body_of(body: &Body, tokens: &Tokens) -> AnyElement {
    let neutral = tokens.neutral;
    let surfaces = Surfaces::of(tokens);
    match body {
        Body::Image(Some(source)) => img(source.clone())
            .size(px(BODY))
            .object_fit(ObjectFit::Cover)
            .into_any_element(),
        Body::Image(None) => div()
            .size(px(BODY))
            .bg(surfaces.surface1)
            .into_any_element(),
        Body::Label {
            icon,
            title,
            subtitle,
        } => div()
            .h(px(BODY))
            .max_w(px(LABEL_MAX))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::S2))
            .px(px(spacing::S3))
            .bg(surfaces.surface1)
            .child(
                div()
                    .w(px(ICON_COLUMN))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(glyph(*icon, ICON, neutral.foreground_muted)),
            )
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .type_(Type::Body)
                            .text_color(neutral.foreground)
                            .truncate()
                            .child(title.clone()),
                    )
                    .child(
                        div()
                            .type_(Type::Meta)
                            .text_color(neutral.foreground_muted)
                            .truncate()
                            .child(subtitle.clone()),
                    ),
            )
            .into_any_element(),
    }
}
