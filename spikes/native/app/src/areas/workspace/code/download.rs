//! The Explorer's Download (Paseo's `fileActions.download`, ADR 0105): the
//! person chooses where the file goes, the link fetches it there, and a
//! toast says how it ended (Paseo's download toast).
//!
//! [Decision Log]
//! - Purpose: keep a copy of a project file on this computer.
//! - Constraint: the file is on the node's machine; this computer's folder
//!   is the person's to choose. Paseo's desktop hands the bytes to the
//!   browser's download, which asks where to save.
//! - Choice: the platform's save dialog, starting in the Downloads folder
//!   with the file's name; the link writes there only once the whole
//!   version arrived (`link::workspace::download`). A refusal, a changed
//!   file, a write that failed or a save dialog the platform could not show
//!   is one toast, as Paseo's.
//! - Cost: no progress is shown while the pages arrive (Paseo shows a
//!   toast that fills); a 64 MiB file is at most 256 pages.

use std::path::PathBuf;

use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::{Ask, Download, FileAsk};
use vessel_app_view::toasts::ToastTone;
use vessel_app_view::workspace::file_panels::DownloadFailure;
use vessel_proto::workspace::RelPath;

use crate::shell::Shell;

impl Shell {
    /// Ask where `path` goes on this computer, then download it there.
    pub(super) fn choose_download(&mut self, path: RelPath, cx: &mut Context<'_, Self>) {
        let Some((machine, root)) = self
            .areas
            .workspace
            .code
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        let name = path.file_name().unwrap_or_default().to_owned();
        let chosen = cx.prompt_for_new_path(&start_folder(), Some(&name));
        cx.spawn(async move |this, cx| {
            let picked = chosen.await;
            let _gone = this.update(cx, |shell, cx| match picked {
                Ok(Ok(Some(to))) => {
                    let download = Download { root, path, to };
                    shell.ask(&machine, Ask::File(FileAsk::Download(download)));
                }
                // The platform could not show its save dialog (no desktop
                // portal): say so rather than nothing.
                Ok(Err(error)) => {
                    let failure = DownloadFailure::Local(error.to_string());
                    shell.file_downloaded(Err(failure), cx);
                }
                // The person closed the dialog, or the window went.
                Ok(Ok(None)) | Err(_) => {}
            });
        })
        .detach();
    }

    /// A download ended: a toast says so.
    pub(in crate::areas::workspace) fn file_downloaded(
        &mut self,
        result: Result<(), DownloadFailure>,
        cx: &mut Context<'_, Self>,
    ) {
        let words = self.words();
        let labels = words.labels();
        let (text, tone) = match result {
            Ok(()) => (labels.download_complete.to_owned(), ToastTone::Ok),
            Err(DownloadFailure::Call(problem)) => (words.problem(&problem), ToastTone::Error),
            Err(DownloadFailure::Inconsistent) => {
                (labels.download_changed.to_owned(), ToastTone::Error)
            }
            Err(DownloadFailure::Local(why)) => (
                format!("{}: {why}", labels.download_failed),
                ToastTone::Error,
            ),
        };
        self.toast(text, tone, cx);
    }
}

/// Where the save dialog starts: the Downloads folder, else home.
#[cfg(not(target_arch = "wasm32"))]
fn start_folder() -> PathBuf {
    dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_default()
}

/// A browser has no folders to start in.
#[cfg(target_arch = "wasm32")]
fn start_folder() -> PathBuf {
    PathBuf::new()
}
