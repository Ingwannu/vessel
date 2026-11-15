//! Browser-owned panel seam in W12's sole registry (ADR 0078).
//!
//! Restored browser slots may exist while this node has no qualified engine.
//! They get an explicit unavailable state, without a synthetic page, URL fetch,
//! open action or separate pane manager. Live owner state will be keyed by
//! (`LayoutKey`, `TabId`) and bound to the node resource when its typed stream exists.

// A10: only the qualified actor/panel wiring may enable frame preparation.
#[cfg(test)]
mod prepared_frame;

use crate::areas::layout::registry::PanelContext;
use crate::shell::Shell;
use vessel_app_kit::theme::spacing::S4;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};

pub(crate) fn render_panel(
    shell: &Shell,
    _panel: &PanelContext<'_>,
    tokens: &Tokens,
    _window: &mut Window,
    _cx: &mut Context<'_, Shell>,
) -> AnyElement {
    div()
        .size_full()
        .p(px(S4))
        .type_(Type::Body)
        .text_color(tokens.text_secondary)
        .child(shell.words.browser().no_engine().to_owned())
        .into_any_element()
}
