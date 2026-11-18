//! The session's retained legacy body, which `session::render` frames
//! through the shared pane adapter for the visible selected agent.
//!
//! [Decision Log]
//! - Purpose: W12's lazy body split `render` into a frame and this body,
//!   and `session.rs` then passed its listed size (720 lines; the size list
//!   only shrinks, budget 500).
//! - Alternatives: raising the listed size (the list does not grow); moving
//!   an unrelated helper (this body is the part that is new).
//! - Choice: the body moves here unchanged, a child module calling the
//!   session's private helpers through `super`.
//! - Cost: one more file; no behaviour change.
//! - Since TRACKER 789 the bar is drawn only above the compact width.

use vessel_app_kit::Tokens;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, ParentElement as _, Styled as _, Window, div,
};

use super::{column, composer, resume_note};
use crate::areas::conversation;
use crate::shell::Shell;
use vessel_app_view::layout::LayoutMode;

// Retained source body, invoked only for the visible selected legacy agent.
// This still reads the existing owner; it grants no scoped resident authority.
pub(super) fn render_body(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let t = shell.t;
    let view = &shell.areas.session;
    let Some(open) = &view.open else {
        return div();
    };
    let detail = format!("{} · {}", open.machine, open.agent);
    let body: AnyElement = if open.structured {
        crate::region::timeline(shell, tokens, window, cx)
    } else {
        crate::areas::terminal::body(shell, tokens, window, cx)
    };
    let working = view.conversation.turn_running();
    // Below the compact width the compact workspace header (TRACKER 789,
    // `layout::compact`) shows the bar's title, state and actions, so the
    // bar is not drawn twice.
    let bar = (crate::areas::layout::mode(window) != LayoutMode::Compact).then(|| {
        conversation::bar(
            shell,
            t.title(&open.title).to_owned(),
            detail,
            tokens,
            window,
            cx,
        )
    });
    div()
        .size_full()
        .flex()
        .flex_col()
        .children(bar)
        .child(body)
        .children(resume_note(shell, open, tokens))
        .children(crate::areas::sessions::lifecycle_error(shell, tokens))
        .when(open.structured, |frame| {
            // The conversation's slot over the card (`conversation::dock`:
            // the queue), at the card's width.
            let dock = conversation::dock(shell, tokens, window, cx);
            // An ended session its agent cannot pick up again has no
            // composer: the banner in the dock says so (Paseo's archived
            // agent shows only its callout); a send would only try to
            // start it again.
            let ended_for_good = shell.conversation_cannot_resume();
            // The draft's pills on the session's own options (Paseo's
            // AgentControls on a running agent).
            let touch = crate::areas::draft::compact(window);
            let (controls, over) =
                crate::areas::draft::running_controls(shell, touch, tokens, window, cx).unzip();
            frame
                .children(dock.map(column))
                .when(!ended_for_good, |frame| {
                    frame.child(composer(shell, tokens, working, controls, window, cx))
                })
                .children(over.into_iter().flatten())
        })
        .children(conversation::overlays(shell, tokens, window, cx))
        .children(crate::areas::sessions::lifecycle_overlay(
            shell, tokens, window, cx,
        ))
}
