//! One dock/header, with either legacy controls or the layout owner's captured
//! controls. The ephemeral panel context is presentation, never file admission.
use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::tabs::{tab, tab_head};
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, breakpoint, glyph, popup};
use vessel_app_view::layout::{PaneId, Target};

use super::{Tab, layout_adapter::ExplorerKind};
use crate::areas::layout::registry::PanelContext;
use crate::shell::Shell;

/// Layout-owned callbacks retain their original key/tab and recheck it before
/// changing metadata. The dock must not resolve a later selected global target.
pub(crate) type ExplorerSelect =
    Rc<dyn Fn(&mut Shell, ExplorerKind, &mut Window, &mut Context<'_, Shell>)>;
/// Both the close button and scrim use this same captured hide operation.
pub(crate) type ExplorerHide = Rc<dyn Fn(&mut Shell, &mut Window, &mut Context<'_, Shell>)>;

/// Borrowed only during the layout owner's synchronous Explorer projection.
/// Its presence, not the legacy open flag, selects the typed rendering branch.
pub(crate) struct ExplorerHost<'a> {
    pub(crate) panel: &'a PanelContext<'a>,
    pub(crate) select: ExplorerSelect,
    pub(crate) hide: ExplorerHide,
}

impl ExplorerHost<'_> {
    pub(super) fn selected(&self) -> Option<ExplorerKind> {
        if self.panel.pane != PaneId::EXPLORER {
            return None;
        }
        match self.panel.target {
            Target::Files => Some(ExplorerKind::Files),
            Target::Changes => Some(ExplorerKind::Changes),
            _ => None,
        }
    }
}

pub(super) fn hide(host: Option<&ExplorerHost<'_>>) -> ExplorerHide {
    host.map_or_else(
        || {
            Rc::new(
                |shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>| {
                    shell.toggle_code(window, cx)
                },
            ) as ExplorerHide
        },
        |host| Rc::clone(&host.hide),
    )
}

pub(super) fn rail(
    shell: &Shell,
    host: Option<&ExplorerHost<'_>>,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let selected = host.map_or_else(
        || {
            Some(match shell.areas.workspace.code.tab {
                Tab::Files => ExplorerKind::Files,
                Tab::Changes => ExplorerKind::Changes,
            })
        },
        ExplorerHost::selected,
    );
    let select = host.map_or_else(
        || {
            Rc::new(
                |shell: &mut Shell, kind, window: &mut Window, cx: &mut Context<'_, Shell>| {
                    super::layout_adapter::show_layout_explorer(shell, kind, window, cx);
                },
            ) as ExplorerSelect
        },
        |host| Rc::clone(&host.select),
    );
    let labels = shell.words().labels();
    let tabs = div().children(
        [
            (
                "code-tab-changes",
                labels.panel_changes,
                ExplorerKind::Changes,
            ),
            ("code-tab-files", labels.panel_files, ExplorerKind::Files),
        ]
        .into_iter()
        .map(|(id, label, kind)| {
            let select = Rc::clone(&select);
            tab(id, label, selected == Some(kind), None, tokens)
                .on_click(cx.listener(move |shell, _, window, cx| select(shell, kind, window, cx)))
        }),
    );
    let neutral = tokens.neutral;
    let hide = hide(host);
    let close = div().child(
        div()
            .id("code-close")
            .size(px(if breakpoint::phone(window) {
                44.0
            } else {
                28.0
            }))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(popup::MENU_RADIUS))
            .cursor_pointer()
            .hover(move |style| style.bg(neutral.surface2))
            .child(glyph(Glyph::Close, 14.0, neutral.foreground_muted))
            .on_click(cx.listener(move |shell, _, window, cx| hide(shell, window, cx))),
    );
    tab_head(tabs, Some(close), tokens).into_any_element()
}

#[cfg(test)]
mod tests;
