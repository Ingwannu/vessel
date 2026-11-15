//! Pane discovery and exact-ticket settlement, with no drawing or root guessing.
use super::Entry;
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::git_panel::{Answer, Ask, Outcome};
use vessel_app_view::connection::{ConnectionContext, ConnectionTag};
use vessel_app_view::layout::LayoutKey;
use vessel_app_view::workspace::git::panel::{
    Comparison, ComparisonMode, LocationRefusal, PanelAddress, PanelReads, PanelTarget,
    ReadFailure, Whitespace,
};

impl Shell {
    pub(in crate::areas::git) fn git_panel_context(
        &self,
        address: &PanelAddress,
    ) -> Option<ConnectionContext> {
        match &address.workspace {
            LayoutKey::Workspace { .. } => None,
            LayoutKey::Session { machine, .. } => self
                .fleet
                .machines
                .iter()
                .find(|candidate| candidate.name == machine.as_str())
                .and_then(vessel_app_view::Machine::connection_context),
        }
    }

    pub(in crate::areas::git) fn git_panel_open(
        &mut self,
        address: PanelAddress,
        target: PanelTarget,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.layout_panel_visible(&address.workspace, address.tab, window) {
            return;
        }
        self.git_panels_prune();
        let context = self.git_panel_context(&address);
        let mut entry = Entry::new(target, context);
        let request = discover(&mut entry, &address, &mut self.areas.git.keys);
        let _old = self.areas.git.panels.entries.insert(address, entry);
        if let Some(ask) = request {
            self.command(vessel_app_link::Command::GitPanel(ask));
        }
        cx.notify();
    }

