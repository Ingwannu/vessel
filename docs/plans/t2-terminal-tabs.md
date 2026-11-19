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
  resizes debounced 100 ms (`TERMINAL_RESIZE_DEBOUNCE_MS`).
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
- A shell another client opened shows within one fleet read (2 s), not at
  once: the node pushes no list until ADR 0116 (`terminal.watch`).
- Another viewer's resize is not refused: no claim/update on the wire until
  ADR 0116.
- The label is a rename's title or "Terminal", never the shell's own (no
  OSC title on the node; a later node packet).
- A create's failure says "Failed to create terminal" (the machine's words
  are not shown), where Paseo toasts its reason.
- "Terminal exited" and "Terminal not found" are translated in Korean;
  Paseo shows them in English in every language.
- Close left/right/others stay disabled when they would close a terminal
  (Paseo's bulk confirm is not drawn).
- No kitty keyboard or win32 input mode (the grid does not track them): a
  modified Enter is a carriage return.
- Not in this packet: selection, copy and find (P3), the key bar (P4),
  profiles (P5, ADR 0117), attention (P6), file drop and image paste (P7),
  terminals that survive a node restart.

## Checks

- View: the encoder and paste tests; the place filter, label and failure
  tests.
- Link: failures cross as codes only.
- App: a create replaces the New tab tab and follows; a failed create says
  so; keys and a paste wait in order until live and then one send at a
  time; Close asks, Cancel keeps, Close removes the tab and closes on the
  node; a list adds the place's shells and drops an ended one; a focused
  tab is the shortcuts' terminal scope; a second create waits for the
  first; a tab whose connection dropped follows again once its machine is
  back.
- Unit tests start no shell (the link is a channel). Captures run
  `scripts/fleet.sh`, whose nodes start under `env -i` with a scratch home:
  the shell is the account's program by ADR 0031's detection (no `SHELL`),
  reading no personal rc file and holding no credential from the
  environment; only harmless commands are typed.
