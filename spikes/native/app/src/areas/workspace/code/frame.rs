//! The panel's frame and its two slots in the lead's and track C's
//! screens: Paseo's Explorer beside the session (a hairline at its left,
//! the tab rail on top with Close at its end), 440 wide; at a phone's
//! width the whole screen, as Paseo's compact layout opens the Explorer
//! over the workspace.
//!
//! Difference from Paseo, named: its Explorer is 320 wide by default and
//! opens a file's diff in the main pane; here the diff is in the panel,
//! so the panel starts wider.
//!
//! [Decision Log]
//! - Purpose: the panel is resized at its left edge (the owner, 2026-11-14,
//!   word 29: the same control as the sidebar's, `areas::edge`), and the
//!   conversation beside it keeps a readable width (480) however wide the
//!   sidebar is.
//! - Constraint: the panel's own bounds do not say where the conversation
//!   starts (the sidebar's width is the lead's), and a window made
//!   narrower must not push the panel's Close out of sight.
//! - Alternatives: the window's width less 480 (the first version: it
//!   left out the sidebar, and a drag gave the conversation about 350);
//!   the sidebar's width read from the lead's layout (couples to its
//!   paddings and borders).
//! - Choice: the row of conversation and panel is drawn here
//!   (`code_beside`, the lead's slot) and measured each frame; the panel's
//!   width is clamped against it (`panels::CODE_PANEL`). In the row the
//!   conversation grows from 480 and the panel, with a far larger shrink,
//!   gives up its width first, down to 320, when the window narrows (the
//!   frame before the row is measured again).
//! - Cost: the slot hands this track the whole row, not only the panel.

use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::tabs::{tab, tab_head};
use vessel_app_kit::theme::COMMAND_SCRIM;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, canvas, div, px,
    rgba,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, breakpoint, glyph, popup};
use vessel_app_view::panels::{CODE_PANEL, Placement, SIDEBAR, code_placement, over_width};

use super::{Place, Tab};
use crate::areas::edge::Panel;
use crate::shell::Shell;

/// The panel as a resizable panel: dragged at its left edge, kept in the
/// layout's `panel_width`.
pub(super) const PANEL: Panel = Panel {
    limits: CODE_PANEL,
    edge: |shell| &shell.areas.workspace.code.edge,
    kept: |shell| shell.prefs.settings.layout.panel_width,
    keep: |layout, width| layout.panel_width = width,
    side: vessel_app_kit::resize::Edge::Left,
};

/// The session bar's toggle (track C's bar, its right actions): Paseo's
/// panel glyph, filled while the panel is out. `None` without an open
/// session.
pub(crate) fn code_toggle(
    shell: &Shell,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let _open = shell.areas.session.open.as_ref()?;
    let neutral = tokens.neutral;
    let open = shell.areas.workspace.code.open;
    let hover = neutral.surface2;
    Some(
        div()
            .id("code-toggle")
            .flex_shrink_0()
            .size(px(if narrow { 44.0 } else { 28.0 }))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(popup::MENU_RADIUS))
            .cursor_pointer()
            .when_open(open, neutral.surface2)
            .hover(move |style| style.bg(hover))
            .child(glyph(
                Glyph::PanelRight,
                16.0,
                if open {
                    neutral.foreground
                } else {
                    neutral.foreground_muted
                },
            ))
            .on_click(cx.listener(|this, _event, window, cx| this.toggle_code(window, cx)))
            .into_any_element(),
    )
}

/// A fill while a toggle is on.
trait WhenOpen {
    fn when_open(self, open: bool, fill: vessel_app_kit::ui::Hsla) -> Self;
}

impl WhenOpen for vessel_app_kit::ui::Stateful<Div> {
    fn when_open(self, open: bool, fill: vessel_app_kit::ui::Hsla) -> Self {
        if open { self.bg(fill) } else { self }
    }
}

/// The width of the window's content row for a window `width` wide: the
/// window less the frame's margins around the content panel (`shell.rs`).
pub(crate) fn frame_row(width: f32) -> f32 {
    spacing::S1.mul_add(-1.0, spacing::S2.mul_add(-1.0, width))
}

impl Shell {
    /// Where the code panel is drawn now on a desktop window, `None` when
    /// it is not out (`panels::code_placement`: beside the conversation
    /// when the row holds both leasts with the sidebar at its least, over
    /// it otherwise). A phone's width draws it over the whole screen.
    pub(crate) fn code_placement(&self, window: &Window) -> Option<Placement> {
        let out = self.areas.workspace.code.open
            && self.route == crate::routes::Route::Session
            && self.areas.session.open.is_some();
        if !out || breakpoint::phone(window) {
            return None;
        }
        let sidebar = if self.sidebar_shown() {
            SIDEBAR.least
        } else {
            0.0
        };
        let row = sidebar.mul_add(-1.0, frame_row(f32::from(window.viewport_size().width)));
        Some(code_placement(row))
    }
}

/// Track F's slot in the lead's session screen: the screen, with the code
/// panel beside it when it is out on an open session, or over it when the
/// window is too narrow for both (`Shell::code_placement`). See the
/// module's decision log for why the row is drawn here.
pub(crate) fn code_beside(
    shell: &Shell,
    screen: Div,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let Some(panel) = code_panel(shell, tokens, window, cx) else {
        return screen;
    };
    let row = Rc::clone(&shell.areas.workspace.code.row);
    // Over the conversation, the conversation is dimmed and a press on it
    // closes the panel and returns to it: Paseo's panel backdrop on its
    // compact layout (`mobile-panels/presentation.tsx`, black at 50 %,
    // `onPress={showMobileAgent}`). A panel as wide as the row covers it
    // all, as Paseo's does; its close button is then the way back.
    let over = shell.code_placement(window) == Some(Placement::Over);
    let scrim = over.then(|| {
        div()
            .absolute()
            .inset_0()
            .bg(rgba(COMMAND_SCRIM))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, window, cx| this.toggle_code(window, cx)),
            )
    });
    div()
        .size_full()
        .relative()
        .flex()
        .flex_row()
        .child(
            canvas(
                move |bounds, _window, _cx| row.set(Some(f32::from(bounds.size.width))),
                |_, (), _, _| {},
            )
            .absolute()
            .size_full(),
        )
        // The conversation grows from its least width and gives it up only
        // when the panel is down to its own.
        .child(
            screen
                .flex_1()
                .flex_basis(px(CODE_PANEL.beside))
                .min_w(px(0.0)),
        )
        .children(scrim)
        .child(panel)
}

