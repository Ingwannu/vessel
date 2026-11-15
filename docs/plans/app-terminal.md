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
`DEFAULT_FLUSH_DELAY_MS` at line 19). There is no byte ceiling in that
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
(`packages/app/src/utils/terminal-keys.ts` lines 20–47). The bytes are
its encoder's (`packages/protocol/src/terminal-key-input.ts`, read on
take-over; the table is in ADR 0073). Copy is Ctrl+C with a selection
(else the key goes to the program as an interrupt), paste Ctrl+V; on a
Mac, Cmd+V (`terminal-emulator-runtime.ts` 460–468).

**Attach and queries.** The client asks for a serialized snapshot of
the daemon's own screen (`terminal-restore-options.ts` 19–21) and
replays it in 64 KiB chunks (`server/src/terminal/terminal-replay.ts`
24). The client's xterm never answers a query (641–660) and writes a
replay with input suppressed (941).

**The pane.** A shell is a workspace tab (`panels/terminal-panel.tsx`):
its label is the terminal's title, else its name, else "Terminal"
(66–68, `en.ts` 634), its icon lucide's `Terminal` (75). New is
Cmd/Ctrl+Shift+T (`keyboard-shortcuts.ts` 844, 854); new terminals go
last (`utils/terminal-list.ts` 22). Rename is "Rename terminal"
(`en.ts` 660). Closing asks "Close terminal?" / "Any running process in
this terminal will be stopped immediately." (`en.ts` 712–713), then
kills it (`workspace-screen.tsx` 1909, 1928). Script runs are listed
apart from shells (`screens/workspace/terminals/state.ts` 75–77). The
pane is `surface0` with the output on `background`
(`terminal-pane.tsx` 1267, 1273), no padding around the grid
(`terminal-emulator.tsx` 60–63), a scrollbar 8 wide
(`terminal-emulator-runtime.ts` 570–572).

**The look.** Font: Paseo's own stack starts with JetBrains Mono
(`terminal-font.ts` 3–21) at the code font size (12 by default,
`storage.ts` 71), line height 1.0 (`terminal-emulator-runtime.ts` 564).
Cursor: a blinking bar (560–561) in the foreground colour. Palette:
`styles/theme.ts`, light 242–256 (foreground `#1a1a1e`, selection
`rgba(0,0,0,0.15)`), dark 373–386 (foreground `#fafafa`, selection
`rgba(255,255,255,0.2)`), the cursor the foreground.

