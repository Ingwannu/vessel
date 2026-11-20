# Native app: shell terminals as workspace tabs (T2, packet P1)

TRACKER 481 (tab lifecycle), 482 (TerminalPane), 483 (resize and focus
claim), 338 (terminal reconciliation), 342 (terminal close confirmation),
354 (the terminal keyboard entry); catalogue F-G8, F-G42, F-G43, F-G44,
F-G49, F-G50, F-G51, F-G55 (hidden input). No node change and no new ADR:
the calls are ADR 0031's (protocol 11), the grid ADR 0073's.

## What Paseo does

Read from Paseo 0.15.4 (`packages/app/src`).

- Entry points: the new-tab launcher's Terminal
  (`workspace-tabs/launcher/index.tsx` 137), the pane header's "+" menu,
  the compact tab switcher's New terminal
  (`compact-workspace-header/tab-switcher.tsx` 348), Cmd+Shift+T on macOS
  and Ctrl+Shift+T elsewhere, the latter off inside a focused terminal
  (`keyboard/keyboard-shortcuts.ts` 841-860). New terminal is disabled
  while a create is pending (`workspace-screen.tsx` 3266
  `createTerminalDisabled`).
- Create: `screens/workspace/terminals/use-workspace-terminals.ts`
  `createMutation`; a failure is a toast of the reason; the new terminal
  opens as a tab, replacing the New tab tab it was chosen in
  (`destination {kind: "replace"}`) or in the pane.
- List: `terminals_changed` keeps the workspace's list; every standalone
  terminal (not a script's) gets a tab without taking focus, and a terminal
  that left the list loses its tab (`stores/workspace-layout-actions.ts`
  `addMissingEntityTabs`, `reconcileWorkspaceTabs`); no tab is added while
  a create is pending.
- Pane: `components/terminal-pane.tsx` 1162-1261 and its styles: "Host is
  not connected" centred; else the emulator on the background, a small
  spinner over a 16 % black scrim while attaching, a red line under it
  (`errorRow`: border on top, `surface1`, two lines) for "Terminal exited",
  "Terminal not found" or "Unable to subscribe to terminal".
- Keys: xterm types text; named keys and chords go through
  `encodeTerminalKeyInput` (`protocol/src/terminal-key-input.ts`); paste is
  Cmd+V on macOS, Ctrl+V elsewhere, bracketed when the program asked
  (`terminal/runtime/terminal-paste.ts`); input made before attach waits.
- Size: a focused pane claims its size (`terminal-pane-focus-claim.ts`),
  resizes debounced 100 ms (`TERMINAL_RESIZE_DEBOUNCE_MS`). The server keeps
  one owner per terminal (`terminal/terminal-size-ownership.ts`): a pane
  claims on focus and on attach, and its later fits are updates, dropped
  once another client claimed it.
- Reconnect: the stream controller attaches again when the host is back
  (`terminal/runtime/terminal-stream-controller.ts`), restoring from where
  the screen stands.
- Tab: label `terminal.title ?? terminal.name`, fallback "Terminal"
  (`panels/terminal-panel.tsx`); menu: Copy terminal id, Rename, the
  closes, Close.
- Close: `workspace-screen.tsx` 1904 `handleCloseTerminalTab`: "Close
  terminal?" / "Any running process in this terminal will be stopped
  immediately.", destructive Close, then the tab goes and the terminal is
  killed.

## What Vessel does now

- `view::terminal::keys` is Paseo's encoder (named keys, Ctrl and Alt
  chords, DECCKM arrows, function keys); `view::terminal::paste` its paste.
- `view::terminal_tabs`: which terminals a place shows (live shells, by
  id), the label, the red line's cases, failures as codes only.
- `link::terminals`: create, list, follow (a task per terminal, history
  paged then live), send, close, unfollow; one command and one event.
