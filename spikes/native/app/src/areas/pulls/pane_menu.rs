//! The activity cards' menus: Paseo's `ActivityKebab` (`pull-request-panel/
//! pane.tsx`, pinned 0.15.4) on each comment and review, and the thread
//! header's (Open alone). The rows are the view's (`pull::card_menu`).
//!
//! [Decision Log]
//! - Purpose: Copy and Open on GitHub beside a card's Add to chat; a
//!   card's press folds it, so without the menu a person could neither.
//! - Constraint: one pane, many cards; the kit's menu is a value the
//!   screen keeps and draws over everything.
//! - Choice: one menu for the pane (the kit's `MenuState`), opened under
//!   the kebab pressed with its right edge on the kebab's (Paseo's
//!   `align="end"`, 200 wide), as the workspace sidebar's project menu
//!   opens. The kebab shows while its card is hovered, while its menu is
//!   open, and always on a phone (Paseo's `useRevealOnHover`). Rows are
//!   the kit's (role, name, Click only when choosable, TRACKER 808) with
//!   Paseo's leading marks.
//! - Differences: Copy puts the markdown source verbatim (Paseo's text
//!   flavour) but no HTML flavour beside it; Copy is translated (Paseo's
//!   says "Copy" in every language).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::menu::{Content, MenuHandlers, MenuItem, MenuState, menu_with};
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::tooltip::Place;
use vessel_app_kit::ui::{
    AnyElement, App, ClickEvent, ClipboardItem, Context, FluentBuilder as _,
    InteractiveElement as _, IntoElement as _, MouseButton, MouseDownEvent, ParentElement as _,
    Pixels, Point, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, anchor_probe, breakpoint};
use vessel_app_view::workspace::pull::card_menu::{CardAct, brand, copied, item_acts, thread_acts};
use vessel_app_view::workspace::pull::timeline::Thread;
use vessel_proto::forge::TimelineItem;

use super::pane_cards::Cards;
use crate::shell::Shell;

/// Paseo's `kebabSlot`: 22 wide.
const SLOT: f32 = 22.0;
/// Paseo's `kebabButton`: 2 around the mark.
const PAD: f32 = 2.0;
/// The kebab's mark (Paseo's `MoreHorizontal` at 14).
const MARK: f32 = 14.0;
/// Paseo's `DropdownMenuContent width={200}`.
const WIDTH: f32 = 200.0;

/// The pane's card menu and the card it is open for.
pub(crate) struct CardMenu {
    menu: MenuState,
    target: Option<Target>,
    /// Each kebab's bounds as last drawn, so the menu opens under it.
    places: RefCell<HashMap<String, Place>>,
}

/// What a kebab's menu is for.
#[derive(Debug, Clone)]
pub(super) struct Target {
    /// The kebab's key (its item, or its thread).
    key: String,
    acts: Vec<CardAct>,
    /// The item Add to chat adds.
    item: String,
    /// The words Copy copies.
    words: String,
    /// The page Open opens.
    url: Option<String>,
}

impl CardMenu {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            menu: MenuState::new(cx),
            target: None,
            places: RefCell::new(HashMap::new()),
        }
    }

    /// Whether the menu is open for `key` (its kebab stays shown).
    fn is_for(&self, key: &str) -> bool {
        self.menu.is_open() && self.target.as_ref().is_some_and(|target| target.key == key)
    }

    fn place(&self, key: &str) -> Place {
        Rc::clone(
            self.places
                .borrow_mut()
                .entry(key.to_owned())
                .or_insert_with(|| Rc::new(std::cell::Cell::new(None))),
        )
    }
}

/// What Copy copies (the markdown source, `card_menu::copied`) and the
/// item's page.
fn parts(item: &TimelineItem) -> (String, Option<String>) {
    let url = match item {
        TimelineItem::Review { url, .. } | TimelineItem::Comment { url, .. } => url,
    };
    (
        copied(item).unwrap_or_default().to_owned(),
        url.as_ref().map(|url| url.as_str().to_owned()),
    )
}

