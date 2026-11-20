//! Open in editor and reveal in the file manager (TRACKER 467, ADR 0123),
//! for a machine whose node runs on this computer: the session header's
//! Open with its editor chooser (`header.rs`), the Files menu's "Open in"
//! and "Reveal in" (`code/actions.rs`), and the sidebar's "Open in file
//! manager" (`sidebar/row_menu.rs`).
//!
//! [Decision Log]
//! - Purpose: Paseo's desktop opens the workspace, a folder or the file
//!   shown in the person's editor and reveals entries in the file manager
//!   (`workspace/open-in-editor/`, `open-in-file-manager/`); the native app
//!   is a desktop app and does the same.
//! - Constraint: a path is the machine's; it means something here only
//!   when the machine's node runs on this computer (816's `ThisComputer`).
//!   Listing and launching read the disk and PATH: never on the UI thread.
//! - Choice: the view's rules (`open_targets`) over a probe of this
//!   computer, run on the background executor. The list is kept per
//!   workspace for 60 s (Paseo's `staleTime`), asked again when shown after
//!   that. A launch checks its paths (canonical, existing, under the
//!   root), plans, and starts through the [`Launcher`] seam; a reveal or a
//!   folder goes to GPUI on the UI thread. A failure is a toast.
//! - Cost: the first header after a session opens has no button until its
//!   list arrives (one directory walk of PATH).

mod header;
mod launcher;
#[cfg(not(target_arch = "wasm32"))]
mod system;
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use vessel_app_kit::agent_mark::{self, Artwork};
use vessel_app_kit::picker::ComboboxState;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_view::open_targets::{
    EditorMark, Kind, Plan, Platform, Probe, Refused, Request, TargetId, Unavailable, checked,
    list, plan, preferred,
};
use vessel_app_view::toasts::ToastTone;
use vessel_proto::ProjectId;

pub(crate) use header::open_button;
pub(crate) use launcher::Launcher;

use crate::shell::Shell;

/// What the rules read this computer through, from any thread.
pub(crate) type SharedProbe = Arc<dyn Probe + Send + Sync>;

/// How long a workspace's list is used before it is asked again.
const FRESH: Duration = Duration::from_secs(60);

/// The most workspaces whose lists are kept.
const MAX_LISTED: usize = 32;

/// A workspace's targets and when they were read.
struct Listed {
    at: Instant,
    targets: Vec<TargetId>,
}

/// The state of Open in editor.
pub(crate) struct OpenTargets {
    listed: HashMap<PathBuf, Listed>,
    /// Workspaces being listed (asked from a frame, which reads the state).
    pending: RefCell<HashSet<PathBuf>>,
    /// The header's editor chooser.
    chooser: ComboboxState,
    /// This computer; `None` where no machine can be on it.
    probe: Option<SharedProbe>,
    launcher: Arc<dyn Launcher>,
}

impl OpenTargets {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        let (probe, launcher) = this_computer();
        Self {
            listed: HashMap::new(),
            pending: RefCell::default(),
            chooser: ComboboxState::new(window, cx),
            probe,
            launcher,
        }
    }

    /// Read this computer through `probe` and start through `launcher`.
    #[cfg(test)]
    pub(crate) fn set_computer(&mut self, probe: SharedProbe, launcher: Arc<dyn Launcher>) {
        self.probe = Some(probe);
        self.launcher = launcher;
        self.listed.clear();
    }

    /// The file manager of this computer's platform (always there).
    pub(crate) fn file_manager(&self) -> Option<TargetId> {
        Some(match self.probe.as_ref()?.platform() {
            Platform::MacOs => TargetId::Finder,
            Platform::Windows => TargetId::Explorer,
            Platform::Other => TargetId::FileManager,
        })
    }

    fn keep(&mut self, root: PathBuf, targets: Vec<TargetId>) {
        let _done = self.pending.borrow_mut().remove(&root);
        if self.listed.len() >= MAX_LISTED && !self.listed.contains_key(&root) {
            let oldest = self
                .listed
                .iter()
                .min_by_key(|(_, listed)| listed.at)
                .map(|(root, _)| root.clone());
            if let Some(oldest) = oldest {
                let _dropped = self.listed.remove(&oldest);
            }
        }
        let _before = self.listed.insert(
            root,
            Listed {
                at: Instant::now(),
                targets,
            },
        );
    }
}

