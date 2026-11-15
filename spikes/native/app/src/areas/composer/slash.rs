//! The `/` menu over a composer: the agent's commands while the prompt
//! starts with a command's name, as Paseo's composer shows them
//! (`hooks/use-agent-autocomplete.ts`, `components/ui/autocomplete.tsx`,
//! `components/ui/autocomplete-popover.tsx`). The rules are
//! `vessel_app_view::composer::slash`'s; the list is the kit's.
//!
//! Two composers have one: a running structured session's, and the new
//! chat's. Each keeps its own choice, scroll and glide ([`SlashMenu`]), so
//! the two never move each other.
//!
//! [Decision Log]
//! - Purpose: Paseo's new chat completes `/` too (its draft asks the daemon
//!   for the commands of the agent it would start); the lead approved the
//!   new chat's menu over the draft's card (two seam lines).
//! - Constraint: the node reports commands only from a running session
//!   (`commands_changed`); `harness.options`, all a new chat can ask,
//!   carries none.
//! - Alternatives: start the agent to ask it (a process per keystroke's
//!   menu, and the node has no such call); no menu in the new chat (Paseo
//!   has one).
//! - Choice: the session's menu lists what its agent said, and until it
//!   has, what its last session said, kept on the device (the store,
//!   `offers.rs`); the new chat's lists the kept ones of the agent it
//!   would start on that machine, and says when none of its sessions has
//!   said them yet (`composer-commands-unknown`).
//! - Cost: the new chat's list is as old as that agent's last session;
//!   commands added since show once a session of it runs.
//!
//! It opens and closes over the kit's `motion::STANDARD` (a surface coming
//! and going; the owner's word 39; Paseo's popover appears at once), its
//! chosen row's fill over `QUICK`: `composer::reveal` holds how open it
//! is, the last rows stay drawn while it closes, and frames are asked only
//! while it moves; held motion (`motion::held`) is the end at once.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use vessel_app_kit::autocomplete::{
    AutocompleteRow, AutocompleteState, Listing, Motion, RowKind, autocomplete,
};
use vessel_app_kit::field::composer_hooks::{ComposerKey, KeyAnswer, hooked};
use vessel_app_kit::ui::{
    AnyElement, App, Bounds, Context, FluentBuilder as _, IntoElement as _, ParentElement as _,
    Pixels, SharedString, Styled, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens, anchor_probe, chrome, motion};
use vessel_app_view::composer::reveal::Reveal;
use vessel_app_view::composer::slash::{Chosen, Menu, SlashRange, Step, apply};
use vessel_app_view::offers::OfferKey;
use vessel_proto::AgentCommand;

use crate::routes::Route;
use crate::shell::Shell;

/// How far the list rises into place as it opens (points).
const LIFT: f32 = 4.0;

/// The list's gap above the composer when it starts to open: Paseo's
/// `OFFSET_FROM_ANCHOR` (`spacing::S3`, 12) less [`LIFT`], written out
/// (the lints keep float arithmetic out of the code; a test holds the sum).
const OFFSET_LOW: f32 = 8.0;

/// The list's rows' motion: the chosen row's fill, a control's own change.
const ROWS: Motion = Motion {
    ease: motion::ease,
    length: motion::QUICK,
};

/// What is taken from the room above the menu's bottom edge: Paseo's
/// popover keeps `OFFSET_FROM_ANCHOR` (12) from the top of its host, here
/// below the app's own title bar (`chrome::HEIGHT`, 36), written out as a
/// negative (the lints keep float arithmetic out of the code; a test holds
/// the sum).
const UNDER_BAR: f32 = -48.0;

/// The same with the system's title bar (the window's content starts at
/// the top).
const AT_TOP: f32 = -12.0;

/// Which composer a `/` menu is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Over {
    /// A running session's.
    Session,
    /// The new chat's.
    Draft,
}

impl Over {
    /// The list's element id (its animations are keyed by it).
    const fn id(self) -> &'static str {
        match self {
            Self::Session => "composer-commands",
            Self::Draft => "draft-commands",
        }
    }
}

/// One composer's `/` menu: the chosen row, the list's scroll, how open it
/// is, the rows it showed last (drawn while it closes) and where it was
/// drawn last (its bottom edge is the card's top, less the offset). The
/// last three change while the window draws, so they are kept behind
/// cells.
#[derive(Debug, Default)]
pub(crate) struct SlashMenu {
    chosen: Chosen,
    list: AutocompleteState,
    reveal: RefCell<Reveal>,
    last: RefCell<Option<(Listing, Option<usize>)>>,
    drawn: Rc<Cell<Option<Bounds<Pixels>>>>,
}

/// How tall the menu may be: the room between its bottom edge (the last
/// frame's) and the top of the window's content, less Paseo's offset
/// (`autocomplete-popover.tsx`: `maxHeight` is the anchor's top less the
/// safe area and two offsets). The list scrolls within it and the chosen
/// command's card keeps its height, so a small window cuts nothing (the
/// owner's word 41: at 800 by 600 thirty commands left the window's top).
fn room_above(bottom: Pixels) -> Pixels {
    let taken = if chrome::system_titlebar() {
        AT_TOP
    } else {
        UNDER_BAR
    };
    // `mul_add`: the difference without an arithmetic operator.
    px(f32::from(bottom).mul_add(1.0, taken).max(0.0))
}

/// What a menu lists: the agent's commands, or `None` while none of its
/// sessions has said them.
type Listed<'a> = Option<&'a [AgentCommand]>;

impl Shell {
    fn slash(&self, over: Over) -> &SlashMenu {
        match over {
            Over::Session => &self.areas.composer.session_slash,
            Over::Draft => &self.areas.composer.draft_slash,
        }
    }

