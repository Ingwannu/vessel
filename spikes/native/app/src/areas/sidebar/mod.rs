//! The sidebar: the routes (`routes::Route::NAV`), a slot for the
//! projects ([`projects`], the workspace track's), the sessions listed as
//! the Display menu says (by machine or by status, `machines`,
//! `statuses`), adding a machine, and the foot (DESIGN.md section 6: rows
//! on the shell, no edge of its own).
//!
//! [Decision Log]
//! - Purpose: the page's and Paseo's sidebar: the person chooses how
//!   sessions are listed (grouping, order, ended ones) from a Display menu
//!   at the list's heading, and every row says what its session is doing.
//! - Before: the native sidebar listed each machine's sessions on a board
//!   lane, in the book's order, whatever the preferences said; the view's
//!   model of the page (`vessel_app_view::sidebar`) was only used by the
//!   keys that walk the list (Alt+[ and Alt+]).
//! - Choice: draw through that model (`Fleet::sidebar_machines`,
//!   `sidebar_statuses`), so the list on screen and the list the keys walk
//!   are one; one file per part (the frame here, the groups, a session's
//!   row, the Display menu, the foot), the quality audit's split.
//! - Cost: a machine shows its first five sessions and a status group its
//!   first ten, then "N more", which opens the board (the page's limits:
//!   the sidebar is a way to switch, not a second board).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use vessel_app_kit::theme::{radius, spacing};
use vessel_app_kit::ui::{
    Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Stateful, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Tooltips, Type, TypeExt as _, dot, kicker};
use vessel_app_view::prefs::Grouping;

use super::edge::{Panel, PanelEdge};
use crate::routes::Route;
use crate::shell::Shell;

mod actions;
mod display;
mod foot;
mod layout;
mod machines;
mod row_menu;
mod session_row;
mod statuses;

/// A row's inset from the sidebar's padding (the page's 10, between the
/// kit's 8 and 12 steps).
const INSET: f32 = 10.0;
/// The list heading closer to its list than the sidebar's gap: one
/// `spacing::S1` taken back (a negative literal, since the workspace
/// denies arithmetic on sizes).
const HEADING_BACK: f32 = -4.0;

/// The sidebar's own state: its tooltips, and where each unreachable
/// machine's line is drawn (by machine name).
pub(crate) struct SidebarState {
    layout: layout::Layout,
    tips: Tooltips,
    places: RefCell<HashMap<String, vessel_app_kit::tooltip::Place>>,
    /// The tooltip of the same line drawn on a page (Settings ›
    /// Projects), which draws its own.
    page_tips: Tooltips,
    /// The Display menu.
    display: display::DisplayMenu,
    /// A session row's menu.
    rows: row_menu::RowMenu,
    /// Its right edge, dragged to resize it.
    pub(crate) edge: PanelEdge,
}

