//! The sidebar: the routes (`routes::Route::NAV`), a slot for the
//! projects ([`projects`], the workspace track's), the machines in the
//! book with their live sessions (DESIGN.md section 6: rows on the shell,
//! no edge of its own), adding a machine, and the build's label.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use vessel_app_kit::ui::{
    Context, Div, ElementId, InteractiveElement as _, IntoElement, ParentElement as _, Stateful,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Tooltips, Type, TypeExt as _, dot, kicker};
use vessel_app_view::settings::Section;
use vessel_app_view::{Card, Machine, Reach};

use super::common::card_color;
use super::session::Open;
use crate::routes::Route;
use crate::shell::Shell;

/// The sidebar's own state: its tooltips, and where each unreachable
/// machine's line is drawn (by machine name).
pub(crate) struct SidebarState {
    tips: Tooltips,
    places: RefCell<HashMap<String, vessel_app_kit::tooltip::Place>>,
}

impl SidebarState {
    pub(crate) fn new() -> Self {
        Self {
            tips: Tooltips::new(),
            places: RefCell::new(HashMap::new()),
        }
    }

    /// `machine`'s place for its tooltip, kept across frames.
    fn place(&self, machine: &str) -> vessel_app_kit::tooltip::Place {
        Rc::clone(
            self.places
                .borrow_mut()
                .entry(machine.to_owned())
                .or_insert_with(|| Rc::new(Cell::new(None))),
        )
    }
}

/// The sidebar; `touch`: drawn in the phone's sheet (no tooltips there,
/// as Paseo's).
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let t = shell.t;
    let mut nav = div().flex().flex_col().gap(px(2.0));
    for route in Route::NAV {
        nav = nav.child(
            row(
                ElementId::Name(format!("nav-{route:?}").into()),
                route.label(t),
                shell.route == route,
                tokens,
            )
            .on_click(cx.listener(move |this, _event, window, cx| this.go(route, window, cx))),
        );
    }
    let mut machines = div().flex().flex_col().gap(px(14.0));
    for machine in &shell.fleet.machines {
        machines = machines.child(machine_rows(machine, shell, tokens, touch, cx));
    }
    if shell.loaded && shell.fleet.machines.is_empty() {
        machines = machines.child(
            div()
                .px(px(10.0))
                .type_(Type::Meta)
                .text_color(tokens.text_tertiary)
                .child(t.no_machines),
        );
    }
    div()
        .w(px(264.0))
        .h_full()
        .flex()
        .flex_col()
        .bg(tokens.bg_shell)
        .px(px(8.0))
        .pb(px(8.0))
        .gap(px(12.0))
        .child(nav.children(schedules(shell, tokens, cx)))
        .children(projects(shell, tokens, cx))
        .child(
            div()
                .id("machines")
                .flex_1()
                .overflow_y_scroll()
                .child(machines),
        )
        .child(
            row(
                "add-machine",
                t.add_machine,
                shell.route == Route::Pair,
                tokens,
            )
            .on_click(cx.listener(|this, _event, window, cx| this.go(Route::Pair, window, cx))),
        )
        .children(
            // The title bar's update status, when the system draws the
            // title bar and the app's is not there.
            vessel_app_kit::chrome::system_titlebar()
                .then(|| crate::chrome::update_status(shell, false, tokens, cx))
                .flatten()
                .map(|status| div().px(px(10.0)).child(status)),
        )
        .child(foot(shell, tokens, cx))
        .children(shell.areas.sidebar.tips.render(tokens))
}

/// The slot for Schedules among the navigation rows, where Paseo's
/// sidebar has it (after its pages, before the projects): the schedules
/// track's (G2) entry; `None`: nothing. As [`projects`], one owner each
/// side of the slot.
const fn schedules(
    _shell: &Shell,
    _tokens: &Tokens,
    _cx: &mut Context<'_, Shell>,
) -> Option<vessel_app_kit::ui::AnyElement> {
    None
}

