//! A workspace's shell terminals as tabs (TRACKER 481, 482, 483, 338, 342,
//! 354): Paseo's terminal panel (`components/terminal-pane.tsx`, opened by
//! `screens/workspace/terminals/use-workspace-terminals.ts`).
//!
//! [Decision Log]
//! - Purpose: Paseo opens a shell beside its agents as a tab: "Terminal" in
//!   the "+" menu, the New tab panel and Ctrl+Shift+T; the tab streams the
//!   shell, takes the keyboard, sizes it on focus, lists the workspace's
//!   other shells and kills one on a confirmed Close. The native app modelled
//!   the tab (`Target::Terminal`) and drew "panel unavailable" in it.
//! - Existing assets: ADR 0031's calls on the node (protocol 11); the
//!   terminal session screen's parser, grid, colours, replay and wheel
//!   (`areas::terminal`, ADR 0073), reused here; the view's key and paste
//!   encoders, `TerminalSender` (one send in flight) and `query_duty`; the
//!   layout's reconcile, close plans and tab menu.
//! - Alternatives: generalising the session screen's single `TerminalArea`
//!   to many (its tests and opening rules rewritten), or a GPUI entity per
//!   tab holding its own state (the app keeps every area's state in the
//!   shell, `areas/mod.rs`; a model entity read inside a region would draw
//!   stale, `crate::region`).
//! - Choice: an area of its own with one `Pane` per open terminal tab, keyed
//!   by machine and terminal, in the shell's state; the grid stays its own
//!   cached view (`TerminalGrid`), and one small entity per pane is the
//!   platform's text input (IME) for that pane, forwarding to the shell.
//! - Place: today a workspace layout is a session's (`LayoutKey::Session`),
//!   so a shell is opened beside that session (`TerminalPlace::Session`, in
//!   its folder), where Paseo opens one in the workspace's directory.
//! - List: the node pushes no list of terminals (ADR 0031); the app asks
//!   for the open workspace's on each fleet read (every 2 s, as the
//!   sessions are listed) and after its own create, close or an end, and
//!   reconciles the tabs as Paseo does on `terminals_changed`. From
//!   protocol 37 the ask is ADR 0116's long poll: asked again at once from
//!   the revision it answered while the list moves, so another client's
//!   shell shows one round trip after it opened.
//! - Size: from protocol 37 a pane claims the shell's size when it takes
//!   the keyboard or attaches and sends updates after (ADR 0116), so the
//!   page and the app on one shell no longer flip its size.
//! - Cost and stated differences: the list is a long poll, not Paseo's
//!   push, and below 37 a shell another client opened shows within one
//!   read; size ownership is per connection, so this app's windows share
//!   one owner; the tab's label is a rename's title, never the shell's own
//!   (no OSC title on the node); no key bar (P4), no file drop (P7).
//! - Find, selection and copy (P3): Paseo's desktop terminal (xterm.js):
//!   Mod+F's bar while the terminal has the keyboard (`find`, `find_bar`),
//!   mouse selection and Paseo's copy keys (`select`); a paste is queued
//!   whole and sent in parts the node takes (the view's `TerminalSender`);
//!   Settings > Terminal's scrollback applies to open tabs at once.

mod beside;
mod body;
mod draw;
mod find;
mod find_bar;
mod input;
mod open;
mod select;
mod stream;
#[cfg(test)]
mod tests;

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{FocusHandle, Task};
use vessel_app_kit::DialogState;
use vessel_app_kit::ui::{App, Bounds, Context, Entity, Pixels, Window};
use vessel_app_view::layout::{ClosePlan, LayoutKey};
use vessel_app_view::terminal::{
    GridSize, Origin, ScrollbackLines, TerminalLog, TerminalModel, TerminalSender, Typed, Viewport,
};
use vessel_app_view::terminal_tabs::Trouble;
use vessel_proto::terminals::{TerminalInfo, TerminalPlace};
use vessel_proto::{EventSeq, TerminalId};

pub(crate) use body::{overlay, panel};
use draw::DrawKey;
use input::TerminalKeys;
pub(crate) use open::closes_terminal;

use crate::areas::terminal::grid::TerminalGrid;
use crate::areas::terminal::scroll::Wheel;
use crate::shell::Shell;

/// One terminal tab's machine and terminal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PaneKey {
    /// The machine, by its book name.
    pub(crate) machine: String,
    /// The terminal.
    pub(crate) terminal: TerminalId,
}

/// Where a pane's stream is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Following was asked; history is being read.
    Attaching,
    /// History drawn; live output applies as it comes.
    Live,
    /// No live screen: the red line under it says why.
    Trouble(Trouble),
    /// Its machine is not connected; it follows again when it is.
    Offline,
}

/// The cursor's blink phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Blink {
    On,
    Off,
}

