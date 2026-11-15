//! Setup embeds each existing settings owner instead of maintaining parallel controls.

use crate::shell::Shell;
use vessel_app_kit::Tokens;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, Window, div,
};
use vessel_app_view::onboarding::Step;

pub(crate) fn step(
    shell: &Shell,
    step: Step,
    tokens: &Tokens,
    compact: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    match step {
        Step::Appearance => super::appearance::render(shell, tokens, compact, cx),
        Step::Providers => super::agents::render(shell, tokens, compact, window, cx),
        Step::Projects => crate::areas::workspace::projects_content(shell, tokens, compact, cx),
        Step::Orchestration => {
            crate::areas::onboarding::orchestration_content(shell, tokens, compact, cx)
        }
        Step::Phone => div()
            .flex()
            .flex_col()
            .child(shell.words.desktop().phone_note())
            .child(super::devices::render(shell, tokens, compact, cx))
            .into_any_element(),
    }
}