- `app::areas::terminal_tabs`: a pane per open terminal tab (the session
  screen's grid, colours, replay and wheel reused), keys through one
  `TerminalSender` (one send in flight), text and IME commits through a
  small `EntityInputHandler` entity per pane with the composing text drawn
  at the cursor, history parsed silently and live output answering
  queries per `query_duty`, the size claimed while focused (100 ms
  settle), the close confirm, the list asked for the shown workspace on
  each fleet read and reconciled into tabs (the layout written only when
  its set of terminal tabs differs).
- Several clients (ADR 0116, protocol 37; packet N1's consumer):
  - The list is a long poll: the first ask answers at once, then the screen
    asks from the revision it saw and the node answers when the list moves;
    an answer that moved is asked again at once (one in flight, so a burst
    is one refresh). An answer that did not move (the node's 30 s bound, or
    a wait it had no room for) waits for the next fleet read, so a node out
    of wait room is never asked in a loop. Below 37 the link drops the
    revision and says the node does not wait: the fleet read asks, as
    before.
  - The size: a pane claims when it takes the keyboard or attaches, even at
    the size already sent (another client may have taken the shell
    meanwhile), and sends updates for later fits while focused. A size
    replacing a queued claim stays a claim. Below 37 no intent is sent.
  - After a confirmed Close the keyboard goes to the shell the focused pane
    now shows (the dialog's opener was the closed pane).
  - A tab the list added and nobody opened yet is labelled with the title
    from the list (it had shown "Terminal" until opened); Paseo labels every
    listed tab.
- The launch table enables Terminal; the New tab panel passes its tab to
  replace; the compact switcher's New terminal opens one; Ctrl+Shift+T goes
  through the layout's shortcut handler; one create at a time (a press
  while one waits does nothing, the switcher's button is disabled); a
  single terminal tab's Close (menu, Ctrl+W) asks first.
- A follow lost to a dropped connection marks the tab offline (not a
  terminal error); each fleet read follows such tabs again once their
  machine reads online, from where their log stands, and the size is
  claimed again.

## Stated differences

- The shell opens beside the shown session (`TerminalPlace::Session`, in
  its folder): a workspace layout is a session's today.
- The list is a long poll, not a push (ADR 0116): a shell another client
  opened shows one round trip after the change. Below protocol 37 it shows
  within one fleet read (2 s).
- Size ownership is per connection, not per window (ADR 0116): the app's
  windows share one connection per machine, so they still share an owner.
- The label is a rename's title or "Terminal", never the shell's own (no
  OSC title on the node; a later node packet).
- A create's failure says "Failed to create terminal" (the machine's words
  are not shown), where Paseo toasts its reason.
- "Terminal exited" and "Terminal not found" are translated in Korean;
  Paseo shows them in English in every language.
- Close left/right/others stay disabled when they would close a terminal,
  even when it is the only tab they close (Paseo's bulk confirm, "Close
  tabs to the right?" with a count, is not drawn).
- No kitty keyboard or win32 input mode (the grid does not track them): a
  modified Enter is a carriage return.
- Not in this packet: the key bar (P4), profiles (P5, ADR 0117), attention
  (P6), file drop and image paste (P7), terminals that survive a node
  restart.

## Checks

- View: the encoder and paste tests; the place filter, label and failure
  tests.
- Link: failures cross as codes only.
- App: a create replaces the New tab tab and follows; a failed create says
  so; keys and a paste wait in order until live and then one send at a
  time; Close asks, Cancel keeps, Close removes the tab and closes on the
  node; a list adds the place's shells and drops an ended one; a focused
  tab is the shortcuts' terminal scope; only a terminal tab's own Close
  asks to close it (a bulk close stays refused); a second create waits for the
  first; a tab whose connection dropped follows again once its machine is
  back; a node that waits is asked again only when the list moved; a pane
  claims its size on focus and updates it after; a confirmed Close hands the
  keyboard to the shell now shown; a listed tab not yet opened shows its
  title.
- Unit tests start no shell (the link is a channel). Captures run
  `scripts/fleet.sh`, whose nodes start under `env -i` with a scratch home:
  the shell is the account's program by ADR 0031's detection (no `SHELL`),
  reading no personal rc file and holding no credential from the
  environment; only harmless commands are typed.

## P3: find, selection and copy, scrollback (native)

Paseo's web and desktop terminal (xterm.js) is the reference; its mobile
grid (long press, grips, floating Copy and Paste) is the phone app's later
(coordinator ruling, P3).

- Find (`pane-find/*`, `terminal/find/index.tsx`, runtime 270-520): Mod+F
  (Cmd on macOS, Ctrl elsewhere; not with Shift or Alt) while a terminal tab
  has the keyboard opens the bar; anywhere else Mod+F is the conversation's
  find. The bar is 340 wide over the pane's top right, a field ("Find in
  pane") with the counter inside ("No matches", "N matches", "x of y", the
  total with a "+" at 20,000), previous, next and "Close Find". Enter goes
  to the next match, Shift+Enter to the previous, Escape closes and gives
  the terminal the keyboard back. Literal, case-insensitive text across soft
  wraps, matches not overlapping; other matches bordered in the terminal
  theme's yellow, the current in its blue; the view scrolls to the current
  one, and the bar moves to the bottom when it would cover it. The search
  runs a slice of rows a step (the first at once), and again after new
  output or a width change, keeping the current match on its text.
- Selection: a drag selects cells, a double click a word (xterm.js's word
  separators), a triple click a line, Shift+click extends; past the top or
  bottom edge a drag scrolls. Typing, a paste, a plain click, a width change
  or the other screen clears it. Drawn with the theme's selection colour.
- Copy: Cmd+C on macOS; Ctrl+C elsewhere copies a selection and is the
  shell's ^C without one (`handleCtrlClipboardShortcut`). No toast.
- Paste (P1 review note 2): queued whole and sent in parts of at most the
  node's 64 KiB a send, cut between characters; keys typed during it go
  after it. Pastes waiting are bounded at 32 MiB, past which a paste is
  refused whole.
- Settings > Terminal: "Terminal scrollback" (0 through 1,000,000, default
  10,000), a number field that commits on Enter or when it loses the
  keyboard; open tabs and the session terminal screen take it at once.
  It is stored in the native preferences as a `terminal` section written
  only once it differs from the default, so the default document stays the
  page's contract; a stored value is read as Paseo's schema reads it
  (clamped, a fraction floored, anything else the default).
- Accessibility (P1 review note 1): the tab's keyboard target is a text
  input named "Terminal input"; the counter is a status named "Find
  matches".
- Stated differences: Paseo's find anchor keeps the inspected line still
  under new output, here a match that output scrolled away is brought back
  when the search runs again; Paseo's scrollback field drops a non-digit as
  it is typed, here at commit; no context menu (Paseo desktop's is its
  Electron shell's); size marks of the selection follow the grid, not the
  pixel.
- Checks: view tests for text addresses, copy, word and line, wide cells,
  find rules and the limit, scrollback trim, the sender's parts and paste
  bound; app tests for Mod+F on both sides, find's counter and moves,
  Ctrl+C with and without a selection, typing clearing it, a paste past
  the room before a key and ^C, and the setting on an open tab.