**Find.** Cmd/Ctrl+F; Paseo's pane find bar at the output's right edge,
at the top, or the bottom when it would cover the selection
(`terminal/find/index.tsx` 110–136, `terminal-emulator-runtime.ts` 511),
340 wide (`pane-find/index.tsx` 360); Enter next, Shift+Enter previous,
Escape closes; its words "No matches", "{current} of {total}",
"{total} matches" (`en.ts` 20–22; Korean `ko.ts` 24–26), a `+` after
the total when limited (`terminal/find/index.tsx` 100). Matches are
outlined in the palette's yellow, the current one in its blue
(`terminal-emulator-runtime.ts` 279, 281). Next and previous wrap (the
xterm add-on's engine, `SearchEngine.ts`).

**Scrollback setting row.** A number field 112 wide, right-aligned,
digits only, kept on blur or submit (`screens/settings/terminal/terminal-section.tsx`
16, 44–65), "Terminal scrollback" / "Lines kept in the built-in terminal
buffer" (`en.ts` 2561; Korean "터미널 스크롤백" / "내장 터미널 버퍼에
보관되는 줄 수", `ko.ts` 2449), under Settings › Terminal.

## What Vessel already does

- The PTY and the ring: ADR 0031, `Scrollback::DEFAULT` (4,096 records,
  1 MiB) in `crates/vessel-session/src/terminals.rs`, reads of 16 KiB in
  `crates/vessel-pty/src/threads.rs`.
- The bytes on the wire: `TerminalBytes` (base64), `TerminalInput::{Data,
  Resize}`, `TerminalSize` default 80×24, in
  `crates/vessel-proto/src/terminal.rs`. Shells use the same input.
- The shell calls (protocol 11, `crates/vessel-proto/src/rpc.rs`
  340–356): `terminal.create` (a place: beside a session or in a
  project; an optional size and title), `terminal.list`, `terminal.get`
  (waits up to 30 s for a change), `terminal.read` (pages of at most 512
  records), `terminal.subscribe`, `terminal.send` (at most 64 KiB, the
  node refuses more), `terminal.rename`, `terminal.kill` (ends it, keeps
  its output) and `terminal.close` (ends it and forgets it). Script runs
  and services are terminals without input. The preview node speaks
  protocols 2 through 21.
- A session that is a terminal keeps its bytes in its log, stored whole;
  only a shell's ring is bounded (4,096 records, 1 MiB). No call turns a
  conversation into a terminal or back.
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
| Parser | none | Adopt `vte` 0.15 without default features (its `std` build has an unbounded OSC buffer). Reject `alacritty_terminal` 0.26 (PTY stack, `home`, Apache-2.0 only, unstable grid) and a parser written or copied here. ADR 0073. |
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
`GridSize { cols, rows }`, default 80×24, at most 1,024 × 512.
Zero on either axis is `TerminalError::Empty`, over the bounds
`TerminalError::TooLarge`, and nothing is resized; `GridSize::within`
draws a node's larger size at the bounds. History is also capped at
4 Mi cells, 80 MiB at 20 bytes a cell (lines keep no trailing blanks).
`FeedBudget` is 16 × 1024 bytes, the PTY read.

**Cells.** A cell is a scalar (or the continuation of the previous
wide scalar), a width of 0, 1 or 2, and attributes: colours as
`Default` / indexed / RGB, weight, slant, underline and polarity as
enums. A continuation is not a second character. Selection and find
skip it.

**Model.** `TerminalModel::new(size, scrollback)`. `replay(&[u8]) -> Feed` applies at most one budget and returns no
replies. History, a fold of the ring, and every viewer that must not
answer use only `replay`. `feed(&[u8]) -> Feed` is the same parse and
may return reply bytes. `query_duty(origin, focus, typed, other)` says
which call the link makes, with no `bool`: `Opened` or `Attached`,
`Held` or `Lost`, `Never` or `Once`, and whether some other viewer
already qualifies. It returns `Answer` for a viewer that has focus and
has typed, and for the view that created the terminal while nobody
qualifies (even with focus lost and nothing typed). It returns `Silent`
for every other viewer, and for an attach, or a creating view that has
gone, when nobody qualifies. `Silent` is `replay`. The caller keeps the
tail of either. `resize(GridSize)` reflows the primary screen's soft-wrapped
lines and resizes the alternate screen without merging it into
scrollback. `paste(text)` encodes bracketed paste when the program has
turned that mode on, and returns the bytes; it does not send them.
`key(Key, Mods)` does the same, in normal or application cursor mode
according to the mode the program set. `snapshot()` is a plain value
(columns, rows, visible cells, cursor, selection, a generation that
changes when the grid does). `find` / `find_next` / `find_previous` /
`clear_find`: literal, case-insensitive, at most 20,000 highlights,
`limited` at that count. Next and previous wrap at the ends (Paseo's
search engine wraps, ADR 0073).

**Replies and unknown sequences.** A sequence the fixtures do not name
is ignored. The bytes after it are still applied. A query a local
terminal answers is a reply, not a draw. The bytes are Paseo's, read
at `terminal.ts` 883–922 and the map at 45–49: `ESC [ ? 62 ; 4 ; 22 c`,
`ESC [ 0 n`, `ESC [ row ; col R`, `ESC [ ? row ; col R` (rows and
columns 1-based), and `OSC 10` / `11` / `12` with the colours the grid is drawn in
(foreground, background, cursor of the scheme shown, given to the model
as values; Paseo's fixed `rgb:e6e6/e6e6/e6e6`, `rgb:0b0b/0b0b/0b0b`,
`rgb:e6e6/e6e6/e6e6` are dark whatever its theme, a stated difference).
The link sends replies as `TerminalInput::Data`, through the existing
sender, so they stay in order with keys; every send is at most 64 KiB,
split at a character boundary. When `query_duty` is `Silent`
on every viewer, nothing is sent and the program waits. The app shows
no error. A query that already passed is not answered later.

**Path paste.** `quote_path(path, platform)` takes the node's platform.
POSIX: a path containing `` ` $ | & > ~ # ! ^ * ; < `` is
`TerminalError::Path` (the message names the rule, not the path). An
accepted path doubles backslashes, then single-quotes, or `$'...'` with
`'` as `\'` when both quote marks occur, or `\'` inside single quotes
when only `'` occurs (Paseo's three forms, `terminal-path.ts` 6–20,
read). Windows: no refusal of those characters (Paseo does not delete
them there); a space is wrapped in `"` and any other path is unchanged
(`escapeWindowsPath`, lines 22–27, read).

**What the fixtures pin** (T1 and T2, beside the model): a coloured run,
`한` as width 2 plus a continuation, the same syllable split across two
feeds, the alternate screen and back, bracketed paste on and off, an
arrow in normal and application mode, a live cursor-position query
answered with the cursor's cell, the same query through `replay`
answered with no bytes, `the_opener_answers_before_anyone_qualifies`
(device attributes, status, cursor, private cursor and the three OSC
colour queries, from the creating view, before anyone has typed and
with focus lost), and
`nobody_answers_a_live_query_when_no_viewer_may` (the same queries on
an attach with nobody qualified, no reply bytes),
`records_buffered_during_load_are_not_answered`,
`a_query_split_across_the_handover_is_not_answered`, and each refused
query of ADR 0073's closed set fed through `feed` with an empty reply.

**Untrusted input.** The parser eats the program's bytes, so every
buffer has a stated bound and no input reaches a panic. The model is
taken from the inherited draft in slices of a few hundred lines, each
reviewed (V2) and gated: the parser's state machine around `vte`; the
grid and scrollback; attributes and colours; wide and combining cells;
reflow of soft-wrapped lines on a width change (after wide cells, since
a wide pair never splits across lines; until then a resize cuts or pads
each row); the answered-query set. Each slice's tests include hostile input
(unterminated sequences, huge parameters and counts, overlong OSC and
DCS strings, scroll regions outside the grid, zero and enormous sizes)
and a fuzz-style test over random bytes with a fixed seed. T1's screen
comes after the parser and grid slices pass.

**Link (T1–T3).** A session that is a terminal is read with
`session.read` pages for its history and followed for live records; a
shell's is `terminal.read` and `follow_terminal`, on the testkit's fake,
never a real shell. Input, replies and resizes go through one
`TerminalSender` per view, each send at most 64 KiB. Create, list,
rename, Stop (kill) and close call the methods ADR 0031 already records.
This track adds no call, so no number is given at merge. A script or a
service does not take input
(`TerminalKind::takes_input`). Resize is still `ResizeRule`: focused and
typed. No claim token is added to the wire.

**Kit (T1).** A control draws the snapshot's visible cells (an element
of its own: runs shaped once per row and placed on the cell grid, rows
that did not change kept, ADR 0073). It is given a new snapshot only
when `generation` changed. The terminal palette is tokens, taken from
Paseo's terminal palette (`styles/theme.ts`, light and dark, the 16
ANSI colours, foreground, cursor, selection); the grid's 256 colours
past the 16 are the standard cube and grey ramp. The grid lives in a
view of its own (`app/src/region.rs`'s rule): its bytes notify only it.
The shell's tab strip is T3. Frames are measured (time for a changed
grid, and that an unchanged generation draws nothing), not described.

