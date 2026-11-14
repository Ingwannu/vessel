//! The session composer's `/` menu: the agent's commands over the composer
//! while the prompt starts with a command's name, as Paseo's composer shows
//! them (`hooks/use-agent-autocomplete.ts`, `components/ui/autocomplete.tsx`,
//! `components/ui/autocomplete-popover.tsx`). The rules are
//! `vessel_app_view::composer::slash`'s; the list is the kit's.
//!
//! Only a running structured session's composer has it: the commands are
//! what this session's agent last said (`commands_changed`), and until it
//! has, what its last session said, kept on the device (the store,
//! `offers.rs`), so the menu opens on a list at once.
//!
//! It opens and closes over `slider::motion`'s glide (the owner's word 39;
//! Paseo's popover appears at once): `composer::reveal` holds how open it
//! is, the last rows stay drawn while it closes, and frames are asked only
//! while it moves.

use std::time::Duration;

use vessel_app_kit::Tokens;
use vessel_app_kit::autocomplete::{AutocompleteRow, Listing, Motion, RowKind, autocomplete};
use vessel_app_kit::field::composer_hooks::{ComposerKey, KeyAnswer, hooked};
use vessel_app_kit::ui::{
    AnyElement, App, Context, IntoElement as _, ParentElement as _, SharedString, Styled as _,
    Window, div, px,
};
use vessel_app_view::composer::slash::{Menu, Step, apply};
use vessel_app_view::slider::motion::{DRAG_GLIDE, SLIDE};

use crate::routes::Route;
use crate::shell::Shell;

/// How far the list rises into place as it opens (points).
const LIFT: f32 = 4.0;

/// The list's gap above the composer when it starts to open: Paseo's
/// `OFFSET_FROM_ANCHOR` (`spacing::S3`, 12) less [`LIFT`], written out
/// (the lints keep float arithmetic out of the code; a test holds the sum).
const OFFSET_LOW: f32 = 8.0;

/// The app's motion for the list's rows (the chosen row's fill).
fn motion() -> Motion {
    Motion {
        ease: |progress| SLIDE.at(progress),
        length: Duration::from_secs_f32(DRAG_GLIDE),
    }
}

impl Shell {
    /// The `/` menu of the session's composer, when its caret is in a
    /// command's name and the session is a running structured one.
    fn slash_menu(&self, cx: &App) -> Option<Menu<'_>> {
        let open = self.areas.session.open.as_ref()?;
        if self.route != Route::Session || !open.structured || !open.live {
            return None;
        }
        let field = &self.areas.session.composer;
        let text = field.text(cx);
        // A recalled message is sent as it was, a leading `/name` too
        // (Paseo's recall: no menu over it).
        if self.areas.composer.recall.showing(&text) {
            return None;
        }
        let commands = self.session_commands();
        Menu::of(&text, field.caret(cx), commands, &self.areas.composer.slash)
    }

    /// A key the session's composer offered: the menu takes the arrows,
    /// Enter, Tab and Escape while it lists commands (Paseo's
    /// `useAutocomplete`: with no rows, every key is the text's, so Enter
    /// sends what is typed).
    pub(crate) fn composer_key(
        &mut self,
        key: ComposerKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> KeyAnswer {
        let Some((query, rows, range, name)) = self.slash_menu(cx).map(|menu| {
            let name = menu
                .taken(&self.areas.composer.slash)
                .map(|command| command.name.as_str().to_owned());
            (menu.range.query.clone(), menu.rows.len(), menu.range, name)
        }) else {
            // No menu: Up and Down walk the messages sent (`drafts.rs`).
            return self.recall_key(key, window, cx);
        };
        if rows == 0 {
            return self.recall_key(key, window, cx);
        }
        let chosen = &mut self.areas.composer.slash;
        let _now = chosen.follow(&query, rows);
        match key {
            ComposerKey::Up => {
                let _moved = chosen.step(Step::Up, rows);
            }
            ComposerKey::Down => {
                let _moved = chosen.step(Step::Down, rows);
            }
            ComposerKey::Enter | ComposerKey::Tab => {
                if let Some(name) = name {
                    self.write_command(&range, &name, window, cx);
                }
            }
            // Paseo empties the prompt when Escape closes a menu opened at
            // its start.
            ComposerKey::Escape => {
                self.areas.composer.slash.reset();
                self.areas.session.composer.replace_all("", window, cx);
            }
        }
        cx.notify();
        KeyAnswer::Taken
    }

    /// Write command `name` over the name being typed, the caret after it.
    fn write_command(
        &mut self,
        range: &vessel_app_view::composer::slash::SlashRange,
        name: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let field = &self.areas.session.composer;
        let text = field.text(cx);
        let applied = apply(&text, range, name);
        let written = applied
            .text
            .get(range.start..applied.caret)
            .unwrap_or_default()
            .to_owned();
        field.replace_range(range.start..range.end, &written, window, cx);
        self.areas.composer.slash.reset();
    }

    /// A row of the `/` menu clicked.
    fn command_clicked(&mut self, index: usize, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some((range, name)) = self.slash_menu(cx).and_then(|menu| {
            let name = menu.rows.get(index)?.name.as_str().to_owned();
            Some((menu.range, name))
        }) else {
            return;
        };
        self.write_command(&range, &name, window, cx);
        self.areas.session.composer.focus(window, cx);
        cx.notify();
    }
}

