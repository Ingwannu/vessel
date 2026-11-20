//! The panel's Pull request tab (Paseo's `pull_request` panel in the
//! explorer, ADR 0088): after Changes and Files while the open session
//! has a pull request, labelled with its bare number as Paseo's
//! `formatPrTabLabel`; its body is the pulls area's pane.
//!
//! [Decision Log]
//! - Purpose: the pull request's checks and activity beside the session,
//!   where Paseo opens them, instead of a dialog over the Changes tab.
//! - Constraint: this panel's tabs are fixed and cannot be closed; Paseo
//!   adds its tab once, in the background, when the branch has a pull
//!   request, and lets it be closed.
//! - Choice: the tab is listed while the session's card names a pull
//!   request, or while it is the chosen tab (so it never vanishes under
//!   the person); choosing it reads the pull request. The pulls area owns
//!   the body and its state; this file only places it.
//! - Cost: no close button (none of this panel's tabs has one); the tab
//!   returns whenever the branch has a pull request.

use vessel_app_kit::Tokens;
use vessel_app_kit::tabs::tab;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, StatefulInteractiveElement as _, Window,
};
use vessel_proto::SessionId;
use vessel_proto::workspace::WorkspaceRoot;

use super::{Place, Tab};
use crate::shell::Shell;

/// The tab in the strip, when it is listed.
pub(super) fn strip_tab(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let panel = &shell.areas.workspace.code;
    let chosen = panel.tab == Tab::PullRequest;
    let number = shell.pulls_tab_number();
    if number.is_none() && !chosen {
        return None;
    }
    let label = number.map_or_else(
        || shell.words.pulls().label().to_owned(),
        |number| number.to_string(),
    );
    Some(
        tab("code-tab-pull-request", label, chosen, None, tokens)
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.code_tab(Tab::PullRequest, cx);
            }))
            .into_any_element(),
    )
}

impl Shell {
    /// The git row's link or a toast's View: the panel opens on the Pull
    /// request tab.
    pub(crate) fn code_show_pull_request(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.areas.workspace.code.open {
            self.toggle_code(window, cx);
        }
        if self.areas.workspace.code.open {
            self.code_tab(Tab::PullRequest, cx);
        }
    }

    /// Whether the Pull request tab is what the panel shows.
    pub(crate) fn code_shows_pull_request(&self) -> bool {
        let panel = &self.areas.workspace.code;
        panel.open && panel.tab == Tab::PullRequest
    }

    /// The session the panel shows and its checkout, once read: the pane's
    /// subject (Paseo's `canQuery`: a git checkout with a folder).
    pub(crate) fn code_checkout(&self) -> Option<(&str, SessionId, &WorkspaceRoot)> {
        let target = self.areas.workspace.code.target.as_ref()?;
        match &target.place {
            Place::Root(root) => Some((target.machine.as_str(), target.session, root)),
            Place::Reading | Place::Outside | Place::Failed(_) => None,
        }
    }
}
