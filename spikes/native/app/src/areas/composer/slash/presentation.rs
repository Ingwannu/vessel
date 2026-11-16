//! Existing autocomplete presentation, kept separate from command admission.

use super::catalog::{Choice, Menu};
use super::{LIFT, OFFSET_LOW, Over, ROWS, Shell, room_above};
use vessel_app_kit::autocomplete::{AutocompleteRow, Listing, RowKind, autocomplete};
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, IntoElement as _, ParentElement as _, SharedString,
    Styled, Window, div, px,
};
use vessel_app_kit::{Tokens, anchor_probe, motion};
use vessel_app_view::composer_shell::commands::ClientCommand;

/// The `/` menu over the session's composer card.
pub(crate) fn menu(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    drawn_menu(shell, Over::Session, tokens, window, cx)
}

/// The `/` menu over the new chat's composer card.
pub(crate) fn draft_menu(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    drawn_menu(shell, Over::Draft, tokens, window, cx)
}

/// The menu over a composer card, as wide as the card, 12 above it
/// (Paseo's popover), when the caret is in a command's name; while it
/// closes, its last rows, fading.
fn drawn_menu(
    shell: &Shell,
    over: Over,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let now = shell.areas.composer.now();
    let state = shell.slash(over);
    let current = shell
        .slash_menu(over, cx)
        .map(|menu| (listing_of(shell, over, &menu), menu.chosen));
    *state.rendered.borrow_mut() = current.as_ref().and_then(|_| {
        let target = shell.composer_current_target()?;
        let field = shell.slash_field(over);
        Some((target, field.text(cx), field.caret(cx)))
    });
    let mut reveal = state.reveal.borrow_mut();
    let length = if motion::held(cx) {
        0.0
    } else {
        motion::STANDARD.as_secs_f32()
    };
    reveal.follow(current.is_some(), now, length);
    let Some(shown) = reveal.shown(now) else {
        drop(state.last.borrow_mut().take());
        return None;
    };
    if reveal.moving(now) {
        window.request_animation_frame();
    }
    drop(reveal);
    let (listing, chosen) = match current {
        Some(current) => {
            *state.last.borrow_mut() = Some(current.clone());
            current
        }
        None => state.last.borrow().clone()?,
    };
    let on_choose: fn(&mut Shell, usize, &mut Window, &mut Context<'_, Shell>) = match over {
        Over::Session => Shell::session_command_clicked,
        Over::Draft => Shell::draft_command_clicked,
    };
    let list = autocomplete(
        over.id(),
        &listing,
        chosen,
        &state.list,
        on_choose,
        ROWS,
        tokens,
        cx,
    );
    // The room is the last frame's: on the menu's first frame there is
    // none yet, so one more frame (as the kit's list does for its scroll).
    let room = state.drawn.get().map(|bounds| room_above(bounds.bottom()));
    if room.is_none() {
        window.request_animation_frame();
    }
    Some(
        div()
            .absolute()
            .left_0()
            .right_0()
            .bottom_full()
            .mb(px(LIFT.mul_add(shown, OFFSET_LOW)))
            .flex()
            .flex_col()
            .when_some(room, Styled::max_h)
            .opacity(shown)
            .child(anchor_probe(&state.drawn))
            .child(list)
            .into_any_element(),
    )
}

/// The menu's rows as the kit's list draws them, or the words for none:
/// in the new chat, that its agent's commands are not known yet when no
/// session of it has said them.
fn listing_of(shell: &Shell, over: Over, menu: &Menu<'_>) -> Listing {
    if menu.rows.is_empty() {
        let words = shell.words.composer();
        let unknown = over == Over::Draft && matches!(shell.slash_listed(over), Some(None));
        Listing::Empty(SharedString::from(
            if unknown {
                words.commands_unknown()
            } else {
                words.commands_empty()
            }
            .to_owned(),
        ))
    } else {
        Listing::Rows(
            menu.rows
                .iter()
                .map(|command| {
                    let (detail, description) = match command {
                        Choice::Client(command) => (
                            None,
                            match command {
                                ClientCommand::Exit => shell.words.composer_shell().client_exit(),
                                ClientCommand::Clear => shell.words.composer_shell().client_clear(),
                            }
                            .to_owned(),
                        ),
                        Choice::Provider(command) => {
                            (command.hint.clone(), command.description.clone())
                        }
                    };
                    AutocompleteRow {
                        label: SharedString::from(format!("/{}", command.name())),
                        detail: detail.map(SharedString::from),
                        description: (!description.is_empty())
                            .then(|| SharedString::from(description)),
                        kind: RowKind::Command,
                    }
                })
                .collect(),
        )
    }
}
