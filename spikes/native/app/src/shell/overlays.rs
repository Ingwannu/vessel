//! Append the existing overlay hosts in order, without a new layout wrapper.
//!
//! These remain unconditional when workspace chrome is hidden. Keeping the
//! registration chain here bounds the root renderer without changing layering,
//! lifecycle policy, focus ownership or any dialog's domain state.

use super::Shell;
use crate::areas;
use vessel_app_kit::Tokens;
use vessel_app_kit::ui::{Context, Div, ParentElement as _, Window};

pub(super) fn append(
    content: Div,
    shell: &mut Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    content
        .children(areas::toasts::render(shell, tokens, window, cx))
        .children(areas::palette::render(shell, tokens, window, cx))
        .children(areas::onboarding::dialog(shell, tokens, window, cx))
        .children(areas::local_node::dialog(shell, tokens, window, cx))
        .children(areas::desktop::render(shell, tokens, window, cx))
        .children(areas::shortcuts::help_dialog::render(
            shell, tokens, window, cx,
        ))
        .children(areas::import::render(shell, tokens, window, cx))
}