impl Cards<'_> {
    /// A comment's or a review's kebab; `group`: the hover group that
    /// shows it. None when its menu would have no row.
    pub(super) fn item_kebab(
        &self,
        item: &TimelineItem,
        group: &SharedString,
        cx: &mut Context<'_, Shell>,
    ) -> Option<AnyElement> {
        let acts = item_acts(item, self.attach);
        let (words, url) = parts(item);
        let key = item.id().as_str().to_owned();
        let target = Target {
            key,
            acts,
            item: item.id().as_str().to_owned(),
            words,
            url,
        };
        self.kebab(
            target,
            group,
            self.shell.words.pulls().comment_actions(),
            cx,
        )
    }

    /// A thread header's kebab: Open, on its first comment's page.
    pub(super) fn thread_kebab(
        &self,
        thread: &Thread<'_>,
        group: &SharedString,
        cx: &mut Context<'_, Shell>,
    ) -> Option<AnyElement> {
        let first = thread.comments.first().copied();
        let url = first.and_then(|item| parts(item).1);
        let target = Target {
            key: thread.key(),
            acts: thread_acts(first),
            item: String::new(),
            words: String::new(),
            url,
        };
        self.kebab(target, group, self.shell.words.pulls().thread_actions(), cx)
    }

    /// Paseo's kebab slot: the dots, a button named `label` that opens the
    /// menu, hidden until `group` is hovered unless its menu is open or the
    /// window is a phone's.
    fn kebab(
        &self,
        target: Target,
        group: &SharedString,
        label: &str,
        cx: &mut Context<'_, Shell>,
    ) -> Option<AnyElement> {
        if target.acts.is_empty() {
            return None;
        }
        let card_menu = &self.shell.areas.pulls.card_menu;
        let shown = self.touch || card_menu.is_for(&target.key);
        let place = card_menu.place(&target.key);
        let neutral = self.tokens.neutral;
        let mark_group = SharedString::from(format!("pr-pane-kebab-{}", target.key));
        let opened = target.clone();
        let activated = target;
        let button = div()
            .id(mark_group.clone())
            .group(mark_group.clone())
            .p(px(PAD))
            .rounded(px(spacing::S1))
            .cursor_pointer()
            .hover(move |style| style.bg(neutral.surface2))
            .child(
                glyph(Glyph::More, MARK, neutral.foreground_muted)
                    .group_hover(mark_group, move |style| style.text_color(neutral.foreground)),
            )
            .role(gpui::Role::Button)
            .aria_label(SharedString::from(label.to_owned()))
            // Its own press: the card's (fold, or open) does not run.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    shell.pulls_card_menu_open(opened.clone(), event.position, window, cx);
                }),
            )
            // A click (assistive technology's, or a pointer's release)
            // opens it once.
            .on_click(cx.listener(move |shell, event: &ClickEvent, window, cx| {
                cx.stop_propagation();
                if !shell.areas.pulls.card_menu.is_for(&activated.key) {
                    shell.pulls_card_menu_open(activated.clone(), event.position(), window, cx);
                }
            }))
            .relative()
            .child(anchor_probe(&place));
        let group = group.clone();
        Some(
            div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .w(px(SLOT))
                .when(!shown, move |slot| {
                    slot.opacity(0.0)
                        .group_hover(group, |style| style.opacity(1.0))
                })
                .child(button)
                .into_any_element(),
        )
    }
}

impl Shell {
    /// Open the card menu for `target`, under its kebab once drawn.
    fn pulls_card_menu_open(
        &mut self,
        target: Target,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let card_menu = &mut self.areas.pulls.card_menu;
        let kebab = card_menu.place(&target.key).get();
        card_menu.target = Some(target);
        match kebab {
            Some(bounds) => card_menu.menu.open_end(bounds, window, cx),
            None => card_menu.menu.open(at, window, cx),
        }
        cx.notify();
    }

    fn pulls_card_menu_chose(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(target) = self.areas.pulls.card_menu.target.take() else {
            return;
        };
        match target.acts.get(index) {
            Some(CardAct::AddToChat) => self.pulls_add_item(&target.item, window, cx),
            Some(CardAct::Copy) => cx.write_to_clipboard(ClipboardItem::new_string(target.words)),
            Some(CardAct::Open) => {
                if let Some(url) = &target.url {
                    cx.open_url(url);
                }
            }
            None => {}
        }
        cx.notify();
    }
}

/// The menu, while it is open.
pub(super) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let card_menu = &shell.areas.pulls.card_menu;
    let target = card_menu.target.as_ref()?;
    let words = shell.words.pulls();
    let forge = shell
        .pulls_pane_pull()
        .map_or_else(|| "GitHub".to_owned(), |pull| brand(&pull.summary.repo));
    let items: Vec<MenuItem> = target
        .acts
        .iter()
        .map(|act| match act {
            CardAct::AddToChat => {
                MenuItem::new(words.add_to_chat().to_owned()).with_leading(Glyph::MessagePlus)
            }
            CardAct::Copy => MenuItem::new(words.copy().to_owned()).with_leading(Glyph::Copy),
            CardAct::Open => MenuItem::new(words.open_on(&forge).unwrap_or_default())
                .with_leading(Glyph::ExternalLink),
        })
        .collect();
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.pulls.card_menu.menu,
        chose: Rc::new(|shell: &mut Shell, index, window, cx| {
            shell.pulls_card_menu_chose(index, window, cx);
        }),
    };
    let content = Content::new("pr-pane-card-menu", &items).with_width(px(WIDTH));
    menu_with(
        content,
        &card_menu.menu,
        &handlers,
        tokens,
        breakpoint::phone(window),
        cx,
    )
}
