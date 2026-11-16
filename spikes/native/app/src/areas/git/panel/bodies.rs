//! Visible-file reads reuse the pane's exact receipt and one bounded cache.
//!
//! [Decision Log]
//! - The node pages files; requesting every summary row defeats lazy reads.
//! - Only an intersecting, actually prepainted section schedules admission.
//!   The deferred callback still validates its original pane/epoch/summary.
//! - Retain at most32 file payloads/engines globally, each already capped by
//!   `FilePages`, with at most4 locally pending reads. This bounds retained
//!   payloads, not total process/transport memory. Evict complete LRU entries,
//!   never a pending entry or the engine owning keyboard selection.
//! - Reuse the Link's tracked calls. Eviction neither aborts nor replays work;
//!   missing read slots cannot be recreated by late replies.
use super::{Entry, Panels};
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::git_panel::Ask;
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::workspace::git::panel::{
    FileReadTarget, PanelAddress, PanelTarget, ReadFailure,
};
use vessel_app_view::workspace::git::{ReadState, RequestKey};
use vessel_proto::workspace::RelPath;

const MAX_BODY_FILES: usize = 32;
const MAX_PENDING_BODIES: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BodyAddress {
    panel: PanelAddress,
    path: RelPath,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum FileDemand {
    Visible,
    Retry,
    Continue,
}

#[derive(Debug, Clone)]
pub(super) struct FileActivation {
    address: BodyAddress,
    target: PanelTarget,
    instance: RequestKey,
    context: ConnectionContext,
    file: FileReadTarget,
    demand: FileDemand,
}
impl FileActivation {
    pub(super) fn capture(
        entry: &Entry,
        address: &PanelAddress,
        path: &RelPath,
        demand: FileDemand,
    ) -> Option<Self> {
        let reads = entry.reads.as_ref()?;
        if entry.binding.repository() != Some(reads.repository())
            || entry.binding.context() != entry.context.as_ref()
        {
            return None;
        }
        let file = match demand {
            FileDemand::Continue => reads.next_file_target(path)?,
            FileDemand::Retry => {
                if !matches!(reads.file(path), Some(ReadState::Failed(_))) {
                    return None;
                }
                reads.file_target(path)?
            }
            FileDemand::Visible => {
                if reads.file(path).is_some() {
                    return None;
                }
                reads.file_target(path)?
            }
        };
        Some(Self {
            address: BodyAddress {
                panel: address.clone(),
                path: path.clone(),
            },
            target: entry.target.clone(),
            instance: entry.instance?,
            context: entry.context.clone()?,
            file,
            demand,
        })
    }

    fn is_current(&self, entry: &Entry, current: Option<&ConnectionContext>) -> bool {
        entry.matches_admission(self.instance, &self.target, &self.context, current)
            && Self::capture(entry, &self.address.panel, &self.address.path, self.demand)
                .is_some_and(|fresh| fresh.file == self.file)
    }
}

#[derive(Debug, Clone)]
pub(super) struct Reveal {
    pub(super) address: PanelAddress,
    pub(super) target: PanelTarget,
    pub(super) instance: RequestKey,
    pub(super) context: ConnectionContext,
    pub(super) ticket: vessel_app_view::workspace::git::panel::FileTicket,
    pub(super) index: usize,
}

impl Panels {
    pub(super) fn touch_body(&mut self, address: &PanelAddress, path: &RelPath) {
        let index = self
            .body_order
            .iter()
            .position(|key| &key.panel == address && &key.path == path);
        if let Some(kept) = index.and_then(|index| self.body_order.remove(index)) {
            self.body_order.push_back(kept);
        }
    }

    fn pending_bodies(&self) -> usize {
        self.body_order
            .iter()
            .filter(|key| {
                self.entries
                    .get(&key.panel)
                    .and_then(|entry| entry.reads.as_ref())
                    .and_then(|reads| reads.file(&key.path))
                    .is_some_and(|read| read.outstanding().is_some())
            })
            .count()
    }

    fn admit_body(&mut self, key: &BodyAddress, window: &Window, cx: &Context<'_, Shell>) -> bool {
        self.body_order.retain(|key| {
            self.entries
                .get(&key.panel)
                .and_then(|entry| entry.reads.as_ref())
                .is_some_and(|reads| reads.file(&key.path).is_some())
        });
        if self.pending_bodies() >= MAX_PENDING_BODIES {
            return false;
        }
        if let Some(index) = self.body_order.iter().position(|kept| kept == key) {
            let _removed = self.body_order.remove(index);
        } else if self.body_order.len() >= MAX_BODY_FILES {
            let victim = self.body_order.iter().position(|key| {
                self.entries.get(&key.panel).is_none_or(|entry| {
                    let pending = entry
                        .reads
                        .as_ref()
                        .and_then(|reads| reads.file(&key.path))
                        .is_some_and(|read| read.outstanding().is_some());
                    let focused = entry
                        .documents
                        .get(&key.path)
                        .is_some_and(|document| document.selection_focused(window, cx));
                    !pending && !focused
                })
            });
            let Some(victim) = victim.and_then(|index| self.body_order.remove(index)) else {
                return false;
            };
            if let Some(entry) = self.entries.get_mut(&victim.panel) {
                if let Some(reads) = &mut entry.reads {
                    let _forgotten = reads.forget_file(&victim.path);
                }
                let _released = entry.documents.remove(&victim.path);
                // Re-measure after the old body becomes a lazy placeholder.
                entry.files.changed(0..entry.files.rows());
            }
        }
        self.body_order.push_back(key.clone());
        true
    }
}

impl Shell {
    pub(super) fn git_panel_reveal(
        &mut self,
        captured: &Reveal,
        bounds: vessel_app_kit::ui::Bounds<vessel_app_kit::ui::Pixels>,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.layout_tab_matches(
            &captured.address.workspace,
            captured.address.tab,
            &super::layout_target(&captured.target),
        ) || !self.layout_panel_visible(
            &captured.address.workspace,
            captured.address.tab,
            window,
        ) {
            return;
        }
        let current = self.git_panel_context(&captured.address);
        let Some(entry) = self.areas.git.panels.entries.get(&captured.address) else {
            return;
        };
        if !entry.matches_admission(
            captured.instance,
            &captured.target,
            &captured.context,
            current.as_ref(),
        ) || !entry.reads.as_ref().is_some_and(|reads| {
            reads.summary_ticket().is_some()
                && reads
                    .file(captured.ticket.path())
                    .and_then(|read| read.shown())
                    .is_some_and(|pages| pages.ticket() == &captured.ticket)
        }) {
            return;
        }
        // Notify the actual owner and invalidate its cached row, not merely the
        // child editor. The one outer list owns keyboard caret reveal.
        entry
            .files
            .reveal_bounds(bounds, vessel_app_kit::ui::Pixels::default());
        entry
            .files
            .changed(captured.index..captured.index.saturating_add(1));
        cx.notify();
    }

    pub(super) fn git_panel_file(
        &mut self,
        captured: &FileActivation,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        let address = &captured.address.panel;
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
        if !self
            .areas
            .git
            .panels
            .entries
            .get(address)
            .is_some_and(|entry| captured.is_current(entry, current.as_ref()))
        {
            return;
        }
        if !self
            .areas
            .git
            .panels
            .admit_body(&captured.address, window, cx)
        {
            return;
        }
        let git = &mut self.areas.git;
        let Some(entry) = git.panels.entries.get_mut(address) else {
            return;
        };
        let Some(reads) = &mut entry.reads else {
            return;
        };
        // Duplicate deferred first-page callbacks do not consume a new key.
        if reads
            .file(&captured.address.path)
            .is_some_and(|read| read.outstanding().is_some())
        {
            return;
        }
        let Ok(key) = git.keys.issue() else {
            entry.problem = Some(ReadFailure::KeysExhausted);
            cx.notify();
            return;
        };
        let Some(ticket) = reads.begin_captured_file(&captured.file, key) else {
            return;
        };
        self.command(vessel_app_link::Command::GitPanel(Ask::File {
            address: address.clone(),
            context: captured.context.clone(),
            ticket,
        }));
        cx.notify();
    }
}

#[cfg(test)]
#[path = "bodies_tests.rs"]
mod tests;
