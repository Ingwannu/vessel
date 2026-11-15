//! One adaptive dialog preserves focus while picker becomes a fresh-plan confirm.
use super::{
    DialogScope, Intent,
    draw::{enabled, problem},
    on_dialog as intent_handler,
};
use crate::shell::Shell;
use vessel_app_kit::dialog::{DialogContent, DialogSize};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonConfig, ButtonContent, ButtonKind, ControlSizing, Tokens, Type, TypeExt as _,
    button_control, settings,
};
use vessel_app_view::orchestration::Confirmation;
use vessel_proto::skills::{SkillOperation, SkillSelection};

pub(crate) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.orchestration;
    let page = state.page.as_ref()?;
    let captured = DialogScope {
        scope: page.scope().clone(),
        ticket: state.dialog_ticket?,
    };
    let content = if let Some(confirm) = &page.confirmation {
        confirmation(
            shell,
            &captured,
            confirm,
            tokens,
            crate::areas::draft::compact(window),
            cx,
        )
    } else if page.picker.is_some() {
        picker(shell, tokens, crate::areas::draft::compact(window), cx)?
    } else {
        return None;
    };
    Some(vessel_app_kit::dialog::dialog(
        &state.dialog,
        content,
        tokens,
        window,
        crate::areas::draft::compact(window),
        {
            move |shell: &mut Shell, window, cx| {
                shell.orchestration_dialog_intent(&captured, Intent::Cancel, window, cx)
            }
        },
        cx,
    ))
}
fn picker(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<DialogContent> {
    let page = shell.areas.settings.orchestration.page.as_ref()?;
    let captured = DialogScope {
        scope: page.scope().clone(),
        ticket: shell.areas.settings.orchestration.dialog_ticket?,
    };
    let picker = page.picker.as_ref()?;
    let words = shell.words.orchestration();
    let working = page.skills.pending();
    let all = matches!(picker.draft, SkillSelection::All);
    let switch = settings::switch("orchestration-all", all, !working, compact, 0, tokens)
        .when(!working, |switch| {
            switch.on_click(intent_handler(&captured, Intent::All, cx))
        });
    let mut body =
        div()
            .flex()
            .flex_col()
            .gap(px(spacing::S4))
            .child(settings::card(tokens).child(settings::row_at(
                words.all_title(),
                Some(words.all_hint().into()),
                None,
                Some(switch.into_any_element()),
                true,
                compact,
                tokens,
            )));
    let mut list = settings::card(tokens);
    for (index, name) in picker.available.as_slice().iter().enumerate() {
        let label = name.as_str().to_owned();
        let name = name.clone();
        let chosen = picker.selected(&name);
        let on = !all && !working;
        let control = settings::switch(
            ElementId::Name(format!("orchestration-skill-{name}").into()),
            chosen,
            on,
            compact,
            0,
            tokens,
        )
        .when(on, |switch| {
            switch.on_click(intent_handler(&captured, Intent::Toggle(name.clone()), cx))
        });
        list = list.child(settings::row_at(
            label,
            None,
            None,
            Some(control.into_any_element()),
            index == 0,
            compact,
            tokens,
        ));
    }
    if picker.available.is_empty() {
        list = list.p(px(spacing::S4)).child(words.empty());
    }
    body = body.child(settings::section(words.available(), list, tokens));
    if let Some(cause) = page.skills.problem() {
        body = body.child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.destructive)
                .child(problem(shell, cause).to_owned()),
        );
    }
    if page.skills.requires_read() && !shell.areas.settings.orchestration.exhausted {
        body = body.child(action(
            "orchestration-picker-refresh",
            words.refresh(),
            config(ButtonKind::Secondary, !working, compact),
            &captured,
            Intent::Refresh,
            tokens,
            cx,
        ));
    }
    Some(DialogContent {
        title: words.choose().into(),
        body: body.into_any_element(),
        footer: picker_footer(shell, &captured, tokens, compact, cx),
        size: DialogSize::Wide,
    })
}
fn confirmation(
    shell: &Shell,
    captured: &DialogScope,
    confirm: &Confirmation,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> DialogContent {
    let words = shell.words.orchestration();
    let (title, message, submit, destructive) = match confirm {
        Confirmation::Uninstall(_) => (
            words.uninstall_title(),
            words.uninstall_message(),
            words.uninstall(),
            true,
        ),
        Confirmation::Update(_) => (
            words.update_title(),
            words.update_message(),
            words.update(),
            false,
        ),
        Confirmation::Removals { .. } => (
            words.remove_title(),
            words.remove_message(),
            words.remove(),
            true,
        ),
    };
    let mut body = div().flex().flex_col().gap(px(spacing::S2)).child(
        div()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(message.to_owned()),
    );
    match confirm {
        Confirmation::Uninstall(names) | Confirmation::Removals { names, .. } => {
            for name in names.as_slice() {
                body = body.child(name_row(name.as_str(), None, tokens));
            }
        }
        Confirmation::Update(ops) => {
            for op in ops {
                let verb = match op {
                    SkillOperation::Add { .. } => words.add(),
                    SkillOperation::Update { .. } => words.operation_update(),
                    SkillOperation::Delete { .. } => words.remove(),
                };
                body = body.child(name_row(op.skill().as_str(), Some(verb), tokens));
            }
        }
    }
    DialogContent {
        title: title.to_owned().into(),
        body: body.into_any_element(),
        footer: vec![
            action(
                "orchestration-confirm-cancel",
                words.cancel(),
                config(ButtonKind::Secondary, true, compact),
                captured,
                Intent::Cancel,
                tokens,
                cx,
            ),
            action(
                "orchestration-confirm",
                submit,
                config(
                    if destructive {
                        ButtonKind::Destructive
                    } else {
                        ButtonKind::Primary
                    },
                    !shell.areas.settings.orchestration.exhausted,
                    compact,
                ),
                captured,
                Intent::Confirm,
                tokens,
                cx,
            ),
        ],
        size: DialogSize::Normal,
    }
}
fn name_row(name: &str, verb: Option<&str>, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_wrap()
        .gap(px(spacing::S2))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground)
        .children(verb.map(|verb| {
            div()
                .text_color(tokens.neutral.foreground_muted)
                .child(verb.to_owned())
        }))
        .child(name.to_owned())
}
fn config(kind: ButtonKind, on: bool, compact: bool) -> ButtonConfig {
    ButtonConfig::new(kind, enabled(on), ControlSizing::from_compact(compact))
}
fn action(
    id: &'static str,
    label: &str,
    config: ButtonConfig,
    captured: &DialogScope,
    intent: Intent,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    button_control(
        id,
        ButtonContent::Label(label.to_owned().into()),
        config,
        tokens,
    )
    .on_click(intent_handler(captured, intent, cx))
    .into_any_element()
}

fn picker_footer(
    shell: &Shell,
    captured: &DialogScope,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let words = shell.words.orchestration();
    let working = shell
        .areas
        .settings
        .orchestration
        .page
        .as_ref()
        .is_some_and(|page| page.skills.pending());
    let blocked = shell.areas.settings.orchestration.exhausted
        || shell
            .areas
            .settings
            .orchestration
            .page
            .as_ref()
            .is_none_or(|page| page.skills.requires_read());
    vec![
        action(
            "orchestration-picker-cancel",
            words.cancel(),
            config(ButtonKind::Secondary, !working, compact),
            captured,
            Intent::Cancel,
            tokens,
            cx,
        ),
        action(
            "orchestration-picker-save",
            if working {
                words.saving()
            } else {
                words.save()
            },
            if working {
                config(ButtonKind::Primary, false, compact).loading()
            } else {
                config(ButtonKind::Primary, !blocked, compact)
            },
            captured,
            Intent::Save,
            tokens,
            cx,
        ),
    ]
}
