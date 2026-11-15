//! The app's title bar (the kit draws the bar and the window's controls;
//! this says what is in it): the mark, the app's name and the build's
//! label at the left; the update's status at the right, before the
//! window's buttons, where Zed shows its own.

use crate::update::UpdateState;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};

use crate::shell::Shell;

pub(crate) fn title_bar(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> impl IntoElement {
    let narrow = vessel_app_kit::breakpoint::phone(window);
    let content = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(7.0))
        // The sidebar's toggle, always there as Paseo's (its window-owned
        // collapse toggle, `PanelLeft`, `desktop-sidebar-layout.ts`): at a
        // phone's width it opens the sidebar's sheet (ADR 0036, the menu
        // mark), wider it shows or hides the sidebar (the layout's
        // preference, as Mod+B). The button occludes the bar so a press is
        // the button's, not the bar's drag.
        .child({
            let glyph = if narrow { Glyph::Menu } else { Glyph::PanelLeft };
            div()
                    .id("open-sidebar")
                    .w(px(32.0))
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(vessel_app_kit::theme::radius::R2))
                    .when(shell.sidebar_open(), |button| button.bg(tokens.bg_active))
                    .occlude()
                    .cursor_pointer()
                    .child(
                        vessel_app_kit::ui::svg()
                            .path(glyph.path())
                            .size(px(14.0))
                            .text_color(tokens.text_secondary),
                    )
                    .on_click(
                        cx.listener(|this, _event, window, cx| this.toggle_sidebar(window, cx)),
                    )
        })
        // The palette's search mark (track S's, `areas::palette::entry`).
        .child(crate::areas::palette::title_trigger(shell, tokens, cx))
        // The mark in the text's colour at the text's size, beside the name
        // (docs/design/brand/README.md: "In the interface").
        .child(vessel_app_kit::mark_mono(14.0, tokens.text_primary))
        .child(
            div()
                .type_(Type::Label)
                .text_color(tokens.text_primary)
                .child(shell.t.app),
        )
        .child(
            div()
                .type_(Type::Tiny)
                .text_color(tokens.text_tertiary)
                .child(crate::LABEL),
        )
        .into_any_element();
    let status = update_status(shell, narrow, tokens, cx).map(IntoElement::into_any_element);
    vessel_app_kit::chrome::title_bar(content, status, tokens, window)
}

/// The update's status as Zed shows it: nothing while idle or up to date;
/// quiet words while checking and downloading; a small button once a
/// build is in place; a quiet failure with "Try again" (never a dialog).
/// At a phone's width, an icon only (the restart and the retry stay
/// buttons). Also drawn in the sidebar's foot when the system draws the
/// title bar.
pub(crate) fn update_status(
    shell: &Shell,
    narrow: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    let words = shell.words.update();
    let quiet = |text: vessel_app_kit::ui::SharedString| {
        if narrow {
            div().child(icon(tokens.text_tertiary))
        } else {
            div()
                .type_(Type::Tiny)
                .text_color(tokens.text_tertiary)
                .child(text)
        }
    };
    match &shell.updates.state {
        UpdateState::Idle | UpdateState::UpToDate { .. } => None,
        UpdateState::Checking => Some(quiet(words.checking().into())),
        UpdateState::Downloading { .. } => Some(quiet(shell.updates.progress(shell.words))),
        UpdateState::Ready { .. } => {
            let button = div()
                .id("restart-to-update")
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .px(px(if narrow { 6.0 } else { 10.0 }))
                .py(px(3.0))
                .rounded(px(vessel_app_kit::theme::radius::R2))
                .bg(tokens.btn_primary_bg)
                .text_color(tokens.btn_primary_fg)
                .type_(Type::Meta)
                .cursor_pointer()
                .child(icon(tokens.btn_primary_fg));
            let button = if narrow {
                button
            } else {
                button.child(words.restart())
            };
            Some(
                div().child(button.on_click(cx.listener(|this, _event, _window, cx| {
                    this.restart_to_update(cx);
                }))),
            )
        }
        UpdateState::Failed { .. } => {
            let retry = div()
                .id("update-try-again")
                .type_(Type::Tiny)
                .text_color(tokens.text_secondary)
                .cursor_pointer()
                .hover(|style| style.text_color(tokens.text_primary))
                .on_click(cx.listener(|this, _event, _window, cx| this.check_for_updates(cx)));
            Some(if narrow {
                div().child(retry.child(icon(tokens.danger)))
            } else {
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .type_(Type::Tiny)
                            .text_color(tokens.text_tertiary)
                            .child(words.failed()),
                    )
                    .child(retry.underline().child(words.try_again()))
            })
        }
    }
}

fn icon(color: Hsla) -> AnyElement {
    vessel_app_kit::ui::svg()
        .path(Glyph::Update.path())
        .size(px(13.0))
        .flex_shrink_0()
        .text_color(color)
        .into_any_element()
}