## Stated differences

- Scrollback default 10,000 (Paseo), not the page's 5,000.
- No 5 ms coalescer and no refit ladder. One feed is at most 16 KiB;
  the screen debounces resize by 100 ms.
- Scalar width, not Paseo's grapheme clusters. Hangul matches; emoji
  clusters and Indic conjuncts do not.
- A zero-width mark is composed into the cell before the cursor when
  Unicode has a composed scalar of the same width (decomposed Hangul, as
  `ls` prints file names on a macOS node, and decomposed Latin accents);
  a mark with no composed form (a Thai vowel sign, a joiner) is dropped.
  xterm.js keeps it; keeping it here would add bytes to every cell
  history holds, or a side table every edit must shift. A wide scalar
  that does not fit before the edge wraps whole, or is dropped with
  auto-wrap off or on a one-column grid (xterm.js).
- A live query is answered by the view that created the terminal until
  some viewer has focus and has typed, then only by that viewer.
  Paseo's daemon always answers, with no viewer in the decision
  (`terminal.ts` 883–922, read). An attach, or a creating view that
  has closed, with nobody qualified, is answered by nobody, and the
  program waits. The app shows no error of its own.
- A cursor report counts rows from the screen's top in origin mode
  too, as Paseo's daemon does (`buffer.cursorY`); xterm counts from the
  region. With a wrap pending it reports the last column.
