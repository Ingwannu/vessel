//! A page arriving (the owner's word 39: a short transition on every change
//! a person causes, none with Animations off).
//!
//! [Decision Log]
//! - Purpose: going to another page (the board, Settings, a session) is a
//!   change the person caused, so the new page fades in instead of
//!   replacing the old one in one frame.
//! - Constraint: GPUI keeps an animation's progress per element id and
//!   starts it again when the id is new, and draws its end at once while
//!   motion is held (`app::motion`: the system, a software renderer, the
//!   Animations setting).
//! - Choice: the frame is wrapped in one box whose id names the page (the
//!   route, and the session shown), faded over the kit's surface length.
//!   The page it replaces goes at once, as Paseo's routes do on the
//!   desktop (no exiting animation); a section inside Settings is the same
//!   page (Paseo's settings sections switch in place).
//! - Rejected: a fade on each area's own root (every area would carry the
//!   same lines); a cross-fade (two pages drawn and laid out for the
//!   length of the fade, for a change no reference makes).
//! - Cost: one box around the frame; it must keep the frame's flex share
//!   (`flex_1`, `min_h(0)`) or the frame would size to its content.

use gpui::AnimationExt as _;
use vessel_app_kit::motion::{STANDARD, transition};
use vessel_app_kit::ui::{
    AnyElement, ElementId, IntoElement, ParentElement as _, SharedString, Styled as _, div,
};

use super::Shell;
use crate::routes::Route;

/// The frame, fading in when the page is another one.
pub(super) fn arriving(shell: &Shell, frame: AnyElement) -> impl IntoElement {
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(frame)
        .with_animation(
            ElementId::Name(key(shell)),
            transition(STANDARD),
            vessel_app_kit::ui::Styled::opacity,
        )
}

/// What makes a page another page: the route, and the session shown.
fn key(shell: &Shell) -> SharedString {
    let session = shell
        .areas
        .session
        .open
        .as_ref()
        .filter(|_| shell.route == Route::Session)
        .map(|open| format!("{}/{}", open.machine, open.session));
    format!("page-{:?}-{}", shell.route, session.unwrap_or_default()).into()
}
