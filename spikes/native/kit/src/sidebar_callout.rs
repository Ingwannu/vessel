//! One sidebar footer strip after Paseo's `SidebarCallout`.
//!
//! [Decision Log]
//! - Purpose: setup and desktop notices share one bounded action presentation.
//! - Constraint: facts, admission, priority and dismissal persistence belong
//!   to callers; this kit must never infer a notice from a node/platform.
//! - Choice: the source's top-border-only strip and at most two private action
//!   buttons. Those buttons are not a second generic Button API.
//! - Mapping: Paseo mounts its viewport in the web slot and returns null from
//!   its native slot; showing this strip in the GPUI app is an explicit choice.

use crate::{
    ButtonState, Tokens, Type, TypeExt as _,
    assets::Glyph,
    glyph,
    theme::{radius, spacing},
};
use gpui::prelude::*;
use gpui::{AnyElement, Context, Div, FontWeight, Hsla, SharedString, Window, div, px};
use std::rc::Rc;

/// The source's variants. Success intentionally keeps the default container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// A normal informational strip.
    Default,
    /// A successful operation, with the same neutral strip.
    Success,
    /// Only the top border becomes destructive.
    Error,
}
/// These action paints belong only to this callout, not generic buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    /// Foreground fill, surface0 label.
    Primary,
    /// Transparent fill with the normal border.
    Secondary,
}
/// A real open UI callback boundary; application state never enters the kit.
pub type OnPress<V> = Rc<dyn Fn(&mut V, &mut Window, &mut Context<'_, V>)>;

/// One private callout action; its handler is the supplied working action.
pub struct Action<V> {
    label: SharedString,
    kind: ActionKind,
    state: ButtonState,
    press: OnPress<V>,
}
impl<V> std::fmt::Debug for Action<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Action")
            .field("label", &self.label)
            .field("kind", &self.kind)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}
impl<V> Action<V> {
    /// Create an enabled action with its whole translated label.
    #[must_use]
    pub fn new(label: impl Into<SharedString>, kind: ActionKind, press: OnPress<V>) -> Self {
        Self {
            label: label.into(),
            kind,
            state: ButtonState::Enabled,
            press,
        }
    }
    /// A disabled action keeps source opacity and never invokes its handler.
    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.state = ButtonState::Disabled;
        self
    }
}
/// The source admits at most two actions; there is no silently clipped Vec.
pub enum Actions<V> {
    /// No action row.
    None,
    /// One full-width action.
    One(Action<V>),
    /// Two equally sized actions.
    Two(Action<V>, Action<V>),
}
/// A dismiss affordance exists only with a real caller-owned dismissal.
pub struct Dismiss<V> {
    /// Updates caller state/persistence.
    pub press: OnPress<V>,
}
/// Plain descriptions skip empty strings; rich content keeps its source slot.
pub enum Description {
    /// One whole translated description.
    Text(SharedString),
    /// Caller-prepared styled content; no data inspection.
    Content(AnyElement),
}
/// Presentation only. The caller selects the one admitted active callout.
pub struct Content<V> {
    /// Optional normal-weight title, at most two lines.
    pub title: Option<SharedString>,
    /// Optional wrapping explanation.
    pub description: Option<Description>,
    /// Caller-prepared shared glyph/content.
    pub icon: Option<AnyElement>,
    /// Zero, one or two private actions.
    pub actions: Actions<V>,
    /// Absent for nondismissible notices.
    pub dismiss: Option<Dismiss<V>>,
}

// UI callbacks and erased elements intentionally do not require Debug on
// the caller's entire application state or attempt to inspect element data.
impl<V> std::fmt::Debug for Actions<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::One(action) => f.debug_tuple("One").field(action).finish(),
            Self::Two(first, second) => f.debug_tuple("Two").field(first).field(second).finish(),
        }
    }
}
impl<V> std::fmt::Debug for Dismiss<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dismiss").finish_non_exhaustive()
    }
}
impl std::fmt::Debug for Description {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(text) => f.debug_tuple("Text").field(text).finish(),
            Self::Content(_) => f.debug_struct("Content").finish_non_exhaustive(),
        }
    }
}
impl<V> std::fmt::Debug for Content<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Content")
            .field("title", &self.title)
            .field("description", &self.description)
            .field("actions", &self.actions)
            .field("dismiss", &self.dismiss)
            .finish_non_exhaustive()
    }
}