- A dangerous path is refused, not stripped.
- Blink (SGR 5, 6), overline (53) and the underline's colour (58) are
  read and dropped; xterm.js draws them. Blink needs a timer over the
  cells, and an underline colour adds four bytes to every cell history
  keeps. Inverse swaps the colours first, then bold draws a foreground
  in one of the eight ANSI colours bright; faint is half ink, concealed
  is no ink: xterm.js's defaults, which Paseo keeps
  (`terminal-emulator-runtime.ts` 555–571: no
  `drawBoldTextInBrightColors`, `minimumContrastRatio: 1`), in xterm.js
  6.1.0-beta.336's order (`addon-webgl/src/TextureAtlas.ts` 519–526
  swaps, then 355 and 364–366 brighten; `DomRendererRowFactory.ts`
  347–355, then 437–438).
- The code face is the app's own Geist Mono (regular, bold, italic,
  bold italic, embedded), then Pretendard for Hangul; Paseo's stack
  starts with JetBrains Mono (`terminal-font.ts` 3–21). One code face
  across the app, and TYPE-07 (a real code face before any fallback)
  holds without a system lookup. Hangul glyphs, narrower than two of
  its cells, are drawn from the pair's left edge, leaving a gap inside
  the pair; how xterm.js places a wide fallback glyph was not read.
- Double, dotted and dashed underlines are drawn as one straight line,
  curly as a wave; xterm.js draws each style. The grid draws through
  GPUI's text runs, which have only those two.
- Under reduced motion the cursor does not blink (the app's motion
  rule); whether Paseo stops it then was not read.
- No inline images, no mouse reports, no saved shell profiles, no
  terminal-activity hooks.
- A file that is not already on the node is not uploaded. There is no
  call for it. The coordinator assigns one, or the paste does not grow
  a protocol field in this work.

## Slices (the coordinator's order, 2026-11-14)

Each a few hundred lines with tests, captures (1280×800, 1024×700,
800×600, 390 wide; both themes; en and ko) and the gate.

| Slice | What | Reads |
| --- | --- | --- |
| T1 | A session that is a terminal opens as a read-only grid: its log read by pages and replayed off the UI thread, then live records; cells, the palette's colours, wide and combining characters, Korean, the cursor, scrollback drawn only where visible. Measured: idle CPU with the terminal shown, the time of a full 200×50 repaint (release build, the renderer named). | `session.read`, `session.subscribe` |
| T2 | Typing, paste (text, a path), resize and the replies to queries, by `query_duty` and `ResizeRule`; never from `replay`. | `session.send` |
| T3 | Shells beside a session: open, list, rename, Stop, close, the tab strip as Paseo's; script runs and services listed without input. | `terminal.*` |
| T4 | Find in the terminal, and the scrollback setting's row (with E3). | none |
| T5 | Switching one session between conversation and terminal: no call does it, so not offered. | none |

## Resume point

### T1 review repair contract

V3 row 104 accepted the ownership, appearance, motion, scheduling, notice
and inspected visual evidence, but returned two dynamic scroll defects.
The next repair modifies the existing viewport and history, rather than
adding a second scroll controller. A typed history row address must pin
the inspected output as live output grows or evicts older rows. The live
position still follows output. The viewport retains the last presented
history range so pointer and touch intent can update it while the executor
owns the model; the current opening alone can apply the returned model.
An alternate screen has no scrollback and must preserve the primary anchor.

