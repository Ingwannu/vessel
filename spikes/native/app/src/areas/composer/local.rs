//! Byte persistence is separate from transport; recovered uncertain inputs stay
//! locked until the canonical receipt owner can reconcile their delivery.
use crate::{areas::Areas, shell::Shell};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use vessel_app_kit::ui::Context;
#[cfg(not(target_arch = "wasm32"))]
use vessel_app_link::attachments::local::{KeptFile, LocalFailure, Manifest, Store};
use vessel_app_view::composer::{drafts::DraftKey, tray::Origin};

#[derive(Debug, Default)]
enum Writing {
    #[default]
    Idle,
    Scheduled,
    Active {
        dirty: bool,
    },
}
#[derive(Debug, Default)]
pub(super) struct LocalState {
    /// None is the isolated headless-test/browser mode; production initializes
    /// explicitly, so tests never resolve the operator's default storage path.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) store: Option<Store>,
    pub(super) production: bool,
    pending: HashMap<DraftKey, usize>,
    recovered: HashSet<DraftKey>,
    writing: Writing,
}
const SAVE_DELAY: Duration = Duration::from_millis(200);

/// Only the real startup calls this; `with_drafts` remains isolated for tests.
pub(crate) fn restore_local(areas: &mut Areas) {
    areas.composer.local.production = true;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let loaded = (|| {
            let store = Store::open()?;
            let manifest = store.load_manifest()?;
            let files = manifest
                .files()
                .iter()
                .map(|kept| {
                    store
                        .get(&kept.record)
                        .map(|file| (kept.draft, kept.unconfirmed, file))
                })
                .collect::<Result<Vec<_>, LocalFailure>>()?;
            Ok::<_, LocalFailure>((store, files))
        })();
        match loaded {
            Ok((store, files)) => {
                areas.composer.local.store = Some(store);
                for (draft, unconfirmed, file) in files {
                    let tray = areas.composer.trays.tray(draft);
                    let _key = tray.add(
                        file.record.name.as_str(),
                        file.record.size.value(),
                        Origin::Stored(std::sync::Arc::clone(&file)),
                    );
                    if unconfirmed {
                        let _new = areas.composer.local.recovered.insert(draft);
                    }
                }
            }
            Err(problem) => {
                tracing::warn!(%problem, "local attachment state could not be restored; preserved without collection");
            }
        }
    }
}
impl Shell {
    /// Both create and existing-session entry points honor asynchronous intake
    /// and recovered unknown delivery before any transport or draft clearing.
    pub(crate) fn composer_attachments_blocked(&self, key: DraftKey) -> bool {
        self.areas
            .composer
            .local
            .pending
            .get(&key)
            .is_some_and(|count| *count > 0)
            || self.areas.composer.local.recovered.contains(&key)
    }
    pub(super) fn local_intake_started(&mut self, key: DraftKey, cx: &mut Context<'_, Self>) {
        let count = self.areas.composer.local.pending.entry(key).or_default();
        *count = count.saturating_add(1);
        cx.notify();
    }
    pub(super) fn local_intake_finished(&mut self, key: DraftKey, cx: &mut Context<'_, Self>) {
        if let Some(count) = self.areas.composer.local.pending.get_mut(&key) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                let _old = self.areas.composer.local.pending.remove(&key);
            }
        }
        cx.notify();
    }
    pub(super) fn local_files_changed(&mut self, cx: &mut Context<'_, Self>) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.areas.composer.local.store.is_none() {
                return;
            }
            match &mut self.areas.composer.local.writing {
                Writing::Idle => self.areas.composer.local.writing = Writing::Scheduled,
                Writing::Scheduled => return,
                Writing::Active { dirty } => {
                    *dirty = true;
                    return;
                }
            }
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(SAVE_DELAY).await;
                let work = this.update(cx, |shell, _cx| {
                    shell.areas.composer.local.writing = Writing::Active { dirty: false };
                    let store = shell.areas.composer.local.store.clone()?;
                    Some((store, shell.local_manifest()))
                });
                let Ok(Some((store, manifest))) = work else {return;};
                let result = cx.background_executor().spawn(async move {
                    let manifest = manifest?;
                    store.save_manifest(&manifest)?;
                    let _removed = store.collect(&manifest)?;
                    Ok::<_, LocalFailure>(())
                }).await;
                let _closed = this.update(cx, |shell, cx| {
                    if let Err(problem) = result {
                        tracing::warn!(%problem, "local attachment persistence failed; bytes retained");
                        shell.toast(shell.words.composer().file_local_failed(), vessel_app_view::toasts::ToastTone::Error, cx);
                    }
                    let again = matches!(shell.areas.composer.local.writing, Writing::Active { dirty: true });
                    shell.areas.composer.local.writing = Writing::Idle;
                    if again {shell.local_files_changed(cx);}
                });
            }).detach();
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _unused = cx;
        }
    }
    pub(super) fn retained_local_files(
        &self,
        key: DraftKey,
    ) -> Vec<std::sync::Arc<vessel_app_view::composer::local::LocalFile>> {
        self.areas
            .composer
            .trays
            .get(key)
            .into_iter()
            .flat_map(vessel_app_view::composer::tray::Tray::items)
            .filter_map(|item| match &item.origin {
                Origin::Stored(file) => Some(std::sync::Arc::clone(file)),
                Origin::File(_) | Origin::Pasted(_) => None,
            })
            .collect()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn local_manifest(&self) -> Result<Manifest, LocalFailure> {
        let mut files = Vec::new();
        for (draft, tray) in self.areas.composer.trays.entries() {
            let unconfirmed = match draft {
                DraftKey::New => self.areas.composer.starting.borrow().is_some(),
                DraftKey::Workspace { .. } => self
                    .areas
                    .composer
                    .workspace_starting
                    .keys()
                    .any(|(held, _)| *held == draft),
                DraftKey::Session { session } => self.areas.composer.sends.contains_key(&session),
            } || self.areas.composer.local.recovered.contains(&draft);
            for item in tray.items() {
                if let Origin::Stored(file) = &item.origin {
                    files.push(KeptFile {
                        draft,
                        record: file.record.clone(),
                        unconfirmed,
                    });
                }
            }
        }
        for (session, pending) in &self.areas.composer.sends {
            let draft = DraftKey::Session { session: *session };
            for file in &pending.local_files {
                if !files
                    .iter()
                    .any(|kept| kept.draft == draft && kept.record.key == file.record.key)
                {
                    files.push(KeptFile {
                        draft,
                        record: file.record.clone(),
                        unconfirmed: true,
                    });
                }
            }
        }
        Manifest::new(files)
    }
}

