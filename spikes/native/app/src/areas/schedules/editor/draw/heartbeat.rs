//! An existing-session schedule has a fixed target and its own cadence surface.
use super::{Shell, field, hint};
use vessel_app_kit::{
    Tokens, Type, TypeExt as _,
    control_sizing::TOUCH_TARGET,
    theme::spacing::{S3, S4, S8},
    ui::{Div, IntoElement as _, ParentElement as _, SharedString, Styled as _, div, px},
};
use vessel_app_view::schedules::{editor::io::Form, locale, presentation::heartbeat_target_label};
use vessel_proto::automation::ScheduleInfo;

pub(super) fn body(
    shell: &Shell,
    form: &Form,
    info: &ScheduleInfo,
    tokens: &Tokens,
    touch: bool,
) -> Div {
    let words = shell.words.schedules();
    let target = heartbeat_target_label(info, form.connection(), &shell.fleet, shell.words)
        .map_or_else(|_| words.display_unavailable().into(), SharedString::from);
    let cadence = locale::cadence(&info.definition.cadence, shell.words)
        .unwrap_or_else(|_| words.display_unavailable().to_owned());
    div()
        .flex()
        .flex_col()
        .gap(px(S4))
        .child(field(
            words.heartbeat_target(),
            readout(target, tokens, touch).into_any_element(),
            None,
            tokens,
        ))
        .child(field(
            words.form_cadence(),
            readout(cadence, tokens, touch).into_any_element(),
            None,
            tokens,
        ))
        .child(hint(words.heartbeat_older(), false, tokens))
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
