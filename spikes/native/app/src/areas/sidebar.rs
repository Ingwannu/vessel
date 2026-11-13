//! The sidebar: the routes (`routes::Route::NAV`), the machines in the
//! book with their live sessions (DESIGN.md section 6: rows on the shell,
//! no edge of its own), adding a machine, and the build's label.

use vessel_app_kit::ui::{
    Context, Div, ElementId, InteractiveElement as _, IntoElement, ParentElement as _, Stateful,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, dot, kicker};
use vessel_app_view::settings::Section;
use vessel_app_view::{Card, Machine, Reach};

use super::common::card_color;
use super::session::Open;
use crate::routes::Route;
use crate::shell::Shell;

pub(crate) fn render(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
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
        machines = machines.child(machine_rows(machine, shell, tokens, cx));
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
        .child(nav)
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

fn machine_rows(
    machine: &Machine,
    shell: &Shell,
    tokens: &Tokens,
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
        rows = rows.child(
            div()
                .px(px(10.0))
                .type_(Type::Tiny)
                .text_color(tokens.text_tertiary)
                .child(why.clone()),
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