/// This computer and the launcher that starts programs on it.
#[cfg(all(not(test), not(target_arch = "wasm32")))]
fn this_computer() -> (Option<SharedProbe>, Arc<dyn Launcher>) {
    (
        Some(Arc::new(system::Computer::here())),
        Arc::new(system::System),
    )
}

/// Tests set their own computer; the page's build has none.
#[cfg(any(test, target_arch = "wasm32"))]
fn this_computer() -> (Option<SharedProbe>, Arc<dyn Launcher>) {
    (None, Arc::new(launcher::Nothing))
}

/// The kit's artwork for a view's editor mark.
pub(crate) const fn artwork(mark: EditorMark) -> Artwork {
    match mark {
        EditorMark::Cursor => agent_mark::CURSOR,
        EditorMark::VsCode => agent_mark::VSCODE,
        EditorMark::Zed => agent_mark::ZED,
        EditorMark::AndroidStudio => agent_mark::ANDROID_STUDIO,
        EditorMark::Xcode => agent_mark::XCODE,
        EditorMark::WebStorm => agent_mark::WEBSTORM,
    }
}

/// Where an open was asked from: it picks a failure's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    /// The header's Open, or a folder's "Open in" in Files.
    Editor,
    /// "Reveal in" in Files.
    Reveal,
    /// The sidebar's "Open in file manager" (Paseo says only that the
    /// folder could not be opened).
    Sidebar,
}

/// What a launch did off the UI thread.
enum Done {
    Started,
    Reveal(PathBuf),
    OpenFolder(PathBuf),
}

/// Why a launch did nothing.
enum Failure {
    Refused(Refused),
    Unavailable(Unavailable),
    Start(io::Error),
}

/// Check, plan and start `target` for `request` (off the UI thread).
fn prepare(
    target: TargetId,
    request: &Request,
    probe: &dyn Probe,
    launcher: &dyn Launcher,
) -> Result<Done, Failure> {
    let request = checked(request, probe).map_err(Failure::Refused)?;
    match plan(target, &request, probe).map_err(Failure::Unavailable)? {
        Plan::Start { program, args } => launcher
            .start(&program, &args)
            .map(|()| Done::Started)
            .map_err(Failure::Start),
        Plan::Reveal(path) => Ok(Done::Reveal(path)),
        Plan::OpenFolder(path) => Ok(Done::OpenFolder(path)),
    }
}

impl Shell {
    /// Whether `machine`'s node runs on this computer.
    pub(crate) fn machine_here(&self, machine: &str) -> bool {
        let node = self
            .fleet
            .machines
            .iter()
            .find(|known| known.name == machine)
            .and_then(|known| known.node);
        self.on_this_computer(node)
    }

    /// The targets this computer has for the workspace at `root`, as last
    /// listed; listed again when not yet or over a minute ago.
    pub(crate) fn open_targets(&self, root: &Path, cx: &mut Context<'_, Self>) -> Option<&[TargetId]> {
        let listed = self.areas.workspace.open.listed.get(root);
        if listed.is_none_or(|listed| listed.at.elapsed() >= FRESH) {
            self.list_open_targets(root, cx);
        }
        listed.map(|listed| listed.targets.as_slice())
    }

    fn list_open_targets(&self, root: &Path, cx: &mut Context<'_, Self>) {
        let open = &self.areas.workspace.open;
        let Some(probe) = open.probe.clone() else {
            return;
        };
        if !open.pending.borrow_mut().insert(root.to_path_buf()) {
            return;
        }
        let root = root.to_path_buf();
        cx.spawn(async move |this, cx| {
            let at = root.clone();
            let targets = cx
                .background_executor()
                .spawn(async move { list(&*probe, Some(&at)) })
                .await;
            let _gone = this.update(cx, |shell, cx| {
                shell.areas.workspace.open.keep(root, targets);
                cx.notify();
            });
        })
        .detach();
    }

