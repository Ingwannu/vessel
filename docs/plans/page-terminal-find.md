# Find in the page terminal

## Decision log

- Purpose. Search the screen and scrollback of the focused terminal.
  The page supports the workspace terminal and the session PTY through
  their shared terminal mount.
- Constraint. ANSI state, wrapping, wide cells and the alternate screen
  are already resolved by xterm.js. Searching log bytes directly would
  duplicate that logic and produce positions in the wrong coordinate space.
- Choice. Pin the official MIT-licensed `@xterm/addon-search` 0.16.0 next to
  the page's xterm.js 6.0.0. `web/package-lock.json` pins the same package
  and integrity. The adapter searches and decorates the rendered buffer,
  with a bound of 20,000 decorations. xterm's proposed API enables those
  decorations on the pinned renderer. The adapter disposes the search on close so
  live output cannot revive the previous query. It sends no search text
  to the node and requires no protocol or storage change.
- Focus. `Ctrl+F` (`⌘F` on a Mac) is captured within the terminal surface
  before xterm encodes the keystroke. The focused conversation keeps its
  own shortcut binding. Enter and Shift+Enter navigate, Escape clears the
  bar and returns focus, and the bar moves to the bottom when it would
  overlap a match. The compact page offers a touch button in the pane.
- Live output. Match borders use the current terminal palette's yellow
  and blue, with Paseo's fallback colors. While Find has matches, a buffer
  marker holds the inspected viewport through writes, including at the
  bottom. It moves with scrollback trimming. Search, navigation or close
  releases the old position so a new match can be revealed.
- Verification. Use the web gate and real-xterm browser fixtures for
  the desktop shortcut, match and focus behavior, and the compact touch
  path. The streaming browser test checks that new output reaches Find
  while the first match stays visible. The terminal input regression
  checks that search text stays in the browser.

## Paseo 0.15.4 reference

- `packages/app/src/terminal/find/index.tsx:44–122` covers pane activation,
  match counts and closing back to the terminal.
- `packages/app/src/pane-find/index.tsx:158–195,203–280` covers the input
  and navigation affordances. `pane-find/find-shortcut.ts:16–23` defines
  the platform shortcut.
- `packages/app/src/terminal/runtime/terminal-emulator-runtime.ts:270–298,353–377,488–521,596–603`
  uses xterm's buffer, bounded highlights and placement near the match.
- Native P3's behavior is in `spikes/native/app/src/areas/terminal_tabs/find.rs`
  and `find_bar.rs`.
