//! The session's title and the line under it, for the compact workspace
//! header (TRACKER 789): what the bar shows, as the title and subtitle of
//! the kit's compact header frame.
//!
//! [Decision Log]
//! - Purpose: below the compact width the bar is not drawn; the compact
//!   header (`layout::compact`) takes its place and needs the bar's words.
//! - Reference: Paseo's `CompactHeader`: the title, one line, over a
//!   subtitle line, both as wide as the row leaves them.
//! - Choice: the bar's own pieces (`status_badge`, `parked_chip`,
//!   `lost_line`, `rename_field`), so the state, the released chip, the lost
//!   line and a refused rename read as on the bar; the kit frame draws the
//!   title. The title is not a button here: Paseo's compact title is not,
//!   and Rename is in the tab switcher's row menu. A rename under way puts
//!   its field in the title's place, and the line is left out meanwhile
//!   (the 44 touch field fills the row; the capture pass found them
//!   overlapping).
//! - Cost: the line can hold less than the bar's row; every piece truncates.

use vessel_app_kit::theme::spacing::{S0, S1_5};
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::session::{can_rename, park_state, session_name};
use vessel_proto::SessionStatus;

use super::bar::{lost_line, parked_chip, rename_field, status_badge};
use crate::shell::Shell;

/// What the compact header shows of the session.
pub(crate) struct Heading {
    /// The title as the bar shows it.
    pub(crate) title: String,
    /// The line under it: the state, where it runs, a lost connection, a
    /// refused rename.
    pub(crate) line: AnyElement,
    /// The rename's field, while the title is renamed.
    pub(crate) editing: Option<AnyElement>,
}

/// The session's heading; `None` without an open session.
pub(crate) fn heading(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<Heading> {
    let open = shell.areas.session.open.as_ref()?;
    let area = &shell.areas.conversation;
    let words = shell.t.lang.conversation();
    let status = area
        .info
        .as_ref()
        .map(|info| &info.status)
        .filter(|status| !matches!(status, SessionStatus::Running));
    let parked = area
        .info
        .as_deref()
        .and_then(park_state)
        .map(|state| parked_chip(shell, &state, tokens, true));
    let badge = status
        .filter(|_| parked.is_none())
        .map(|status| status_badge(status, *words, tokens));
    let refused = area.refused.as_ref().map(|(_, why)| {
        div()
            .min_w(px(S0))
            .truncate()
            .type_(Type::Meta)
            .text_color(tokens.neutral.destructive)
            .child(why.clone())
    });
    let line = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(S1_5))
        .min_w(px(S0))
        .children(badge)
        .children(parked)
        .child(
            div()
                .min_w(px(S0))
                .truncate()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_extra_muted)
                .child(format!("{} · {}", open.machine, open.agent)),
        )
        .children(lost_line(shell, tokens, true))
        .children(refused)
        .into_any_element();
    Some(Heading {
        title: shown_title(shell),
        line,
        editing: rename_field(shell, tokens, true, cx),
    })
}

/// The title as the bar shows it: the node's answer to a rename first, then
/// the session's own report, then the frame's title.
fn shown_title(shell: &Shell) -> String {
    let area = &shell.areas.conversation;
    let fallback = shell
        .areas
        .session
        .open
        .as_ref()
        .map(|open| shell.t.title(&open.title).to_owned())
        .unwrap_or_default();
    area.info
        .as_deref()
        .and_then(|info| session_name(info, area.timeline.title()))
        .unwrap_or(fallback)
}

impl Shell {
    /// The tab switcher row menu's Rename: the header's title becomes the
    /// field, as a press on the bar's title does.
    pub(crate) fn rename_from_header(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let renamable = self
            .areas
            .conversation
            .info
            .as_deref()
            .is_some_and(can_rename);
        if renamable {
            let shown = shown_title(self);
            self.start_rename(&shown, window, cx);
        }
    }
}
