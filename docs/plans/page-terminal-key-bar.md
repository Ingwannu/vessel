# Page terminals: the virtual key bar, and long pastes (P4, page)

The native half is `docs/plans/t2-terminal-tabs.md`, section "P4: the virtual key bar (native)", on the app line.

## Paseo 0.15.4
`packages/app/src/terminal/runtime/terminal-virtual-keyboard.ts`:
- Three tables (25-82), picked by the bar's width (84-97): under 300 three columns; 660 or more and not compact one
  row; else two rows.
- Shown on its mobile app, and elsewhere only when compact, under 600 (99-104).
- Paste only on its mobile app (121-123).

`components/terminal-pane.tsx`:
- The buttons (161-257), with the keyboard toggle showing "Show keyboard" or "Hide keyboard".
- Sticky modifiers (795-830, 1052-1083), with the typed mapping in `utils/terminal-keys.ts` 204-263.
- The toggle blurs or focuses the terminal (999-1010).
- The bar under the terminal, at most 880 wide (117, 1241-1257, styles).

## Page
- `web/src/data/terminal-keys.ts`:
  - the tables and widths;
  - `encodeKey`, the bar's keys and Ctrl/Alt/Shift with characters as native's `keys.rs`, with xterm's
    `modes.applicationCursorKeysMode` for the arrows;
  - `pressKey`;
  - `typedWith`, Paseo's mapping of the next typed character.
- `web/src/features/session/mount-terminal.ts` returns `keyBar`:
  - the sticky modifiers;
  - `press`, which goes through the same path as typing and gives the terminal the focus back;
  - `toggle`;
  - `toggleKeyboard`, where the soft keyboard is shown while the terminal's textarea holds the focus;
  - a `subscribe` for `useSyncExternalStore`.
  Typed input goes through `typedWith` while a modifier is set.
- `web/src/features/session/key-bar.tsx`:
  - the bar, under both terminal views while `(width < 600px)`, its rows from its own measured width;
  - a modifier is `aria-pressed`;
  - no Paste, as Paseo's web;
  - a press keeps the focus where it is (`mousedown` prevented).
- The `keyboard-off` icon is added to the kit's set for the toggle's "Hide keyboard".

## Long pastes (the P1 review carry-over)
- The node refuses a `session.send` or `terminal.send` of more than `SendTerminal::MAX_BYTES` (64 KiB) whole.
- So `web/src/data/terminal-sender.ts` cuts every send at 64 KiB of UTF-8, at a character boundary (`cutAt`). The
  rest goes first in the next send, so nothing slips between the parts. Native's `sender.rs` does the same.

## Tests
- vitest:
  - `terminal-keys.test.ts`: rows, tables, native's encoder cases, typed mapping;
  - `terminal-sender.test.ts`: `cutAt`, a long paste in parts with the keys after it, the bound test on parts.
- e2e `terminal-key-bar.spec.ts`:
  - At 1280 there is no bar.
  - At 390:
    - Ctrl then ↑, Alt then a typed `x`, Esc and Enter send `ESC[1;5A`, `ESC x`, `ESC` and CR;
    - the toggle hides and shows the keyboard;
    - there is no Paste.
  - A 100 KiB paste reaches a real shell whole (`head -c` and `wc -c` in raw mode), every send at most 64 KiB.
