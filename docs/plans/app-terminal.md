# Native app: terminals

The native app draws terminals. A session that is itself a terminal, and
a shell beside a session: create, rename, kill, scrollback, find, and
paste of text, of a path and of an image's path. ADR 0073 is the
decision; this file is the contract and the resume point. No new wire.

## What Paseo does

Read from Paseo 0.15.4. Paths are inside that tree.

**Two terminals.** The agent's own terminal is the session. A shell is a
separate tab in the workspace (`packages/app/src/screens/workspace/terminals/use-workspace-terminals.ts`
creates and kills; `packages/protocol/src/messages.ts` `create_terminal_request`,
`terminal.rename.request`, `kill_terminal_request`, `subscribe_terminal_request`,
`terminal_input`). Create takes an optional size; without one the daemon
starts at 24×80 and the first resize corrects it (the comment on
`CreateTerminalRequestSchema`). The daemon's own headless buffer is
created at `packages/server/src/terminal/terminal.ts` line 799 with
`scrollback: 10000`, and the default rows and columns are 24 and 80 at
lines 759–760.

**Scrollback.** A setting, `terminalScrollbackLines`, default 10,000,
minimum 0, maximum 1,000,000
(`packages/app/src/hooks/use-settings/storage.ts` lines 54–56, applied
at 227–230 and clamped again at 491–492). The English label is
"Terminal scrollback", "Lines kept in the built-in terminal buffer"
(`packages/app/src/i18n/resources/en.ts` around the `terminalScrollback`
entry). The pane passes it into the emulator
(`packages/app/src/components/terminal-pane.tsx` lines 599 and 1183).

**Output coalescing.** The daemon flushes PTY output on a 5 ms leading
edge, then a trailing timer of the same delay
(`packages/server/src/terminal/terminal-output-coalescer.ts`,
`DEFAULT_FLUSH_DELAY_MS` at line 18). There is no byte ceiling in that
coalescer.

**Resize.** The pane debounces 100 ms
(`TERMINAL_RESIZE_DEBOUNCE_MS` at `terminal-pane.tsx` line 116) through
`createTerminalResizeDebouncer` (`packages/app/src/components/terminal-resize-debouncer.ts`):
while a resize waits, `shouldClaim` and `forceClaim` are or-ed, not
replaced. A keyboard refit also fires at 0, 48, 144 and 320 ms
(`TERMINAL_REFIT_DELAYS_MS`, line 115), and the emulator runtime waits
again at 0, 16, 48, 120, 250, 500, 1,000 and 2,000 ms
(`FIT_TIMEOUT_DELAYS_MS` in `terminal-emulator-runtime.ts` line 159).
Size ownership (`packages/server/src/terminal/terminal-size-ownership.ts`):
an `update` from anyone but the current owner is ignored; a `claim`
becomes the owner; the default intent for an older client is `claim`.
The client policy is `packages/app/src/terminal/native-renderer/terminal-resize-policy.ts`.

**The grid.** xterm, headless on the daemon, with
`@xterm/addon-unicode-graphemes` (`packages/server/src/terminal/headless-terminal.ts`,
the whole file). Paseo's rendering note
(`docs/terminal-rendering.md` lines 17–25): every emoji cluster is one
cell of width 2 (VS16, skin tone, flag, ZWJ), and an Indic conjunct is
one column. Clients re-parse the daemon's ANSI with the same table,
because the daemon answers cursor-position reports from its buffer.
Continuation cells of a wide character have width 0
(`packages/app/src/terminal/native-renderer/terminal-frame.ts`).

**Find.** `@xterm/addon-search` with `highlightLimit` 20,000
(`FIND_HIGHLIGHT_LIMIT`, `terminal-emulator-runtime.ts` line 53). Search
is not a regular expression, not case-sensitive, incremental when no
direction is given, and next/previous otherwise (lines 273–286). The
result reports `limited` when the count is at the cap (line 516).

