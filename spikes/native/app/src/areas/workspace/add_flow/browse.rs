//! Add project's Browse (TRACKER 816): offered on this computer's own
//! machine, it opens the system folder chooser and adds the folder picked
//! as a directory search row adds one (Paseo's `browse` in
//! `add-project-flow.tsx`: `pickDirectory`, then `openAddedProject(path,
//! "method")`).
//!
//! [Decision Log]
//! - Purpose: Paseo's fourth method on the desktop app.
//! - Constraint: the chooser shows this computer's folders, so it is only
//!   offered where the node runs here (`view::this_computer`, decided by
//!   the link per fleet read). The chooser answers later, without the
//!   window, and the sheet may have moved or closed by then.
//! - Choice: GPUI's `prompt_for_paths` for one folder (each desktop's
//!   chooser can create one, as Paseo's `createDirectory`), one at a time;
//!   its folder is asked with `project.add` from the method page, whose
//!   answer is said there. A cancel does nothing; a chooser that fails, or
//!   a folder whose path is not UTF-8, says Paseo's "Unable to browse for
//!   a directory".
//! - Cost: the browser build has no chooser; there the link never names
//!   this computer, so Browse is never offered.

use std::path::PathBuf;

use gpui::PathPromptOptions;
use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::{Ask, ProjectAsk};
use vessel_app_view::workspace::add_flow::model::PageId;
use vessel_proto::NodeId;
use vessel_proto::workspace::AddProject;

use crate::shell::Shell;

/// What the chooser gave.
enum Picked {
    Folder(String),
    Cancelled,
    Failed,
}

impl Picked {
    fn of(paths: Option<Vec<PathBuf>>) -> Self {
        match paths {
            None => Self::Cancelled,
            Some(paths) => paths
                .into_iter()
                .next()
                .and_then(|path| path.into_os_string().into_string().ok())
                .map_or(Self::Failed, Self::Folder),
        }
    }
}

impl Shell {
    /// The link's nodes on this computer, sent with every fleet read
    /// (2 s): drawn again only when they changed.
    pub(crate) fn add_flow_this_computer(
        &mut self,
        nodes: Vec<NodeId>,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.workspace.add_flow.this_computer == nodes {
            return;
        }
        self.areas.workspace.add_flow.this_computer = nodes;
        self.add_flow_fleet();
        cx.notify();
    }

    /// Whether `node` runs on this computer.
    pub(super) fn on_this_computer(&self, node: Option<NodeId>) -> bool {
        node.is_some_and(|node| self.areas.workspace.add_flow.this_computer.contains(&node))
    }

    /// Browse chosen on `machine`'s methods: the chooser, once at a time.
    pub(super) fn add_flow_browse(&mut self, machine: String, cx: &mut Context<'_, Self>) {
        let sheet = &mut self.areas.workspace.add_flow;
        if sheet.browsing {
            return;
        }
        sheet.browsing = true;
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            let picked = match picked.await {
                Ok(Ok(paths)) => Picked::of(paths),
                Ok(Err(_)) | Err(_) => Picked::Failed,
            };
            let _closed = this.update(cx, |shell, cx| shell.add_flow_browsed(&machine, picked, cx));
        })
        .detach();
    }

    /// The chooser answered: add the folder from the method page, if the
    /// sheet is still there.
    fn add_flow_browsed(&mut self, machine: &str, picked: Picked, cx: &mut Context<'_, Self>) {
        let words = self.t.lang.add_flow();
        let sheet = &mut self.areas.workspace.add_flow;
        sheet.browsing = false;
        let Some(open) = &mut sheet.open else {
            return;
        };
        let page = open.flow.page();
        if page.kind.id() != PageId::Method || page.kind.machine() != Some(machine) {
            return;
        }
        match picked {
            Picked::Cancelled => return,
            Picked::Failed => open
                .flow
                .fail(PageId::Method, words.cannot_browse.to_owned()),
            Picked::Folder(path) => {
                open.flow.begin(PageId::Method);
                self.ask(
                    machine,
                    Ask::Project(ProjectAsk::Add(AddProject {
                        path: path.into(),
                        name: None,
                    })),
                );
            }
        }
        cx.notify();
    }
}
