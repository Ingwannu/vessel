//! Access removal uses the kit's adaptive confirm dialog, with the same
//! consequence and cancel-first ordering as the page's `RevokeDialog`.

use crate::shell::Shell;
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, DialogContent, DialogSize, Tokens, button_with_kind,
    dialog as draw_dialog, outline_button,
};
use vessel_app_kit::{breakpoint::TOUCH_TARGET, theme::spacing};
use vessel_app_link::devices::Ask;
use vessel_app_view::devices::{Revoking, Target};
use vessel_proto::DeviceId;

impl Shell {
    pub(super) fn devices_choose_revoke(
        &mut self,
        context: &vessel_app_view::devices::ConnectionContext,
        id: DeviceId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.devices_press(context, window) {
            return;
        }
        let state = &mut self.areas.settings.devices;
        if state.page.choose_bound(context, id).is_err() {
            cx.notify();
            return;
        }
        state.dialog.focus(window, cx);
        cx.notify();
    }

    fn devices_revoke(
        &mut self,
        context: &vessel_app_view::devices::ConnectionContext,
        intent: vessel_app_view::devices::RevokeIntent,
        window: &Window,
    ) {
        if !self.devices_press(context, window) {
            return;
        }
        let state = &mut self.areas.settings.devices;
        if state.page.check_context(context).is_err() {
            return;
        }
        let ticket = match state.page.tickets.next_ticket() {
            Ok(ticket) => ticket,
            Err(why) => {
                state.page.revoking = Revoking::Failed(why);
                return;
            }
        };
        let Some(device) = state.page.confirm_revoke(context, intent, ticket) else {
            return;
        };
        if let Err(why) = self.devices_send(context.clone(), ticket, Ask::Revoke { device }) {
            let _accepted = self
                .areas
                .settings
                .devices
                .page
                .revoking
                .answer(ticket, Err(why));
        }
    }

    fn devices_cancel_revoke(
        &mut self,
        context: &vessel_app_view::devices::ConnectionContext,
        intent: vessel_app_view::devices::RevokeIntent,
    ) {
        self.areas
            .settings
            .devices
            .page
            .cancel_revoke(context, intent);
    }
}

pub(super) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.devices;
    let device = state.page.revoking.confirmation()?;
    let context = state.page.revoking.context()?.clone();
    if state.page.check_context(&context).is_err() {
        return None;
    }
    let intent = state.page.revoke_intent()?;
    let cancel_context = context.clone();
    let dismiss_context = context.clone();
    let sending = matches!(state.page.revoking, Revoking::Sending { .. });
    let Target::Online(machine) = &state.page.target else {
        return None;
    };
    let words = shell.t.lang.devices();
    let compact = window.viewport_size().width < px(vessel_app_kit::picker::COMPACT);
    let layout = DialogContent {
        title: words.revoke_title(&device.label).into(),
        body: div()
            .child(words.revoke_hint(machine.as_str()))
            .when(sending, |body| {
                body.child(div().mt(px(spacing::S3)).child(super::rows::busy(
                    "devices-revoking",
                    words.revoking,
                    state.page.revoking.variant(),
                    tokens,
                )))
            })
            .into_any_element(),
        footer: vec![
            outline_button(
                format!("device-revoke-cancel-{}-{intent}", context.epoch()),
                words.cancel,
                !sending,
                tokens,
            )
            .when(compact, |button| button.min_h(px(TOUCH_TARGET)))
            .when(!sending, |button| {
                button.on_click(cx.listener(move |shell, _event, _window, cx| {
                    shell.devices_cancel_revoke(&cancel_context, intent);
                    cx.notify();
                }))
            })
            .into_any_element(),
            button_with_kind(
                format!("device-revoke-confirm-{}-{intent}", context.epoch()),
                words.revoke,
                ButtonKind::Destructive,
                if sending {
                    ButtonState::Disabled
                } else {
                    ButtonState::Enabled
                },
                tokens,
            )
            .when(compact, |button| button.min_h(px(TOUCH_TARGET)))
            .when(!sending, |button| {
                button.on_click(cx.listener(move |shell, _event, window, cx| {
                    shell.devices_revoke(&context, intent, window);
                    cx.notify();
                }))
            })
            .into_any_element(),
        ],
        size: DialogSize::Normal,
    };
    Some(draw_dialog(
        &state.dialog,
        layout,
        tokens,
        window,
        compact,
        move |shell, _window, cx| {
            shell.devices_cancel_revoke(&dismiss_context, intent);
            cx.notify();
        },
        cx,
    ))
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests;
