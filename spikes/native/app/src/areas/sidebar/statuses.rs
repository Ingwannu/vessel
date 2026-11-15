//! The sidebar grouped by status (the Display menu's "Status"; Paseo's
//! status buckets in the board's words): Needs you, Working, Idle, and
//! Ended when ended sessions are listed, each session naming its machine,
//! each group's "N more" opening the board (the page's
//! `sidebar-statuses.tsx`, `Fleet::sidebar_statuses`).

use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    Context, Div, ElementId, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div,
    px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, kicker};
use vessel_app_view::prefs::SidebarView;
use vessel_app_view::sidebar::SIDEBAR_STATUS_SESSIONS;

use super::{INSET, more_row};
use crate::routes::Route;
use crate::shell::Shell;

/// Every status group with sessions, in the marks' order; "No sessions"
/// when there are none.
pub(super) fn groups(
    view: SidebarView,
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.t.lang.sidebar();
    let groups = shell.fleet.sidebar_statuses(view, SIDEBAR_STATUS_SESSIONS);
    let mut list = div().flex().flex_col().gap(px(spacing::S3_5));
    if groups.is_empty() && shell.loaded {
        list = list.child(
            div()
                .px(px(INSET))
                .type_(Type::Meta)
                .text_color(tokens.text_tertiary)
                .child(words.no_sessions),
        );
    }
    for group in groups {
        let mut rows = div().flex().flex_col().gap(px(spacing::S0_5)).child(
            div()
                .px(px(INSET))
                .pb(px(spacing::S0_5))
                .child(kicker(words.mark(group.mark), tokens)),
        );
        for card in &group.sessions {
            rows = rows.child(super::session_row::session_row(
                card,
                Some(card.machine.as_str()),
                shell,
                tokens,
                touch,
                cx,
            ));
        }
        if group.more > 0 {
            rows = rows.child(
                more_row(
                    ElementId::Name(format!("more-{:?}", group.mark).into()),
                    group.more,
                    shell,
                    tokens,
                )
                .on_click(cx.listener(|this, _event, window, cx| {
                    this.scope_board(None, cx);
                    this.go(Route::Board, window, cx);
                })),
            );
        }
        list = list.child(rows);
    }
    list
}
