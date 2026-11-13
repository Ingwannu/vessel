//! The panel's frame and its two slots in the lead's and track C's
//! screens: Paseo's Explorer beside the session (a hairline at its left,
//! the tab rail on top with Close at its end), 440 wide; at a phone's
//! width the whole screen, as Paseo's compact layout opens the Explorer
//! over the workspace.
//!
//! Difference from Paseo, named: its Explorer is 320 wide by default and
//! opens a file's diff in the main pane; here the diff is in the panel,
//! so the panel starts wider (it does not resize yet).

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::tabs::{tab, tab_head};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, breakpoint, glyph, popup};

use super::{Place, Tab};
use crate::shell::Shell;

/// The panel's width beside the conversation.
const WIDTH: f32 = 440.0;

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
            .on_click(cx.listener(|this, _event, _window, cx| this.toggle_code(cx)))
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

/// The panel beside the conversation column (the lead's session screen),
/// when it is out on an open session.
pub(crate) fn code_panel(
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
            .on_click(cx.listener(|this, _event, _window, cx| this.toggle_code(cx))),
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
        frame
            .w(px(WIDTH))
            .flex_none()
            .border_l_1()
            .border_color(neutral.border)
            .into_any_element()
    })
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
        self.areas.workspace.code.tab = tab;
        self.code_read_tab();
        cx.notify();
    }
}
