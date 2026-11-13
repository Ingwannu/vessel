//! Access removal uses the kit's adaptive confirm dialog, with the same
//! consequence and cancel-first ordering as the page's `RevokeDialog`.

use crate::shell::Shell;
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    DialogContent, DialogSize, Tokens, button, dialog as draw_dialog, outline_button,
};
use vessel_app_link::devices::Ask;
use vessel_app_view::devices::{Listing, Revoking, Target};
use vessel_proto::DeviceId;

impl Shell {
    pub(super) fn devices_choose_revoke(
        &mut self,
        id: DeviceId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.settings.devices;
        let Listing::Ready(devices) = &state.page.listing else {
            return;
        };
        if matches!(state.page.revoking, Revoking::Sending { .. }) {
            return;
        }
        let Some(device) = devices
            .iter()
            .find(|device| device.id == id && device.revoked_at.is_none())
        else {
            return;
        };
        state.page.revoking = Revoking::Confirm(device.clone());
        state.dialog.focus(window, cx);
        cx.notify();
    }

    fn devices_revoke(&mut self) {
        let state = &mut self.areas.settings.devices;
        let Target::Online(machine) = &state.page.target else {
            return;
        };
        let ticket = match state.page.tickets.next_ticket() {
            Ok(ticket) => ticket,
            Err(why) => {
                state.page.revoking = Revoking::Failed(why);
                return;
            }
        };
        let Some(device) = state.page.revoking.confirm(ticket) else {
            return;
        };
        let machine = machine.clone();
        if let Err(why) = self.devices_send(machine, ticket, Ask::Revoke { device }) {
            let _accepted = self
                .areas
                .settings
                .devices
                .page
                .revoking
                .answer(ticket, Err(why));
        }
    }

    fn devices_cancel_revoke(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        if matches!(
            self.areas.settings.devices.page.revoking,
            Revoking::Confirm(_)
        ) {
            self.areas.settings.devices.page.revoking = Revoking::Empty;
        }
        cx.notify();
    }
}

pub(super) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.devices;
    let Revoking::Confirm(device) = &state.page.revoking else {
        return None;
    };
    let Target::Online(machine) = &state.page.target else {
        return None;
    };
    let words = shell.t.lang.devices();
    let compact = window.viewport_size().width < px(vessel_app_kit::picker::COMPACT);
    let content = DialogContent {
        title: words.revoke_title(&device.label).into(),
        body: div()
            .child(words.revoke_hint(&device.label, machine.as_str()))
            .into_any_element(),
        footer: vec![
            outline_button("device-revoke-cancel", words.cancel, true, tokens)
                .when(compact, |button| button.min_h(px(44.0)))
                .on_click(cx.listener(|shell, _event, window, cx| {
                    shell.devices_cancel_revoke(window, cx);
                }))
                .into_any_element(),
            button("device-revoke-confirm", words.revoke, true, tokens)
                .bg(tokens.neutral.destructive)
                .when(compact, |button| button.min_h(px(44.0)))
                .on_click(cx.listener(|shell, _event, _window, cx| {
                    shell.devices_revoke();
                    cx.notify();
                }))
                .into_any_element(),
        ],
        size: DialogSize::Normal,
    };
    Some(draw_dialog(
        &state.dialog,
        content,
        tokens,
        window,
        compact,
        Shell::devices_cancel_revoke,
        cx,
    ))
}
