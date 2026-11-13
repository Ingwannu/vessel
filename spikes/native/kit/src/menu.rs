//! A menu of actions behind a trigger (DESIGN.md section 6.13; the page's
//! `ui/kit/menu.tsx`): the Session actions, a machine's, a project's.
//!
//! [Decision Log]
//! - Purpose: one menu for every screen, with the page's look (13-pixel
//!   rows about 32 pixels tall, the highlighted row on a 4% wash, a
//!   hairline between groups, the one destructive row in danger) and its
//!   behaviour (arrow keys, Enter, Escape and a click outside to close).
//! - Alternatives: `gpui-component`'s popup menu (its own theme, actions
//!   and key contexts registered app-wide, submenus we do not need yet;
//!   screens would then depend on its types).
//! - Choice: the menu's state (open where, which row is highlighted, its
//!   focus) is a value the screen keeps; the menu is drawn from it each
//!   frame, over everything, at the point it was opened from, kept
//!   inside the window. A chosen row is handed back by index, so the
//!   screen keeps its own list of what each row does.
//! - Cost: no typeahead and no submenus yet (no menu of ours needs them);
//!   rows are 44 pixels on a narrow window, for a finger.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, ElementId, FocusHandle, KeyDownEvent, MouseButton, Pixels, Point,
    SharedString, Window, anchored, deferred, div, px,
};

use crate::fonts::Type;
use crate::popup;
use crate::theme::{Tokens, radius};
use crate::widgets::TypeExt as _;

/// A row's tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// An ordinary action.
    Plain,
    /// The one destructive action (delete, kill), kept apart by a line.
    Danger,
}

/// One row of a menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    /// Its words.
    pub label: SharedString,
    /// Its tone.
    pub tone: Tone,
    /// Shown but not chosen.
    pub disabled: bool,
    /// A hairline above it: it starts a group.
    pub starts_group: bool,
}

impl MenuItem {
    /// A plain row.
    #[must_use]
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            tone: Tone::Plain,
            disabled: false,
            starts_group: false,
        }
    }

    /// The destructive row, in a group of its own.
    #[must_use]
    pub fn danger(label: impl Into<SharedString>) -> Self {
        Self {
            tone: Tone::Danger,
            starts_group: true,
            ..Self::new(label)
        }
    }

    /// The same row, starting a group.
    #[must_use]
    pub fn grouped(self) -> Self {
        Self {
            starts_group: true,
            ..self
        }
    }
}

/// A menu's state, kept by the screen that shows it.
#[derive(Debug)]
pub struct MenuState {
    at: Option<Point<Pixels>>,
    highlighted: Option<usize>,
    focus: FocusHandle,
}

impl MenuState {
    /// A closed menu.
    pub fn new(cx: &mut App) -> Self {
        Self {
            at: None,
            highlighted: None,
            focus: cx.focus_handle(),
        }
    }

    /// Open it at `at` (where its trigger was pressed), taking the keys.
    pub fn open(&mut self, at: Point<Pixels>, window: &mut Window, cx: &mut App) {
        self.at = Some(at);
        self.highlighted = None;
        self.focus.focus(window, cx);
    }

    /// Close it.
    pub const fn close(&mut self) {
        self.at = None;
        self.highlighted = None;
    }

    /// Whether it is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.at.is_some()
    }

    /// Move the highlight to the next row that can be chosen, forward or
    /// back, wrapping.
    fn step(&mut self, items: &[MenuItem], forward: bool) {
        let count = items.len();
        if count == 0 {
            return;
        }
        let mut at = self.highlighted;
        for _ in 0..count {
            let next = match (at, forward) {
                (None, true) => 0,
                (None | Some(0), false) => count.saturating_sub(1),
                (Some(index), true) => index.saturating_add(1).checked_rem(count).unwrap_or(0),
                (Some(index), false) => index.saturating_sub(1),
            };
            at = Some(next);
            if items.get(next).is_some_and(|item| !item.disabled) {
                self.highlighted = at;
                return;
            }
        }
    }
}