/// The session composer's text with its keys offered to the menu first
/// (`field::composer_hooks`). Pastes are the text's until the attachments
/// slice takes images.
pub(crate) fn text(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let field = &shell.areas.session.composer;
    let element = field.element(tokens, touch, cx);
    let weak = cx.entity().downgrade();
    hooked(
        field,
        element,
        move |key, window: &mut Window, cx: &mut App| {
            weak.update(cx, |shell, cx| shell.composer_key(key, window, cx))
                .unwrap_or(KeyAnswer::Pass)
        },
        |_pasted, _window, _cx| KeyAnswer::Pass,
    )
}

/// The `/` menu over the composer card, as wide as the card, 12 above it
/// (Paseo's popover), when the caret is in a command's name; while it
/// closes, its last rows, fading.
pub(crate) fn menu(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.composer;
    let now = area.now();
    let current = shell
        .slash_menu(cx)
        .map(|menu| (listing_of(shell, &menu), menu.chosen));
    let mut reveal = area.reveal.borrow_mut();
    reveal.follow(current.is_some(), now, cx.reduce_motion());
    let Some(shown) = reveal.shown(now) else {
        drop(area.last_menu.borrow_mut().take());
        return None;
    };
    if reveal.moving(now) {
        window.request_animation_frame();
    }
    drop(reveal);
    let (listing, chosen) = match current {
        Some(current) => {
            *area.last_menu.borrow_mut() = Some(current.clone());
            current
        }
        None => area.last_menu.borrow().clone()?,
    };
    let list = autocomplete(
        "composer-commands",
        &listing,
        chosen,
        &area.list,
        Shell::command_clicked,
        motion(),
        tokens,
        cx,
    );
    Some(
        div()
            .absolute()
            .left_0()
            .right_0()
            .bottom_full()
            .mb(px(LIFT.mul_add(shown, OFFSET_LOW)))
            .opacity(shown)
            .child(list)
            .into_any_element(),
    )
}

/// The menu's rows as the kit's list draws them, or the words for none.
fn listing_of(shell: &Shell, menu: &Menu<'_>) -> Listing {
    if menu.rows.is_empty() {
        Listing::Empty(SharedString::from(
            shell.words.composer().commands_empty().to_owned(),
        ))
    } else {
        Listing::Rows(
            menu.rows
                .iter()
                .map(|command| AutocompleteRow {
                    label: SharedString::from(format!("/{}", command.name.as_str())),
                    detail: command.hint.clone().map(SharedString::from),
                    description: (!command.description.is_empty())
                        .then(|| SharedString::from(command.description.clone())),
                    kind: RowKind::Command,
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use vessel_app_kit::theme::spacing;

    use super::*;

    #[test]
    fn the_list_comes_to_rest_at_paseos_offset_above_the_card() {
        let open = LIFT.mul_add(1.0, OFFSET_LOW);
        assert!((open - spacing::S3).abs() < 1e-6, "{open}");
    }
}