Reference reread for this repair: Paseo's runtime construction at
`terminal-emulator-runtime.ts` 557–574 and app `package.json` 81; the exact
installed `@xterm/xterm` 6.1.0-beta.336 at
`/home/dev/paseo/node_modules/@xterm/xterm/src/common/services/BufferService.ts`
99–110, 121–124 and 137–149. While user-scrolling, xterm leaves the display
line still, adjusts for evictions, and resumes following at the bottom.
Fail-first cases cover growth before the history cap, eviction at the cap,
alternate-screen output, and pointer/touch gestures in both directions
inside an outstanding live replay. The unchanged static capture matrix and
the prior native gate remain valid for their recorded sources; changed
behavior needs its own narrow checks and final package gate.

Dynamic repair `df9a934e` pins history and retains in-flight input; test-only
`84b96489` makes the new assertions exhaustive under the strict lint set.
Before repair, `scroll-anchor-red.log` has two actual failures (expected
row 2, got 3/5), and `scroll-intent-red.log` fails to show requested row 000
after live replay. After repair, view's terminal tests pass 94 plus eight
page cases; app's terminal tests pass all 15, including both input sources,
return-to-live and opening replacement. The exact final source's strict
clippy passed, then whole native run
`appconv-20261115T060251Z-84b96489` passed all six steps: 1,155 passed,
zero failed, two ignored. Logs: `/tmp/work/m12-conv-c2/rgate/dynamic-scroll/`;
start/end load 11.35/14.87. V3 re-review is still required. The accepted
static matrix remains at its recorded source; no layout, palette or grid
renderer changed in this dynamic repair, and no new capture is claimed.

The address is a physical row, not a durable logical-cell marker. Width
reflow still resolves that index in the reflowed history and clamps it;
it does not adjust the viewport around inserted/deleted reflow rows as
xterm does (`Buffer.ts` 346–365, 460–485 and 530–539, read). This remains a
stated difference: the scalar grid's reflow currently returns only the
cursor mapping, with no persistent line/cell marker mapping. Extending that
mapping belongs with the resize behavior rather than assuming a physical
row identifies a logical cell after reflow. The guarantee here covers live output,
eviction and alternate-screen switching at the existing geometry. A clear
or RIS invalidates the old history epoch rather than pinning unrelated rows.

The first screen review returned lifecycle, appearance, motion, scrolling,
scheduling and notice defects. T1 remains unaccepted until these repairs,
the native gate and the complete visual matrix are reviewed.

[Decision Log]
- Purpose and intent: one opening owns its read and replay, a quiet screen
  follows appearance immediately, and queued output never monopolizes input.
- Existing implementation and constraints: keep the existing parser, grid,
  log fold, app area and the single kit renderer; asynchronous reads outlive
  navigation and one link update may contain many small records.
- Alternatives considered: cancellation alone (cannot reject an already
  delivered result), per-event byte limits (do not bound an update), and an
  area-local notice or second renderer (duplicate shared controls).
- Choice: modify the existing assets. A typed opening identity accompanies
  the history request, its refusal and both replay completions. Compare the
  owning typed book name and opening before moving any retained state.
  Replay all output on the background executor, in order, with one model
  owner. UI updates collect pending bytes and paint completed models only.
  There is no inline parse budget to accidentally reset per record.
- Why: no dependency or wire change is needed; standard ownership and the
  existing executor remove the unbounded UI parsing path altogether.
- Advantages, costs and effects: a small echo requires an executor handover;
  queued output has zero parsing work on the UI thread. A typed bounded
  viewport exposes history by wheel/touch scrolling. Appearance identity
  includes scheme and font metrics; changes rebuild without output. Cursor
  timers exist only while presented, focused and motion is allowed, and
  cancellation restores a steady visible cursor. Failed history is one
  shared error alert while live output continues.

Reference reread: `components/terminal-emulator.tsx` 313–317 and
`terminal/runtime/terminal-emulator-runtime.ts` 980–994 invalidate theme
and refresh visible rows; 555–575 configure a blinking bar, line height
and scrollbar. The scheduling difference is deliberate: Vessel parses
on its executor, not Paseo's client xterm event loop. Reduced motion
continues to hold the cursor steady. The original font/cluster/underline
differences above remain. No new wire, protocol assignment or dependency.

The opening identity is an `Arc` ownership token alongside the existing
`NodeName` and `SessionId`, echoed by `Act::TerminalLog` and `ActAnswer`,
including link-queue refusal. It is process-local and adds no wire field.
An old token is rejected before touching model, log cursor, held output
or pending bytes. Terminal receipts are consumed before conversation
failure handling, so one failed history read has one shared error alert.