/// Draw one strip, with natural height and no section height budget.
#[must_use]
pub fn sidebar_callout<V: 'static>(
    id: impl Into<SharedString>,
    content: Content<V>,
    variant: Variant,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> Div {
    let id = id.into();
    let mut body = div().flex().flex_col().gap(px(spacing::S2));
    if let Some(header) = header(
        &id,
        content.title,
        content.icon,
        content.dismiss,
        tokens,
        cx,
    ) {
        body = body.child(header);
    }
    if let Some(description) = description(content.description, tokens) {
        body = body.child(description);
    }
    if let Some(actions) = action_row(&id, content.actions, tokens, cx) {
        body = body.child(actions);
    }
    div()
        .flex()
        .flex_col()
        .w_full()
        .min_w_0()
        .px(px(spacing::S4))
        .py(px(spacing::S3))
        .border_t_1()
        .border_color(border(variant, tokens))
        .gap(px(spacing::S2))
        .child(body)
}
fn header<V: 'static>(
    id: &SharedString,
    title: Option<SharedString>,
    icon: Option<AnyElement>,
    dismiss: Option<Dismiss<V>>,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> Option<Div> {
    if title.is_none() && icon.is_none() && dismiss.is_none() {
        return None;
    }
    let mut top = div().flex().items_start().gap(px(spacing::S2));
    if title.is_some() || icon.is_some() {
        let mut leading = div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(spacing::S2));
        if let Some(icon) = icon {
            leading = leading.child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon),
            );
        }
        if let Some(title) = title {
            leading = leading.child(
                div()
                    .min_w_0()
                    .flex_shrink_1()
                    .type_(Type::Body)
                    .font_weight(FontWeight::NORMAL)
                    .text_color(tokens.neutral.foreground)
                    .line_clamp(2)
                    .text_ellipsis()
                    .child(title),
            );
        }
        top = top.child(leading);
    }
    if let Some(dismiss) = dismiss {
        top = top.child(dismiss_button(
            format!("{id}-dismiss").into(),
            dismiss,
            tokens,
            cx,
        ));
    }
    Some(top)
}
fn action_row<V: 'static>(
    id: &SharedString,
    actions: Actions<V>,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> Option<Div> {
    let row = div().flex().gap(px(spacing::S2)).mt(px(spacing::S2));
    match actions {
        Actions::None => None,
        Actions::One(action) => Some(row.child(action_button(
            format!("{id}-first").into(),
            action,
            tokens,
            cx,
        ))),
        Actions::Two(first, second) => Some(
            row.child(action_button(
                format!("{id}-first").into(),
                first,
                tokens,
                cx,
            ))
            .child(action_button(
                format!("{id}-second").into(),
                second,
                tokens,
                cx,
            )),
        ),
    }
}

fn description(content: Option<Description>, tokens: &Tokens) -> Option<Div> {
    match content {
        None => None,
        Some(Description::Text(text)) if text.is_empty() => None,
        Some(Description::Text(text)) => Some(
            div()
                .min_w_0()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .whitespace_normal()
                .child(text),
        ),
        Some(Description::Content(content)) => Some(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(spacing::S2))
                .child(content),
        ),
    }
}
const DISMISS_SIZE: f32 = 14.0;
const HIT_OUT: f32 = -8.0;
fn dismiss_button<V: 'static>(
    id: SharedString,
    dismiss: Dismiss<V>,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> Div {
    let press = dismiss.press;
    let group = id.clone();
    let icon = glyph(Glyph::Close, DISMISS_SIZE, tokens.neutral.foreground_muted)
        .group_hover(group.clone(), |style| {
            style.text_color(tokens.neutral.foreground)
        });
    let hit = div()
        .id(id)
        .group(group)
        .absolute()
        .inset(px(HIT_OUT))
        .cursor_pointer()
        .on_click(cx.listener(move |this, _, window, cx| press(this, window, cx)));
    // The current GPUI host has no semantic-label bridge; no complete
    // platform accessibility is claimed by this pointer affordance.
    div()
        .relative()
        .flex_none()
        .size(px(DISMISS_SIZE))
        .child(icon)
        .child(hit)
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct ActionPaint {
    fill: Hsla,
    border: Hsla,
    ink: Hsla,
}
fn action_paint(kind: ActionKind, tokens: &Tokens) -> ActionPaint {
    match kind {
        ActionKind::Primary => ActionPaint {
            fill: tokens.neutral.foreground,
            border: tokens.neutral.foreground,
            ink: tokens.neutral.surface0,
        },
        ActionKind::Secondary => ActionPaint {
            fill: gpui::transparent_black(),
            border: tokens.neutral.border,
            ink: tokens.neutral.foreground,
        },
    }
}
fn border(variant: Variant, tokens: &Tokens) -> Hsla {
    match variant {
        Variant::Default | Variant::Success => tokens.neutral.border,
        Variant::Error => tokens.neutral.destructive,
    }
}
fn action_button<V: 'static>(
    id: SharedString,
    action: Action<V>,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> gpui::Stateful<Div> {
    let paint = action_paint(action.kind, tokens);
    let enabled = action.state == ButtonState::Enabled;
    let press = Rc::clone(&action.press);
    div()
        .id(id)
        .flex_1()
        .min_w_0()
        .py(px(spacing::S2))
        .px(px(spacing::S3))
        .rounded(px(radius::MD))
        .border_1()
        .border_color(paint.border)
        .bg(paint.fill)
        .flex()
        .items_center()
        .justify_center()
        .type_(Type::Meta)
        .font_weight(FontWeight::MEDIUM)
        .text_color(paint.ink)
        .when(enabled, |button| {
            button
                .cursor_pointer()
                .active(|style| style.opacity(0.8))
                .on_click(cx.listener(move |this, _, window, cx| press(this, window, cx)))
        })
        .when(!enabled, |button| button.opacity(0.5))
        .child(div().min_w_0().truncate().child(action.label))
}

#[cfg(test)]
mod tests;
