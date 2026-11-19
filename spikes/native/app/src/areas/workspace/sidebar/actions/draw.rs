//! A project row's actions drawn (split from `actions.rs`, at the file
//! budget, docs/QUALITY.md M3): the plus and the kebab at the row's end,
//! and the menu, the remove question and the plus's tooltip over the
//! sidebar. What they do is `actions.rs`'s.

use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::menu::{Content, ItemStatus, MenuHandlers, MenuItem, menu_with};
use vessel_app_kit::theme::{SidebarSurfaces, radius};
use vessel_app_kit::tooltip::{Configuration, Delay, Offset, Side};
use vessel_app_kit::ui::{
    AnyElement, ClickEvent, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, MouseButton, MouseDownEvent, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, px, svg,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, ControlSizing, DialogContent, DialogSize, Tokens, Type, TypeExt as _,
    anchor_probe, breakpoint, button, button_with_kind, dialog,
};

use super::Target;
use crate::shell::Shell;

/// The plus and the kebab's square on a wide window, the kebab's on a
/// phone (Paseo's `projectIconActionButton`, `projectKebabButton`).
const ACTION: f32 = 24.0;
const ACTION_COMPACT: f32 = 32.0;
/// The plus's glyph (Paseo's 15).
const PLUS: f32 = 15.0;
/// The kebab's glyph (`sm` density: 14, 18 on a phone).
const KEBAB: f32 = 14.0;
const KEBAB_COMPACT: f32 = 18.0;
/// Between the two (Paseo's 2).
const ACTIONS_GAP: f32 = 2.0;
/// The actions pulled through the kebab's unpainted edge onto the row's
/// rail (Paseo's `marginRight: -6`). Negative literals: sizes take no
/// arithmetic.
const ACTIONS_BACK: f32 = -6.0;
/// On a phone the 32 kebab hangs out by 4 so the row keeps its 44
/// (`marginRight: -4`, `marginVertical: -4`).
const KEBAB_BACK: f32 = -4.0;
/// The menu's width (`DropdownMenuContent width={220}`).
const MENU_WIDTH: f32 = 220.0;

/// The kebab's name, and its place's key.
pub(super) fn menu_key(target: &Target) -> String {
    format!("sidebar-project-{}-{}-menu", target.machine, target.project)
}

/// The row's end: the plus (when `new_workspace`), then the kebab.
pub(in crate::areas::workspace::sidebar) fn trailing(
    target: &Target,
    new_workspace: Option<&Target>,
    group: &SharedString,
    compact: bool,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let rows = &shell.areas.workspace.sidebar;
    let menu_open = rows.menu_is_for(target);
    let hidden = |element: vessel_app_kit::ui::Stateful<Div>, keep: bool| {
        element.when(!(compact || keep), |element| {
            element
                .opacity(0.0)
                .group_hover(group.clone(), |style| style.opacity(1.0))
        })
    };
    let words = shell.t.lang.sidebar();
    let plus = new_workspace.map(|target| {
        let pressed = target.clone();
        let key = format!("{group}-new");
        let button = icon_button(
            ElementId::Name(key.clone().into()),
            Glyph::Plus,
            PLUS,
            ACTION,
            SidebarSurfaces::of(tokens).hover,
            tokens,
        )
        .role(gpui::Role::Button)
        .aria_label(words.create_workspace_for(&target.name))
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .on_click(cx.listener(move |this, _event, window, cx| {
            cx.stop_propagation();
            this.new_workspace_in(&pressed, window, cx);
        }));
        let button = rows.tips.on_configured(
            button,
            &rows.place(&key),
            words.new_workspace,
            Configuration::new(Side::Bottom, Offset::EIGHT, Delay::IMMEDIATE),
            ControlSizing::from_compact(compact),
        );
        hidden(button, false)
    });
    let opened = target.clone();
    let activated = target.clone();
    let (side, mark) = if compact {
        (ACTION_COMPACT, KEBAB_COMPACT)
    } else {
        (ACTION, KEBAB)
    };
    let key = menu_key(target);
    let kebab = icon_button(
        ElementId::Name(key.clone().into()),
        Glyph::MoreVertical,
        mark,
        side,
        tokens.neutral.surface2,
        tokens,
    )
    .when(compact, |kebab| kebab.mr(px(KEBAB_BACK)).my(px(KEBAB_BACK)))
    .role(gpui::Role::Button)
    .aria_label(words.project_actions)
    .on_mouse_down(
        MouseButton::Left,
        cx.listener(move |this, event: &MouseDownEvent, window, cx| {
            cx.stop_propagation();
            this.project_menu_open(opened.clone(), event.position, window, cx);
        }),
    )
    // A click (assistive technology's Click action, or the end of a
    // pointer's press) opens it too, once.
    .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
        cx.stop_propagation();
        this.project_menu_activate(activated.clone(), event.position(), window, cx);
    }))
    .relative()
    .child(anchor_probe(&rows.place(&key)));
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(ACTIONS_GAP))
        .flex_none()
        .mr(px(ACTIONS_BACK))
        .children(plus)
        .child(hidden(kebab, menu_open))
}