    /// The Files menu's targets for the open session on this computer: the
    /// preferred editor for a folder's "Open in", and the file manager.
    pub(crate) fn files_open_targets(
        &self,
        cx: &mut Context<'_, Self>,
    ) -> (Option<TargetId>, Option<TargetId>) {
        let Some(shown) = self.shown_workspace() else {
            return (None, None);
        };
        let editor = self
            .open_targets(&shown.root, cx)
            .and_then(|listed| self.preferred_editor(&shown.machine, shown.project, listed));
        (editor, self.areas.workspace.open.file_manager())
    }

    /// The target Open uses for `project` on `machine` among `listed`
    /// (Paseo's `resolvePreferredEditorId`).
    pub(crate) fn preferred_target(
        &self,
        machine: &str,
        project: Option<ProjectId>,
        listed: &[TargetId],
    ) -> Option<TargetId> {
        let stored = project.and_then(|project| {
            self.prefs
                .settings
                .preferred_editors
                .get(machine, project)
        });
        preferred(listed, stored)
    }

    /// The editor a folder's "Open in" uses: the preferred among editors.
    pub(crate) fn preferred_editor(
        &self,
        machine: &str,
        project: Option<ProjectId>,
        listed: &[TargetId],
    ) -> Option<TargetId> {
        let editors: Vec<TargetId> = listed
            .iter()
            .copied()
            .filter(|target| target.kind() == Kind::Editor)
            .collect();
        self.preferred_target(machine, project, &editors)
    }

    /// Keep `target` as the one Open uses for `project` on `machine`
    /// (Paseo keeps nothing without a project).
    pub(crate) fn choose_target(
        &mut self,
        machine: &str,
        project: Option<ProjectId>,
        target: TargetId,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(project) = project else {
            return;
        };
        if self
            .prefs
            .settings
            .preferred_editors
            .set(machine, project, target)
        {
            self.command(Command::SavePrefs(Box::new(self.prefs.clone())));
        }
        cx.notify();
    }

    /// Open `request` in `target`: checked, planned and started off the UI
    /// thread; a reveal or a folder on it; a failure said in a toast.
    pub(crate) fn open_in_target(
        &self,
        target: TargetId,
        request: Request,
        origin: Origin,
        cx: &mut Context<'_, Self>,
    ) {
        let open = &self.areas.workspace.open;
        let Some(probe) = open.probe.clone() else {
            return;
        };
        let launcher = Arc::clone(&open.launcher);
        cx.spawn(async move |this, cx| {
            let starter = Arc::clone(&launcher);
            let done = cx
                .background_executor()
                .spawn(async move { prepare(target, &request, &*probe, &*starter) })
                .await;
            let _gone = this.update(cx, |shell, cx| match done {
                Ok(Done::Started) => {}
                Ok(Done::Reveal(path)) => launcher.reveal(&path, cx),
                Ok(Done::OpenFolder(path)) => launcher.open_folder(&path, cx),
                Err(failure) => {
                    let words = shell.failure_words(target, failure, origin);
                    shell.toast(words, ToastTone::Error, cx);
                }
            });
        })
        .detach();
    }

    fn failure_words(&self, target: TargetId, failure: Failure, origin: Origin) -> String {
        let words = self.t.lang.open_targets();
        if let Failure::Start(error) = &failure {
            tracing::warn!(%error, target = target.id(), "a target did not start");
        }
        if origin == Origin::Sidebar {
            return words.open_folder_failed.to_owned();
        }
        match failure {
            Failure::Refused(refused) => words.refused(&refused),
            Failure::Unavailable(unavailable) => words.unavailable(&unavailable),
            Failure::Start(_error) => words.start_failed(target),
        }
    }
}
