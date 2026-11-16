//! One modifier-event observer drives delayed workspace badges. The sidebar
//! owns target ordering/number placement; this area owns effective-key policy.
use super::ShortcutListener;
use crate::shell::Shell;
use gpui::ModifiersChangedEvent;
use vessel_app_kit::{
    TextField,
    ui::{App, Context, Window},
};
use vessel_app_view::shortcuts::badges::{Change, HeldModifiers, Modifier, index_modifier};
use vessel_app_view::shortcuts::context::{FocusScope, ShortcutContext, When};
use vessel_app_view::shortcuts::{Action, Platform, binding_id, default_context};

pub(super) fn clear(listener: &mut ShortcutListener) {
    listener.badge_reveal.clear();
    listener.badge_when = None;
    listener.badge_task = None;
}
fn context(shell: &Shell, window: &Window) -> ShortcutContext {
    let mut context = default_context(Platform::native());
    if shell.terminal_focused(window) {
        context.focus = FocusScope::Terminal;
    }
    context
}
fn eligible(shell: &Shell, window: &mut Window, cx: &mut App) -> bool {
    shell.global_shortcuts_enabled()
        && super::available(window)
        && window.is_visible()
        && window.is_window_active()
        && !window
            .context_stack()
            .iter()
            .any(|entry| entry.contains(vessel_app_kit::popup::KEY_CONTEXT))
        && !shell.areas.palette.is_open()
        && !shell.areas.shortcuts.help.is_open()
        && !shell.areas.shortcuts.page.recording()
        && !TextField::focused_composing(window, cx)
}
fn has_target_owner(shell: &Shell, window: &Window, cx: &App) -> bool {
    const FIRST_WORKSPACE: u8 = 1;
    let request = super::dispatch::ShortcutRequest {
        action: Action::JumpSession,
        scope: Action::JumpSession.scope(),
        index: vessel_app_view::shortcuts::engine::ShortcutIndex::new(FIRST_WORKSPACE),
        repeat: vessel_app_view::shortcuts::engine::KeyRepeat::Initial,
        context: context(shell, window),
    };
    shell.shortcut_handler_available(&request, window, cx)
}
fn required(shell: &mut Shell, window: &mut Window, cx: &mut App) -> Option<(Modifier, When)> {
    if !eligible(shell, window, cx) || !has_target_owner(shell, window, cx) {
        return None;
    }
    let context = context(shell, window);
    let id = binding_id(Action::JumpSession, context)?;
    if shell
        .areas
        .shortcuts
        .compiled
        .refresh(&shell.prefs.settings.shortcuts)
        .ok()?
    {
        shell.areas.shortcuts.engine.reset();
        shell.areas.shortcuts.held.release();
        clear(&mut shell.areas.shortcuts);
    }
    let binding = shell
        .areas
        .shortcuts
        .compiled
        .bindings
        .iter()
        .find(|binding| binding.id == id)?;
    if !binding.when.matches(context) {
        return None;
    }
    Some((
        index_modifier(&binding.keys, context.platform)?,
        binding.when,
    ))
}
impl Shell {
    /// Source-owned sidebar rows ask this; no targets or defaults are duplicated.
    pub(crate) fn shortcut_badges_visible(&self, window: &mut Window, cx: &mut App) -> bool {
        self.areas.shortcuts.badge_reveal.visible()
            && eligible(self, window, cx)
            && has_target_owner(self, window, cx)
            && self
                .areas
                .shortcuts
                .badge_when
                .is_some_and(|when| when.matches(context(self, window)))
    }
    pub(crate) fn shortcut_modifiers_changed(
        &mut self,
        event: &ModifiersChangedEvent,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let was_visible = self.shortcut_badges_visible(window, cx);
        let required = required(self, window, cx);
        let listener = &mut self.areas.shortcuts;
        listener.badge_when = required.map(|(_, when)| when);
        let keys = event.modifiers;
        let held = HeldModifiers {
            control: keys.control,
            meta: keys.platform,
            alt: keys.alt,
            shift: keys.shift,
        };
        let change = listener.badge_reveal.update(
            required.map(|(key, _)| key),
            held,
            required.is_some(),
            listener.since.elapsed(),
        );
        match change {
            Change::Unchanged => {}
            Change::Hidden => {
                listener.badge_task = None;
                if was_visible {
                    cx.notify();
                }
            }
            Change::Waiting(deadline) => {
                let delay = deadline.saturating_sub(listener.since.elapsed());
                listener.badge_task = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(delay).await;
                    let _gone = this.update(cx, |shell, cx| {
                        let listener = &mut shell.areas.shortcuts;
                        // Completion of this cancellable timer supplies the
                        // deadline; do not mix the GPUI test clock with std Instant.
                        if listener.badge_reveal.reveal(deadline, deadline) {
                            listener.badge_task = None;
                            cx.notify();
                        }
                    });
                }));
            }
        }
    }
}
