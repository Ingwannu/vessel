//! Platform selection captures its destination before asynchronous inspection.

use super::super::attach::{Incoming, LocalFileProblem};
mod images;
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
        self.composer_destination().filter(|key| match key {
            DraftKey::New => new_chat,
            DraftKey::Session { .. } => !new_chat,
            DraftKey::Workspace { .. } => false,
        })
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
        self.composer_pasted_at(key, pasted, cx)
    }

    pub(crate) fn composer_pasted_at(
        &mut self,
        key: DraftKey,
        pasted: Vec<Pasted>,
        cx: &mut Context<'_, Self>,
    ) -> KeyAnswer {
        if pasted.is_empty() || self.composer_field(key).is_none() {
            return KeyAnswer::Pass;
        }
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
        let Some(key) = self
            .areas
            .composer
            .attach_for
            .take()
            .filter(|key| self.composer_field(*key).is_some())
        else {
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
        _mime: &str,
        bytes: Vec<u8>,
        cx: &mut Context<'_, Self>,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        let store = self.areas.composer.local.store.clone();
        #[cfg(not(target_arch = "wasm32"))]
        if self.areas.composer.local.production && store.is_none() {
            self.toast(
                self.words.composer().file_local_failed(),
                ToastTone::Error,
                cx,
            );
            return;
        }
        self.local_intake_started(key, cx);
        let saved = cx.background_executor().spawn(async move {
            let normalized = vessel_app_kit::image_input::normalize(
                bytes,
                vessel_app_kit::image_input::InputLimits {
                    source: vessel_proto::AttachmentLimits::DEFAULT.file,
                    result: vessel_proto::AttachmentLimits::DEFAULT.image,
                },
            )
            .map_err(LocalFileProblem::Image)?;
            let incoming = Incoming::pasted(normalized.format.mime(), normalized.bytes);
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(store) = store {
                use vessel_app_view::composer::attach::{attachment_name, mime_of};
                let name =
                    attachment_name(&incoming.name, "image.png").ok_or(LocalFileProblem::NoName)?;
                let mime = mime_of(&incoming.name).ok_or(LocalFileProblem::NoName)?;
                let vessel_app_view::composer::tray::Origin::Pasted(bytes) = incoming.origin else {
                    return Err(LocalFileProblem::NoName);
                };
                return store
                    .put_bytes(name, mime, &bytes)
                    .map(Incoming::stored)
                    .map_err(LocalFileProblem::Storage);
            }
            Ok(incoming)
        });
        cx.spawn(async move |this, cx| {
            let result = saved.await;
            let _closed = this.update(cx, |shell, cx| {
                shell.local_intake_finished(key, cx);
                match result {
                    Ok(incoming) => shell.composer_add(key, vec![incoming], cx),
                    Err(problem) => {
                        tracing::info!(?problem, "image intake refused before preview");
                        let message = match problem {
                            LocalFileProblem::Image(_) => {
                                shell.words.composer().image_format_unsupported()
                            }
                            LocalFileProblem::Read(_)
                            | LocalFileProblem::NoName
                            | LocalFileProblem::NotFile => shell.words.composer().file_unreadable(),
                            #[cfg(not(target_arch = "wasm32"))]
                            LocalFileProblem::Storage(_) => {
                                shell.words.composer().file_local_failed()
                            }
                        };
                        shell.toast(message, ToastTone::Error, cx);
                    }
                }
            });
        })
        .detach();
    }

    /// Dropped paths belong to the composer under the drop, even if the
    /// person changes conversations before inspection returns.
    pub(super) fn composer_paths_into(
        &mut self,
        key: DraftKey,
        paths: Vec<std::path::PathBuf>,
        cx: &mut Context<'_, Self>,
    ) {
        self.inspect_paths(key, paths, cx);
    }

    fn inspect_paths(
        &mut self,
        key: DraftKey,
        paths: Vec<std::path::PathBuf>,
        cx: &mut Context<'_, Self>,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        let store = self.areas.composer.local.store.clone();
        #[cfg(not(target_arch = "wasm32"))]
        if self.areas.composer.local.production && store.is_none() {
            self.toast(
                self.words.composer().file_local_failed(),
                ToastTone::Error,
                cx,
            );
            return;
        }
        self.local_intake_started(key, cx);
        let inspected = cx.background_executor().spawn(async move {
            paths
                .into_iter()
                .map(|path| {
                    let incoming = Incoming::file(path.clone())?;
                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(store) = &store {
                        use vessel_app_view::composer::attach::{attachment_name, mime_of};
                        let name = attachment_name(&incoming.name, "file.bin")
                            .ok_or(LocalFileProblem::NoName)?;
                        let mime = mime_of(&incoming.name).ok_or(LocalFileProblem::NoName)?;
                        let file = store
                            .copy_file(name, mime, &path)
                            .map_err(LocalFileProblem::Storage)?;
                        if images::is_image(&incoming.name) {
                            let normalized = images::normalize(&file.path, &incoming.name)?;
                            return images::store_image(store, normalized);
                        }
                        return Ok(Incoming::stored(file));
                    }
                    if images::is_image(&incoming.name) {
                        return images::normalize(&path, &incoming.name);
                    }
                    Ok(incoming)
                })
                .collect::<Vec<_>>()
        });
        cx.spawn(async move |this, cx| {
            let results = inspected.await;
            let _closed = this.update(cx, |shell, cx| {
                shell.local_intake_finished(key, cx);
                let mut files = Vec::new();
                for result in results {
                    match result {
                        Ok(file) => files.push(file),
                        Err(why) => {
                            if let LocalFileProblem::Read(error) = &why {
                                tracing::info!(kind = ?error.kind(), "an attached file could not be inspected");
                            }
                            let words = shell.words.composer();
                            let message = match &why {
                                LocalFileProblem::Read(_) | LocalFileProblem::NoName | LocalFileProblem::NotFile => words.file_unreadable(),
                                LocalFileProblem::Image(_) => words.image_format_unsupported(),
                                #[cfg(not(target_arch = "wasm32"))]
                                LocalFileProblem::Storage(problem) => {
                                    tracing::info!(%problem, "attached bytes could not be retained locally");
                                    words.file_local_failed()
                                }
                            };
                            shell.toast(message, ToastTone::Error, cx);
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
