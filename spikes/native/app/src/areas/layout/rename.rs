//! A workspace layout's way to rename the open session (TRACKER 800): the
//! original node's authority and an answer matched by its ticket, for the
//! one rename dialog (`conversation::rename`).
//!
//! [Decision Log]
//! - Purpose: a workspace layout renames through the node it was opened
//!   from (its alias and node id), so a rename never lands on another node
//!   that took the alias; the answer is taken only for this form's ticket.
//! - Before: this module drew a dialog of its own (the strip's Rename),
//!   beside the session's (789's Paseo frame). The coordinator ruled one
//!   dialog (2026-11-18): this route stays, its dialog went.
//! - Constraint: the legacy route cannot supply original-node authority
//!   until its lifecycle binding adapter lands. Never bless it from the
//!   latest fleet: a session layout renames through the session's act.
use super::with_layout;
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_view::layout::{
    HostAlias, LayoutKey, RenamePhase, RenameProblem, RenameSubmission, RenameTarget, Target,
};
use vessel_proto::{NodeId, SessionTitle};

/// Where a workspace layout's rename goes, and its outstanding request.
#[derive(Debug)]
pub(crate) struct LayoutRoute {
    workspace: LayoutKey,
    machine: HostAlias,
    node: NodeId,
    target: RenameTarget,
    phase: RenamePhase,
}

impl LayoutRoute {
    /// The layout it was opened from (kept while the form is open).
    #[cfg(test)]
    pub(crate) const fn workspace(&self) -> &LayoutKey {
        &self.workspace
    }
}

impl Shell {
    /// Whether the session is shown in a workspace layout (which renames
    /// through its original node).
    pub(crate) fn on_workspace_layout(&self) -> bool {
        matches!(super::key(self), Ok(LayoutKey::Workspace { .. }))
    }

    /// The route for renaming the open session on a workspace layout, when
    /// its tab is in that layout and its machine has a valid alias.
    pub(crate) fn layout_rename_route(&self) -> Option<LayoutRoute> {
        let Ok(workspace @ LayoutKey::Workspace { node, .. }) = super::key(self) else {
            return None;
        };
        let open = self.areas.session.open.as_ref()?;
        let session = open.session;
        let in_layout = with_layout(self, |layout| {
            Ok(layout
                .root()
                .panes()
                .into_iter()
                .flat_map(|pane| pane.tabs().to_vec())
                .any(|tab| {
                    layout.tab(tab).map(vessel_app_view::layout::Tab::target)
                        == Some(&Target::Agent(session))
                }))
        })
        .unwrap_or(false);
        if !in_layout {
            return None;
        }
        Some(LayoutRoute {
            workspace,
            machine: HostAlias::parse(&open.machine).ok()?,
            node,
            target: RenameTarget::Agent(session),
            phase: RenamePhase::Editing,
        })
    }

    /// Send `title` by the form's layout route; its answer comes to
    /// `layout_renamed`. `false` when no request could be made (the ticket
    /// space is spent).
    pub(crate) fn layout_send_rename(&mut self, title: SessionTitle) -> bool {
        let Ok(ticket) = self.areas.layout.rename_ticket.next() else {
            return false;
        };
        let Some(route) = self.areas.conversation.rename_layout_route() else {
            return false;
        };
        let submission = RenameSubmission {
            ticket,
            target: route.target,
            title,
        };
        route.phase = RenamePhase::Sending(submission.clone());
        let ask = vessel_app_link::workspace::layout::Ask {
            node: route.node,
            machine: route.machine.clone(),
            submission,
        };
        self.areas.layout.rename_ticket = ticket;
        self.command(vessel_app_link::Command::Workspace {
            machine: ask.machine.as_str().to_owned(),
            ask: vessel_app_link::workspace::Ask::Layout(ask),
        });
        true
    }

    /// The node's answer to a layout rename: taken only for the open form's
    /// ticket, target, alias and node. Success names the cards, the open
    /// session (the tab's label) and closes the dialog; a refusal keeps the
    /// typed title with the reason under it.
    pub(crate) fn layout_renamed(
        &mut self,
        answer: vessel_app_link::workspace::layout::Answer,
        cx: &mut Context<'_, Self>,
    ) {
        let active_workspace = super::key(self).ok();
        let active_node = match &active_workspace {
            Some(LayoutKey::Workspace { node, .. }) => Some(*node),
            Some(LayoutKey::Session { .. }) | None => None,
        };
        let Some(route) = self.areas.conversation.rename_layout_route() else {
            return;
        };
        if route.machine != answer.ask.machine
            || route.node != answer.ask.node
            || !route.phase.answered(
                answer.ask.submission.ticket,
                answer.ask.submission.target,
                answer
                    .result
                    .as_ref()
                    .map(|_| ())
                    .map_err(|problem| *problem),
            )
        {
            return;
        }
        let (machine, node, workspace, target) = (
            route.machine.clone(),
            route.node,
            route.workspace.clone(),
            route.target,
        );
        match answer.result {
            Ok(title) => {
                if let RenameTarget::Agent(session) = target {
                    for slot in &mut self.fleet.machines {
                        if slot.name == machine.as_str() && slot.node == Some(node) {
                            for card in &mut slot.sessions {
                                if card.session == session {
                                    title.as_str().clone_into(&mut card.title);
                                }
                            }
                        }
                    }
                    if let Some(open) = self.areas.session.open.as_mut().filter(|open| {
                        open.session == session
                            && open.machine == machine.as_str()
                            && active_node == Some(node)
                            && active_workspace.as_ref() == Some(&workspace)
                    }) {
                        title.as_str().clone_into(&mut open.title);
                    }
                }
                self.rename_done(cx);
            }
            Err(problem) => {
                let why = self.layout_rename_problem(problem);
                self.rename_failed(why, cx);
            }
        }
        cx.notify();
    }

    /// A layout refusal in the dialog's words.
    fn layout_rename_problem(&self, problem: RenameProblem) -> String {
        let words = self.words.layout();
        match problem {
            RenameProblem::NodeChanged => words.rename_node_changed(),
            RenameProblem::Offline => words.rename_offline(),
            RenameProblem::TimedOut => words.rename_timeout(),
            RenameProblem::Busy => words.rename_queue(),
            RenameProblem::Node(_) => words.rename_failed(),
            RenameProblem::MalformedResponse => words.rename_malformed(),
        }
        .to_owned()
    }
}

#[cfg(test)]
mod tests;