    pub(crate) fn git_panel_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        if !self.layout_tab_exists(&answer.address.workspace, answer.address.tab) {
            let _removed = self.areas.git.panels.entries.remove(&answer.address);
            return;
        }
        let current = self.git_panel_context(&answer.address);
        let Some(entry) = self.areas.git.panels.entries.get_mut(&answer.address) else {
            return;
        };
        if current.as_ref() != Some(&answer.context)
            || entry.context.as_ref() != Some(&answer.context)
        {
            retire(entry);
            return;
        }
        let located = match answer.outcome {
            Outcome::Located(ticket, result) => entry.binding.finish(&ticket, result),
            Outcome::Status(ticket, result) => {
                let _accepted = entry
                    .reads
                    .as_mut()
                    .is_some_and(|reads| reads.finish_status(ticket, result));
                false
            }
            Outcome::Summary(ticket, result) => {
                if let Some(reads) = &mut entry.reads {
                    if reads.finish_summary(&ticket, result) {
                        entry.files.reset(
                            reads
                                .summary()
                                .shown()
                                .map_or(0, |summary| summary.files.len()),
                        );
                    }
                }
                false
            }
            Outcome::File(ticket, result) => {
                let _accepted = entry
                    .reads
                    .as_mut()
                    .is_some_and(|reads| reads.finish_file(&ticket, result));
                false
            }
        };
        if located {
            self.git_panel_located(&answer.address);
        }
        cx.notify();
    }

    fn git_panel_located(&mut self, address: &PanelAddress) {
        let git = &mut self.areas.git;
        let Some(entry) = git.panels.entries.get_mut(address) else {
            return;
        };
        let Some(repository) = entry.binding.repository() else {
            if entry.binding.context().is_some() {
                entry.problem = Some(ReadFailure::WorkspaceUnavailable);
            }
            return;
        };
        let comparison = match &entry.target {
            PanelTarget::CommitDiff(id) => Comparison::at_commit(id.clone(), Whitespace::Show),
            PanelTarget::Changes | PanelTarget::WorkingDiff => {
                Comparison::new(ComparisonMode::All, None, Whitespace::Show)
            }
        };
        let (Ok(status), Ok(summary)) = (git.keys.issue(), git.keys.issue()) else {
            entry.problem = Some(ReadFailure::KeysExhausted);
            return;
        };
        let mut reads = PanelReads::new(repository, comparison);
        let context = entry.context.clone();
        let requests = context.map(|context| {
            [
                Ask::Status {
                    address: address.clone(),
                    context: context.clone(),
                    ticket: reads.begin_status(status),
                },
                Ask::Summary {
                    address: address.clone(),
                    context,
                    ticket: reads.begin_summary(summary),
                },
            ]
        });
        entry.reads = Some(reads);
        if let Some(requests) = requests {
            for ask in requests {
                self.command(vessel_app_link::Command::GitPanel(ask));
            }
        }
    }

    // Closing a tab can leave no read in flight to trigger answer-side cleanup.
    // Drop those documents on the next open/fleet reconciliation, using the sole
    // layout book's read-only query. This list is local to one synchronous turn,
    // not another retained tab/identity registry.
    fn git_panels_prune(&mut self) {
        let closed = self
            .areas
            .git
            .panels
            .entries
            .keys()
            .filter(|address| !self.layout_tab_exists(&address.workspace, address.tab))
            .cloned()
            .collect::<Vec<_>>();
        for address in closed {
            let _dropped = self.areas.git.panels.entries.remove(&address);
        }
    }

    /// Retirement happens before any device-area early return can skip it.
    #[must_use]
    pub(crate) fn git_panels_retired(&mut self, tag: &ConnectionTag) -> bool {
        self.areas.git.panels.retire(tag)
    }

    /// A fleet update cannot leave facts from a retired admission actionable.
    pub(crate) fn git_panels_fleet(&mut self) {
        self.git_panels_prune();
        let retired = self
            .areas
            .git
            .panels
            .entries
            .keys()
            .filter(|address| {
                self.areas
                    .git
                    .panels
                    .entries
                    .get(*address)
                    .is_some_and(|entry| entry.context != self.git_panel_context(address))
            })
            .cloned()
            .collect::<Vec<_>>();
        for address in retired {
            if let Some(entry) = self.areas.git.panels.entries.get_mut(&address) {
                retire(entry);
            }
        }
    }
}
fn retire(entry: &mut Entry) {
    let _changed = entry.binding.reconcile(None);
    if let Some(reads) = &mut entry.reads {
        reads.retire();
    }
    entry.context = None;
    entry.problem = Some(ReadFailure::DestinationChanged);
    entry.path_places.clear();
    entry.tips = vessel_app_kit::Tooltips::new();
}

fn discover(
    entry: &mut Entry,
    address: &PanelAddress,
    keys: &mut vessel_app_view::workspace::git::RequestKeys,
) -> Option<Ask> {
    match (&address.workspace, entry.context.clone()) {
        (LayoutKey::Workspace { .. }, _) => {
            entry.problem = Some(ReadFailure::WorkspaceUnavailable);
            None
        }
        (LayoutKey::Session { .. }, None) => {
            entry.problem = Some(ReadFailure::DestinationClosed);
            None
        }
        (LayoutKey::Session { .. }, Some(context)) => match keys.issue() {
            Ok(key) => match entry.binding.begin(address.clone(), context, key) {
                Ok(ticket) => Some(Ask::Locate(ticket)),
                Err(LocationRefusal::NeedsRegistry) => {
                    entry.problem = Some(ReadFailure::WorkspaceUnavailable);
                    None
                }
                Err(LocationRefusal::DestinationChanged) => {
                    entry.problem = Some(ReadFailure::DestinationChanged);
                    None
                }
            },
            Err(_exhausted) => {
                entry.problem = Some(ReadFailure::KeysExhausted);
                None
            }
        },
    }
}

impl super::Panels {
    pub(super) fn retire(&mut self, tag: &ConnectionTag) -> bool {
        let mut changed = false;
        for entry in self.entries.values_mut() {
            if entry
                .context
                .as_ref()
                .is_some_and(|context| context.tag() == *tag)
            {
                retire(entry);
                changed = true;
            }
        }
        changed
    }
}
