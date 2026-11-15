//! Session resource construction accepts one retained field, never allocates then swaps.
//! The selected-session product constructor remains until the sole coordinated cutover.
use super::{Open, Pending};
use crate::shell::Shell;
use vessel_app_kit::TextField;
use vessel_app_kit::ui::{Context, ListAlignment, ListState, Window, px};
use vessel_app_view::Conversation;

/// All per-session UI resources remain in the bundle owned by the resident registry.
pub(crate) struct SessionView {
    pub(crate) open: Option<Open>,
    pub(crate) resuming: Option<String>,
    pub(crate) resume_error: Option<String>,
    pub(super) restore: std::cell::RefCell<Option<String>>,
    pub(crate) pending: Option<Pending>,
    pub(crate) conversation: Conversation,
    pub(crate) entries: ListState,
    pub(crate) composer: TextField,
    #[cfg(test)]
    pub(crate) composer_retention: crate::areas::composer_shell::ResidentComposer,
}
impl SessionView {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self::with_composer(TextField::composer(
            "",
            |shell: &mut Shell, text, window, cx| shell.submit_prompt(text, window, cx),
            window,
            cx,
        ))
    }
    /// A caller constructs the one field with its exact callback owner first.
    pub(crate) fn with_composer(composer: TextField) -> Self {
        Self {
            open: None,
            resuming: None,
            resume_error: None,
            restore: std::cell::RefCell::new(None),
            pending: None,
            conversation: Conversation::default(),
            entries: ListState::new(0, ListAlignment::Bottom, px(800.0)),
            composer,
            #[cfg(test)]
            composer_retention: crate::areas::composer_shell::ResidentComposer::default(),
        }
    }

    /// Staged: no scoped resource can enter the product beside legacy globals.
    #[cfg(test)]
    pub(crate) fn new_scoped(
        captured: vessel_app_view::residents::SessionContext,
        submit: impl Fn(
            &mut Shell,
            &vessel_app_view::residents::SessionContext,
            String,
            &mut Window,
            &mut Context<'_, Shell>,
        ) + 'static,
        window: &mut Window,
        cx: &mut Context<'_, Shell>,
    ) -> Self {
        use vessel_app_view::residents::{PanelFocus, Visibility};
        Self::with_composer(TextField::composer(
            "",
            move |shell: &mut Shell, text, window, cx| {
                // Pane/focus values can change while this same Entity survives.
                // Refresh only the captured lifetime+claim, never selected globals.
                let Ok(current) = shell
                    .areas
                    .session_residents
                    .context(captured.resident, &captured.claim)
                else {
                    return;
                };
                if current.visibility != Visibility::Visible || current.focus != PanelFocus::Focused
                {
                    return;
                }
                submit(shell, &current, text, window, cx);
            },
            window,
            cx,
        ))
    }
}

#[cfg(test)]
mod tests;
