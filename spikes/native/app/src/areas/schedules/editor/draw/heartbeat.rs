//! An existing-session schedule has a fixed target and its own cadence surface.
use super::{Shell, cadence, field, hint};
use vessel_app_kit::{
    Tokens, Type, TypeExt as _,
    control_sizing::TOUCH_TARGET,
    theme::spacing::{S3, S4, S8},
    ui::{Context, Div, IntoElement as _, ParentElement as _, SharedString, Styled as _, div, px},
};
use vessel_app_view::schedules::{
    editor::{heartbeat, io::Form},
    locale,
    presentation::heartbeat_target_label,
};
use vessel_proto::automation::ScheduleInfo;

pub(super) fn body(
    shell: &Shell,
    form: &Form,
    info: &ScheduleInfo,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.schedules();
    let target = heartbeat_target_label(info, form.connection(), &shell.fleet, shell.words)
        .map_or_else(|_| words.display_unavailable().into(), SharedString::from);
    let detail = div().flex().flex_col().gap(px(S4)).child(field(
        words.heartbeat_target(),
        readout(target, tokens, touch).into_any_element(),
        None,
        tokens,
    ));
    if form.heartbeat_editable() {
        detail.child(cadence::body(shell, form, tokens, touch, cx))
    } else {
        detail.child(read_only(shell, form, info, tokens, touch))
    }
}

fn read_only(shell: &Shell, form: &Form, info: &ScheduleInfo, tokens: &Tokens, touch: bool) -> Div {
    let words = shell.words.schedules();
    let version = shell
        .fleet
        .protocol_of(form.connection().machine().as_str());
    let notice = if !heartbeat::supported(version) {
        words.heartbeat_older()
    } else if !heartbeat::editable(info) {
        words.newer_value()
    } else {
        words.read_only()
    };
    let cadence = locale::cadence(&info.definition.cadence, shell.words)
        .unwrap_or_else(|_| words.display_unavailable().to_owned());
    div()
        .flex()
        .flex_col()
        .gap(px(S4))
        .child(field(
            words.form_cadence(),
            readout(cadence, tokens, touch).into_any_element(),
            None,
            tokens,
        ))
        .child(hint(notice, false, tokens))
}

fn readout(value: impl Into<SharedString>, tokens: &Tokens, touch: bool) -> Div {
    div()
        .min_w_0()
        .min_h(px(if touch { TOUCH_TARGET } else { S8 }))
        .flex()
        .items_center()
        .px(px(S3))
        .rounded_md()
        .border_1()
        .border_color(tokens.neutral.border)
        .bg(tokens.neutral.surface2)
        .text_color(tokens.neutral.foreground)
        .type_(Type::Body)
        .child(div().truncate().child(value.into()))
}
