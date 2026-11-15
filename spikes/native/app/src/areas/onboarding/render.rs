//! The setup shell draws the existing settings controls without a second implementation.

use vessel_app_kit::dialog::{DialogContent, DialogSize, dialog as kit_dialog};
use vessel_app_kit::theme::spacing::{S2, S4};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, button, button_with_kind, outline_button, settings,
};
use vessel_app_view::onboarding::{Open, State, Step, Visit};

use crate::shell::Shell;

// Paseo onboarding-modal.tsx styles.rail.width; the shared kit owns card bounds.
const RAIL_WIDTH: f32 = 200.0;

pub(super) fn title(shell: &Shell, step: Step) -> &str {
    let words = shell.words.desktop();
    match step {
        Step::Appearance => words.appearance(),
        Step::Providers => words.providers(),
        Step::Projects => words.projects(),
        Step::Orchestration => words.orchestration(),
        Step::Phone => words.phone(),
    }
}

fn description(shell: &Shell, step: Step) -> &str {
    let words = shell.words.desktop();
    match step {
        Step::Appearance => words.appearance_note(),
        Step::Providers => words.providers_note(),
        Step::Projects => words.projects_note(),
        Step::Orchestration => words.tools_note(),
        Step::Phone => words.phone_note(),
    }
}

/// A desktop-only Settings entry, disabled until a setup host is known.
pub(crate) fn setup_row(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.desktop();
    let control = outline_button(
        "show-setup",
        words.show_setup(),
        !shell.fleet.machines.is_empty(),
        tokens,
    )
    .on_click(cx.listener(|shell, _event, window, cx| shell.reopen_setup(window, cx)))
    .into_any_element();
    settings::row_at(
        words.setup(),
        Some(words.setup_note().into()),
        None,
        Some(control),
        false,
        compact,
        tokens,
    )
}

/// A bounded scrolling dialog preserves the footer at compact and minimum sizes.
pub(crate) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let State::Open(open) = shell.areas.onboarding.model.state() else {
        return None;
    };
    let words = shell.words.desktop();
    let step = open.step();
    let visit = open.visit();
    let compact = crate::areas::draft::compact(window);
    let ready = super::scope::actionable(shell);
    let advancing = ready && (step != Step::Orchestration || !super::orchestration::busy(shell));
    let content = if ready {
        crate::areas::settings::onboarding::step(shell, step, tokens, compact, window, cx)
    } else {
        div()
            .child(super::scope::hint(shell).to_owned())
            .into_any_element()
    };
    let current = open
        .steps()
        .iter()
        .position(|item| *item == step)
        .map_or(1, |index| index.saturating_add(1));
    let counter = words
        .step_counter(&current.to_string(), &open.steps().len().to_string())
        .unwrap_or_else(|_format| words.setup().to_owned());
    let pane = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .gap(px(S4))
        .when(compact, |pane| pane.child(counter))
        .child(title(shell, step).to_owned())
        .child(description(shell, step).to_owned())
        .child(content);
    let body = div()
        .flex()
        .flex_row()
        .gap(px(S4))
        .when(!compact, |body| body.child(rail(shell, open, tokens, cx)))
        .child(pane);
    let last = open.steps().last() == Some(&step);
    let footer = footer(shell, step, visit, last, advancing, tokens, cx);
    Some(kit_dialog(
        &shell.areas.onboarding.dialog,
        DialogContent {
            title: words.setup().into(),
            body: body.into_any_element(),
            footer,
            size: DialogSize::Setup,
        },
        tokens,
        window,
        compact,
        move |shell: &mut Shell, window, cx| shell.setup_complete(visit, window, cx),
        cx,
    ))
}

fn rail(shell: &Shell, open: &Open, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let step = open.step();
    let visit = open.visit();
    let mut rail = div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .w(px(RAIL_WIDTH))
        .gap(px(S2));
    for item in open.steps() {
        let item = *item;
        rail = rail.child(
            button(
                gpui::ElementId::Name(format!("setup-{item:?}").into()),
                title(shell, item).to_owned(),
                item == step,
                tokens,
            )
            .on_click(cx.listener(move |shell, _event, window, cx| {
                shell.setup_move(visit, Some(item), window, cx);
            })),
        );
    }
    rail
}

fn footer(
    shell: &Shell,
    step: Step,
    visit: Visit,
    last: bool,
    advancing: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let words = shell.words.desktop();
    vec![
        button("setup-skip", words.skip(), false, tokens)
            .when(last, vessel_app_kit::ui::Styled::invisible)
            .on_click(cx.listener(move |shell, _event, window, cx| {
                shell.setup_complete(visit, window, cx);
            }))
            .into_any_element(),
        button("setup-back", words.back(), false, tokens)
            .when(
                step == Step::Appearance,
                vessel_app_kit::ui::Styled::invisible,
            )
            .on_click(cx.listener(move |shell, _event, window, cx| {
                shell.setup_move(visit, None, window, cx);
            }))
            .into_any_element(),
        button_with_kind(
            "setup-next",
            if last { words.finish() } else { words.next() },
            ButtonKind::Primary,
            if advancing {
                ButtonState::Enabled
            } else {
                ButtonState::Disabled
            },
            tokens,
        )
        .when(advancing, |button| {
            button.on_click(cx.listener(move |shell, _event, window, cx| {
                shell.setup_continue(visit, window, cx);
            }))
        })
        .into_any_element(),
    ]
}