pub(super) fn notice(
    shell: &Shell,
    target: DraftKey,
    tokens: &vessel_app_kit::Tokens,
) -> Option<vessel_app_kit::ui::AnyElement> {
    use vessel_app_kit::alert::{Content, Size, Variant, alert};
    use vessel_app_kit::ui::{IntoElement as _, ParentElement as _, div};
    let local = &shell.areas.composer.local;
    let words = shell.words.composer();
    let (text, variant) = if local.recovered.contains(&target) {
        (words.file_local_recovered(), Variant::Error)
    } else if local.pending.get(&target).is_some_and(|count| *count > 0) {
        (words.file_submission_preparing(), Variant::Info)
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        if local.production && local.store.is_none() {
            return Some(
                alert(
                    Content {
                        title: None,
                        description: Some(
                            div()
                                .child(words.file_local_failed().to_owned())
                                .into_any_element(),
                        ),
                        action: None,
                    },
                    Variant::Error,
                    Size::Small,
                    tokens,
                )
                .into_any_element(),
            );
        }
        return None;
    };
    Some(
        alert(
            Content {
                title: None,
                description: Some(div().child(text.to_owned()).into_any_element()),
                action: None,
            },
            variant,
            Size::Small,
            tokens,
        )
        .into_any_element(),
    )
}

#[cfg(test)]
mod tests;
