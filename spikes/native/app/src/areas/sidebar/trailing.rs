//! What a session row shows after its title (Paseo's
//! `sidebar/workspace-trailing`): the workspace's diff stat, the session's
//! last activity in Paseo's compact form, or nothing. The menu offers the
//! first two; `none` is only a stored value.

use std::time::Duration;

use chrono::Utc;
use vessel_app_kit::status_mark::SLOT_HEIGHT;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::Card;
use vessel_app_view::ago::compact_ago;
use vessel_app_view::diff_count::format_diff_count;
use vessel_app_view::prefs::SidebarTrailing;
use vessel_proto::registry::DiffStat;

use crate::shell::Shell;

/// Move "now" on to "1m" while rows show last activity. The compact form
/// changes at a minute at its finest, so a minute is the tick.
pub(super) fn activity_clock(cx: &mut Context<'_, Shell>) {
    cx.spawn(async move |this, cx| {
        loop {
            cx.background_executor()
                .timer(Duration::from_secs(60))
                .await;
            let ticked = this.update(cx, |shell, cx| {
                if shell.prefs.settings.sidebar.trailing == SidebarTrailing::Timestamp {
                    cx.notify();
                }
            });
            if ticked.is_err() {
                break;
            }
        }
    })
    .detach();
}

/// The row's trailing slot, when the preference draws one.
#[must_use]
pub(super) fn slot(card: &Card, shell: &Shell, tokens: &Tokens) -> Option<AnyElement> {
    match shell.prefs.settings.sidebar.trailing {
        SidebarTrailing::None => None,
        SidebarTrailing::Timestamp => Some(stamp(card, tokens)),
        SidebarTrailing::Diff => {
            let workspace = shell.areas.labels.workspace_of(
                &card.machine,
                card.session,
                card.member_project(),
            )?;
            let stat = shell.diff_stat_of(&card.machine, workspace)?;
            Some(counts(stat, tokens))
        }
    }
}

/// `+N -N`, the page's card colours. The minus is ASCII, as Paseo's
/// `DiffStat` writes it (`components/diff-stat.tsx`).
fn counts(stat: DiffStat, tokens: &Tokens) -> AnyElement {
    div()
        .flex_none()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S1))
        .h(px(SLOT_HEIGHT))
        .type_(Type::Meta)
        .child(
            div()
                .text_color(tokens.st_ready)
                .child(format!("+{}", format_diff_count(stat.additions))),
        )
        .child(
            div()
                .text_color(tokens.danger)
                .child(format!("-{}", format_diff_count(stat.deletions))),
        )
        .into_any_element()
}

/// The session's own last activity (`Card::active_at`).
fn stamp(card: &Card, tokens: &Tokens) -> AnyElement {
    div()
        .flex_none()
        .h(px(SLOT_HEIGHT))
        .type_(Type::Meta)
        .text_color(tokens.text_tertiary)
        .child(compact_ago(card.detail.active_at, Utc::now()))
        .into_any_element()
}
