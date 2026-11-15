//! Platform selection captures its destination before asynchronous inspection.

use super::super::attach::{Incoming, LocalFileProblem};
use crate::shell::Shell;
use gpui::PathPromptOptions;
use vessel_app_kit::field::composer_hooks::{KeyAnswer, Pasted};
use vessel_app_kit::ui::{Context, SharedString, Window};
use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::toasts::ToastTone;

/// The existing menu keeps a typed action instead of overloading a row index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Acquire {
    Image,
    ClipboardImage,
    File,
}

impl Shell {
    fn file_destination(&self, new_chat: bool) -> Option<DraftKey> {
        if new_chat {
            Some(DraftKey::New)
        } else {
            self.session_tray_key()
        }
    }

    /// Images/files consume paste; ordinary text still goes to the editor.
    pub(crate) fn composer_pasted(
        &mut self,
        new_chat: bool,
        pasted: Vec<Pasted>,
        cx: &mut Context<'_, Self>,
    ) -> KeyAnswer {
        if pasted.is_empty() {
            return KeyAnswer::Pass;
        }
        let Some(key) = self.file_destination(new_chat) else {
            return KeyAnswer::Pass;
        };
        for paste in pasted {
            match paste {
                Pasted::Image { mime, bytes } => self.composer_pasted_image(key, mime, bytes, cx),
                Pasted::Files(paths) => self.inspect_paths(key, paths, cx),
            }
        }
        KeyAnswer::Taken
    }

    /// Clipboard is acquired immediately; dialogs capture their destination
    /// before awaiting the operating system, just like drops and key paste.
    pub(super) fn composer_choose(
        &mut self,
        choice: Acquire,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let new_chat = self.areas.composer.attach_for.take().unwrap_or(false);
        let Some(key) = self.file_destination(new_chat) else {
            return;
        };
        if choice == Acquire::ClipboardImage {
            let image = cx.read_from_clipboard().and_then(|item| {
                item.entries().iter().find_map(|entry| match entry {
                    gpui::ClipboardEntry::Image(image) => {
                        Some((image.format.mime_type(), image.bytes.clone()))
                    }
                    gpui::ClipboardEntry::String(_) | gpui::ClipboardEntry::ExternalPaths(_) => {
                        None
                    }
                })
            });
            match image {
                Some((mime, bytes)) => self.composer_pasted_image(key, mime, bytes, cx),
                None => self.toast(
                    self.words.composer().image_clipboard_empty(),
                    ToastTone::Error,
                    cx,
                ),
            }
            if let Some(field) = self.composer_field(key) {
                field.focus(window, cx);
            }
            return;
        }
        let words = self.words.composer();
        let prompt = match choice {
            Acquire::Image => words.attach_image(),
            Acquire::File => words.attach_file(),
            Acquire::ClipboardImage => return,
        };
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some(SharedString::from(prompt.to_owned())),
        });
        cx.spawn(async move |this, cx| match picked.await {
            Ok(Ok(Some(paths))) => {
                let _closed = this.update(cx, |shell, cx| shell.inspect_paths(key, paths, cx));
            }
            Ok(Ok(None)) => {}
            Ok(Err(_)) | Err(_) => {
                let _closed = this.update(cx, |shell, cx| {
                    shell.toast(
                        shell.words.composer().file_pick_failed(),
                        ToastTone::Error,
                        cx,
                    );
                });
            }
        })
        .detach();
    }

    fn composer_pasted_image(
        &mut self,
        key: DraftKey,
        mime: &str,
        bytes: Vec<u8>,
        cx: &mut Context<'_, Self>,
    ) {
        if matches!(
            mime,
            "image/png" | "image/jpeg" | "image/gif" | "image/webp"
        ) {
            self.composer_add(key, vec![Incoming::pasted(mime, bytes)], cx);
        } else {
            self.toast(
                self.words.composer().image_format_unsupported(),
                ToastTone::Error,
                cx,
            );
        }
    }

    /// Dropped paths belong to the composer under the drop, even if the
    /// person changes conversations before inspection returns.
    pub(super) fn composer_paths(
        &mut self,
        new_chat: bool,
        paths: Vec<std::path::PathBuf>,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(key) = self.file_destination(new_chat) {
            self.inspect_paths(key, paths, cx);
        }
    }

    fn inspect_paths(
        &mut self,
        key: DraftKey,
        paths: Vec<std::path::PathBuf>,
        cx: &mut Context<'_, Self>,
    ) {
        let inspected = cx
            .background_executor()
            .spawn(async move { paths.into_iter().map(Incoming::file).collect::<Vec<_>>() });
        cx.spawn(async move |this, cx| {
            let results = inspected.await;
            let _closed = this.update(cx, |shell, cx| {
                let mut files = Vec::new();
                for result in results {
                    match result {
                        Ok(file) => files.push(file),
                        Err(why) => {
                            if let LocalFileProblem::Read(error) = &why {
                                tracing::info!(kind = ?error.kind(), "an attached file could not be inspected");
                            }
                            shell.toast(shell.words.composer().file_unreadable(), ToastTone::Error, cx);
                        }
                    }
                }
                shell.composer_add(key, files, cx);
            });
        }).detach();
    }
}

#[cfg(test)]
mod tests;