**Paste.** Bracketed paste wraps with `ESC [ 200 ~` and `ESC [ 201 ~`,
and a payload that already contains the end sequence loses the ESC
(`packages/app/src/terminal/runtime/terminal-paste.ts` lines 18–28). A
dropped file pastes its path when the file is on the node, and is
uploaded otherwise; the type does not matter
(`packages/app/src/terminal/drop/terminal-file-drop.ts` lines 46–63).
An image with no accompanying text is a file named `clipboard.<ext>`
(`imageExtension` in `packages/app/src/terminal/file-paste/index.ts`
lines 128–140). The upload limit is 50 MiB
(`packages/app/src/attachments/upload-limit.ts` line 2). The pasted path
is quoted, and on a non-Windows path the characters
`` ` $ | & > ~ # ! ^ * ; < `` are deleted first
(`packages/app/src/terminal/drop/terminal-path.ts`).

**Keys.** The DOM names Paseo treats as keys are Enter, Tab, Backspace,
Escape, the arrows, Home, End, Insert, Delete, Page Up/Down and F1–F12
(`packages/app/src/utils/terminal-keys.ts` lines 20–47). The encoding
that reaches the PTY is xterm's, not a table in that file.

## What Vessel already does

- The PTY and the ring: ADR 0031, `Scrollback::DEFAULT` (4,096 records,
  1 MiB) in `crates/vessel-session/src/terminals.rs`, reads of 16 KiB in
  `crates/vessel-pty/src/threads.rs`.
- The bytes on the wire: `TerminalBytes` (base64), `TerminalInput::{Data,
  Resize}`, `TerminalSize` default 80×24, in
  `crates/vessel-proto/src/terminal.rs`. Shells use the same input.
- The page: xterm.js, scrollback hardcoded to 5,000, size sent only
  after focus and a key (`web/src/features/session/terminal-view.tsx`).
- The app, already: `view/src/terminal/frame.rs` (`ResizeRule`,
  `TerminalLog`) and `sender.rs` (`TerminalSender`, `MAX_PENDING`).
  `link/src/session.rs` `Act::Terminal` turns that into `session.send`.
  `vessel-client` already has `follow_terminal`. The kit has no terminal
  palette and no cell control.

## Existing asset, reuse, alternatives

| Piece | Already there | Decision |
| --- | --- | --- |
| Ordered keys and sizes, "typed and focused" resize | `view::terminal` | Modify nothing in those two files unless a test forces it. The grid sits beside them. |
| Session follow and `Act::Terminal` | `link` | Keep. The grid consumes `TerminalLog`'s slices. |
| Shell create, rename, kill, follow, send | proto and `vessel-client` | Extend `link` with commands that call them. No new crate, no new method number. |
| PTY | `vessel-pty` | Not used by the app. |
| Page emulator | xterm.js | Not shareable. Its resize rule is already ported. |
| Parser | none | Adopt `vte` 0.15, `std` only. Reject `alacritty_terminal` 0.26 (PTY stack, `home`, Apache-2.0 only, unstable grid) and a parser written or copied here. ADR 0073. |
| Scalar width | `unicode-width` 0.2.3 already in the native lock | Adopt as a direct dependency of `view`. Not an exclusive owner. |
| Renderer | none | New kit control, later. It takes a snapshot; it does not import `view`. |

`alacritty_terminal` 0.26.0 was read as crate metadata (2026-04-06,
411,445 bytes, 8,625 lines, Apache-2.0, dependencies including
`rustix-openpty`, `polling`, `signal-hook`, `home`, `vte`). `vte` 0.15.0
(2025-02-02, 57,487 bytes, 2,942 lines, Apache-2.0 OR MIT, `memchr` via
`std`). `unicode-width` 0.2.3 (2026-10-28, 209,503 bytes, MIT OR
Apache-2.0).

## Contract

Types cross a module boundary as types. No `bool` parameter. No string
match to decide a mode. `vte`'s callbacks are the decoding edge; past
them the model holds enums.

**Counts.** `ScrollbackLines`, 0 through 1,000,000, default 10,000.
`CellCount`, a non-zero `u16`. `GridSize { cols, rows }`, default 80×24.
Zero on either axis is `GridError::Empty`, and nothing is resized.
`FeedBudget` is 16 × 1024 bytes, the PTY read.

**Cells.** A cell is a scalar (or the continuation of the previous
wide scalar), a width of 0, 1 or 2, and attributes: colours as
`Default` / indexed / RGB, weight, slant, underline and polarity as
enums. A continuation is not a second character. Selection and find
skip it.

**Model.** `TerminalModel::new(size, scrollback)`. `feed(&[u8]) -> Feed`
applies at most one budget and returns `consumed` plus any reply bytes
(cursor position, primary device attributes). The caller keeps the
tail. `resize(GridSize)` reflows the primary screen's soft-wrapped
lines and resizes the alternate screen without merging it into
scrollback. `paste(text)` encodes bracketed paste when the program has
turned that mode on, and returns the bytes; it does not send them.
`key(Key, Mods)` does the same, in normal or application cursor mode
according to the mode the program set. `snapshot()` is a plain value
(columns, rows, visible cells, cursor, selection, a generation that
changes when the grid does). `find` / `find_next` / `find_previous` /
`clear_find`: literal, case-insensitive, at most 20,000 highlights,
`limited` when the count is past that.

**Replies and unknown sequences.** A sequence the fixtures do not name
is ignored. The bytes after it are still applied. A query a local
terminal answers is a reply, not a draw. The link sends replies as
`TerminalInput::Data`, through the existing sender, so they stay in
order with keys.

**Path paste.** `quote_path` produces the bytes Paseo would paste for a
path that contains none of `` ` $ | & > ~ # ! ^ * ; < ``. A path that
contains one is `PathError::Refused`, naming the path only by the fact
of the refusal (the message must not need the path to be useful, and
must not echo a secret; a path is not a secret, but the error names the
rule, not a rewritten path). Windows-shaped paths (`A:\`, `\\`) are
quoted Paseo's way; the shape is external text, decoded once.

**What the fixtures pin** (slice 2, beside the model): a coloured run,
`한` as width 2 plus a continuation, the same syllable split across two
feeds, the alternate screen and back, bracketed paste on and off, an
arrow in normal and application mode, a cursor-position query answered
with the cursor's cell.

**Link (slice 3).** A shell's follow is `follow_terminal` on the testkit's
fake, never a real shell. Input, replies and resizes go through one
`TerminalSender` per view. Create, rename and kill call the existing
protocol 11 methods. A script or a service does not take input
(`TerminalKind::takes_input`). Resize is still `ResizeRule`: focused and
typed. No claim token is added to the wire.

**Kit (slice 4).** A control draws the snapshot's cells. It is given a
new snapshot only when `generation` changed. The terminal palette is
tokens, taken from the page's `--term-bg`, `--term-fg`, `--term-cursor`,
`--term-selection` and the ANSI colours beside them. The session area
and the shell's tab strip are other work: the tab is not mounted until
the coordinator names the line. Frames are measured (time for a changed
grid, and that an unchanged generation draws nothing), not described.

## Stated differences

- Scrollback default 10,000 (Paseo), not the page's 5,000.
- No 5 ms coalescer and no refit ladder. One feed is at most 16 KiB;
  the screen debounces resize by 100 ms.
- Scalar width, not Paseo's grapheme clusters. Hangul matches; emoji
  clusters and Indic conjuncts do not.
- A dangerous path is refused, not stripped.
- No inline images, no mouse reports, no saved shell profiles, no
  terminal-activity hooks.
- A file that is not already on the node is not uploaded. There is no
  call for it. The coordinator assigns one, or the paste does not grow
  a protocol field in this work.

## Resume point

- Done: ADR 0073 and this contract. `vte` is reserved to
  `vessel-app-view` in `spikes/native/README.md`. No code, no dependency
  added yet.
- Half-done: nothing.
- Next: the grid in `view/src/terminal/`, `vte` 0.15 (`std` only) and
  `unicode-width` 0.2.3 (default features) on that crate, fixtures as
  above, then the native gate. Do not start `link` or `kit` in that
  commit.
