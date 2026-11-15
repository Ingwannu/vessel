//! Settings Sidebar body only; the settings frame owns title/back/scroll.
//! Source: screens/settings/sidebar/sidebar-nav-section.tsx (whole).
//!
//! [Decision Log]
//! Existing kit settings section/row/info-tip, guarded G2 buttons and switch
//! already own every control here. This body only binds the single typed
//! navigation inventory to the existing PreferenceChange path; duplicating a
//! store, a page frame or an area button would split the displayed and saved
//! order. Unavailable plugin choices remain in the lower model until the
//! plugin owner supplies its checked available groups. Retained tooltip places
//! and animation turns are UI state only; settings navigation clears the tips.
use crate::shell::Shell;
use std::{cell::RefCell, rc::Rc};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonConfig, ButtonContent, ButtonIcon, ButtonKind, ButtonState, Tokens, Tooltips, Type,
    TypeExt as _, assets::Glyph, button_control, control_sizing::ControlSizing, settings,
};
use vessel_app_view::prefs::PreferenceChange;
use vessel_app_view::sidebar::navigation::{Direction, Key, Section, Visibility};

pub(super) struct State {
    tips: Tooltips,
    header: vessel_app_kit::tooltip::Place,
    footer: vessel_app_kit::tooltip::Place,
    turns: RefCell<Vec<(Section, Key, u64)>>,
}
impl State {
    pub(super) fn new() -> Self {
        Self {
            tips: Tooltips::new(),
            header: Rc::default(),
            footer: Rc::default(),
            turns: RefCell::new(Vec::new()),
        }
    }
    fn place(&self, section: Section) -> &vessel_app_kit::tooltip::Place {
        match section {
            Section::Header => &self.header,
            Section::Footer => &self.footer,
        }
    }
    fn turn(&self, section: Section, key: &Key) -> u64 {
        self.turns
            .borrow()
            .iter()
            .find(|(band, known, _)| *band == section && known == key)
            .map_or(0, |(_, _, turn)| *turn)
    }
    fn changed(&self, section: Section, key: &Key) {
        let mut turns = self.turns.borrow_mut();
        if let Some((_, _, turn)) = turns
            .iter_mut()
            .find(|(band, known, _)| *band == section && known == key)
        {
            *turn = turn.saturating_add(1);
        } else {
            turns.push((section, key.clone(), 1));
        }
    }
}
/// Registered by the existing Settings sections table; no extra frame.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    _frame_compact: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let compact = vessel_app_view::panels::sidebar_form(f32::from(window.viewport_size().width))
        == vessel_app_view::panels::SidebarForm::Sheet;
    let state = &shell.areas.sidebar.settings;
    let all = super::navigation::inventory(shell);
    state.turns.borrow_mut().retain(|(band, key, _)| {
        all.iter()
            .any(|entry| entry.section == *band && entry.item.key == *key)
    });
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S6))
        .children(
            [Section::Header, Section::Footer]
                .into_iter()
                .map(|section| card(section, &all, shell, tokens, compact, cx)),
        )
        .children(state.tips.render(tokens))
        .into_any_element()
}
fn card(
    section: Section,
    all: &[super::navigation::NavItem],
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.sidebar();
    let state = &shell.areas.sidebar.settings;
    let entries = all
        .iter()
        .filter(|entry| entry.section == section)
        .collect::<Vec<_>>();
    let rows = entries
        .iter()
        .enumerate()
        .fold(settings::card(tokens), |card, (at, entry)| {
            card.child(row(
                section,
                entry,
                Position {
                    at,
                    count: entries.len(),
                },
                shell,
                tokens,
                compact,
                cx,
            ))
        });
    let info = settings::info_tip(
        format!("sidebar-{section:?}-info"),
        words.band_description(section).into(),
        &state.tips,
        state.place(section),
        if compact {
            vessel_app_kit::screen_header::Density::Compact
        } else {
            vessel_app_kit::screen_header::Density::Desktop
        },
        tokens,
    );
    settings::section_with_info(
        words.band_title(section),
        rows,
        Some(info.into_any_element()),
        tokens,
    )
}
#[derive(Clone, Copy)]
struct Position {
    at: usize,
    count: usize,
}
fn row(
    section: Section,
    entry: &super::navigation::NavItem,
    position: Position,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let main = div()
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .child(vessel_app_kit::glyph(
            entry.glyph,
            16.0,
            tokens.neutral.foreground_muted,
        ))
        .child(
            div()
                .min_w_0()
                .truncate()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .child(entry.label.clone()),
        )
        .children(super::navigation::hint(
            entry.item.key.builtin(),
            shell,
            compact,
            tokens,
        ));
    let mut actions = div().flex_none().flex().items_center().gap(px(spacing::S2));
    for (direction, glyph, enabled) in [
        (Direction::Up, Glyph::ArrowUp, position.at > 0),
        (
            Direction::Down,
            Glyph::ArrowDown,
            position
                .at
                .checked_add(1)
                .is_some_and(|next| next < position.count),
        ),
    ] {
        let key = entry.item.key.clone();
        let button = button_control(
            format!("sidebar-{section:?}-{direction:?}-{key}"),
            ButtonContent::Icon(ButtonIcon::muted(glyph)),
            ButtonConfig::new(
                ButtonKind::Ghost,
                if enabled {
                    ButtonState::Enabled
                } else {
                    ButtonState::Disabled
                },
                ControlSizing::from_compact(compact),
            ),
            tokens,
        )
        .on_click(cx.listener(move |shell, _, window, cx| {
            shell.sidebar_move_item(section, &key, direction, window, cx)
        }));
        actions = actions.child(button);
    }
    let on = entry.item.visible == Visibility::Shown;
    let key = entry.item.key.clone();
    let switch = settings::switch(
        format!("sidebar-{section:?}-visible-{key}"),
        on,
        true,
        compact,
        shell.areas.sidebar.settings.turn(section, &key),
        tokens,
    )
    .on_click(cx.listener(move |shell, _, window, cx| {
        shell.sidebar_set_item_visible(
            section,
            &key,
            if on {
                Visibility::Hidden
            } else {
                Visibility::Shown
            },
            window,
            cx,
        )
    }));
    actions = actions.child(switch);
    settings::row_with_content_profile(
        main.into_any_element(),
        Some(actions.into_any_element()),
        position.at == 0,
        compact,
        settings::RowProfile::SidebarNavigation,
        tokens,
    )
}
impl Shell {
    pub(crate) fn sidebar_settings_clear_info(&mut self) {
        self.areas.sidebar.settings.tips.clear();
    }
    fn sidebar_move_item(
        &mut self,
        section: Section,
        key: &Key,
        direction: Direction,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let available = super::navigation::items(self, section);
        match self
            .prefs
            .settings
            .sidebar_navigation
            .entries(section)
            .moved(&available, key, direction)
        {
            Ok(entries) => self.prefer(
                &PreferenceChange::SidebarNavigation { section, entries },
                window,
                cx,
            ),
            Err(_) => {
                self.areas.settings.refused =
                    Some(self.words.sidebar().navigation_limit().to_owned());
                cx.notify();
            }
        }
    }
    fn sidebar_set_item_visible(
        &mut self,
        section: Section,
        key: &Key,
        visible: Visibility,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let available = super::navigation::items(self, section);
        match self
            .prefs
            .settings
            .sidebar_navigation
            .entries(section)
            .visibility(&available, key, visible)
        {
            Ok(entries) => {
                self.areas.sidebar.settings.changed(section, key);
                self.prefer(
                    &PreferenceChange::SidebarNavigation { section, entries },
                    window,
                    cx,
                );
            }
            Err(_) => {
                self.areas.settings.refused =
                    Some(self.words.sidebar().navigation_limit().to_owned());
                cx.notify();
            }
        }
    }
}

#[cfg(test)]
mod tests;
