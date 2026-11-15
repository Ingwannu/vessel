//! The find bar as GOAT draws it, over the conversation's top right: its
//! box, the count, the previous and next buttons and close. Its state and
//! what its keys and buttons do are the parent module's.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    KeyDownEvent, ParentElement as _, Stateful, StatefulInteractiveElement as _, Styled as _,
    Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, breakpoint, popup};

use crate::shell::Shell;

use super::{INSET, WIDTH};

/// The bar over the conversation's top right, when it is open.
pub(crate) fn bar(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let bar = shell.areas.conversation.find.as_ref()?;
    let words = shell.t.lang.conversation();
    let surfaces = popup::Surfaces::of(tokens);
    let narrow = breakpoint::phone(window);
    let has_query = !bar.query.trim().is_empty();
    let none = has_query && bar.finder.is_none() && bar.matches.is_empty();
    let count = if !has_query {
        String::new()
    } else if bar.finder.is_some() {
        words.count_pending().to_owned()
    } else {
        words.count_text(bar.matches.len(), bar.index)
    };
    let can_move = bar.finder.is_none() && !bar.matches.is_empty();
    let shell_row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .h(px(if narrow { 44.0 } else { 36.0 }))
        .pl(px(12.0))
        .pr(px(4.0))
        .bg(surfaces.surface1)
        .border_1()
        .border_color(surfaces.border_accent)
        .rounded(px(popup::MENU_RADIUS))
        .shadow(popup::shadow_md(tokens))
        .on_key_down(cx.listener(|shell, event: &KeyDownEvent, _window, cx| {
            match event.keystroke.key.as_str() {
                "escape" => {
                    shell.close_find(cx);
                    cx.stop_propagation();
                }
                "enter" if event.keystroke.modifiers.shift => {
                    shell.find_move(false, cx);
                    cx.stop_propagation();
                }
                _ => {}
            }
        }))
        .child(bar.field.input(tokens, cx))
        .child(count_label(count, none, tokens))
        .children(buttons(can_move, tokens, narrow, cx));
    // On a phone the bar takes its own place above the conversation: laid
    // over it, it would hide the very match it scrolls to (the page lays
    // it over at every width, the same fault).
    let frame = if narrow {
        div().flex_none().px(px(INSET)).pt(px(8.0)).pb(px(4.0))
    } else {
        div()
            .absolute()
            .top(px(INSET))
            .right(px(INSET))
            .w(px(WIDTH))
    };
    Some(frame.child(shell_row).into_any_element())
}

/// The count of matches (or that find is still looking), in the danger
/// colour when there is none.
fn count_label(count: String, none: bool, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .min_w(px(40.0))
        .flex_none()
        .text_center()
        .type_(Type::Meta)
        .text_color(if none {
            neutral.destructive
        } else {
            neutral.foreground_muted
        })
        .child(count)
}

/// Previous and next (answering only while there are matches to move
/// between), and close.
fn buttons(
    can_move: bool,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> [Stateful<Div>; 3] {
    [
        small_button("find-previous", Glyph::ChevronUp, can_move, tokens, narrow).when(
            can_move,
            |button| {
                button.on_click(cx.listener(|shell, _event, _window, cx| {
                    shell.find_move(false, cx);
                }))
            },
        ),
        small_button("find-next", Glyph::ChevronDown, can_move, tokens, narrow).when(
            can_move,
            |button| {
                button.on_click(cx.listener(|shell, _event, _window, cx| {
                    shell.find_move(true, cx);
                }))
            },
        ),
        small_button("find-close", Glyph::Close, true, tokens, narrow).on_click(cx.listener(
            |shell, _event, _window, cx| {
                shell.close_find(cx);
            },
        )),
    ]
}

/// One of the bar's buttons (GOAT's `icon-xs` ghost button): its glyph,
/// dimmed while it cannot act.
fn small_button(
    id: &'static str,
    mark: Glyph,
    enabled: bool,
    tokens: &Tokens,
    narrow: bool,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let hover = popup::Surfaces::of(tokens).surface2;
    div()
        .id(id)
        .size(px(if narrow { 32.0 } else { 24.0 }))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .when(enabled, |button| {
            button.cursor_pointer().hover(move |style| style.bg(hover))
        })
        .child(glyph(
            mark,
            // The close cross fills its box; the chevrons do not.
            if mark == Glyph::Close { 12.0 } else { 14.0 },
            if enabled {
                neutral.foreground_muted
            } else {
                neutral.foreground_extra_muted
            },
        ))
}
