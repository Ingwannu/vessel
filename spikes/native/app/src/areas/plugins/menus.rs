use super::{SurfaceEvent, SurfaceView, draw};
use std::rc::Rc;
use vessel_app_kit::{
    MenuHandlers, MenuItem, Tokens,
    ui::{AnyElement, Context, Pixels, Point, Window},
};
use vessel_app_view::plugins::SurfaceProblem;
use vessel_proto::plugins::{Availability, ButtonBehavior, ContributionId, MenuEntry};

pub(super) struct Shown {
    control: ContributionId,
    path: Vec<ContributionId>,
    entries: Vec<MenuEntry>,
    at: Point<Pixels>,
}
impl SurfaceView {
    pub(super) fn press(
        &mut self,
        control: &ContributionId,
        behavior: &ButtonBehavior,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Err(problem) = self.with_registry(|registry| self.state.check_current(registry)) {
            cx.emit(SurfaceEvent::Rejected(problem));
            return;
        }
        match behavior {
            ButtonBehavior::Action { .. } => self.publish(
                self.with_registry(|registry| self.state.activate(registry, control, &[])),
                cx,
            ),
            ButtonBehavior::Menu { items } => {
                self.shown_menu = Some(Shown {
                    control: control.clone(),
                    path: vec![],
                    entries: items.clone(),
                    at,
                });
                self.menu.open(at, window, cx);
                cx.notify();
            }
            ButtonBehavior::Popover { .. } => {
                cx.emit(SurfaceEvent::Rejected(SurfaceProblem::Value));
            }
        }
    }

    fn choose(&mut self, index: usize, window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Err(problem) = self.with_registry(|registry| self.state.check_current(registry)) {
            cx.emit(SurfaceEvent::Rejected(problem));
            return;
        }
        let Some(shown) = &self.shown_menu else {
            return;
        };
        let (_, identities) = rows(&shown.entries);
        let Some(identity) = identities.get(index) else {
            return;
        };
        let Some(MenuEntry::Item {
            availability,
            behavior,
            ..
        }) = shown.entries.iter().find(|item| match item {
            MenuEntry::Separator { id } | MenuEntry::Item { id, .. } => id == identity,
        })
        else {
            return;
        };
        if *availability != Availability::Enabled {
            cx.emit(SurfaceEvent::Rejected(SurfaceProblem::Disabled));
            return;
        }
        let mut path = shown.path.clone();
        path.push(identity.clone());
        match behavior {
            ButtonBehavior::Action { .. } => {
                let request = self
                    .with_registry(|registry| self.state.activate(registry, &shown.control, &path));
                self.publish(request, cx);
            }
            ButtonBehavior::Menu { items } => {
                self.shown_menu = Some(Shown {
                    control: shown.control.clone(),
                    path,
                    entries: items.clone(),
                    at: shown.at,
                });
                if let Some(shown) = &self.shown_menu {
                    self.menu.open(shown.at, window, cx);
                }
            }
            ButtonBehavior::Popover { .. } => {
                cx.emit(SurfaceEvent::Rejected(SurfaceProblem::Value));
            }
        }
        cx.notify();
    }
}

fn rows(entries: &[MenuEntry]) -> (Vec<MenuItem>, Vec<ContributionId>) {
    let mut items = Vec::new();
    let mut identities = Vec::new();
    let mut group = false;
    for entry in entries {
        match entry {
            MenuEntry::Separator { .. } => group = true,
            MenuEntry::Item {
                id,
                title,
                availability,
                behavior,
            } => {
                if *availability == Availability::Hidden {
                    continue;
                }
                let mut row = MenuItem::new(title.as_str().to_owned());
                row.disabled = *availability != Availability::Enabled
                    || matches!(behavior, ButtonBehavior::Popover { .. });
                row.starts_group = group;
                group = false;
                items.push(row);
                identities.push(id.clone());
            }
        }
    }
    (items, identities)
}

pub(super) fn overlay(
    view: &SurfaceView,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, SurfaceView>,
) -> Option<AnyElement> {
    let shown = view.shown_menu.as_ref()?;
    let (mut items, _identities) = rows(&shown.entries);
    if view
        .with_registry(|registry| view.state.check_current(registry))
        .is_err()
    {
        for item in &mut items {
            item.disabled = true;
        }
    }
    vessel_app_kit::menu(
        "plugin-surface-menu",
        &view.menu,
        &items,
        &MenuHandlers {
            state: |view: &mut SurfaceView| &mut view.menu,
            chose: Rc::new(SurfaceView::choose),
        },
        tokens,
        draw::narrow(window),
        cx,
    )
}
