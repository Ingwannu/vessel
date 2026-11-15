//! Menus use the same typed operations and close planning as keyboard commands.
use super::with_layout;
use crate::shell::Shell;
use vessel_app_kit::ui::{AnyElement, Context, Window};
use vessel_app_kit::{MenuHandlers, MenuItem, Tokens};
use vessel_app_view::layout::{CloseEffect, CloseScope, LayoutAction, TabId, Target};

#[derive(Clone, Copy)]
enum Action {
    Copy,
    Rename,
    Close(CloseScope),
}
fn actions(shell: &Shell, tab: TabId) -> Vec<(Action, MenuItem)> {
    let words = shell.words.layout();
    let target = with_layout(shell, |layout| {
        Ok(layout.tab(tab).map(|tab| tab.target().clone()))
    })
    .ok()
    .flatten();
    let Some(target) = target else {
        return Vec::new();
    };
    let mut items = Vec::new();
    if matches!(
        target,
        Target::Agent(_) | Target::Terminal(_) | Target::File { .. }
    ) {
        let label = if matches!(target, Target::File { .. }) {
            words.copy_path()
        } else {
            words.copy_id()
        };
        items.push((Action::Copy, MenuItem::new(label.to_owned())));
    }
    if matches!(target, Target::Agent(_))
        && matches!(
            super::key(shell),
            Ok(vessel_app_view::layout::LayoutKey::Workspace { .. })
        )
    {
        items.push((Action::Rename, MenuItem::new(words.rename().to_owned())));
    }
    let local = with_layout(shell, |layout| {
        layout
            .request_close(tab)
            .map(|plan| plan.effect() == CloseEffect::Local)
    })
    .unwrap_or(false);
    if local {
        for (scope, label) in [
            (CloseScope::Before, words.close_left()),
            (CloseScope::After, words.close_right()),
            (CloseScope::Others, words.close_others()),
            (CloseScope::Current, words.close()),
        ] {
            let disabled = with_layout(shell, |layout| {
                layout.close_plans(tab, scope).map(|plans| {
                    plans.is_empty() || plans.iter().any(|plan| plan.effect() != CloseEffect::Local)
                })
            })
            .unwrap_or(true);
            let mut item = MenuItem::new(label.to_owned());
            item.disabled = disabled;
            items.push((Action::Close(scope), item));
        }
    }
    items
}
pub(super) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let (workspace, tab) = shell.areas.layout.menu_tab.as_ref()?;
    if super::key(shell).as_ref() != Ok(workspace) {
        return None;
    }
    let workspace = workspace.clone();
    let tab = *tab;
    let actions = actions(shell, tab);
    let items: Vec<_> = actions.iter().map(|(_, item)| item.clone()).collect();
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.layout.menu,
        chose: std::rc::Rc::new(move |shell: &mut Shell, index, window, cx| {
            if super::key(shell).as_ref() != Ok(&workspace) {
                return;
            }
            let Some((action, _)) = actions.get(index) else {
                return;
            };
            match action {
                Action::Rename => shell.layout_start_rename(tab, window, cx),
                Action::Copy => {
                    let target = with_layout(shell, |layout| {
                        Ok(layout.tab(tab).map(|tab| tab.target().clone()))
                    })
                    .ok()
                    .flatten();
                    let text = match target {
                        Some(Target::Agent(id)) => Some(id.to_string()),
                        Some(Target::Terminal(id)) => Some(id.to_string()),
                        Some(Target::File { path, .. }) => Some(path.as_str().to_owned()),
                        Some(
                            Target::NewTab
                            | Target::Draft(_)
                            | Target::ProviderSubagent { .. }
                            | Target::Browser(_)
                            | Target::Changes
                            | Target::Files
                            | Target::Scripts
                            | Target::PullRequest
                            | Target::WorkingDiff
                            | Target::Plugin { .. }
                            | Target::Setup(_)
                            | Target::CommitDiff(_),
                        )
                        | None => None,
                    };
                    if let Some(text) = text {
                        cx.write_to_clipboard(vessel_app_kit::ui::ClipboardItem::new_string(text));
                    }
                }
                Action::Close(scope) => {
                    shell.layout_select(tab, cx);
                    let _handled = shell.run_layout_action(LayoutAction::Close(*scope), window, cx);
                }
            }
        }),
    };
    vessel_app_kit::menu(
        "layout-tab-menu",
        &shell.areas.layout.menu,
        &items,
        &handlers,
        tokens,
        vessel_app_kit::breakpoint::phone(window),
        cx,
    )
}
