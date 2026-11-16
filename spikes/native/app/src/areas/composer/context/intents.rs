//! Callbacks retain an opaque context incarnation, including its source revision.
use super::{ContextAttachment, ContextIntent, DraftKey, Shell};
use vessel_app_kit::attachment::ScopedPillClick;

pub(super) fn callbacks(
    target: DraftKey,
    item: &ContextAttachment,
    intent: ContextIntent,
) -> (Option<ScopedPillClick<Shell>>, ScopedPillClick<Shell>) {
    let opened = intent.clone();
    let open = (!matches!(item, ContextAttachment::WorkspaceFile(_))).then(
        || -> ScopedPillClick<Shell> {
            std::rc::Rc::new(move |shell, _index, window, cx| {
                shell.context_opened(target, &opened, window, cx);
            })
        },
    );
    let remove: ScopedPillClick<Shell> = std::rc::Rc::new(move |shell, _index, _window, cx| {
        shell.context_removed(target, &intent, cx);
    });
    (open, remove)
}

#[cfg(test)]
mod tests;
