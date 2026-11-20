//! The shells beside a workspace's sessions (ADR 0121's native consumer):
//! from protocol 40 a workspace's strip is its place's list, plus the live
//! shells placed beside its unarchived sessions. Those are made by older
//! nodes, by an agent below 40, or before the upgrade (the page's
//! `readStrip`, TRACKER 829).
//!
//! [Decision Log]
//! - Purpose: a shell placed beside a session still belongs to the
//!   session's workspace for the person; leaving it out would hide a
//!   running program until the session's archive ends it.
//! - Choice: one plain `terminal.list { place: session }` per workspace
//!   session on each fleet read (every 2 s), at most one in flight per
//!   session, merged with the place's long-poll list by id. The node's
//!   `Worktree`, `Project` and `Folder` lists leave these shells out.
//! - Rejected: one unfiltered list read client-side. A folder place is
//!   matched on its canonical path, which only the node can work out, so
//!   the place's own list stays the node's.
//! - Cost: a request per workspace session each read, while the place is a
//!   workspace place; none below 40, where the place is the session.

use std::collections::{BTreeMap, BTreeSet};

use vessel_app_link::Command;
use vessel_app_link::terminals::{Ask, Request};
use vessel_app_view::layout::LayoutKey;
use vessel_app_view::terminal_tabs::{beside_sessions, workspace_sessions};
use vessel_proto::terminals::{TerminalInfo, TerminalList, TerminalPlace};
use vessel_proto::{SessionId, TerminalId};

use super::pane_key;
use crate::shell::Shell;

/// The beside-session reads of the shown workspace.
#[derive(Debug, Default)]
pub(crate) struct Beside {
    /// The workspace they were asked for; another one starts afresh.
    workspace: Option<LayoutKey>,
    /// Sessions asked and not yet answered.
    asked: BTreeSet<SessionId>,
    /// Each session's live shells as last answered.
    shells: BTreeMap<SessionId, Vec<TerminalInfo>>,
}

impl Shell {
    /// On a fleet read: ask the shells beside the workspace's sessions,
    /// while the shown place is a workspace place.
    pub(super) fn terminal_tabs_poll_beside(
        &mut self,
        machine: &str,
        workspace: &LayoutKey,
        place: &TerminalPlace,
    ) {
        let beside = &mut self.areas.terminal_tabs.beside;
        if beside.workspace.as_ref() != Some(workspace) {
            *beside = Beside {
                workspace: Some(workspace.clone()),
                ..Beside::default()
            };
        }
        let LayoutKey::Session { session, .. } = workspace else {
            return;
        };
        if matches!(place, TerminalPlace::Session { .. }) {
            return;
        }
        let sessions = self.terminal_workspace_sessions(machine, *session);
        let beside = &mut self.areas.terminal_tabs.beside;
        beside.shells.retain(|session, _| sessions.contains(session));
        let asks: Vec<SessionId> = sessions
            .into_iter()
            .filter(|session| beside.asked.insert(*session))
            .collect();
        for session in asks {
            self.command(Command::Terminals(Ask {
                machine: machine.to_owned(),
                request: Request::List {
                    place: TerminalPlace::Session { session },
                    changed_from: None,
                },
            }));
        }
    }

    /// A list answered: whether it was one of the beside-session reads
    /// (kept and merged into the shown strip), not the place's own.
    pub(super) fn terminal_tabs_beside_listed(
        &mut self,
        machine: &str,
        place: &TerminalPlace,
        list: Option<&TerminalList>,
    ) -> bool {
        let TerminalPlace::Session { session } = place else {
            return false;
        };
        let beside = &mut self.areas.terminal_tabs.beside;
        let ours = beside.workspace.as_ref().is_some_and(|workspace| {
            matches!(workspace, LayoutKey::Session { machine: alias, .. } if alias.as_str() == machine)
        });
        if !ours || !beside.asked.remove(session) {
            return false;
        }
        if let Some(list) = list {
            let only: BTreeSet<SessionId> = [*session].into_iter().collect();
            let _was = beside
                .shells
                .insert(*session, beside_sessions(list, &only));
        }
        if let Some(workspace) = beside.workspace.clone() {
            self.terminal_tabs_merge(&workspace);
        }
        true
    }

    /// The shown workspace's strip: the place's live shells and the ones
    /// beside its sessions, by id, waiting for the next frame's reconcile.
    pub(super) fn terminal_tabs_merge(&mut self, workspace: &LayoutKey) {
        let tabs = &mut self.areas.terminal_tabs;
        let Some((at, place_shells)) = tabs.place_shells.as_ref() else {
            return;
        };
        if at != workspace {
            return;
        }
        let mut infos: Vec<TerminalInfo> = place_shells.clone();
        if tabs.beside.workspace.as_ref() == Some(workspace) {
            for info in tabs.beside.shells.values().flatten() {
                if infos.iter().all(|kept| kept.id != info.id) {
                    infos.push(info.clone());
                }
            }
        }
        let mut ids: Vec<TerminalId> = infos.iter().map(|info| info.id).collect();
        ids.sort_unstable();
        for info in &infos {
            if let Some(key) = pane_key(workspace, info.id)
                && let Some(pane) = tabs.panes.get_mut(&key)
            {
                pane.info = Some(info.clone());
            }
        }
        tabs.listed_infos = infos.into_iter().map(|info| (info.id, info)).collect();
        tabs.listed = Some((workspace.clone(), ids));
    }

    /// `session`'s workspace sessions on `machine`, from its fleet read.
    fn terminal_workspace_sessions(&self, machine: &str, session: SessionId) -> BTreeSet<SessionId> {
        let Some(on) = self.fleet.machines.iter().find(|on| on.name == machine) else {
            return [session].into_iter().collect();
        };
        on.sessions
            .iter()
            .find(|card| card.session == session)
            .map_or_else(
                || [session].into_iter().collect(),
                |card| workspace_sessions(card, &on.sessions),
            )
    }
}