Further reference reread: `terminal/runtime/terminal-emulator-runtime.ts`
1031–1043 delegates focus and blur to xterm. Vessel ties blink work to
window presentation and terminal focus; the explicit no-blink motion
policy remains its stated difference. Wheel and recognized touch pans
use the existing snapshot viewport through a bounded `Viewport`; only
visible rows are converted and painted. A fractional gesture accumulates
until it crosses a complete cell row. A quiet appearance change checks
scheme, code size and measured cell metrics before the equal-size path.

The ignored Linux `terminal_states_capture` test uses the real shell,
link and a verified disposable fake fleet. Ready state is an ordinary
history read. Loading holds that read's completion; Unread supplies its
typed refusal to the actual handler while the real follow continues.
Those two images prove application rendering, not transport failure.
No fixture override exists in an installed build.

Repair resume: six old-code behaviour regressions failed as expected;
13 repaired terminal tests now pass, including completion order and
motion lifetime. The queue-refusal regression has also been added.
One unnecessary qualification in that new test was corrected after the
first source commit. The completion-order test now delivers both the old
history replay and old output replay on each side of the new history
handover, and verifies the log cursor too. Product code is unchanged by
this extra coverage. The queued gate was cancelled before execution so
the next whole gate includes the strengthened test.
Private captures have been inspected; the final whole native gate passed.
The burst regression now sends real ordered records through the terminal
record handler, with each payload smaller than the parser feed budget and
their aggregate larger than it. Its final screen and log cursor are checked.
The quiet-theme regression explicitly retains equal measured cell geometry.

### T1 repair evidence (2026-11-15)

The product at `2ce9d83b` is unchanged through `f7f51e06` by the subsequent
test-only strengthening. Native run `appconv-20261115T041605Z-92be8aa6` passed
all six steps: 1,150 passed, zero failed, two ignored (the explicit Linux
display fixtures). The final geometry/burst assertions also pass their
13-test area run and strict all-target app clippy. Their exact whole gate,
`appconv-20261115T045213Z-f7f51e06`, passed all six steps on `f7f51e06`:
1,150 passed, zero failed, two ignored. Logs are retained under
`/tmp/work/m12-conv-c2/rgate/repair-3/`; start/end loads were 9.90/19.21.
The earlier narrow app/link/view terminal run was 114/0/1.

The inspected matrix has 33 images: every combination of 1280×800,
1024×700, 800×600 and 390×844 with EN/KO and light/dark; 800×600 glyphs
at 1, 1.25, 1.5 and 2 in both themes (including minimum-window 1.5);
loading in wide English/dark and narrow Korean/light; four unread states
including narrow Korean; two scrolled states; and motion enabled. Eight
additional glyph crops preserve source pixels without resampling. Every
image and crop was opened for inspection. The matrix test executable's
SHA-256 is `783c5ab3dd5a628e37c685927f9744034ed5f7e5672f747245db277cbfb108cc`,
from `2ce9d83b`; per-image manifests retain source, binary and image hashes.

The error alert remains readable at 390 in Korean, with actual followed
output visible below and no duplicate conversation/foot error. Scrolling
shows early records rather than the styled live tail; the bar is outside
that viewport. Scale crops retain baseline separation, the four mono
styles, fills and wide Hangul pairs. The reference comparison uses Paseo's
`assets/store-screenshots/raw/iphone/terminal.png` plus the cited source:
edge-to-edge console, bar and terminal palette; Geist Mono/fallback gap is
already a stated difference. This is a structural comparison, not pixel
parity between different contents/chrome. The reference phone key toolbar
belongs to input, outside read-only T1.

An extra real Settings sequence retained one quiet opening through
Light → Dark → Light with motion held and no terminal output added.
Cells, background and cursor recoloured on Back to that retained route.
The test binary at `92be8aa6` has SHA-256
`238759e195901469d8ee25099489225c572bd77e4074b6a5e2c6e0af2b2bdb57`.
The disposable preference was restored afterwards.

