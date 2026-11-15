//! The setup shell draws the existing settings controls without a second implementation.

use vessel_app_kit::dialog::{DialogContent, DialogSize, dialog as kit_dialog};
use vessel_app_kit::theme::spacing::{S2, S4};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, button, outline_button, settings};
use vessel_app_view::onboarding::{State, Step};

use crate::shell::Shell;

/// Setup is above the command palette; both reuse their existing dialog focus.
pub(crate) fn overlays(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> impl Iterator<Item = AnyElement> {
    crate::areas::palette::render(shell, tokens, window, cx)
        .into_iter()
        .chain(dialog(shell, tokens, window, cx))
}

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
    let mut rail = div().flex().flex_row().flex_wrap().gap(px(S2));
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
                shell.setup_move(Some(item), window, cx)
            })),
        );
    }
    let body = div().flex().flex_col().gap(px(S4)).child(rail).child(
        crate::areas::settings::onboarding::step(shell, step, tokens, compact, window, cx),
    );
    let last = open.steps().last() == Some(&step);
    let footer = vec![
        button("setup-skip", words.skip(), false, tokens)
            .when(last, |button| button.invisible())
            .on_click(cx.listener(|shell, _event, window, cx| shell.setup_complete(window, cx)))
            .into_any_element(),
        button("setup-back", words.back(), false, tokens)
            .when(step == Step::Appearance, |button| button.invisible())
            .on_click(cx.listener(|shell, _event, window, cx| shell.setup_move(None, window, cx)))
            .into_any_element(),
        button(
            "setup-next",
            if last { words.finish() } else { words.next() },
            true,
            tokens,
        )
        .on_click(
            cx.listener(move |shell, _event, window, cx| shell.setup_continue(visit, window, cx)),
        )
        .into_any_element(),
    ];
    Some(kit_dialog(
        &shell.areas.onboarding.dialog,
        DialogContent {
            title: words.setup().into(),
            body: body.into_any_element(),
            footer,
            size: DialogSize::Wide,
        },
        tokens,
        window,
        compact,
        |shell: &mut Shell, window, cx| shell.setup_complete(window, cx),
        cx,
    ))
}
