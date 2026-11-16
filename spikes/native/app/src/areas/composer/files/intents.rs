//! Render-time resource leases prevent retained callbacks selecting a replacement.
use super::*;
use vessel_app_kit::attachment::ScopedPillClick;
use vessel_app_view::composer::tray::{Again, ItemIntent};

impl Shell {
    // Resolve captured resources without interpreting the kit's draw index.
    fn composer_retry(&mut self, key: DraftKey, intent: &ItemIntent, cx: &mut Context<'_, Self>) {
        let DraftKey::Session { session } = key else {
            return;
        };
        let Some(tray) = self.areas.composer.trays.get_mut(key) else {
            return;
        };
        let Some(item) = tray
            .items()
            .iter()
            .find(|item| intent.matches(item))
            .map(|item| item.key)
        else {
            return;
        };
        let _retried = tray.retry(item);
        self.retry_creation_capabilities(session, cx);
        self.composer_pump(session, cx);
    }

    fn tray_image_opened(
        &mut self,
        key: DraftKey,
        intent: &ItemIntent,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(item) = self
            .areas
            .composer
            .trays
            .get(key)
            .and_then(|tray| tray.items().iter().find(|item| intent.matches(item)))
        else {
            return;
        };
        if item.kind == AttachmentKind::Image {
            if let vessel_app_view::composer::tray::Origin::Stored(file) = &item.origin {
                self.open_stored_image(std::sync::Arc::clone(file), window, cx);
            } else {
                let source = picture(self, &item.origin, &item.mime);
                self.open_local_image(source, window, cx);
            }
        }
    }

    fn composer_remove(&mut self, key: DraftKey, intent: &ItemIntent, cx: &mut Context<'_, Self>) {
        if let DraftKey::Session { session } = key
            && self
                .areas
                .composer
                .sends
                .get(&session)
                .is_some_and(|waiting| {
                    waiting.phase != super::super::transmission::Phase::Preparing
                })
        {
            return;
        }
        let Some(tray) = self.areas.composer.trays.get_mut(key) else {
            return;
        };
        let Some(item) = tray
            .items()
            .iter()
            .find(|item| intent.matches(item))
            .map(|item| item.key)
        else {
            return;
        };
        let _removed = tray.remove(item);
        self.local_files_changed(cx);
        match key {
            DraftKey::Session { session } => self.composer_pump(session, cx),
            DraftKey::New | DraftKey::Workspace { .. } => cx.notify(),
        }
    }
}

pub(super) fn callbacks(
    key: DraftKey,
    item: &Item,
) -> (Option<ScopedPillClick<Shell>>, ScopedPillClick<Shell>) {
    let retry = matches!(
        item.state,
        ItemState::Failed {
            again: Again::Resume(_) | Again::Restart,
            ..
        }
    ) && matches!(key, DraftKey::Session { .. });
    let intent = item.intent();
    let open: Option<ScopedPillClick<Shell>> = if retry {
        Some(std::rc::Rc::new(move |shell, _index, _window, cx| {
            shell.composer_retry(key, &intent, cx);
        }))
    } else if item.kind == AttachmentKind::Image {
        Some(std::rc::Rc::new(move |shell, _index, window, cx| {
            shell.tray_image_opened(key, &intent, window, cx);
        }))
    } else {
        None
    };
    let intent = item.intent();
    let remove: ScopedPillClick<Shell> = std::rc::Rc::new(move |shell, _index, _window, cx| {
        shell.composer_remove(key, &intent, cx);
    });
    (open, remove)
}

#[cfg(test)]
mod tests;
