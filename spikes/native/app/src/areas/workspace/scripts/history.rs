//! Retired project intentions and panel receipts share one bounded UI owner.
//! Acknowledgement discards local received results only, never a pending task.
mod draw;
use super::{MAX_RETAINED_SURFACES, Scripts};
use crate::shell::Shell;
pub(super) use draw::render;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::workspace::ScriptAnswer;
use vessel_app_view::workspace::git::RequestKey;
use vessel_app_view::workspace::scripts::Target;

type ReceiptId = (Target, RequestKey);
fn same_scope(left: &Target, right: &Target) -> bool {
    left.context == right.context && left.place == right.place
}
impl Scripts {
    pub(super) fn retire_project(&mut self) {
        if let Some(write) = self.model.retire() {
            // The in-flight project slot was charged before dispatch, so
            // transferring it to history cannot exceed the shared bound.
            self.retired_projects.push(write);
        }
    }
    pub(super) fn admits_project_write(&self) -> bool {
        self.panels
            .borrow()
            .len()
            .checked_add(self.retired_projects.len())
            .and_then(|count| count.checked_add(usize::from(self.model.writing.is_some())))
            .is_some_and(|count| count < MAX_RETAINED_SURFACES)
    }
    pub(super) fn has_pending(&self, target: &Target) -> bool {
        self.model.writing.is_some()
            && self
                .model
                .target
                .as_ref()
                .is_some_and(|old| same_scope(old, target))
            || self
                .retired_projects
                .iter()
                .any(|write| write.receipt().is_none() && same_scope(write.target(), target))
            || self.panels.borrow().values().any(|controls| {
                controls
                    .outstanding()
                    .is_some_and(|old| same_scope(old, target))
                    || controls
                        .pending_write()
                        .is_some_and(|write| same_scope(write.target(), target))
            })
    }
    fn received(&self) -> Vec<ReceiptId> {
        let mut received: Vec<_> = self
            .retired_projects
            .iter()
            .filter(|write| write.receipt().is_some())
            .map(|write| (write.target().clone(), write.key()))
            .collect();
        received.extend(self.panels.borrow().values().filter_map(|controls| {
            controls
                .returned_write()
                .map(|write| (write.target().clone(), write.key()))
        }));
        received
    }
    fn retained(&self) -> bool {
        !self.retired_projects.is_empty()
            || self.panels.borrow().values().any(|controls| {
                controls.pending_write().is_some() || controls.returned_write().is_some()
            })
    }
    fn dismiss_received(&mut self, received: &[ReceiptId]) {
        self.retired_projects.retain(|write| {
            write.receipt().is_none()
                || !received
                    .iter()
                    .any(|(target, key)| write.matches(target, *key))
        });
        self.panels.borrow_mut().retain(|_address, controls| {
            !received
                .iter()
                .any(|(target, key)| controls.acknowledge(target, *key))
        });
    }
}
impl Shell {
    pub(super) fn retired_project_answer(
        &mut self,
        answer: &ScriptAnswer,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let ScriptAnswer::Written {
            target,
            key,
            result,
        } = answer
        else {
            return false;
        };
        let current = self.scripts_target_is_current(target);
        let scripts = &mut self.areas.workspace.scripts;
        if !current
            && scripts.model.target.as_ref() == Some(target)
            && scripts
                .model
                .writing
                .as_ref()
                .is_some_and(|(pending, _)| pending == key)
        {
            scripts.retire_project();
        }
        let Some(write) = scripts
            .retired_projects
            .iter_mut()
            .find(|write| write.matches(target, *key) && write.receipt().is_none())
        else {
            return false;
        };
        let accepted = write.accept(target, *key, result.clone());
        if accepted {
            cx.notify();
        }
        accepted
    }
    fn dismiss_script_results(
        &mut self,
        received: &[ReceiptId],
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !window.is_window_active() || !self.scripts_visible() {
            return;
        }
        self.areas.workspace.scripts.dismiss_received(received);
        cx.notify();
    }
}

#[cfg(test)]
mod tests;
