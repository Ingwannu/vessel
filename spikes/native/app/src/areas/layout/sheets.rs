//! Compact overlays retain the tab id and consult dirty state on every dismissal.
use super::registry;
use crate::shell::Shell;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    Context, Div, IntoElement, ParentElement as _, StatefulInteractiveElement as _, Styled as _,
    Window, div, px,
};
use vessel_app_kit::{Tokens, button};
use vessel_app_view::layout::{Layout, LayoutKey, SheetPromotion, TabId};

pub(super) fn overlay(
    shell: &Shell,
    layout: &Layout,
    key: &LayoutKey,
    agent: &mut Option<registry::AgentRenderer>,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    let sheet = layout.sheets().last()?;
    let tab = layout.tab(sheet.tab)?;
    let id = tab.id();
    let close_key = key.clone();
    let promote_key = key.clone();
    let dismiss_key = key.clone();
    let words = shell.words.layout();
    let mut controls = div().flex().items_center().gap(px(spacing::S2)).child(
        button(
            "layout-sheet-close",
            words.close().to_owned(),
            false,
            tokens,
        )
        .on_click(cx.listener(move |shell, _event, _window, cx| {
            shell.layout_dismiss_sheet(&close_key, id, cx);
        })),
    );
    if sheet.promotion == SheetPromotion::Allowed {
        controls = controls.child(
            button(
                "layout-sheet-promote",
                words.promote().to_owned(),
                false,
                tokens,
            )
            .on_click(cx.listener(move |shell, _event, _window, cx| {
                let result =
                    super::with_layout_at(shell, &promote_key, |layout| layout.promote_sheet(id));
                shell.layout_result(result, cx);
            })),
        );
    }
    let body = registry::render(
        shell,
        &registry::PanelContext {
            workspace: key,
            pane: layout.focused_pane(),
            tab: id,
            target: tab.target(),
            focus: registry::PanelFocus::Focused,
        },
        agent,
        tokens,
        window,
        cx,
    );
    let content = div()
        .flex()
        .flex_col()
        .min_h_0()
        .p(px(spacing::S2))
        .child(controls)
        .child(body);
    Some(vessel_app_kit::sheet(
        "layout-sheet",
        vessel_app_kit::Edge::Bottom,
        content.into_any_element(),
        tokens,
        cx.listener(move |shell, _event, _window, cx| {
            shell.layout_dismiss_sheet(&dismiss_key, id, cx);
        }),
    ))
}
impl Shell {
    pub(super) fn layout_dismiss_sheet(
        &mut self,
        key: &LayoutKey,
        id: TabId,
        cx: &mut Context<'_, Self>,
    ) {
        #[cfg(test)]
        let before = registry::retirement::Before::layout(self, key);
        let result = super::with_layout_at(self, key, |layout| layout.dismiss_sheet(id));
        #[cfg(test)]
        if result.is_ok() {
            self.layout_retired(before, registry::retirement::Source::Local, cx);
        }
        self.layout_result(result, cx);
    }
}