/// One open terminal tab.
pub(crate) struct Pane {
    /// The terminal as last reported (a follow's start, a list).
    pub(crate) info: Option<TerminalInfo>,
    pub(crate) stage: Stage,
    /// Whether this app opened it (it answers queries until someone types).
    origin: Origin,
    /// Whether the person typed in it.
    typed: Typed,
    /// `None` while a parse runs on the background executor.
    model: Option<TerminalModel>,
    /// Which records were applied; its `next` is where a follow resumes.
    log: TerminalLog,
    /// Records before this are history (applied without answering).
    live_from: EventSeq,
    history: Vec<u8>,
    live: Vec<u8>,
    /// Identity of this pane's current follow: stale parses are dropped.
    ticket: Rc<()>,
    pub(crate) grid: Entity<TerminalGrid>,
    pub(crate) probe: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// The cells that fit the pane's box.
    fit: Option<GridSize>,
    /// The size last sent to the node.
    sent_size: Option<GridSize>,
    /// Whether the pane claimed the shell's size (ADR 0116) since it last
    /// took the keyboard or attached; until then its next size is a claim.
    claimed: bool,
    settling: Option<Task<()>>,
    drawn: Option<DrawKey>,
    blink: Blink,
    viewport: Viewport,
    wheel: Wheel,
    pub(crate) focus: FocusHandle,
    pub(crate) keys: Entity<TerminalKeys>,
    /// The IME's text being composed, drawn at the cursor.
    pub(crate) preedit: Option<String>,
    sender: TerminalSender,
    /// Its focus observers.
    observers: Vec<gpui::Subscription>,
    /// The mouse selection, with the screen and width it was made on (a
    /// width change re-wraps the text under it; another screen is other
    /// text).
    selected: Option<select::Selected>,
    /// A drag selecting: the pointer's moves extend the selection.
    selecting: bool,
    /// Find, while its bar is open.
    find: Option<find::PaneFind>,
    /// Bumped whenever the selection or find's marks change, so the frame
    /// is drawn again.
    marks: u64,
    /// The scrollback the setting asks for; set on the model when it is
    /// home from a parse.
    scrollback: ScrollbackLines,
}

/// Where a create opens its tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Creating {
    /// The machine asked.
    machine: String,
    /// The place asked for.
    place: TerminalPlace,
    /// The layout it opens in.
    workspace: LayoutKey,
    /// The New tab tab it replaces, else `None`: the focused pane.
    replaces: Option<vessel_app_view::layout::TabId>,
    /// The window's layout mode when it was asked.
    mode: vessel_app_view::layout::LayoutMode,
}

/// A `terminal.list` in flight.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Listing {
    machine: String,
    place: TerminalPlace,
    /// The revision it waits to move from; `None` answers at once.
    from: Option<u64>,
}

/// The terminal tabs' state.
pub(crate) struct TerminalTabs {
    panes: HashMap<PaneKey, Pane>,
    /// Creates sent, oldest first.
    creating: Vec<Creating>,
    /// The list asked for and not yet answered: one at a time for the
    /// shown place (a long poll for a place no longer shown is left to end).
    listing: Option<Listing>,
    /// The shown place's list revision last answered (ADR 0116): a long
    /// poll asks from it.
    revision: Option<(String, TerminalPlace, u64)>,
    /// A new terminal's pane takes the keyboard on its first frame.
    focus_next: Option<PaneKey>,
    /// A list of the open workspace's terminals, waiting for a frame (the
    /// layout's reconcile needs the window's layout mode).
    listed: Option<(LayoutKey, Vec<TerminalId>)>,
    /// The shown place's terminals as last listed: a tab the list added
    /// and nobody opened yet has no pane, and still shows its title.
    listed_infos: HashMap<TerminalId, TerminalInfo>,
    /// The shown place's live shells as its list last answered, before the
    /// shells beside the workspace's sessions are merged in (`beside`).
    place_shells: Option<(LayoutKey, Vec<TerminalInfo>)>,
    /// The shells beside the workspace's sessions (ADR 0121's native
    /// consumer).
    beside: beside::Beside,
    /// The Close asked of the person.
    confirm: Option<(LayoutKey, ClosePlan)>,
    pub(crate) dialog: DialogState,
    /// The cursor blinks in this pane only.
    blinking: Option<(PaneKey, Task<()>)>,
    /// The pane that has the keyboard (its focus observers keep it).
    focused: Option<PaneKey>,
}

impl TerminalTabs {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            panes: HashMap::new(),
            creating: Vec::new(),
            listing: None,
            revision: None,
            listed_infos: HashMap::new(),
            place_shells: None,
            beside: beside::Beside::default(),
            focus_next: None,
            listed: None,
            confirm: None,
            dialog: DialogState::new(cx),
            blinking: None,
            focused: None,
        }
    }

    /// A pane by its key.
    pub(crate) fn pane(&self, key: &PaneKey) -> Option<&Pane> {
        self.panes.get(key)
    }
}

