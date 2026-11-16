//! Node stop/restart joins the one document phase before command admission.

use super::{Intent, Prompt, Shell};
use gpui::Context;
use vessel_app_link::local_node::{Action, ManagerError, Snapshot};

pub(super) fn request(shell: &mut Shell, action: Action, cx: &mut Context<'_, Shell>) {
    if shell.local_node_busy(cx) {
        return;
    }
    let Some(controller) = cx
        .try_global::<crate::local_node::Managed>()
        .and_then(|managed| managed.controller.clone())
    else {
        shell.notice = Some(shell.words.local_node().failed().to_owned());
        cx.notify();
        return;
    };
    let node = action.observed_node();
    if node.is_some() && !crate::quit::begin(cx) {
        return;
    }
    let intent = match &action {
        Action::Start => None,
        Action::Stop(_) | Action::InterruptConfirmed(_) => Some(Intent::Stop),
        Action::Restart(_) | Action::RestartInterruptConfirmed(_) => Some(Intent::Restart),
    };
    shell.areas.local_node.action_pending = true;
    cx.spawn(async move |this, cx| {
        // End the initiating Shell borrow before traversing all registered owners.
        // request() enqueues synchronously, so it must follow this admission.
        if let Some(node) = node
            && !cx.update(|cx| crate::quit::activate(cx) && crate::local_node::admit_node(node, cx))
        {
            cx.update(crate::quit::finish);
            let _closed = this.update(cx, |shell, cx| {
                shell.areas.local_node.action_pending = false;
                cx.notify();
            });
            return;
        }
        let result = match controller.request(action) {
            Ok(work) => work.await,
            Err(error) => Err(error),
        };
        if node.is_some() {
            // The origin window may have gone away; release the global phase
            // independently so another window cannot remain permanently frozen.
            cx.update(crate::quit::finish);
        }
        let _closed = this.update(cx, |shell, cx| {
            shell.areas.local_node.action_pending = false;
            settle(shell, result, intent);
            cx.notify();
        });
    })
    .detach();
    cx.notify();
}

fn settle(shell: &mut Shell, result: Result<Snapshot, ManagerError>, intent: Option<Intent>) {
    match result {
        Ok(snapshot) if snapshot.failure.is_some() => {
            if let Some(intent) = intent.filter(|_| snapshot.active_sessions().is_some()) {
                shell.areas.local_node.prompt = Some(Prompt {
                    identity: std::rc::Rc::new(()),
                    intent,
                    snapshot,
                });
                shell.areas.local_node.focus_prompt = true;
            } else {
                shell.notice = Some(shell.words.local_node().failed().to_owned());
            }
        }
        Ok(_snapshot) => {}
        Err(error) => {
            tracing::warn!(%error, "local node action did not finish");
            shell.notice = Some(shell.words.local_node().failed().to_owned());
        }
    }
}
