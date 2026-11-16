//! Domain admission surrounds the one shared desktop-popover/compact-sheet control.
use super::{
    accounts, modal,
    rows::{Density, notice},
};
use crate::shell::Shell;
use std::rc::Rc;
use vessel_app_kit::{
    ButtonState, ControlSizing, Tokens,
    assets::Glyph,
    popover::{self, Alignment, Dismissal, Lease, PopoverContent, PopoverHandlers, Width},
    toolbar_button::{Disclosure, ToolbarButton, ToolbarConfig, ToolbarIcon, ToolbarInk},
    ui::{AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, Window, div},
};
use vessel_app_view::{connection::ConnectionContext, devices::Ticket, usage::OptionsContext};

const OPTIONS_WIDTH: u16 = 340;

#[derive(Debug, Clone)]
pub(super) struct Opening {
    context: OptionsContext,
    lease: Lease,
}
impl Opening {
    fn current(&self, shell: &Shell) -> bool {
        let state = &shell.areas.usage;
        state.options_open.is_some()
            && state.options.is_current(&self.lease)
            && self
                .context
                .matches(state.connection.as_ref(), state.modal.current())
            && shell.usage_connection_current(self.context.machine())
    }
}
pub(super) fn admitted(shell: &Shell, receipt: &Opening) -> bool {
    receipt.current(shell)
}
impl Shell {
    fn usage_toggle_options(
        &mut self,
        machine: ConnectionContext,
        parent: Option<Ticket>,
        opening: Option<Opening>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.usage_connection_current(&machine)
            || self.areas.usage.connection.as_ref() != Some(&machine)
            || self.areas.usage.modal.current() != parent
        {
            return;
        }
        if let Some(opening) = opening {
            if !opening.current(self) {
                return;
            }
            let _retired = self.areas.usage.options_open.take();
            let _closed = self.areas.usage.options.close(&opening.lease, window, cx);
            cx.notify();
            return;
        }
        self.retire_usage_options(window, cx);
        let state = &mut self.areas.usage;
        // A retained closed-trigger frame must not replace an opening created
        // since it was drawn. Close intents above carry that opening's lease.
        if state.options_open.is_some() {
            return;
        }
        let Some(anchor) = state.options_anchor.get() else {
            return;
        };
        let lease = state.options.open(anchor, Alignment::End, window, cx);
        state.options_open = Some(Opening {
            context: OptionsContext::new(machine, parent),
            lease,
        });
        cx.notify();
    }
    fn retire_usage_options(&mut self, window: &Window, cx: &mut Context<'_, Self>) {
        if self
            .areas
            .usage
            .options_open
            .as_ref()
            .is_none_or(|opening| opening.current(self))
        {
            return;
        }
        // Withhold the obsolete body/intents immediately; the shared lease
        // checks again before its deferred actual-close focus acknowledgment.
        if let Some(retired) = self.areas.usage.options_open.take() {
            cx.defer_in(window, move |shell, window, cx| {
                let _closed = shell.areas.usage.options.close(&retired.lease, window, cx);
                cx.notify();
            });
        }
    }
    pub(super) fn close_usage_options_for_parent(
        &mut self,
        parent: Ticket,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self
            .areas
            .usage
            .options_open
            .as_ref()
            .is_some_and(|opening| opening.context.belongs_to(parent))
            && let Some(opening) = self.areas.usage.options_open.take()
        {
            let _closed = self.areas.usage.options.close(&opening.lease, window, cx);
        }
    }
}

pub(super) fn trigger(
    shell: &Shell,
    id: &'static str,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let machine = shell.areas.usage.connection.clone()?;
    let parent = shell.areas.usage.modal.current();
    let opening = shell
        .areas
        .usage
        .options_open
        .as_ref()
        .filter(|opening| opening.current(shell))
        .cloned();
    let disclosure = if opening.is_some() {
        Disclosure::Open
    } else {
        Disclosure::Closed
    };
    let button = ToolbarButton::new(
        id,
        shell.words.usage().options(),
        ToolbarIcon::new(Glyph::Gear, ToolbarInk::ExtraMuted).adaptive(),
        ToolbarConfig::new(ControlSizing::from_compact(compact), ButtonState::Enabled)
            .disclosure(disclosure),
        tokens,
    )
    .on_click(cx.listener(move |shell, _, window, cx| {
        shell.usage_toggle_options(machine.clone(), parent, opening.clone(), window, cx);
    }));
    let button = shell.areas.usage.source_tips.action(button, id);
    Some(
        div()
            .relative()
            .child(vessel_app_kit::anchor_probe(
                &shell.areas.usage.options_anchor,
            ))
            .child(button)
            .into_any_element(),
    )
}

pub(crate) fn dialogs(
    shell: &mut Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    shell.retire_usage_options(window, cx);
    let modal = modal::dialogs(shell, tokens, window, cx);
    let options = draw(shell, tokens, window, cx);
    let tips = shell.areas.usage.source_tips.render(tokens);
    if modal.is_none() && options.is_none() && tips.is_none() {
        return None;
    }
    Some(
        div()
            .children(modal)
            .children(options)
            .children(tips)
            .into_any_element(),
    )
}
fn draw(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.usage;
    let opening = state.options_open.as_ref()?.clone();
    if !opening.current(shell) {
        return None;
    }
    let compact = vessel_app_kit::breakpoint::phone(window);
    let density = if compact {
        Density::Compact
    } else {
        Density::Wide
    };
    let policy = state
        .inventory
        .phase()
        .observation()
        .and_then(vessel_app_view::usage::Inventory::policy);
    let body = div()
        .children(super::draw::read_host_status(
            state.inventory.phase(),
            shell,
            tokens,
        ))
        .children(state.policy_error.map(|reason| {
            notice(
                reason.message(shell.words),
                vessel_app_kit::alert::Variant::Error,
                tokens,
            )
        }))
        .child(accounts::policy_fields(
            shell,
            policy,
            tokens,
            density,
            opening.clone(),
            cx,
        ));
    let handlers = PopoverHandlers {
        state: |shell: &mut Shell| &mut shell.areas.usage.options,
        admit: Rc::new(move |shell: &mut Shell, lease, _, _| {
            if !opening.current(shell) || !shell.areas.usage.options.is_current(lease) {
                return Dismissal::KeepOpen;
            }
            shell.areas.usage.options_open = None;
            Dismissal::Close
        }),
    };
    popover::popover(
        &state.options,
        PopoverContent::new(
            "usage-options-surface",
            shell.words.usage().options(),
            body.into_any_element(),
            Width::new(OPTIONS_WIDTH)?,
        ),
        &handlers,
        tokens,
        window,
        compact,
        cx,
    )
}
