//! Parts of the window drawn as GPUI views of their own, kept from one
//! frame to the next while nothing they show has changed.
//!
//! [Decision Log]
//! - Purpose: a focused field's caret blinks twice a second; each blink
//!   notifies the field's state, a GPUI view, and GPUI marks that view and
//!   every view above it to be drawn again. The shell is the only view
//!   above it, and everything else on screen is the shell's elements, so
//!   each blink rendered, laid out and painted the whole window (track
//!   S3's measure, 2026-11-14: 28 % of a core on a software renderer with
//!   the caret blinking, 0.5 % with no field focused).
//! - Constraint: GPUI reuses a view's last frame only for a view drawn
//!   with `cached` whose own entity was not notified (`view.rs`,
//!   `prepaint_view`); the shell keeps all the app's state, so a part
//!   drawn as its own view must be drawn again whenever the shell changes.
//! - Alternatives: splitting the state out of the shell into per-area
//!   entities (each area's events, listeners and tests rewritten); a
//!   caret that does not blink (already so while motion is held, ADR 0057
//!   and the gpui-base patch; the owner wants it to blink otherwise).
//! - Choice: a `Region` view per heavy part (the sidebar, the
//!   conversation's timeline) that draws through the shell's own
//!   function and observes the shell, so any shell change draws it again
//!   and a blink in a field outside it does not. A field drawn inside a
//!   region still draws that region again, as its view sits under it.
//! - Rule for a part drawn in a region: what it shows must come from the
//!   shell or from views drawn inside it. A model entity read there and
//!   notified on its own would leave the region showing old content
//!   (GPUI dirties views, not readers of models); the app has none today.
//! - Cost: one entity and one observation per region; a region's box is
//!   its parent's (`size_full`), as a cached view is not measured from
//!   its content.

use std::time::Instant;

use gpui::AppContext as _;
use vessel_app_kit::Tokens;
use vessel_app_kit::tokens;
use vessel_app_kit::ui::{
    AnyElement, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    Subscription, WeakEntity, Window, div,
};

use crate::shell::Shell;

/// How a region draws: one of the shell's area functions.
pub(crate) type Draw = fn(&Shell, &Tokens, &mut Window, &mut Context<'_, Shell>) -> AnyElement;

/// A part of the window drawn as its own view.
pub(crate) struct Region {
    shell: WeakEntity<Shell>,
    draw: Draw,
    /// Draws it again whenever the shell changes.
    _shell_changed: Subscription,
}

impl Render for Region {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let draw = self.draw;
        self.shell
            .update(cx, |shell, cx| draw(shell, &tokens(cx), window, cx))
            .unwrap_or_else(|_gone| div().into_any_element())
    }
}

/// The shell's regions, made on its first frame (the shell's own entity is
/// needed to observe it).
#[derive(Default)]
pub(crate) struct Regions {
    /// What stands in the sidebar's place on a wide window.
    pub(crate) sidebar: Option<Entity<Region>>,
    /// The open session's conversation.
    pub(crate) timeline: Option<Entity<Region>>,
}

/// Make the regions on the first frame, and say when the frame began (for
/// the draw probe, `vessel_app_kit::frame_log::drawn`).
pub(crate) fn begin(shell: &mut Shell, cx: &mut Context<'_, Shell>) -> Option<Instant> {
    let started = vessel_app_kit::frame_log::started();
    if shell.areas.regions.sidebar.is_none() {
        let me = cx.entity();
        let make = |draw: Draw, cx: &mut Context<'_, Shell>| {
            cx.new(|cx| Region {
                shell: me.downgrade(),
                draw,
                _shell_changed: cx.observe(&me, |_region, _shell, cx| cx.notify()),
            })
        };
        shell.areas.regions.sidebar = Some(make(crate::routes::sidebar_inner, cx));
        shell.areas.regions.timeline = Some(make(timeline_region, cx));
    }
    started
}

/// `region`, filling its parent's box and kept between frames; before the
/// first frame made it, drawn directly by `draw`.
pub(crate) fn place(
    region: Option<&Entity<Region>>,
    draw: Draw,
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    match region {
        Some(region) => vessel_app_kit::cached_view(region),
        None => draw(shell, tokens, window, cx),
    }
}

/// The open session's conversation, kept between frames: a blink in the
/// composer under it does not draw it again.
pub(crate) fn timeline(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let region = shell.areas.regions.timeline.as_ref();
    div()
        .flex_1()
        .min_h_0()
        .child(place(region, timeline_region, shell, tokens, window, cx))
        .into_any_element()
}

/// The timeline as its region draws it.
fn timeline_region(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    crate::areas::conversation::timeline(shell, tokens, window, cx)
}
