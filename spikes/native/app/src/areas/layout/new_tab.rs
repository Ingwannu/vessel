//! The New tab panel: Paseo's `NewTabPanel` (pinned `panels/new-tab-panel.tsx`,
//! TRACKER 336), the body of a New tab tab and of a pane with no tab.
//!
//! [Decision Log]
//! - Purpose and intent: an empty pane offers what the pane header's "+"
//!   offers, in Paseo's form: a centred rail of grouped launcher rows.
//! - Existing implementation and constraints: the pane drew "Choose what to
//!   open in this pane." and a New agent button. Only Agent can start today
//!   (`layout::launch`); the app knows no plugin panels and no node terminal
//!   profiles; Settings › Terminals is not drawn.
//! - Alternatives considered: more buttons under the line (not Paseo); the
//!   "+" menu's rows in the pane (a menu's look, not the panel's).
//! - Chosen approach: the view's catalog and `layout::launch` for the
//!   built-ins, as the "+" menu; the shipped profiles (Paseo's list when it
//!   has no configuration); one focus handle for every panel, owned by the
//!   panel last pressed or opened, with a highlighted row.
//! - Why: one launch table keeps the menu and the panel the same; one handle
//!   is enough because one panel holds the keys at a time.
//! - Pros, cons and impact: most rows and every profile row are disabled
//!   until their panels land, drawn at half opacity with no hover and
//!   skipped by the arrows (the coordinator, 2026-11-18); Edit profiles is
//!   disabled the same way; no chords yet. Plan:
//!   `docs/plans/wp-new-tab-panel.md`.
use super::launch;
use crate::shell::Shell;
use vessel_app_kit::agent_mark::{Size, Space};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::spacing::{S1, S2, S3, S6, S12};
use vessel_app_kit::theme::{icon, radius};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, KeyDownEvent, MouseButton, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, px, relative,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, glyph};
use vessel_app_view::agent_mark::AgentMark;
use vessel_app_view::layout::{Availability, Launch, LaunchPurpose, LayoutKey, PaneId, TabId, catalog};
use vessel_proto::terminal_profiles::Profiles;

/// Paseo's `LAUNCHER_MAX_WIDTH`.
const RAIL_MAX: f32 = 380.0;
/// Paseo's row `minHeight`.
const ROW_MIN: f32 = 44.0;
/// Paseo's `EDIT_PROFILES_HIT_SIZE`: the xs icon and 8.
const EDIT_HIT: f32 = 20.0;
/// Paseo's `ICON_SIZE.xs`, the pencil.
const EDIT_GLYPH: f32 = 12.0;
/// Paseo's `rowPressed`.
const PRESSED: f32 = 0.85;
/// The kit's disabled controls (the menu's disabled rows).
const DISABLED: f32 = 0.5;
/// The pencil at rest is Paseo's 50%; disabled, half of that.
const EDIT_DISABLED: f32 = 0.25;
/// Paseo's `-(EDIT_PROFILES_HIT_SIZE - ICON_SIZE.xs) / 2`: the box grows
/// round the glyph without growing the header.
const EDIT_MARGIN: f32 = -4.0;

/// Which panel holds the keys: a New tab tab, or a pane with no tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Owner {
    /// A New tab tab, wherever it moves.
    Tab(LayoutKey, TabId),
    /// A pane drawn with no tab.
    Pane(LayoutKey, PaneId),
}

/// The panels' one focus handle, the panel that owns it and its
/// highlighted row (an index over every row, always a ready one).
pub(super) struct Keys {
    focus: gpui::FocusHandle,
    owner: Option<Owner>,
    row: usize,
}
impl Keys {
    pub(super) fn new(cx: &mut vessel_app_kit::ui::App) -> Self {
        Self {
            focus: cx.focus_handle(),
            owner: None,
            row: 0,
        }
    }
}

enum Icon {
    Glyph(Glyph),
    Mark(AgentMark),
}

struct Row {
    /// Its place over every row of every group.
    index: usize,
    id: SharedString,
    label: SharedString,
    icon: Icon,
    /// What it starts; `None`: listed, disabled.
    launch: Option<Launch>,
}

struct Group {
    label: Option<SharedString>,
    /// Paseo's group accessory (Edit profiles), drawn disabled.
    edit: bool,
    rows: Vec<Row>,
}