impl Pane {
    fn new(
        origin: Origin,
        scrollback: ScrollbackLines,
        cx: &mut Context<'_, Shell>,
        key: &PaneKey,
    ) -> Self {
        use gpui::AppContext as _;
        let shell = cx.weak_entity();
        let keys_key = key.clone();
        Self {
            info: None,
            stage: Stage::Attaching,
            origin,
            typed: Typed::Never,
            model: Some(TerminalModel::new(GridSize::default(), scrollback)),
            log: TerminalLog::default(),
            live_from: EventSeq::ZERO,
            history: Vec::new(),
            live: Vec::new(),
            ticket: Rc::new(()),
            grid: cx.new(|_cx| TerminalGrid::empty()),
            probe: Rc::new(Cell::new(None)),
            fit: None,
            sent_size: None,
            claimed: false,
            settling: None,
            drawn: None,
            blink: Blink::On,
            viewport: Viewport::default(),
            wheel: Wheel::default(),
            focus: cx.focus_handle(),
            keys: cx.new(|_cx| TerminalKeys::new(shell, keys_key)),
            preedit: None,
            sender: TerminalSender::default(),
            observers: Vec::new(),
            selected: None,
            selecting: false,
            find: None,
            marks: 0,
            scrollback,
        }
    }

    /// The selection or find's marks changed: draw again.
    const fn marked(&mut self) {
        self.marks = self.marks.wrapping_add(1);
    }
}

impl Shell {
    /// The scrollback setting changed: every open terminal keeps that many
    /// lines from now on, as Paseo's desktop applies it live (a model at its
    /// parse takes it when it comes back).
    pub(crate) fn terminal_scrollback(
        &mut self,
        lines: ScrollbackLines,
        cx: &mut Context<'_, Self>,
    ) {
        let keys: Vec<PaneKey> = self.areas.terminal_tabs.panes.keys().cloned().collect();
        for key in keys {
            if let Some(pane) = self.areas.terminal_tabs.panes.get_mut(&key) {
                pane.scrollback = lines;
                if let Some(model) = pane.model.as_mut() {
                    model.set_scrollback(lines);
                }
            }
            self.terminal_tab_draw(&key, cx);
        }
        self.terminal_session_scrollback(lines, cx);
    }
}

/// The pane key of a terminal tab in `workspace`: its machine is the
/// layout's (a session's layout today).
pub(crate) fn pane_key(workspace: &LayoutKey, terminal: TerminalId) -> Option<PaneKey> {
    match workspace {
        LayoutKey::Session { machine, .. } => Some(PaneKey {
            machine: machine.as_str().to_owned(),
            terminal,
        }),
        LayoutKey::Workspace { .. } => None,
    }
}

/// A terminal tab's label: its title, else "Terminal" (Paseo's
/// `terminal.title ?? terminal.name`, its fallback word). A tab not opened
/// yet takes its title from the last list.
pub(crate) fn label(shell: &Shell, terminal: TerminalId) -> String {
    let tabs = &shell.areas.terminal_tabs;
    tabs.panes
        .iter()
        .find(|(key, _)| key.terminal == terminal)
        .and_then(|(_, pane)| pane.info.as_ref())
        .or_else(|| tabs.listed_infos.get(&terminal))
        .and_then(vessel_app_view::terminal_tabs::title)
        .map_or_else(
            || shell.words.terminal_tabs().fallback().to_owned(),
            str::to_owned,
        )
}

/// The place a workspace opens and lists its shells in (ADR 0121): from
/// protocol 40 the shown session's workspace (its worktree, its project,
/// or its folder), else beside the session
/// (`vessel_app_view::terminal_tabs::place_for`). A session the fleet read
/// does not list yet keeps the session place.
pub(crate) fn place_of(shell: &Shell, workspace: &LayoutKey) -> Option<TerminalPlace> {
    let LayoutKey::Session { machine, session } = workspace else {
        return None;
    };
    let beside = TerminalPlace::Session { session: *session };
    let Some(on) = shell
        .fleet
        .machines
        .iter()
        .find(|on| on.name == machine.as_str())
    else {
        return Some(beside);
    };
    Some(
        on.sessions
            .iter()
            .find(|card| card.session == *session)
            .map_or(beside, |card| {
                vessel_app_view::terminal_tabs::place_for(card, on.protocol)
            }),
    )
}

impl Shell {
    /// Whether a terminal tab has the keyboard (the shortcuts' terminal
    /// focus scope: Paseo's `terminal:false` shortcuts stay off).
    pub(crate) fn terminal_tab_focused(&self, window: &Window) -> bool {
        self.areas
            .terminal_tabs
            .panes
            .values()
            .any(|pane| pane.focus.is_focused(window))
    }

    /// Whether `machine` is connected.
    fn machine_online(&self, machine: &str) -> bool {
        self.fleet.online().any(|online| online.name == machine)
    }
}