A normal release app (no fixture overrides), at the same product source,
was also opened on the private fake terminal. Binary SHA-256:
`e17f14e177d4be79c97cd4bbb838c0e6804feee6bc566c5f7c466655410313c6`.
Held motion, 1280×800, Mesa lavapipe: 30.00095 s, process ticks
472 → 489 at 100 Hz (0.57% of one core), draw markers 13 → 13. The debug
display test's separate 10 s sample was 2.10%, draw 4 → 4; those profiles
are not interchangeable. No hardware-GPU, physical-touch, real-agent or
owner-platform result is claimed. Painted-row reuse and T2 have not begun;
T1 still awaits the final exact-commit gate and review acceptance.

- Done: ADR 0073 (status proposed) and this contract. `replay` never
  answers. Path quoting is stated for both node platforms. Scrollback
  is a field of `view::prefs::Preferences`. The view that created a
  terminal answers a live query until a viewer qualifies under
  `ResizeRule`; with no such view and nobody qualified, nobody answers
  and the program waits. The two grid tests are named above. `vte` is
  reserved to `vessel-app-view`. No settings key on the node, no new
  call, no number at merge.
- Taken over by track C on 2026-11-14 (the first author's quota ran
  out). The first author's uncommitted draft (`view/src/terminal/`:
  `types`, `grid/` with parse, edit, reflow and find, `keys`, `paste`,
  `duty`, `tests`; about 2,500 lines, in `/home/dev/vessel-gk-terminal`)
  builds and its 20 tests pass (short lane, 2026-11-14). It was never
  reviewed and the parser eats untrusted bytes, so it is suspect, not a
  head start (the coordinator): it is taken in the reviewed slices of
  "Untrusted input" above, each against this contract (find wraps;
  replies only from the closed set, in the drawn colours; the hand-over
  is `replay`; sends split at 64 KiB with the paste rules).
- P1, the parser (`view/src/terminal/parse/`), f3406a18: V2 passed the
  source (2026-11-14); its gate runs with P2. P2, the grid and
  scrollback (`view/src/terminal/grid/`, `size.rs`, `model.rs`): two
  screens, history bounded by lines and cells, the cursor and modes,
  DECSTBM, tabs, the CUx/ED/EL/IL/DL/ICH/DCH/ECH/SU/SD family, the
  alternate screen as xterm.js keeps it, resize without reflow;
  `TerminalModel::replay`. Its saved-cursor test found a P1 defect: `vte` sends a
  bare `CSI s` as one parameter of 0, so the empty-list check never
  held (fixed with a test).
- P3, attributes and colours (`grid/pen.rs`, `grid/sgr.rs`,
  `paint/`): the pen on every cell, SGR in its semicolon and colon
  forms (a malformed colour is skipped with its values, not read as
  codes), erase in the background only, DECSC keeping the pen, resize
  padding plain; `paint` resolves a pen in a `Palette` (the 16 from
  `tokens.terminal`, the cube and grey ramp past them).
- P4, wide and combining cells (`grid/width.rs`, `grid/print.rs`):
  `unicode-width` 0.2.3 as a direct `view` dependency (in the lock
  already; About lists it); a wide scalar is a `Scalar` and a
  `Continuation`, and every edit that can split a pair blanks the half
  left behind (overwrite, ECH, ICH, DCH, a narrower grid, a history line
  cut in a snapshot).
- P5, the answered set (`reply/`, `duty.rs`, `TerminalModel::feed`,
  `set_palette`): Paseo's bytes for DA1, DSR 5, CPR and its private
  form, OSC 10/11/12 in the palette shown (none until a palette is
  given); `query` matches every action with no catch-all arm and lists
  ENQ, DECRQSS, XTGETTCAP, OSC 52 `?`, DA2/DA3, DECRQM and `CSI t` as
  never answered; a reply is computed before its action applies; the
  named fixtures (`the_opener_answers_before_anyone_qualifies`,
  `nobody_answers_a_live_query_when_no_viewer_may`, the two hand-over
  tests at the model) and a flood bound (under four times the bytes,
  plus one carried answer).