impl SidebarState {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            layout: layout::Layout::new(cx),
            tips: Tooltips::new(),
            places: RefCell::new(HashMap::new()),
            page_tips: Tooltips::new(),
            display: display::DisplayMenu::new(window, cx),
            rows: row_menu::RowMenu::new(cx),
            edge: PanelEdge::new(),
        }
    }

    /// An unreachable machine's state and what to do, as the sidebar says
    /// them, for a page (Settings › Projects, track F's): the transport's
    /// text `why` only in the line's tooltip. The page's own tooltip is
    /// drawn with it.
    pub(crate) fn unreachable_on_page(
        &self,
        machine: &str,
        why: &str,
        t: &vessel_app_view::Text,
        tokens: &Tokens,
        touch: bool,
    ) -> vessel_app_kit::ui::AnyElement {
        let line = div()
            .id(ElementId::Name(
                format!("page-unreachable-{machine}").into(),
            ))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::S1_5))
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(dot(tokens.danger))
            .child(format!("{} · {}", t.unreachable, t.unreachable_hint));
        let place = self.place(&format!("page:{machine}"));
        div()
            .child(self.page_tips.on(line, &place, why.to_owned(), touch))
            .children(self.page_tips.render(tokens))
            .into_any_element()
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
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let t = shell.t;
    let mut nav = div().flex().flex_col().gap(px(spacing::S0_5));
    for route in Route::NAV {
        nav = nav.child(
            row(
                ElementId::Name(format!("nav-{route:?}").into()),
                route.label(t),
                shell.route == route,
                tokens,
            )
            .on_click(cx.listener(move |this, _event, window, cx| this.go(route, window, cx)))
            .when(route == Route::Draft, |row| {
                row.track_focus(&shell.areas.sidebar.layout.navigation)
            }),
        );
    }
    // Paseo's Search row, after the pages (track S's, `areas::palette`).
    nav = nav.child(crate::areas::palette::sidebar_row(shell, tokens, touch, cx));
    let view = shell.prefs.settings.sidebar;
    let list = if shell.loaded && shell.fleet.machines.is_empty() {
        div().child(
            div()
                .px(px(INSET))
                .type_(Type::Meta)
                .text_color(tokens.text_tertiary)
                .child(t.no_machines),
        )
    } else {
        match view.group {
            Grouping::Machine => machines::groups(view, shell, tokens, touch, cx),
            Grouping::Status => statuses::groups(view, shell, tokens, touch, cx),
        }
    };
    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(tokens.bg_shell)
        .px(px(spacing::S2))
        .pb(px(spacing::S2))
        .gap(px(spacing::S3))
        .child(nav.flex_none().child(schedules(shell, tokens, touch, cx)))
        .child(layout::body(
            &shell.areas.sidebar.layout,
            projects(shell, tokens, cx),
            heading(shell, tokens, cx),
            list,
        ))
        .child(
            row(
                "add-machine",
                t.add_machine,
                shell.route == Route::Pair,
                tokens,
            )
            .flex_none()
            .on_click(cx.listener(|this, _event, window, cx| this.go(Route::Pair, window, cx))),
        )
        .children(
            // The title bar's update status, when the system draws the
            // title bar and the app's is not there.
            vessel_app_kit::chrome::system_titlebar()
                .then(|| crate::chrome::update_status(shell, false, tokens, cx))
                .flatten()
                .map(|status| div().flex_none().px(px(INSET)).child(status)),
        )
        .child(foot::foot(shell, tokens, cx).flex_none())
        .children(shell.areas.sidebar.tips.render(tokens))
        .children(display::menu(shell, tokens, window, cx))
        .children(row_menu::render(shell, tokens, touch, window, cx))
}

/// The list's heading, as Paseo's section header: what the list is
/// ("Machines", or "Sessions" grouped by status, the page's words) and the
/// Display menu's trigger at its right.
fn heading(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.sidebar();
    let title = match shell.prefs.settings.sidebar.group {
        Grouping::Machine => words.machines,
        Grouping::Status => words.sessions,
    };
    div()
        .flex()
        .flex_row()
        .items_center()
        .pl(px(INSET))
        // Closer to its list than the sidebar's gap (`spacing::S1` back).
        .mb(px(HEADING_BACK))
        .child(div().flex_1().child(kicker(title, tokens)))
        .child(display::trigger(shell, tokens, cx))
}

/// The row under a list that opens the rest on the board ("N more").
fn more_row(id: ElementId, count: usize, shell: &Shell, tokens: &Tokens) -> Stateful<Div> {
    div()
        .id(id)
        .px(px(INSET))
        .py(px(spacing::S1))
        .rounded(px(radius::R4))
        .type_(Type::Meta)
        .text_color(tokens.text_tertiary)
        .hover(|style| style.bg(tokens.bg_hover).text_color(tokens.text_secondary))
        .cursor_pointer()
        .child(shell.t.lang.sidebar().more(count))
}

/// The sidebar as a resizable panel: dragged at its right edge
/// (`super::edge`), kept in the layout's `sidebar_width`.
const PANEL: Panel = Panel {
    limits: vessel_app_view::panels::SIDEBAR,
    edge: |shell| &shell.areas.sidebar.edge,
    kept: |shell| shell.prefs.settings.layout.sidebar_width,
    keep: |layout, width| layout.sidebar_width = width,
    side: vessel_app_kit::resize::Edge::Right,
};