/// A square action: `glyph` muted, the foreground and `wash` behind it on
/// hover.
fn icon_button(
    id: ElementId,
    glyph: Glyph,
    size: f32,
    square: f32,
    wash: vessel_app_kit::ui::Hsla,
    tokens: &Tokens,
) -> vessel_app_kit::ui::Stateful<Div> {
    let neutral = tokens.neutral;
    let group: SharedString = format!("{id}").into();
    div()
        .id(id)
        .group(group.clone())
        .size(px(square))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(radius::MD))
        .cursor_pointer()
        .hover(move |style| style.bg(wash))
        .child(
            svg()
                .path(glyph.path())
                .size(px(size))
                .flex_none()
                .text_color(neutral.foreground_muted)
                .group_hover(group, move |style| style.text_color(neutral.foreground)),
        )
}

/// The menu while open, the remove question while asked, and the plus's
/// tooltip.
pub(crate) fn layers(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let rows = &shell.areas.workspace.sidebar;
    let words = shell.t.lang.sidebar();
    let phone = breakpoint::phone(window);
    let mut drawn = Vec::new();
    if let Some(target) = rows.target.as_ref() {
        let removing = rows.is_removing(&target.machine, target.project);
        let mut remove = MenuItem::new(words.remove_project).with_leading(Glyph::Trash2);
        if removing {
            remove.status = ItemStatus::Pending(words.removing_project.into());
        }
        let items = [
            MenuItem::new(words.open_project_settings).with_leading(Glyph::Gear),
            remove,
        ];
        let handlers = MenuHandlers {
            state: |shell: &mut Shell| &mut shell.areas.workspace.sidebar.menu,
            chose: Rc::new(|shell: &mut Shell, index, window, cx| {
                shell.project_menu_chose(index, window, cx);
            }),
        };
        let content = Content::new("sidebar-project-menu", &items)
            .with_width(px(MENU_WIDTH))
            .with_sheet(words.project_actions);
        drawn.extend(menu_with(content, &rows.menu, &handlers, tokens, phone, cx));
    }
    if let Some(target) = rows.confirm.as_ref() {
        let footer = vec![
            button("project-remove-cancel", words.cancel, false, tokens)
                .on_click(cx.listener(|shell, _event, window, cx| {
                    shell.project_remove_dismiss(window, cx);
                }))
                .into_any_element(),
            button_with_kind(
                "project-remove-confirm",
                words.remove_confirm,
                ButtonKind::Destructive,
                ButtonState::Enabled,
                tokens,
            )
            .on_click(cx.listener(|shell, _event, window, cx| {
                shell.project_remove_confirmed(window, cx);
            }))
            .into_any_element(),
        ];
        drawn.push(dialog(
            &rows.dialog,
            DialogContent {
                title: words.remove_project_title.into(),
                body: div()
                    .type_(Type::Body)
                    .child(words.remove_project_body(&target.name))
                    .into_any_element(),
                footer,
                size: DialogSize::Normal,
            },
            tokens,
            window,
            phone,
            Shell::project_remove_dismiss,
            cx,
        ));
    }
    drawn.extend(rows.tips.render(tokens));
    drawn
}
