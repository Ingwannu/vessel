//! Public metadata uses the existing adaptive dialog and settings rows.
//! This surface reads no credential and offers no destructive action.
use crate::shell::Shell;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, px,
};
use vessel_app_kit::{
    DialogContent, DialogSize, Tokens, breakpoint::TOUCH_TARGET, dialog as draw_dialog,
    outline_button, settings,
};
use vessel_app_view::devices::DeviceMetadata;
use vessel_proto::DeviceId;

impl Shell {
    pub(super) fn devices_inspect(
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
        if state.page.inspect_bound(context, id).is_err() {
            return;
        }
        state.dialog.focus(window, cx);
        cx.notify();
    }

    fn devices_close_inspection(
        &mut self,
        context: &vessel_app_view::devices::ConnectionContext,
        device: DeviceId,
    ) {
        let page = &mut self.areas.settings.devices.page;
        if page.check_context(context).is_ok()
            && page
                .inspected_device()
                .is_some_and(|selected| selected.id == device)
        {
            page.close_inspection();
        }
    }
}

pub(super) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.devices;
    let device = state.page.inspected_device()?;
    let context = state.page.context()?.clone();
    let dismiss_context = context.clone();
    let id = device.id;
    let words = shell.t.lang.devices();
    let metadata = DeviceMetadata::of(shell.t.lang, device, state.offset);
    let compact = window.viewport_size().width < px(vessel_app_kit::picker::COMPACT);
    let body = metadata_card(shell.t.lang, metadata, tokens);
    let close = outline_button(
        format!("device-details-close-{}", context.epoch()),
        words.close_details,
        true,
        tokens,
    )
    .when(compact, |button| button.min_h(px(TOUCH_TARGET)))
    .on_click(cx.listener(move |shell, _event, _window, cx| {
        shell.devices_close_inspection(&context, id);
        cx.notify();
    }));
    Some(draw_dialog(
        &state.dialog,
        DialogContent {
            title: device.label.clone().into(),
            body: body.into_any_element(),
            footer: vec![close.into_any_element()],
            size: DialogSize::Normal,
        },
        tokens,
        window,
        compact,
        move |shell, _window, cx| {
            shell.devices_close_inspection(&dismiss_context, id);
            cx.notify();
        },
        cx,
    ))
}

fn metadata_card(lang: vessel_app_view::Lang, metadata: DeviceMetadata, tokens: &Tokens) -> Div {
    let words = lang.devices();
    let mut body = settings::card(tokens);
    for (index, (title, value)) in [
        (words.paired_at_label, metadata.paired_at),
        (words.last_seen_label, metadata.last_seen),
        (words.access_label, metadata.grants),
    ]
    .into_iter()
    .enumerate()
    {
        body = body.child(settings::row(
            title,
            Some(value.into()),
            None,
            index == 0,
            tokens,
        ));
    }
    body
}