/// One sidebar-only layout decision, including the width currently dragged.
/// Settings retains its old compact form until its frame owner consumes the
/// shared budget; docking it at 600 now would duplicate its compact nav list.
pub(crate) fn arrangement(
    shell: &Shell,
    window: &Window,
) -> vessel_app_view::panels::SidebarLayout {
    use vessel_app_view::panels::{SidebarContext, SidebarLayout, sidebar_layout};
    if shell.route == Route::Settings && vessel_app_kit::breakpoint::phone(window) {
        return SidebarLayout::Sheet;
    }
    let context = match shell.route {
        Route::Settings => SidebarContext::Settings,
        Route::Draft
        | Route::Board
        | Route::Archive
        | Route::Session
        | Route::Pair
        | Route::Schedules => SidebarContext::Workspace(shell.code_placement(window)),
    };
    let viewport = f32::from(window.viewport_size().width);
    sidebar_layout(
        viewport,
        crate::areas::workspace::frame_row(viewport),
        PANEL.asked(shell),
        shell.prefs.settings.layout.sidebar,
        context,
    )
}

/// What stands in the sidebar's place on a wide window (the workspace
/// sidebar, or Settings' list: one width for both, so the frame beside
/// it does not move when Settings opens), at the width dragged to,
/// measured against the window's width.
pub(crate) fn frame(
    shell: &Shell,
    inner: vessel_app_kit::ui::AnyElement,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let vessel_app_view::panels::SidebarLayout::Docked { width, .. } = arrangement(shell, window)
    else {
        return div();
    };
    div()
        .relative()
        .flex_shrink_0()
        .h_full()
        .w(px(width))
        .child(inner)
        .children(PANEL.handle("sidebar-edge", shell, tokens, window, cx))
}

/// The slot for Schedules among the navigation rows, where Paseo's
/// sidebar has it (after its pages, before the projects): the schedules
/// track's (G2) installed entry. As [`projects`], one owner each side
/// of the slot; the area still supplies the one shared navigation row.
/// The row to draw there is [`nav_row`] with `Some(Glyph::CalendarClock)`
/// and the sidebar's `touch`.
fn schedules(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> vessel_app_kit::ui::AnyElement {
    super::schedules::sidebar(shell, tokens, touch, cx)
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

fn row(
    id: impl Into<ElementId>,
    label: &'static str,
    active: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    nav_row(id, label, None, active, false, tokens)
}

/// A navigation row of the sidebar, for an entry in its slots (track G2's
/// Schedules, after Paseo's `SidebarHeaderRow` `compact`): `leading`, a
/// glyph before the words in the words' colour, 8 from them (Paseo's
/// `spacing[2]`), at Paseo's `sm` density icon: 14 points, 18 on `touch`
/// (`control-geometry.ts`, `ICON_SIZE` and `TOUCH_ICON_SIZE`). `None`
/// draws the row exactly as the sidebar's own rows were drawn (the flex
/// row is set only with a glyph). The caller adds what a press does.
///
/// Differences from Paseo's row, named, for the sidebar's full drawing
/// (track S's slice) to settle for every row at once: a hovered row here
/// only fills (Paseo also turns the words and the glyph to the
/// foreground); the row's padding, type and radius are the sidebar's own,
/// not Paseo's compact values; no shortcut on hover.
pub(crate) fn nav_row(
    id: impl Into<ElementId>,
    label: &'static str,
    leading: Option<vessel_app_kit::assets::Glyph>,
    active: bool,
    touch: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let color = if active {
        tokens.text_primary
    } else {
        tokens.text_secondary
    };
    let glyph_size = if touch { 18.0 } else { 14.0 };
    div()
        .id(id)
        .when(leading.is_some(), |row| {
            row.flex().flex_row().items_center().gap(px(spacing::S2))
        })
        .px(px(INSET))
        .py(px(spacing::S1_5))
        .rounded(px(radius::R4))
        .type_(Type::Label)
        .text_color(color)
        .when(active, |row| row.bg(tokens.bg_active))
        .hover(|style| style.bg(tokens.bg_hover))
        .cursor_pointer()
        .children(leading.map(|glyph| {
            vessel_app_kit::ui::svg()
                .path(glyph.path())
                .size(px(glyph_size))
                .flex_none()
                .text_color(color)
        }))
        .child(label)
}

#[cfg(test)]
mod policy_tests;