/// The slot between the navigation rows and the machines: the projects
/// section (track F's, Paseo's sidebar lists projects there); `None`:
/// nothing.
///
/// [Decision Log]
/// - Purpose: the projects are the workspace track's; the sidebar's frame
///   is this one's (and track S's when it draws the full sidebar). One
///   slot with one owner each side keeps both from editing the other's
///   drawing, as `conversation::dock` does over the composer.
/// - Choice: the sidebar asks for the slot's content and puts it between
///   the navigation and the machines, at the sidebar's width; F fills
///   this function (or replaces its body with a call into its module).
fn projects(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<vessel_app_kit::ui::AnyElement> {
    crate::areas::workspace::sidebar_projects(shell, tokens, cx)
}

/// The sidebar's foot, as Paseo's (`components/left-sidebar.tsx`): a
/// hairline over one line, its items at the left and Settings, a gear, at
/// the right end. Here the build and "새로고침" at the left; the gear opens
/// Settings, the build Settings › About.
fn foot(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let t = shell.t;
    div()
        .mx(px(-8.0))
        .mb(px(-8.0))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(8.0))
        .py(px(6.0))
        .border_t_1()
        .border_color(tokens.neutral.border)
        .type_(Type::Tiny)
        .text_color(tokens.text_tertiary)
        .child(
            // The build, and its updates: Settings › About.
            div()
                .id("build")
                .pl(px(2.0))
                .cursor_pointer()
                .hover(|style| style.text_color(tokens.text_secondary))
                .child(t.build(crate::LABEL))
                .on_click(cx.listener(|this, _event, window, cx| {
                    this.open_settings(Some(Section::About), window, cx);
                })),
        )
        .child("·")
        .child(
            div()
                .id("reload")
                .cursor_pointer()
                .hover(|style| style.text_color(tokens.text_secondary))
                .child(t.reload)
                .on_click(cx.listener(|this, _event, _window, _cx| this.reload())),
        )
        .child(div().flex_1())
        .child(
            vessel_app_kit::pill_parts(
                "settings",
                vessel_app_kit::PillParts {
                    leading: Some(vessel_app_kit::assets::Glyph::Gear),
                    ..vessel_app_kit::PillParts::default()
                },
                shell.route == Route::Settings,
                false,
                tokens,
            )
            .on_click(cx.listener(|this, _event, window, cx| {
                this.open_settings(None, window, cx);
            })),
        )
}

/// A machine's name, its state, and its live sessions.
///
/// [Decision Log]
/// - Purpose: a stopped node showed the transport's English text and the
///   endpoint under the machine's name on a Korean screen (track G2's
///   find, 2026-11-13).
/// - Choice: the reason is typed (`Reach::Unreachable`), so the row says
///   it in the app's words in both languages (the state beside the name,
///   one line of what to do under it); the transport's text is only the
///   tooltip on that line, for whoever needs the detail.
/// - Trade-off: on touch (no tooltips) the transport's text is not on
///   screen at all; the line says what to check, which is what a phone's
///   owner can act on.
fn machine_rows(
    machine: &Machine,
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let t = shell.t;
    let (state, color) = match &machine.reach {
        Reach::Online => (t.online, tokens.st_ready),
        Reach::Unreachable(_) => (t.unreachable, tokens.danger),
    };
    let mut rows = div().flex().flex_col().gap(px(2.0)).child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .px(px(10.0))
            .pb(px(2.0))
            .child(kicker(machine.name.clone(), tokens))
            .child(dot(color))
            .child(
                div()
                    .type_(Type::Tiny)
                    .text_color(tokens.text_tertiary)
                    .child(state),
            ),
    );
    if let Reach::Unreachable(why) = &machine.reach {
        let state = &shell.areas.sidebar;
        let line = div()
            .id(ElementId::Name(
                format!("unreachable-{}", machine.name).into(),
            ))
            .px(px(10.0))
            .type_(Type::Tiny)
            .text_color(tokens.text_tertiary)
            .child(t.unreachable_hint);
        rows = rows.child(
            state
                .tips
                .on(line, &state.place(&machine.name), why.clone(), touch),
        );
    }
    for card in machine.sessions.iter().filter(|card| card.lane.is_some()) {
        rows = rows.child(session_row(card, shell, tokens, cx));
    }
    rows
}

fn session_row(
    card: &Card,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> impl IntoElement {
    let active = shell.route == Route::Session
        && shell
            .areas
            .session
            .open
            .as_ref()
            .is_some_and(|open| open.session == card.session);
    let open = Open::of(card);
    div()
        .id(ElementId::Name(format!("session-{}", card.session).into()))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(10.0))
        .py(px(5.0))
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .when(active, |row| row.bg(tokens.bg_active))
        .hover(|style| style.bg(tokens.bg_hover))
        .cursor_pointer()
        .child(dot(card_color(card, tokens)))
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .type_(Type::Label)
                .text_color(if active {
                    tokens.text_primary
                } else {
                    tokens.text_secondary
                })
                .child(shell.t.title(&card.title).to_owned()),
        )
        .on_click(cx.listener(move |this, _event, window, cx| {
            this.open_session(open.clone(), window, cx);
        }))
}

fn row(
    id: impl Into<ElementId>,
    label: &'static str,
    active: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    div()
        .id(id)
        .px(px(10.0))
        .py(px(6.0))
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .type_(Type::Label)
        .text_color(if active {
            tokens.text_primary
        } else {
            tokens.text_secondary
        })
        .when(active, |row| row.bg(tokens.bg_active))
        .hover(|style| style.bg(tokens.bg_hover))
        .cursor_pointer()
        .child(label)
}

use vessel_app_kit::ui::FluentBuilder as _;
