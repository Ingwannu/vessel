//! The layout registry retires UI synchronously; detached link writes keep running.
use super::{Controls, Surface, key, poll, target_matches};
use crate::areas::layout::registry::retirement::{Notice, Owner};
use crate::shell::Shell;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, button_with_kind};
use vessel_app_link::workspace::ScriptAnswer;
use vessel_app_view::layout::{LayoutKey, TabId};

// Closed tabs can retain an uncertain detached write. Bound this owner-local
// presentation history independently of the node's process and tab limits.
pub(super) const MAX_PANEL_STATES: usize = 128;

pub(crate) fn start(shell: &mut Shell, _cx: &mut Context<'_, Shell>) {
    if shell.areas.workspace.scripts.retirement_ready {
        return;
    }
    match shell.layout_register_retirement(Owner::Scripts, retire) {
        Ok(()) => shell.areas.workspace.scripts.retirement_ready = true,
        Err(_duplicate) => tracing::warn!("script retirement owner could not be registered"),
    }
}

fn retire(shell: &mut Shell, notice: &Notice, cx: &mut Context<'_, Shell>) {
    let address = (notice.workspace().clone(), notice.tab());
    let mut states = shell.areas.workspace.scripts.panels.borrow_mut();
    if let Some(controls) = states.get_mut(&address) {
        withdraw(controls);
        if controls.last_write.is_none() {
            let _removed = states.remove(&address);
        }
        tracing::debug!(reason = ?notice.reason(), "script panel UI retired");
        cx.notify();
    }
}

pub(super) fn withdraw(controls: &mut Controls) {
    if let Some(write) = controls.model.retire() {
        controls.last_write = Some(write);
    }
    controls.retired = true;
}

/// Settle the original sent payload even when its tab is now hidden or closed.
/// No receipt reaches a successor's model or starts another read or write.
pub(super) fn accept(
    shell: &mut Shell,
    answer: &ScriptAnswer,
    cx: &mut Context<'_, Shell>,
) -> bool {
    let ScriptAnswer::Written {
        target,
        key: request,
        result,
    } = answer
    else {
        return false;
    };
    let Some((workspace, tab)) = key(&target.surface) else {
        return false;
    };
    let current = shell.script_panel_is_current(target);
    let mut states = shell.areas.workspace.scripts.panels.borrow_mut();
    let Some(controls) = states.get_mut(&(workspace.clone(), tab)) else {
        return false;
    };
    // A connection may retire before the panel's next timer or layout change.
    // Preserve that original receipt too, without rebinding its admission.
    if !current
        && controls.model.target.as_ref() == Some(target)
        && controls
            .model
            .writing
            .as_ref()
            .is_some_and(|(pending, _)| pending == request)
    {
        withdraw(controls);
    }
    let Some(write) = controls.last_write.as_mut() else {
        return false;
    };
    if write.accept(target, *request, result.clone()) {
        cx.notify();
        true
    } else {
        false
    }
}

pub(super) fn draw_retired(
    shell: &Shell,
    address: &(LayoutKey, TabId),
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let states = shell.areas.workspace.scripts.panels.borrow();
    let controls = states.get(address).filter(|controls| controls.retired)?;
    let words = shell.words.scripts();
    let can_reload = controls
        .last_write
        .as_ref()
        .is_some_and(|write| write.receipt().is_some());
    let mut reload = button_with_kind(
        "scripts-reload-retired",
        words.reload_retired(),
        ButtonKind::Secondary,
        if can_reload {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        tokens,
    );
    if can_reload {
        let original = controls.surface.clone();
        reload = reload.on_click(cx.listener(move |shell, _event, window, cx| {
            reload_after_receipt(shell, &original, window, cx);
        }));
    }
    Some(
        div()
            .flex()
            .flex_col()
            .child(super::super::ports::notice(words.retired(), tokens))
            .child(reload)
            .into_any_element(),
    )
}

fn reload_after_receipt(
    shell: &mut Shell,
    original: &Surface,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let Some((workspace, tab)) = key(original) else {
        return;
    };
    if !window.is_window_active()
        || !target_matches(shell, workspace, tab)
        || !shell.layout_panel_visible(workspace, tab, window)
    {
        return;
    }
    let admitted = shell
        .areas
        .workspace
        .scripts
        .panels
        .borrow()
        .get(&(workspace.clone(), tab))
        .is_some_and(|controls| {
            controls.retired
                && &controls.surface == original
                && controls
                    .last_write
                    .as_ref()
                    .is_some_and(|write| write.receipt().is_some())
        });
    if !admitted {
        return;
    }
    let Ok(instance) = shell
        .areas
        .workspace
        .scripts
        .panel_keys
        .borrow_mut()
        .issue()
    else {
        return;
    };
    let surface = Surface::Panel {
        workspace: workspace.clone(),
        tab,
        instance,
    };
    {
        let mut states = shell.areas.workspace.scripts.panels.borrow_mut();
        let Some(controls) = states.get_mut(&(workspace.clone(), tab)) else {
            return;
        };
        controls.host.clear(window, cx);
        controls.surface = surface.clone();
        controls.model.select(None);
        controls.retired = false;
    }
    shell.read_script_panel(&surface, window, cx);
    poll(&surface, window, cx);
    cx.notify();
}

#[cfg(test)]
mod tests;
