use super::*;
use crate::areas::settings::agents::{page, with_original};
use vessel_app_kit::ui::{Div, IntoElement as _, ParentElement as _, Styled as _, div, px};
use vessel_app_kit::{
    ButtonConfig, ButtonContent, ButtonKind, ButtonState, ControlSizing, DialogContent, DialogSize,
    Tokens, Type, TypeExt as _, button_control, dialog as modal, theme::spacing,
};
use vessel_app_view::agent_status::{
    addition::{self, Control},
    pages::Operation,
};

pub(in crate::areas::settings::agents) fn notice(shell: &Shell, tokens: &Tokens) -> Option<Div> {
    let state = &shell.areas.settings.agents.addition;
    let words = shell.words.agent_catalog();
    if let Some(error) = &state.refused {
        return Some(alert(words.form_problem(error), error.variant(), tokens));
    }
    if state.form.is_none()
        && let Some(issue) = page(shell)
            .and_then(|page| page.issue())
            .filter(|issue| matches!(issue.operation, Operation::Add { .. }))
    {
        let problem = issue.source.addition_problem();
        return Some(with_original(
            alert(words.addition_failure(problem), problem.variant(), tokens),
            issue.source.to_string(),
            shell,
        ));
    }
    None
}
pub(in crate::areas::settings::agents) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.agents.addition;
    if matches!(state.restore.replace(Restore::Keep), Restore::Dismiss) {
        state.dialog.dismissed(window, cx);
    }
    let form = state.form.as_ref()?;
    if !shell.agent_context(Some(&form.machine)) {
        return None;
    }
    let words = shell.words.agent_catalog();
    let compact = vessel_app_kit::breakpoint::phone(window);
    let close = Rc::clone(&form.intent);
    Some(modal(
        &state.dialog,
        DialogContent {
            title: words.form_title().into(),
            body: body(form, shell, tokens, window, cx).into_any_element(),
            footer: footer(form, shell, tokens, compact, cx),
            size: DialogSize::Normal,
        },
        tokens,
        window,
        compact,
        move |shell: &mut Shell, window, cx| shell.agent_close_add(&close, window, cx),
        cx,
    ))
}
fn body(
    form: &Form,
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.agent_catalog();
    let compact = vessel_app_kit::breakpoint::phone(window);
    let pending = matches!(form.phase, Phase::Sending(_));
    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S4))
        .child(super::super::note(words.form_help(), tokens));
    if let Some(invalid) = &form.invalid {
        body = body.child(alert(
            words.form_problem(invalid),
            invalid.variant(),
            tokens,
        ));
    }
    if let Some(issue) = page(shell)
        .filter(|_| form.invalid.is_none())
        .and_then(|page| page.issue())
        .filter(|issue| matches!(issue.operation, Operation::Add { .. }))
    {
        let problem = issue.source.addition_problem();
        body = body.child(with_original(
            alert(words.addition_failure(problem), problem.variant(), tokens),
            issue.source.to_string(),
            shell,
        ));
    }
    body = body
        .child(field(
            words.name_label(),
            form.name.element(tokens, compact, cx),
            tokens,
        ))
        .child(super::super::note(words.name_help(), tokens));
    if form.registry.is_some() {
        body = body.child(preview(form, shell, tokens, window, cx));
    } else {
        body = body
            .child(field(
                words.program_label(),
                form.program.element(tokens, compact, cx),
                tokens,
            ))
            .child(field(
                words.arguments_label(),
                form.arguments.element(tokens, compact, cx),
                tokens,
            ))
            .child(super::super::note(words.arguments_help(), tokens));
    }
    if pending {
        body = body.child(super::super::note(words.pending_help(), tokens));
    }
    body
}
fn preview(
    form: &Form,
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.agent_catalog();
    let mut body = div().flex().flex_col().gap(px(spacing::S3));
    if let Some(entry) = &form.registry {
        body = body.child(
            div()
                .type_(Type::Mono)
                .text_color(tokens.neutral.foreground)
                .child(format!("{}@{}", entry.id, entry.version)),
        );
        body = body.child(super::super::note(words.launch_preview(), tokens));
        if let Some(inventory) = page(shell).and_then(|page| page.inventory()) {
            for (index, argument) in inventory
                .command_shell
                .registry_arguments(entry)
                .into_iter()
                .enumerate()
            {
                body = body.child(super::super::detail::widgets::command_box(
                    format!("registry-launch-{index}"),
                    &argument,
                    tokens,
                    window,
                    cx,
                ));
            }
        }
        if let vessel_proto::agents::CatalogueRun::Command { env, .. } = &entry.runs
            && !env.is_empty()
        {
            body = body.child(super::super::note(words.registry_environment(), tokens));
            for (index, variable) in env.iter().enumerate() {
                body = body
                    .child(
                        div()
                            .type_(Type::Mono)
                            .text_color(tokens.neutral.foreground)
                            .child(variable.name.as_str().to_owned()),
                    )
                    .child(super::super::detail::widgets::command_box(
                        format!("registry-env-{index}"),
                        &variable.value,
                        tokens,
                        window,
                        cx,
                    ));
            }
        }
    }
    body
}
fn footer(
    form: &Form,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let words = shell.words.agent_catalog();
    let pending = matches!(form.phase, Phase::Sending(_));
    let can_send = !pending && addition::command_control(page(shell)) == Control::Ready;
    let origin = Rc::clone(&form.intent);
    let submit = Rc::clone(&form.intent);
    let sizing = ControlSizing::from_compact(compact);
    let cancel = button_control(
        "provider-add-cancel",
        ButtonContent::Label(words.close().into()),
        ButtonConfig::new(ButtonKind::Ghost, ButtonState::Enabled, sizing),
        tokens,
    )
    .on_click(cx.listener(move |shell, _, window, cx| shell.agent_close_add(&origin, window, cx)))
    .into_any_element();
    let config = ButtonConfig::new(
        ButtonKind::Primary,
        if can_send {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        sizing,
    );
    let config = if pending { config.loading() } else { config };
    let add = button_control(
        "provider-add-confirm",
        ButtonContent::Label(if pending { words.adding() } else { words.add() }.into()),
        config,
        tokens,
    )
    .on_click(cx.listener(move |shell, _, _, cx| shell.agent_submit_add(&submit, cx)))
    .into_any_element();
    vec![cancel, add]
}

fn field(label: &str, input: AnyElement, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(
            div()
                .type_(Type::Label)
                .text_color(tokens.neutral.foreground)
                .child(label.to_owned()),
        )
        .child(input)
}
fn alert(text: &str, variant: vessel_app_view::alert::Variant, tokens: &Tokens) -> Div {
    vessel_app_kit::alert::alert(
        vessel_app_kit::alert::Content {
            title: Some(text.to_owned().into()),
            description: None,
            action: None,
        },
        crate::badge::alert_variant(variant),
        vessel_app_kit::alert::Size::Small,
        tokens,
    )
}
