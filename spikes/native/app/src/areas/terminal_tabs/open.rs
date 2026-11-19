//! Opening, listing and closing terminal tabs (Paseo's
//! `use-workspace-terminals.ts`, `workspace-screen.tsx`
//! `handleCloseTerminalTab`, `workspace-layout-actions.ts`
//! `reconcileWorkspaceTabs`).

use std::collections::BTreeSet;

use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_link::terminals::{Ask, Request};
use vessel_app_view::layout::{
    AgentRole, CloseEffect, ClosePlan, EntitySnapshot, Hydration, LayoutKey, LayoutMode,
    OpenIntent, Placement, RetargetRequest, TabId, Target,
};
use vessel_app_view::terminal::Origin;
use vessel_app_view::terminal_tabs::{Failure, standalone};
use vessel_app_view::toasts::ToastTone;
use vessel_proto::TerminalId;
use vessel_proto::terminals::{TerminalInfo, TerminalList, TerminalPlace};

use super::{Creating, Pane, PaneKey, pane_key, place_of};
use crate::routes::Route;
use crate::shell::Shell;

/// Whether a set of close plans is one terminal tab, which this area
/// closes after the person confirms. Several tabs with a terminal among
/// them stay refused (Paseo's bulk confirm is not drawn).
pub(crate) fn closes_terminal(plans: &[ClosePlan]) -> bool {
    matches!(plans, [plan] if matches!(plan.effect(), CloseEffect::CloseTerminal(_)))
}

/// The terminal ids a layout shows as tabs.
fn terminal_tabs(layout: &vessel_app_view::layout::Layout) -> BTreeSet<TerminalId> {
    layout
        .tabs()
        .filter_map(|tab| match tab.target() {
            Target::Terminal(id) => Some(*id),
            Target::NewTab
            | Target::Draft(_)
            | Target::Agent(_)
            | Target::ProviderSubagent { .. }
            | Target::Browser(_)
            | Target::Changes
            | Target::Files
            | Target::Scripts
            | Target::PullRequest
            | Target::File { .. }
            | Target::WorkingDiff
            | Target::Plugin { .. }
            | Target::Setup(_)
            | Target::CommitDiff(_) => None,
        })
        .collect()
}

impl Shell {
    /// "Terminal" chosen: a shell opened in the workspace shown, its tab
    /// replacing `replaces` (the New tab tab it was chosen in) or opened in
    /// the focused pane. "Host is not connected" while its machine is away.
    pub(crate) fn terminal_tab_new(
        &mut self,
        replaces: Option<TabId>,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Ok(workspace) = crate::areas::layout::current_key(self) else {
            return;
        };
        let (Some(place), LayoutKey::Session { machine, .. }) = (place_of(&workspace), &workspace)
        else {
            return;
        };
        let machine = machine.as_str().to_owned();
        if !self.machine_online(&machine) {
            let translator = self.words;
            let words = translator.terminal_tabs();
            self.toast(words.host_disconnected(), ToastTone::Error, cx);
            return;
        }
        self.areas.terminal_tabs.creating.push(Creating {
            machine: machine.clone(),
            place: place.clone(),
            workspace,
            replaces,
            mode: crate::areas::layout::mode(window),
        });
        self.command(Command::Terminals(Ask {
            machine,
            request: Request::Create { place },
        }));
    }

    /// The node answered a create: the tab opens and takes the keyboard,
    /// or the failure is said (the machine's words are not shown).
    pub(super) fn terminal_tab_created(
        &mut self,
        machine: &str,
        place: &TerminalPlace,
        result: Result<Box<TerminalInfo>, Failure>,
        cx: &mut Context<'_, Self>,
    ) {
        let creating = &mut self.areas.terminal_tabs.creating;
        let Some(at) = creating
            .iter()
            .position(|asked| asked.machine == machine && asked.place == *place)
        else {
            return;
        };
        let asked = creating.remove(at);
        let info = match result {
            Ok(info) => *info,
            Err(_failure) => {
                let translator = self.words;
                let words = translator.terminal_tabs();
                self.toast(words.create_failed(), ToastTone::Error, cx);
                return;
            }
        };
        let Some(key) = pane_key(&asked.workspace, info.id) else {
            return;
        };
        let target = Target::Terminal(info.id);
        let mut pane = Pane::new(Origin::Opened, cx, &key);
        pane.info = Some(info);
        let _replaced = self.areas.terminal_tabs.panes.insert(key.clone(), pane);
        self.areas.terminal_tabs.focus_next = Some(key.clone());
        self.terminal_tab_follow(&key);
        let mode = asked.mode;
        let opened = self.update_layout(
            &asked.workspace,
            |layout| match asked.replaces.filter(|tab| layout.tab(*tab).is_some()) {
                Some(tab) => layout.retarget(RetargetRequest {
                    tab,
                    target,
                    intent: OpenIntent::New,
                    role: AgentRole::Unknown,
                }),
                None => layout.open(
                    target,
                    OpenIntent::New,
                    Placement::Focused,
                    AgentRole::Unknown,
                    mode,
                ),
            },
            cx,
        );
        if opened.is_err() {
            tracing::warn!("a new terminal's tab could not be opened");
        }
    }

    /// Ask for the shown workspace's terminals (every fleet read, and after
    /// a create, a close or an end), as Paseo's `terminals_changed` keeps
    /// its list.
    pub(crate) fn terminal_tabs_poll(&mut self) {
        if self.route != Route::Session || self.areas.terminal_tabs.listing.is_some() {
            return;
        }
        let Ok(workspace) = crate::areas::layout::current_key(self) else {
            return;
        };
        let (Some(place), LayoutKey::Session { machine, .. }) = (place_of(&workspace), &workspace)
        else {
            return;
        };
        let machine = machine.as_str().to_owned();
        if !self.machine_online(&machine) {
            return;
        }
        self.areas.terminal_tabs.listing = Some((machine.clone(), place.clone()));
        self.command(Command::Terminals(Ask {
            machine,
            request: Request::List { place },
        }));
    }