- P6, reflow (`grid/reflow.rs`): a width change joins the primary
  screen's and history's soft-wrapped lines and cuts them again (a row
  ends a cell early rather than split a wide pair; a one-column grid
  blanks pairs), as xterm.js does with Paseo's options: the run of lines
  holding the cursor is only cut (`reflowCursorLine` false; the shell
  redraws it), a soft row gives up only the one blank a wide scalar
  left (an erased gap stays), rows going to history are not padded (the
  peak is history's cells and one screen). Blank rows below the cursor
  go before any line leaves for history; rows a program printed below
  the cursor that no longer fit under it are dropped. The alternate
  screen is cut or padded. While it shows, the primary's saved cursor
  is clamped, not remapped.
- V2's returns folded in before the P1–P6 gate: rows 56 (paint order,
  `CSI ? u`, one-parameter DSR and the OSC trim stated, the error
  messages and the reply bound named) and 57 (reflow's memory, the
  cursor's run, the soft trim).
- For T1: a line feed at the bottom allocates a blank row and trims the
  leaving one; a 16 KiB call of line feeds at 1,024 columns is about
  320 MiB of allocation and fill. Measure one worst-case `feed` before
  choosing the thread live bytes are applied on.
- T1, the screen (2026-11-15): the palette as `tokens.terminal`
  (`kit/src/theme/terminal.rs`, d004dfe4), the grid control
  (`kit/src/terminal_grid.rs`, 923add77: runs shaped with every glyph
  forced to the cell's width, a wide scalar a run of its own, fills
  under them, the bar cursor), the painted rows in `view`
  (`paint/rows.rs`, 6bb89631) and the app's screen
  (`app/src/areas/terminal/`). The log is read whole and replayed on
  the background executor; the follow's records are held meanwhile and
  applied after through `TerminalLog`. The original per-event inline
  cutoff was returned in review; all live bytes now replay on the
  background executor. The earlier measurements which motivated that
  original cutoff (release, a scratch bench with a full history) were:
  text costs about 3 ms per 64 KiB, line feeds a row each (16 KiB:
  6.2 ms at 80 columns, 11.8 ms at 200, 51 ms at 1,024; 64 KiB at 200:
  47 ms), so the line is drawn by line feeds, not bytes. The screen's
  box is measured on the frame after it is drawn, while the log is
  read too, so the history is replayed at its drawn width; a later
  size waits 100 ms and reflows once (a full 10,000-line history:
  5.8 ms from 80 to 200 columns, 16–17 ms from 200 to 120 or 80, on
  the UI thread). Text at the code size, as Paseo's pane passes it.
- Found by T1's first captures and fixed: GPUI's descent is negative
  (rows were four pixels short); an unlined blank took the underline of
  the word before it; a terminal session asked the node for its
  conversation and showed the refusal; only Geist Mono Regular was
  embedded, so bold and italic were drawn regular (three faces added,
  461 KB, `scripts/fonts.sh`).
- T1 measured (2026-11-15, release, Mesa lavapipe on Xvfb, a 1740×900
  window, the grid about 200×51; `VESSEL_APP_FRAME_LOG`'s draw lines
  and the process's CPU ticks): on the software renderer the app holds
  motion still, so the cursor does not blink: idle 30 s, 14 ticks
  (0.5% of a core) and no frame (an unchanged generation draws
  nothing); `flood 4000 200` (800 KB), 20 s, 207 ticks and 5 frames,
  draw 6.8 ms median. With `VESSEL_APP_REDUCE_MOTION=0`: the blink
  draws 2 frames a second, draw 3.5 ms median (p90 4.9, max 7.9) for
  the whole window with the grid rebuilt; the process's 71% of a core
  is mostly lavapipe rasterising the window (it grows with the
  pixels, frame_log's note); the flood drew 146 frames, 2.9 ms median
  (p90 4.0, max 9.2). Output no longer draws the shell (c55cd2ea).
- Captures (release): 1280×800 en dark and ko light, 1024×700 en light
  and ko dark, 800×600 en dark and ko light, 390×844 en dark and ko
  light, the sample session's colours, attributes, Hangul and cursor;
  narrower windows reflow the history, nothing cut.
- Next: a blink rebuilds every row (3.5 ms a frame above); keep the
  painted rows (`Rc`) while the generation and scheme hold and change
  only the cursor, then measure again. Then T2 (input, paste, resize,
  the query duty), T3, T4.