/// What a screen does with a chosen row: the screen, the row's index.
pub type Chose<V> = Rc<dyn Fn(&mut V, usize, &mut Window, &mut Context<'_, V>)>;

/// What a screen hands the menu: how to reach its state, and what to do
/// with a chosen row.
pub struct MenuHandlers<V> {
    /// The screen's menu state.
    pub state: fn(&mut V) -> &mut MenuState,
    /// A row was chosen (the menu has closed).
    pub chose: Chose<V>,
}

impl<V> std::fmt::Debug for MenuHandlers<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuHandlers").finish_non_exhaustive()
    }
}

/// The menu, when open: drawn over everything at the point it was opened
/// from. `narrow`: a touch window (44-pixel rows).
pub fn menu<V: 'static>(
    id: impl Into<ElementId>,
    state: &MenuState,
    items: &[MenuItem],
    handlers: &MenuHandlers<V>,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    let at = state.at?;
    let get = handlers.state;
    let keys_items: Rc<[MenuItem]> = items.into();
    let chose = Rc::clone(&handlers.chose);
    let on_key = cx.listener(move |view: &mut V, event: &KeyDownEvent, window, cx| {
        let menu = get(view);
        match event.keystroke.key.as_str() {
            "escape" => menu.close(),
            "down" | "tab" => menu.step(&keys_items, true),
            "up" => menu.step(&keys_items, false),
            "home" => {
                menu.highlighted = None;
                menu.step(&keys_items, true);
            }
            "end" => {
                menu.highlighted = None;
                menu.step(&keys_items, false);
            }
            "enter" | "space" => {
                if let Some(index) = menu.highlighted {
                    menu.close();
                    chose(view, index, window, cx);
                }
            }
            _other => return,
        }
        cx.stop_propagation();
        cx.notify();
    });
    let on_outside = cx.listener(move |view: &mut V, _event, _window, cx| {
        get(view).close();
        cx.notify();
    });
    let mut panel = div()
        .id(id.into())
        .track_focus(&state.focus)
        .on_key_down(on_key)
        .on_mouse_down_out(on_outside)
        .occlude()
        .flex()
        .flex_col()
        .gap(px(1.0))
        .min_w(px(160.0))
        .max_w(px(320.0))
        .p(px(4.0))
        .bg(popup::fill(tokens))
        .border_1()
        .border_color(popup::edge(tokens))
        .rounded(px(radius::R4))
        .shadow(popup::shadow(tokens));
    for (index, item) in items.iter().enumerate() {
        if item.starts_group && index > 0 {
            panel = panel.child(div().my(px(4.0)).h(px(1.0)).bg(tokens.border_subtle));
        }
        panel = panel.child(row(index, item, state, handlers, tokens, narrow, cx));
    }
    Some(
        deferred(
            anchored()
                .position(at)
                .offset(gpui::point(px(0.0), px(6.0)))
                .snap_to_window_with_margin(px(8.0))
                .child(panel),
        )
        .with_priority(1)
        .into_any_element(),
    )
}

fn row<V: 'static>(
    index: usize,
    item: &MenuItem,
    state: &MenuState,
    handlers: &MenuHandlers<V>,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let highlighted = state.highlighted == Some(index);
    let (ink, wash) = match item.tone {
        Tone::Plain => (
            if highlighted {
                tokens.text_primary
            } else {
                tokens.text_secondary
            },
            tokens.bg_hover,
        ),
        Tone::Danger => (
            tokens.danger,
            gpui::Hsla {
                a: 0.12,
                ..tokens.danger
            },
        ),
    };
    let ink = if item.disabled {
        tokens.text_disabled
    } else {
        ink
    };
    let get = handlers.state;
    let chose = Rc::clone(&handlers.chose);
    let mut element = div()
        .id(ElementId::NamedInteger(
            "menu-row".into(),
            u64::try_from(index).unwrap_or(u64::MAX),
        ))
        .flex()
        .items_center()
        .min_h(px(popup::row_height(narrow)))
        .px(px(8.0))
        .py(px(6.0))
        .rounded(px(radius::R2))
        .type_(Type::Label)
        .text_color(ink)
        .when(highlighted && !item.disabled, |row| row.bg(wash))
        .child(item.label.clone());
    if !item.disabled {
        element = element
            .cursor_pointer()
            .hover(move |style| style.bg(wash))
            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                cx.stop_propagation();
            })
            .on_click(cx.listener(move |view: &mut V, _event, window, cx| {
                get(view).close();
                chose(view, index, window, cx);
                cx.notify();
            }));
    }
    element.into_any_element()
}