    /// A list answered: kept for the next frame's reconcile.
    pub(super) fn terminal_tabs_listed(
        &mut self,
        machine: &str,
        place: &TerminalPlace,
        list: Option<&TerminalList>,
    ) {
        let tabs = &mut self.areas.terminal_tabs;
        if tabs
            .listing
            .as_ref()
            .is_some_and(|(asked, at)| asked == machine && at == place)
        {
            tabs.listing = None;
        }
        let Some(list) = list else {
            return;
        };
        let Ok(workspace) = crate::areas::layout::current_key(self) else {
            return;
        };
        let shown = matches!(
            &workspace,
            LayoutKey::Session { machine: alias, .. } if alias.as_str() == machine
        );
        if shown && place_of(&workspace).as_ref() == Some(place) {
            for info in &list.terminals {
                if let Some(key) = pane_key(&workspace, info.id)
                    && let Some(pane) = self.areas.terminal_tabs.panes.get_mut(&key)
                {
                    pane.info = Some(info.clone());
                }
            }
            self.areas.terminal_tabs.listed = Some((workspace, standalone(list, place)));
        }
    }

    /// Apply a kept list to the shown layout, as Paseo's reconcile: a tab
    /// for each of the place's live shells (in the background), none for a
    /// shell that is gone. While a create waits, no tab is added (the
    /// create opens its own). The layout is written only when its set of
    /// terminal tabs differs, so a quiet list writes nothing.
    pub(super) fn terminal_tabs_reconcile(&mut self, mode: LayoutMode, cx: &mut Context<'_, Self>) {
        let Some((workspace, ids)) = self.areas.terminal_tabs.listed.take() else {
            return;
        };
        if crate::areas::layout::current_key(self).as_ref() != Ok(&workspace) {
            return;
        }
        let waiting = self
            .areas
            .terminal_tabs
            .creating
            .iter()
            .any(|asked| asked.workspace == workspace);
        let open = crate::areas::layout::read_layout(self, &workspace, terminal_tabs);
        let listed: BTreeSet<TerminalId> = ids.iter().copied().collect();
        let wanted: BTreeSet<TerminalId> = if waiting {
            listed.intersection(&open).copied().collect()
        } else {
            listed
        };
        if wanted != open {
            let ids: Vec<TerminalId> = ids.into_iter().filter(|id| wanted.contains(id)).collect();
            let snapshot = EntitySnapshot {
                agents: Hydration::Pending,
                sessions: &[],
                terminals: Hydration::Ready,
                terminal_ids: &ids,
            };
            if self
                .update_layout(&workspace, |layout| layout.reconcile(&snapshot, mode), cx)
                .is_err()
            {
                tracing::warn!("the workspace's terminal tabs could not be reconciled");
            }
        }
        self.terminal_tabs_prune();
    }

    /// Drop the panes (and their follows) whose tab is not in the shown
    /// layout: a tab shown again reads its terminal from the start.
    pub(super) fn terminal_tabs_prune(&mut self) {
        let Ok(workspace) = crate::areas::layout::current_key(self) else {
            return;
        };
        let open = crate::areas::layout::read_layout(self, &workspace, terminal_tabs);
        let gone: Vec<PaneKey> = self
            .areas
            .terminal_tabs
            .panes
            .keys()
            .filter(|key| {
                pane_key(&workspace, key.terminal).as_ref() != Some(*key)
                    || !open.contains(&key.terminal)
            })
            .cloned()
            .collect();
        for key in gone {
            let _pane = self.areas.terminal_tabs.panes.remove(&key);
            if self.areas.terminal_tabs.focused.as_ref() == Some(&key) {
                self.areas.terminal_tabs.focused = None;
            }
            self.command(Command::Terminals(Ask {
                machine: key.machine,
                request: Request::Unfollow {
                    terminal: key.terminal,
                },
            }));
        }
    }

    /// Close pressed on a terminal tab: Paseo's confirm first.
    pub(crate) fn terminal_tab_ask_close(
        &mut self,
        workspace: LayoutKey,
        plan: ClosePlan,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.terminal_tabs.dialog.focus(window, cx);
        self.areas.terminal_tabs.confirm = Some((workspace, plan));
        cx.notify();
    }

    /// Cancel, Escape or the scrim: the terminal stays.
    pub(super) fn terminal_tab_keep(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.terminal_tabs.confirm = None;
        self.areas.terminal_tabs.dialog.dismissed(window, cx);
        cx.notify();
    }

    /// Close confirmed: the tab goes at once and the node ends and forgets
    /// the terminal (Paseo kills it and drops it from its list).
    pub(super) fn terminal_tab_close(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some((workspace, plan)) = self.areas.terminal_tabs.confirm.take() else {
            return;
        };
        self.areas.terminal_tabs.dialog.dismissed(window, cx);
        let CloseEffect::CloseTerminal(terminal) = plan.effect() else {
            return;
        };
        self.layout_close_settled(&workspace, &plan, Ok(()), cx);
        if let Some(key) = pane_key(&workspace, terminal) {
            let _pane = self.areas.terminal_tabs.panes.remove(&key);
            self.command(Command::Terminals(Ask {
                machine: key.machine,
                request: Request::Close { terminal },
            }));
        }
        cx.notify();
    }
}