/// Paseo's groups in order: the built-ins (`PRIMARY_LAUNCH_ORDER`, no
/// label), plugin panels (none in the native app yet, so never drawn), the
/// terminal profiles under their label.
fn groups(shell: &Shell, window: &Window) -> Vec<Group> {
    let mut index = 0_usize;
    let mut next = || {
        let this = index;
        index = index.saturating_add(1);
        this
    };
    let tabs = catalog(LaunchPurpose::Primary, super::mode(window), launch::availability)
        .into_iter()
        .map(|(chosen, availability)| Row {
            index: next(),
            id: format!("{chosen:?}").to_lowercase().into(),
            label: launch::label(shell, chosen).to_owned().into(),
            icon: Icon::Glyph(launch::glyph(chosen)),
            launch: (availability == Availability::Ready).then_some(chosen),
        })
        .collect();
    let mut groups = vec![Group {
        label: None,
        edit: false,
        rows: tabs,
    }];
    // The node's profiles do not reach the app yet; Paseo shows the
    // shipped ones when it has no configuration. They start as Terminal
    // does, so they are disabled while Terminal is.
    let profiles: Vec<Row> = Profiles::shipped()
        .map(|shipped| {
            shipped
                .entries()
                .iter()
                .map(|profile| Row {
                    index: next(),
                    id: format!("profile-{}", profile.id().as_str()).into(),
                    label: profile.name().as_str().to_owned().into(),
                    icon: AgentMark::of_profile(profile)
                        .map_or(Icon::Glyph(Glyph::SquareTerminal), Icon::Mark),
                    launch: None,
                })
                .collect()
        })
        .unwrap_or_default();
    if !profiles.is_empty() {
        groups.push(Group {
            label: Some(shell.words.layout().terminal_profiles().to_owned().into()),
            edit: true,
            rows: profiles,
        });
    }
    groups
}

/// The ready rows' places, in order.
fn ready(groups: &[Group]) -> Vec<(usize, Launch)> {
    groups
        .iter()
        .flat_map(|group| &group.rows)
        .filter_map(|row| row.launch.map(|chosen| (row.index, chosen)))
        .collect()
}

/// The panel for `owner`.
pub(super) fn panel(
    shell: &Shell,
    owner: Owner,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let groups = groups(shell, window);
    let keys = &shell.areas.layout.new_tab;
    let owns = keys.owner.as_ref() == Some(&owner);
    let lit = (owns && keys.focus.is_focused(window)).then_some(keys.row);
    let rail = groups.into_iter().fold(
        div()
            .debug_selector(|| "new-tab-rail".to_owned())
            .w_full()
            .max_w(px(RAIL_MAX))
            .flex()
            .flex_col()
            .gap(px(S12)),
        |rail, group| rail.child(group_view(group, &owner, lit, tokens, cx)),
    );
    let id = match &owner {
        Owner::Tab(_, tab) => format!("new-tab-panel-tab-{}", tab.get()),
        Owner::Pane(_, pane) => format!("new-tab-panel-pane-{}", pane.get()),
    };
    let pressed = owner.clone();
    div()
        .id(ElementId::Name(id.into()))
        .debug_selector(|| "new-tab-panel".to_owned())
        .size_full()
        .overflow_y_scroll()
        .when(owns, |panel| panel.track_focus(&keys.focus))
        // Paseo: a press outside a row focuses the first row.
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |shell, _event, window, cx| {
                shell.new_tab_take_keys(pressed.clone(), None, window, cx);
            }),
        )
        .on_key_down(cx.listener(|shell, event: &KeyDownEvent, window, cx| {
            if !event.keystroke.modifiers.modified() && shell.new_tab_key(&event.keystroke.key, window, cx)
            {
                cx.stop_propagation();
            }
        }))
        .child(
            div()
                .min_h(relative(1.0))
                .w_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .p(px(S6))
                .child(rail),
        )
        .into_any_element()
}

