//! Query authority through the same bounded, exact-context device adapter.
//!
//! [Decision Log]
//! - Purpose: unknown permissions do not become management controls.
//! - Constraint: Admin can exist without session Read; a failed self read
//!   proves neither permission nor denial. The node enforces every action.
//! - Choice: explicit Page-owned self facts with whole recovery copy and retry.
//! - Why: one admitted context and ticket source already fence stale replies.
//! - Cost: older nodes need an update; no guessed Admin compatibility mode.
use crate::shell::Shell;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, breakpoint::TOUCH_TARGET, outline_button, theme::spacing};
use vessel_app_link::devices::Ask;
use vessel_app_view::devices::{Authority, Revoking};

impl Shell {
    pub(super) fn devices_authority(&mut self) {
        let page = &mut self.areas.settings.devices.page;
        if matches!(page.authority(), Authority::Loading(_))
            || matches!(page.revoking, Revoking::Sending { .. })
        {
            return;
        }
        let Some(context) = page.context().cloned() else {
            return;
        };
        let ticket = match page.tickets.next_ticket() {
            Ok(ticket) => ticket,
            Err(why) => {
                let _accepted = page.cannot_check_authority(&context, why);
                return;
            }
        };
        if !page.checking_authority(&context, ticket) {
            return;
        }
        if let Err(why) = self.devices_send(context.clone(), ticket, Ask::Authority) {
            let _accepted =
                self.areas
                    .settings
                    .devices
                    .page
                    .authority_answer(&context, ticket, Err(why));
        }
    }
}

pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.device_authority();
    match shell.areas.settings.devices.page.authority() {
        Authority::Unchecked | Authority::Loading(_) => {
            div().p(px(spacing::S4)).child(super::rows::busy(
                "devices-authority",
                words.loading(),
                vessel_app_view::badge::Variant::Muted,
                tokens,
            ))
        }
        Authority::Known(_) => vessel_app_kit::settings::row_at(
            words.management(),
            Some(words.denied().into()),
            None,
            Some(retry(shell, tokens, compact, cx)),
            true,
            compact,
            tokens,
        ),
        Authority::Failed(why) => div().p(px(spacing::S4)).child(vessel_app_kit::alert::alert(
            vessel_app_kit::alert::Content {
                title: None,
                description: Some(
                    div()
                        .child(shell.t.lang.devices().authority_problem(*why, shell.words))
                        .into_any_element(),
                ),
                action: Some(retry(shell, tokens, compact, cx)),
            },
            crate::badge::alert_variant(why.notice()),
            vessel_app_kit::alert::Size::Small,
            tokens,
        )),
    }
}

fn retry(shell: &Shell, tokens: &Tokens, compact: bool, cx: &mut Context<'_, Shell>) -> AnyElement {
    let Some(context) = shell.areas.settings.devices.page.context().cloned() else {
        return div().into_any_element();
    };
    outline_button(
        format!("devices-authority-retry-{}", context.epoch()),
        shell.words.device_authority().retry(),
        true,
        tokens,
    )
    .when(compact, |button| button.min_h(px(TOUCH_TARGET)))
    .on_click(cx.listener(move |shell, _event, _window, cx| {
        if shell
            .areas
            .settings
            .devices
            .page
            .check_context(&context)
            .is_ok()
        {
            shell.devices_authority();
            cx.notify();
        }
    }))
    .into_any_element()
}

#[cfg(test)]
mod tests;
