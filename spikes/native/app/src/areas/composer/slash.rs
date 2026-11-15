//! The `/` menu over a composer: the agent's commands while the prompt
//! starts with a command's name, as Paseo's composer shows them
//! (`hooks/use-agent-autocomplete.ts`, `components/ui/autocomplete.tsx`,
//! `components/ui/autocomplete-popover.tsx`). Typed precedence lives in
//! `composer_shell::catalog`; replacement/choice reuse `composer::slash`.
//!
//! Two composers have one: a running structured session's, and the new
//! chat's. Each keeps its own choice, scroll and glide ([`SlashMenu`]), so
//! the two never move each other.
//!
//! [Decision Log]
//! - Purpose: Paseo's new chat completes `/` too (its draft asks the daemon
//!   for the commands of the agent it would start); the lead approved the
//!   new chat's menu over the draft's card (two seam lines).
//! - Constraint: older peers report only running-session commands. The new
//!   prompt-free draft catalog is gated separately; its app adapter is pending.
//! - Alternatives: start the agent to ask it (a process per keystroke's
//!   menu, and the node has no such call); no menu in the new chat (Paseo
//!   has one).
//! - Choice: typed built-ins precede provider entries. The session lists what its agent said, and until it
//!   has, what its last session said, kept on the device (the store,
//!   `offers.rs`); the new chat's lists the kept ones of the agent it
//!   would start on that machine, and says when none of its sessions has
//!   said them yet (`composer-commands-unknown`).
//! - Cost: until the gated draft adapter lands, the new chat's provider list is
//!   as old as that agent's last session. Unknown kinds remain root-only.
//!
//! It opens and closes over the kit's `motion::STANDARD` (a surface coming
//! and going; the owner's word 39; Paseo's popover appears at once), its
//! chosen row's fill over `QUICK`: `composer::reveal` holds how open it
//! is, the last rows stay drawn while it closes, and frames are asked only
//! while it moves; held motion (`motion::held`) is the end at once.

mod admission;
mod catalog;
mod client;
mod input;
mod presentation;
pub(crate) use input::{draft_text, text, workspace_text};
pub(crate) use presentation::{draft_menu, menu};

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use catalog::Menu;
use vessel_app_kit::autocomplete::{AutocompleteState, Listing, Motion};
use vessel_app_kit::field::composer_hooks::{ComposerKey, KeyAnswer};
use vessel_app_kit::ui::{App, Bounds, Context, Pixels, Window, px};
use vessel_app_kit::{TextField, chrome, motion};
use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::composer::reveal::Reveal;
use vessel_app_view::composer::slash::{Chosen, SlashRange, Step, apply};
use vessel_app_view::composer_shell::catalog::Target;
use vessel_app_view::composer_shell::commands::ClientCommand;
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
    fn for_target(target: DraftKey) -> Option<Self> {
        match target {
            DraftKey::New => Some(Self::Draft),
            DraftKey::Session { .. } => Some(Self::Session),
            DraftKey::Workspace { .. } => None,
        }
    }

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
    /// The owner and input actually drawn, so an old row cannot act on a new session.
    rendered: RefCell<Option<(DraftKey, String, usize)>>,
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
        let target = match over {
            Over::Draft => DraftKey::New,
            Over::Session => DraftKey::Session {
                session: self.areas.session.open.as_ref()?.session,
            },
        };
        if self.composer_current_target() != Some(target)
            || self
                .composer_surface(target, cx)
                .is_none_or(|surface| !surface.autocomplete())
        {
            return None;
        }
        let listed = self.slash_listed(over)?;
        let field = self.slash_field(over);
        let text = field.text(cx);
        // A recalled message is sent as it was, a leading `/name` too
        // (Paseo's recall: no menu over it).
        if over == Over::Session && self.areas.composer.recall.showing(&text) {
            return None;
        }
        let target = match over {
            Over::Session => Target::Session,
            Over::Draft => Target::Draft,
        };
        Menu::of(
            &text,
            field.caret(cx),
            target,
            listed,
            &self.slash(over).chosen,
        )
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
        let target = match over {
            Over::Draft => Some(DraftKey::New),
            Over::Session => self
                .areas
                .session
                .open
                .as_ref()
                .map(|open| DraftKey::Session {
                    session: open.session,
                }),
        };
        if !target.is_some_and(|target| self.composer_command_menu_owns_key(target, key, cx)) {
            return self.key_without_menu(over, key, window, cx);
        }
        let taken = self.slash_menu(over, cx).and_then(|menu| {
            if !menu.takes_key(key) {
                return None;
            }
            let choice = menu
                .taken(&self.slash(over).chosen)
                .map(|command| (command.name().to_owned(), command.client()));
            let rows = menu.rows.len();
            (rows > 0).then(|| (menu.range.query.clone(), rows, menu.range, choice))
        });
        let Some((query, rows, range, choice)) = taken else {
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
                if let Some((name, client)) = choice {
                    self.choose_command(over, &range, &name, client, window, cx);
                }
            }
            // Paseo empties the prompt when Escape closes a menu opened at
            // its start.
            ComposerKey::Escape => {
                if range.start != 0 {
                    return KeyAnswer::Pass;
                }
                self.slash_mut(over).chosen.reset();
                self.slash_field(over).replace_all("", window, cx);
            }
        }
        cx.notify();
        KeyAnswer::Taken
    }

    fn choose_command(
        &mut self,
        over: Over,
        range: &SlashRange,
        name: &str,
        client: Option<ClientCommand>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if over == Over::Session
            && let Some(command) = client
        {
            let token = format!("/{}", command.name());
            if self.composer_client_command(&token, window, cx) {
                self.slash_mut(over).chosen.reset();
                return;
            }
        }
        self.write_command(over, range, name, window, cx);
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
        let field = self.slash_field(over);
        let expected = self.slash(over).rendered.borrow().clone();
        let Some((target, text, caret)) = expected else {
            return;
        };
        if self.composer_current_target() != Some(target)
            || field.text(cx) != text
            || field.caret(cx) != caret
        {
            return;
        }
        let Some((range, name, client)) = self.slash_menu(over, cx).and_then(|menu| {
            let choice = *menu.rows.get(index)?;
            let name = choice.name().to_owned();
            let last = self.slash(over).last.borrow();
            let Some((Listing::Rows(rows), _)) = last.as_ref() else {
                return None;
            };
            if rows.get(index)?.label.as_ref() != format!("/{name}") {
                return None;
            }
            Some((menu.range, name, choice.client()))
        }) else {
            return;
        };
        self.choose_command(over, &range, &name, client, window, cx);
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

#[cfg(test)]
mod tests;
