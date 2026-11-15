//! One native admission/viewport, with provider-owned facts and callbacks.
//! Source: contexts/sidebar-callout-context.tsx:85–164 and left-sidebar.tsx:782.
//! The source native slot is null; the GPUI slot is an explicit native mapping.

use std::rc::Rc;
use vessel_app_kit::Tokens;
use vessel_app_kit::sidebar_callout::{self as strip, Content, Dismiss, OnPress, Variant};
use vessel_app_kit::ui::{Context, Div, Window};
use vessel_app_view::prefs::PreferenceChange;
use vessel_app_view::sidebar::callouts::{
    DismissalKey, Dismissals, Priority, Problem, State, Token,
};

use crate::shell::Shell;

pub(crate) use vessel_app_view::sidebar::callouts::SourceId;

/// Rebuilt from current translations/theme; captured domain targets belong to
/// the provider, not a lookup of the latest machine alias at click time.
pub(crate) type Draw = Rc<dyn Fn(&Shell, &Tokens) -> Presentation>;
pub(crate) struct Presentation {
    pub(crate) content: Content<Shell>,
    pub(crate) variant: Variant,
}
/// Admission metadata and working presentation, prepared by one real owner.
pub(crate) struct Registration {
    pub(crate) source: SourceId,
    pub(crate) priority: Priority,
    pub(crate) dismissal: Option<DismissalKey>,
    pub(crate) draw: Draw,
    pub(crate) dismissible: bool,
    pub(crate) on_dismiss: Option<OnPress<Shell>>,
}
struct Body {
    draw: Draw,
    dismissible: bool,
    on_dismiss: Option<OnPress<Shell>>,
}
#[derive(Default)]
pub(super) struct Callouts {
    admission: State<SourceId, Body>,
    loaded: bool,
}
impl Callouts {
    fn load(&mut self, dismissed: &Dismissals) -> Result<(), Problem> {
        if !self.loaded {
            self.admission.load_dismissed(dismissed.keys())?;
            self.loaded = true;
        }
        Ok(())
    }
}

/// Reconcile after the sole canonical preference owner applies or rolls back
/// its projected document. This boundary neither saves nor recreates providers.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "desktop preference owner wires the checked producer"
    )
)]
pub(crate) fn preferences_changed(shell: &mut Shell, cx: &mut Context<'_, Shell>) {
    let callouts = &mut shell.areas.sidebar.callouts;
    match callouts
        .admission
        .load_dismissed(shell.prefs.settings.sidebar_callout_dismissals.keys())
    {
        Ok(()) => {
            callouts.loaded = true;
            cx.notify();
        }
        Err(error) => tracing::warn!(%error, "canonical sidebar dismissals refused"),
    }
}

pub(super) fn render(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Option<Div> {
    let active = shell.areas.sidebar.callouts.admission.active()?;
    let source = active.id().clone();
    let token = active.token();
    let body = active.content();
    let mut presentation = (body.draw)(shell, tokens);
    // All dismissals go through the single admission/persistence boundary;
    // providers receive their callback after removal, just as in the source.
    let dismissed_source = source.clone();
    presentation.content.dismiss = body.dismissible.then(|| Dismiss {
        press: Rc::new(move |shell: &mut Shell, window, cx| {
            shell.sidebar_callout_dismiss(&dismissed_source, token, window, cx);
        }),
    });
    Some(strip::sidebar_callout(
        format!("sidebar-callout-{}", source.as_str()),
        presentation.content,
        presentation.variant,
        tokens,
        cx,
    ))
}

impl Shell {
    /// Register/replace one admitted provider, retaining original tie order.
    /// The caller owns readiness, priority, durable key and current-target facts.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "provider owners consume the checked callout boundary"
        )
    )]
    pub(crate) fn sidebar_callout_show(
        &mut self,
        registration: Registration,
        cx: &mut Context<'_, Self>,
    ) -> Result<Token, Problem> {
        let callouts = &mut self.areas.sidebar.callouts;
        // Native preferences have already loaded before Shell exists; unlike
        // AsyncStorage, no asynchronous second read or visible flash is needed.
        callouts.load(&self.prefs.settings.sidebar_callout_dismissals)?;
        let token = callouts.admission.register(
            registration.source,
            registration.priority,
            registration.dismissal,
            Body {
                draw: registration.draw,
                dismissible: registration.dismissible,
                on_dismiss: registration.on_dismiss,
            },
        )?;
        cx.notify();
        Ok(token)
    }

    /// Cleanup only the matching registration; stale owner cleanup is harmless.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "provider owners consume the checked callout boundary"
        )
    )]
    pub(crate) fn sidebar_callout_unregister(
        &mut self,
        source: &SourceId,
        token: Token,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas
            .sidebar
            .callouts
            .admission
            .unregister(source, token);
        cx.notify();
    }

    fn sidebar_callout_dismiss(
        &mut self,
        source: &SourceId,
        token: Token,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.sidebar.callouts.admission;
        let body = match state.dismiss(source, token) {
            Ok(Some(body)) => body,
            Ok(None) => return,
            Err(error) => {
                tracing::warn!(%error, "sidebar callout dismissal refused");
                return;
            }
        };
        // The admission bound guarantees this conversion; keep failure explicit
        // instead of writing a partial list or silently evicting an older key.
        match Dismissals::try_from(state.dismissed().to_vec()) {
            Ok(keys) if keys != self.prefs.settings.sidebar_callout_dismissals => {
                self.prefer(
                    &PreferenceChange::SidebarCalloutDismissals(keys),
                    window,
                    cx,
                );
            }
            Ok(_) => {}
            Err(error) => tracing::error!(%error, "sidebar dismissal could not be retained"),
        }
        if let Some(on_dismiss) = body.on_dismiss {
            on_dismiss(self, window, cx);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests;
