//! The shown file's watch carried out (`view::workspace::watch` decides,
//! ADR 0105): an observation asked while the Files tab shows the file on a
//! machine that has `file.observe`, its answer reading the file again when
//! it changed, and the pause before the next ask.
//!
//! [Decision Log]
//! - Purpose: Paseo's open file follows the disk while its tab is on screen
//!   (`file-panels/file/handle.ts`, `setObserving`).
//! - Constraint: one file is shown at a time here; answers come without a
//!   window and in any order, so each is matched to the observation out.
//! - Choice: ask after the shown file's read answers, when the panel opens
//!   on Files or the tab turns to Files, and after each pause; a change runs
//!   the same re-read a git write does (`reread_shown_file`). Hidden, the
//!   watch asks nothing more; shown again, its anchor reports what changed
//!   meanwhile (Paseo: "coming back reports what changed meanwhile").
//! - Cost: an observation out when the file is left still ends on the node
//!   at its heartbeat; its answer finds no viewer and is dropped.

use std::time::{Duration, Instant};

use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::{Ask, FileAsk};
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::watch::{Answered, offered};
use vessel_proto::workspace::panel_files::{ObserveFile, ObservedFile};
use vessel_proto::workspace::{ContentHash, ReadFile, RelPath};

use super::Tab;
use crate::shell::Shell;

impl Shell {
    /// Ask the shown file's next observation: the panel is out on Files,
    /// its machine has `file.observe`, and the watch has none out.
    pub(super) fn code_watch(&mut self) {
        let panel = &self.areas.workspace.code;
        if !panel.open || panel.tab != Tab::Files {
            return;
        }
        let Some((machine, root)) = panel
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        let protocol = self
            .fleet
            .machines
            .iter()
            .find(|known| known.name == machine)
            .and_then(|known| known.protocol);
        if !offered(protocol) {
            return;
        }
        let Some(viewer) = self.areas.workspace.code.files.viewer.as_mut() else {
            return;
        };
        if let Some(ask) = viewer.watch.ask(root, viewer.path.clone(), Instant::now()) {
            self.ask(&machine, Ask::File(FileAsk::Observe(ask)));
        }
    }

    /// A read of `ask` answered with content `seen`: what the panel shows
    /// read-only now; then the watch may ask.
    pub(in crate::areas::workspace) fn code_file_seen(
        &mut self,
        ask: &ReadFile,
        seen: Option<ContentHash>,
    ) {
        let panel = &mut self.areas.workspace.code;
        if panel.root().map(|(_, root)| root) != Some(&ask.root) {
            return;
        }
        if let Some(viewer) = panel
            .files
            .viewer
            .as_mut()
            .filter(|viewer| viewer.path == ask.path)
        {
            viewer.watch.seen(seen);
        }
        self.code_watch();
    }

    /// An observation of the shown file answered.
    pub(in crate::areas::workspace) fn file_observed(
        &mut self,
        ask: &ObserveFile,
        result: Result<ObservedFile, Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        let panel = &mut self.areas.workspace.code;
        if panel.root().map(|(_, root)| root) != Some(&ask.root) {
            return;
        }
        let Some(viewer) = panel
            .files
            .viewer
            .as_mut()
            .filter(|viewer| viewer.path == ask.path)
        else {
            return;
        };
        let editor = viewer
            .edit
            .as_ref()
            .and_then(|edit| edit.model.version())
            .and_then(|version| version.hash.clone());
        let now = Instant::now();
        let answered = viewer.watch.answered(ask, result, editor.as_ref(), now);
        let (rest, wait) = match answered {
            Answered::Stale | Answered::Stopped => return,
            Answered::Quiet { rest, wait } => (rest, wait),
            Answered::Changed { rest, wait } => {
                self.reread_shown_file();
                (rest, wait)
            }
        };
        rest_watch(ask.path.clone(), rest, wait, cx);
    }
}

/// After the pause `rest` of `wait`, the watch of `path` (if still shown)
/// may ask again.
fn rest_watch(path: RelPath, rest: u64, wait: Duration, cx: &mut Context<'_, Shell>) {
    cx.spawn(async move |this, cx| {
        cx.background_executor().timer(wait).await;
        let _gone = this.update(cx, |shell, _cx| {
            let viewer = shell.areas.workspace.code.files.viewer.as_mut();
            if let Some(viewer) = viewer.filter(|viewer| viewer.path == path) {
                viewer.watch.rested(rest);
            }
            shell.code_watch();
        });
    })
    .detach();
}