/// The panel beside the conversation column, when it is out on an open
/// session.
fn code_panel(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let panel = &shell.areas.workspace.code;
    if !panel.open || shell.areas.session.open.is_none() {
        return None;
    }
    // The open session changed while the panel was out: read the new
    // one's place after this frame.
    if shell.code_session().is_some_and(|(machine, session)| {
        panel
            .target
            .as_ref()
            .is_none_or(|target| target.machine != machine || target.session != session)
    }) {
        cx.defer_in(window, |this, _window, cx| {
            this.code_follow();
            cx.notify();
        });
    }
    let neutral = tokens.neutral;
    let labels = shell.words().labels();
    let phone = breakpoint::phone(window);
    let tabs = div()
        .child(
            tab(
                "code-tab-changes",
                labels.panel_changes,
                panel.tab == Tab::Changes,
                None,
                tokens,
            )
            .on_click(cx.listener(|this, _event, _window, cx| this.code_tab(Tab::Changes, cx))),
        )
        .child(
            tab(
                "code-tab-files",
                labels.panel_files,
                panel.tab == Tab::Files,
                None,
                tokens,
            )
            .on_click(cx.listener(|this, _event, _window, cx| this.code_tab(Tab::Files, cx))),
        );
    let close = div().child(
        div()
            .id("code-close")
            .size(px(if phone { 44.0 } else { 28.0 }))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(popup::MENU_RADIUS))
            .cursor_pointer()
            .hover(move |style| style.bg(neutral.surface2))
            .child(glyph(Glyph::Close, 14.0, neutral.foreground_muted))
            .on_click(cx.listener(|this, _event, window, cx| this.toggle_code(window, cx))),
    );
    let body: AnyElement = match panel.target.as_ref().map(|target| &target.place) {
        None | Some(Place::Reading) => note(labels.loading, tokens).into_any_element(),
        Some(Place::Outside) => note(labels.not_in_project, tokens).into_any_element(),
        Some(Place::Failed(why)) => note(why.clone(), tokens).into_any_element(),
        Some(Place::Root(_)) => match panel.tab {
            Tab::Changes => super::changes::render(shell, tokens, window, cx),
            Tab::Files => super::files::render(shell, tokens, window, cx),
        },
    };
    let frame = div()
        .flex()
        .flex_col()
        .h_full()
        .min_h(px(0.0))
        .bg(neutral.surface0)
        .child(tab_head(tabs, Some(close), tokens))
        .child(div().flex_1().min_h(px(0.0)).flex().flex_col().child(body));
    Some(if phone {
        frame.absolute().inset_0().size_full().into_any_element()
    } else {
        // Before the row is first measured, no room limit: the row's flex
        // keeps the conversation's least width for that frame.
        let room = panel.row.get().unwrap_or(f32::INFINITY);
        let frame = match shell.code_placement(window) {
            // Too narrow for both: over the conversation, which keeps
            // the whole row under it (`panels::code_placement`).
            Some(Placement::Over) => over(frame, over_width(PANEL.asked(shell), room), tokens),
            Some(Placement::Beside) | None => beside(frame, PANEL.width(shell, room), tokens),
        };
        frame
            .children(PANEL.handle("code-edge", shell, tokens, window, cx))
            .into_any_element()
    })
}

/// The panel beside the conversation: its hairline and its width (the
/// handle on the hairline is `PANEL`'s). Its shrink outweighs the
/// conversation's a thousandfold, so a narrower window takes the panel's
/// width first, down to `CODE_PANEL.least`.
fn beside(frame: Div, width: f32, tokens: &Tokens) -> Div {
    frame
        .relative()
        .w(px(width))
        .flex_shrink(1000.0)
        .min_w(px(CODE_PANEL.least))
        .border_l_1()
        .border_color(tokens.neutral.border)
}

/// The panel over the conversation at the row's right, `width` wide, with
/// its hairline and the floating surfaces' shadow so it reads as above
/// what it covers.
fn over(frame: Div, width: f32, tokens: &Tokens) -> Div {
    frame
        .absolute()
        .top_0()
        .right_0()
        .h_full()
        .w(px(width))
        .border_l_1()
        .border_color(tokens.neutral.border)
        .shadow(popup::shadow_md(tokens))
        // The scrim under it closes the panel on a press; without this a
        // press on the panel's own controls reached it too (a link's
        // on_click lost its element before the click ended).
        .occlude()
}

/// A line where the panel has nothing else to show.
pub(super) fn note(text: impl Into<vessel_app_kit::ui::SharedString>, tokens: &Tokens) -> Div {
    div()
        .p(px(16.0))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.into())
}

impl Shell {
    fn code_tab(&mut self, tab: Tab, cx: &mut Context<'_, Self>) {
        if self.areas.workspace.code.tab != tab {
            self.areas.workspace.code.changes.sources.invalidate();
        }
        self.areas.workspace.code.tab = tab;
        self.code_read_tab();
        cx.notify();
    }
}