    fn slash_mut(&mut self, over: Over) -> &mut SlashMenu {
        match over {
            Over::Session => &mut self.areas.composer.session_slash,
            Over::Draft => &mut self.areas.composer.draft_slash,
        }
    }

    fn slash_field(&self, over: Over) -> &TextField {
        match over {
            Over::Session => &self.areas.session.composer,
            Over::Draft => &self.areas.draft.composer,
        }
    }

    /// The commands the menu over `over` would list; `None` where that
    /// composer has no menu now (another screen; a session not running
    /// or not structured; a new chat with no agent).
    fn slash_listed(&self, over: Over) -> Option<Listed<'_>> {
        match over {
            Over::Session => {
                let open = self.areas.session.open.as_ref()?;
                let here = self.route == Route::Session && open.structured && open.live;
                here.then(|| self.session_commands())
            }
            Over::Draft => {
                let draft = &self.areas.draft;
                let (machine, agent) = draft.machine.as_ref().zip(draft.agent.as_ref())?;
                let key = OfferKey::new(machine.as_str(), agent.clone());
                (self.route == Route::Draft).then(|| self.areas.composer.offers.commands(&key))
            }
        }
    }

    /// The `/` menu over `over`, when its caret is in a command's name.
    fn slash_menu(&self, over: Over, cx: &App) -> Option<Menu<'_>> {
        let listed = self.slash_listed(over)?;
        let field = self.slash_field(over);
        let text = field.text(cx);
        // A recalled message is sent as it was, a leading `/name` too
        // (Paseo's recall: no menu over it).
        if over == Over::Session && self.areas.composer.recall.showing(&text) {
            return None;
        }
        Menu::of(&text, field.caret(cx), listed, &self.slash(over).chosen)
    }

    /// A key the composer under `over` offered: the menu takes the arrows,
    /// Enter, Tab and Escape while it lists commands (Paseo's
    /// `useAutocomplete`: with no rows, every key is the text's, so Enter
    /// sends what is typed).
    fn slash_key(
        &mut self,
        over: Over,
        key: ComposerKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> KeyAnswer {
        let taken = self.slash_menu(over, cx).and_then(|menu| {
            let name = menu
                .taken(&self.slash(over).chosen)
                .map(|command| command.name.as_str().to_owned());
            let rows = menu.rows.len();
            (rows > 0).then(|| (menu.range.query.clone(), rows, menu.range, name))
        });
        let Some((query, rows, range, name)) = taken else {
            return self.key_without_menu(over, key, window, cx);
        };
        let chosen = &mut self.slash_mut(over).chosen;
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
                    self.write_command(over, &range, &name, window, cx);
                }
            }
            // Paseo empties the prompt when Escape closes a menu opened at
            // its start.
            ComposerKey::Escape => {
                self.slash_mut(over).chosen.reset();
                self.slash_field(over).replace_all("", window, cx);
            }
        }
        cx.notify();
        KeyAnswer::Taken
    }

    /// A key with no menu to take it: in a session, Up and Down walk the
    /// messages sent (`drafts.rs`); the new chat has none, so its keys are
    /// the text's.
    fn key_without_menu(
        &mut self,
        over: Over,
        key: ComposerKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> KeyAnswer {
        match over {
            Over::Session => self.recall_key(key, window, cx),
            Over::Draft => KeyAnswer::Pass,
        }
    }

    /// Write command `name` over the name being typed, the caret after it.
    fn write_command(
        &mut self,
        over: Over,
        range: &SlashRange,
        name: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let field = self.slash_field(over);
        let text = field.text(cx);
        let applied = apply(&text, range, name);
        let written = applied
            .text
            .get(range.start..applied.caret)
            .unwrap_or_default()
            .to_owned();
        field.replace_range(range.start..range.end, &written, window, cx);
        self.slash_mut(over).chosen.reset();
    }

    /// A row of the menu over `over` clicked.
    fn command_clicked(
        &mut self,
        over: Over,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some((range, name)) = self.slash_menu(over, cx).and_then(|menu| {
            let name = menu.rows.get(index)?.name.as_str().to_owned();
            Some((menu.range, name))
        }) else {
            return;
        };
        self.write_command(over, &range, &name, window, cx);
        self.slash_field(over).focus(window, cx);
        cx.notify();
    }

    // The kit's list takes a plain function per row click; one per
    // composer, each naming its own.
    fn session_command_clicked(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.command_clicked(Over::Session, index, window, cx);
    }

    fn draft_command_clicked(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.command_clicked(Over::Draft, index, window, cx);
    }
}

/// The session composer's text with its keys offered to the menu first
/// (`field::composer_hooks`); pasted images and files are attached
/// (`files.rs`).
pub(crate) fn text(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    hooked_text(shell, Over::Session, tokens, touch, cx)
}

/// The new chat's composer text, its keys offered to its `/` menu first
/// (the draft's card draws it in place of the bare field).
pub(crate) fn draft_text(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    hooked_text(shell, Over::Draft, tokens, touch, cx)
}

fn hooked_text(
    shell: &Shell,
    over: Over,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let field = shell.slash_field(over);
    let element = field.element(tokens, touch, cx);
    let weak = cx.entity().downgrade();
    let pasting = weak.clone();
    let new_chat = over == Over::Draft;
    hooked(
        field,
        element,
        move |key, window: &mut Window, cx: &mut App| {
            weak.update(cx, |shell, cx| shell.slash_key(over, key, window, cx))
                .unwrap_or(KeyAnswer::Pass)
        },
        // Pasted images and files are the composer's attachments.
        move |pasted, _window, cx: &mut App| {
            pasting
                .update(cx, |shell, cx| shell.composer_pasted(new_chat, pasted, cx))
                .unwrap_or(KeyAnswer::Pass)
        },
    )
}

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
mod tests;
