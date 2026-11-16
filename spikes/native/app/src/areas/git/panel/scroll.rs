//! Invalidate only the original current pane after handled list scroll input.
//! Notification grants no request, repository, or mutation authority.
use super::Entry;
use crate::shell::Shell;
use vessel_app_kit::ui::{App, Context, WeakEntity, Window};
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::workspace::git::RequestKey;
use vessel_app_view::workspace::git::panel::{PanelAddress, PanelTarget};

#[derive(Clone)]
struct Scroll {
    address: PanelAddress,
    target: PanelTarget,
    instance: RequestKey,
    context: ConnectionContext,
}
impl Scroll {
    fn capture(entry: &Entry, address: &PanelAddress) -> Option<Self> {
        Some(Self {
            address: address.clone(),
            target: entry.target.clone(),
            instance: entry.instance?,
            context: entry.context.clone()?,
        })
    }

    fn defer(&self, owner: &WeakEntity<Shell>, window: &mut Window, cx: &mut App) {
        let owner = owner.clone();
        let captured = self.clone();
        window.defer(cx, move |window, cx| {
            let _updated = owner.update(cx, |shell, cx| {
                shell.git_panel_scrolled(&captured, window, cx);
            });
        });
    }
}

pub(super) fn listen(entry: &Entry, address: &PanelAddress, cx: &Context<'_, Shell>) {
    let Some(captured) = Scroll::capture(entry, address) else {
        return;
    };
    let owner = cx.entity().downgrade();
    entry.files.on_scroll(move |window, cx| {
        // GPUI still holds ListState here. Defer both owner access and any
        // eventual current-geometry query until that borrow has been released.
        captured.defer(&owner, window, cx);
    });
}
impl Shell {
    fn git_panel_scrolled(
        &mut self,
        captured: &Scroll,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        let address = &captured.address;
        if self.areas.git.panels.registration.problem().is_some()
            || !self.layout_tab_matches(
                &address.workspace,
                address.tab,
                &super::layout_target(&captured.target),
            )
            || !self.layout_panel_visible(&address.workspace, address.tab, window)
        {
            return;
        }
        let current = self.git_panel_context(address);
        let Some(entry) = self.areas.git.panels.entries.get(address) else {
            return;
        };
        if !entry.matches_admission(
            captured.instance,
            &captured.target,
            &captured.context,
            current.as_ref(),
        ) {
            return;
        }
        // No old event range is forwarded. The actual owner's next prepaint
        // rebuilds visible vetoes from the current clipped section bounds.
        entry.visible_bodies.clear();
        cx.notify();
    }
}

#[cfg(test)]
#[path = "scroll_tests.rs"]
mod tests;