fn group_view(
    group: Group,
    owner: &Owner,
    lit: Option<usize>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let neutral = tokens.neutral;
    let header = group.label.map(|label| {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(2.0))
            .mb(px(S1))
            .child(
                div()
                    .debug_selector(|| "new-tab-group-label".to_owned())
                    .type_(Type::Meta)
                    .line_height(px(EDIT_HIT))
                    .text_color(neutral.foreground_extra_muted)
                    .child(label),
            )
            .when(group.edit, |header| {
                // Its box keeps the row 20 tall (Paseo's negative margin).
                header.child(
                    div()
                        .debug_selector(|| "new-tab-edit-profiles".to_owned())
                        .size(px(EDIT_HIT))
                        .m(px(EDIT_MARGIN))
                        .flex()
                        .items_center()
                        .justify_center()
                        .opacity(EDIT_DISABLED)
                        .child(glyph(Glyph::Pencil, EDIT_GLYPH, neutral.foreground_extra_muted)),
                )
            })
    });
    group.rows.into_iter().fold(
        div().flex().flex_col().gap(px(S1)).children(header),
        |column, row| column.child(row_view(row, owner, lit, tokens, cx)),
    )
}

fn row_view(
    row: Row,
    owner: &Owner,
    lit: Option<usize>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let highlighted = lit == Some(row.index);
    let mark = match row.icon {
        Icon::Glyph(shape) => glyph(shape, icon::MD, neutral.foreground_muted),
        Icon::Mark(mark) => {
            crate::agent_mark::mark_muted_of(mark, Size::Sixteen, Space::Compact, tokens)
        }
    };
    let id = row.id.clone();
    let body = div()
        .debug_selector(move || format!("new-tab-row-{id}"))
        .flex()
        .items_center()
        .gap(px(S2))
        .min_h(px(ROW_MIN))
        .px(px(S3))
        .rounded(px(radius::R2))
        .border_1()
        .border_color(if highlighted {
            neutral.border_accent
        } else {
            gpui::transparent_black()
        })
        .bg(if highlighted {
            neutral.surface2
        } else {
            neutral.surface1
        })
        .child(div().flex_none().child(mark))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .truncate()
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .child(row.label),
        );
    let Some(chosen) = row.launch else {
        return body.opacity(DISABLED).into_any_element();
    };
    let surface2 = neutral.surface2;
    let pressed = owner.clone();
    let index = row.index;
    body.id(ElementId::Name(format!("new-tab-launch-{}", row.id).into()))
        .cursor_pointer()
        .hover(move |style| style.bg(surface2))
        .active(|style| style.opacity(PRESSED))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |shell, _event, window, cx| {
                shell.new_tab_take_keys(pressed.clone(), Some(index), window, cx);
                cx.stop_propagation();
            }),
        )
        .on_click(cx.listener(move |shell, _event, window, cx| {
            launch::run(shell, chosen, window, cx);
        }))
        .into_any_element()
}

impl Shell {
    /// `owner` takes the keys, its highlight on `row` (else its first ready
    /// row): a press in it, or the New tab shortcut opening it.
    pub(super) fn new_tab_take_keys(
        &mut self,
        owner: Owner,
        row: Option<usize>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let first = ready(&groups(self, window)).first().map_or(0, |(index, _)| *index);
        let keys = &mut self.areas.layout.new_tab;
        keys.owner = Some(owner);
        keys.row = row.unwrap_or(first);
        keys.focus.focus(window, cx);
        cx.notify();
    }

    /// Paseo's keys: Up and Down move over the ready rows and wrap, Enter
    /// starts the highlighted one. Whether the key was the panel's.
    fn new_tab_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<'_, Self>) -> bool {
        let rows = ready(&groups(self, window));
        let at = rows.iter().position(|(index, _)| *index == self.areas.layout.new_tab.row);
        match key {
            "up" | "down" => {
                let Some(last) = rows.len().checked_sub(1) else {
                    return true;
                };
                let next = match (key, at) {
                    ("down", Some(at)) if at < last => at.saturating_add(1),
                    ("down", _) => 0,
                    (_, Some(at)) if at > 0 => at.saturating_sub(1),
                    _ => last,
                };
                if let Some((index, _)) = rows.get(next) {
                    self.areas.layout.new_tab.row = *index;
                }
                cx.notify();
                true
            }
            "enter" => {
                if let Some((_, chosen)) = at.and_then(|at| rows.get(at)) {
                    launch::run(self, *chosen, window, cx);
                }
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests;
