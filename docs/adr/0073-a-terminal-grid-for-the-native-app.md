# ADR 0073: A terminal grid for the native app

Status: proposed, 2026-11-14 (milestone 12, the native app,
`docs/plans/app-terminal.md`). Taken over by track C the same day: the
corrections of that reading (the node's other terminal calls, Paseo's
attach, find, keys and colours read from its source, where the grid is
drawn) are folded into the text below. No new wire and no number to be given at
merge: a session's bytes stay `TerminalInput` / `SessionEvent::Terminal`
(ADR 0003), and a shell uses the calls ADR 0031 already records. The
parser is `vte` 0.15, reserved to `vessel-app-view` in
`spikes/native/README.md`.

## Purpose and intent

The native app draws no terminal. The page draws one with xterm.js, and
only for a session that is itself a terminal. Paseo also draws shells
beside a session: create, rename, kill, a scrollback of 10,000 lines
unless the person changed it, find in that scrollback, and a paste of
text, of a file's path and of a clipboard image's path. The app has to
do those from the bytes the node already forwards. It must not grow a
second PTY, and it must not copy another editor's terminal.

## Existing implementation and constraints

The node already runs the PTY (`vessel-pty`, `portable-pty`) and keeps
each terminal's log in memory: 4,096 records and 1 MiB
(`vessel_session::terminals::Scrollback::DEFAULT`, ADR 0031). One read
is at most 16 KiB (`vessel_pty::threads::READ_BUFFER`). A shell is a
`TerminalId` with the session's terminal transport: `terminal.subscribe`
/ `terminal.read` (pages of at most 512 records) / `terminal.send` (at
most 64 KiB a call, `SendTerminal::MAX_BYTES`, refused above it in
`vessel-node/src/service/terminals.rs`), plus `terminal.create`,
`terminal.list`, `terminal.get` (which waits up to 30 s for a change),
`terminal.rename`, `terminal.kill` (ends the process, keeps the terminal
and its output) and `terminal.close` (ends it if it runs, and forgets
it), all protocol 11. A project's script run and service are terminals
too, without input (`TerminalKind::takes_input`). The client crate
already follows one (`follow_terminal`). Nothing in that path parses the
bytes. A session that is itself a terminal keeps its bytes in its log,
which the node stores whole (`vessel-session/src/ring.rs`: a session's
ring is a cache; only a shell's ring is bounded by bytes), so its
history can be far larger than a shell's 1 MiB. No call turns a
conversation into a terminal or back.

Paseo's daemon parses and keeps a screen of its own: on attach a client
asks for a serialized snapshot (`terminal-restore-options.ts` 19–21,
`mode: "visible-snapshot"`, with the scrollback setting's line count) and
gets it in chunks of 64 KiB (`server/src/terminal/terminal-replay.ts`
24). Its client xterm draws and never answers a query
(`terminal-emulator-runtime.ts` 641–660: CSI `c`, `n`, `R`, DECRQM and
OSC 10–12 are swallowed; 941 writes a replay with input suppressed).
Vessel's node keeps raw bytes only, so the app replays those.

The page's view (`web/src/features/session/terminal-view.tsx`) replays
those bytes into xterm.js with a hardcoded scrollback of 5,000 lines,
and sends a size only after the view has focus and the person has typed.
`view::terminal` already has that rule (`ResizeRule`), the ordered
sender (`TerminalSender`, one send in flight, at most 1,048,576
characters waiting) and the fold of a session log into byte slices
(`TerminalLog`). `link` already delivers a session's keys and sizes
(`Act::Terminal`). There is no grid, no shell follow in `link`, and no
cell drawing in the kit.

`view` does no I/O and no UI, and nothing of GPUI reaches it (ADR 0035).
`kit` does not know sessions. So the grid cannot live in `kit`, and the
renderer cannot import `view`. The app is the only crate that sees both.
Library code takes no `unsafe` (the workspace forbids it); a dependency
may, inside itself. A new dependency is named here with its licence, its
size and why the standard library does not do the job. No call and no
field are added, so there is no number to give at merge. Mouse reports,
resize intent and a file upload are not added either.

This ADR adds no `node.settings` key. Paseo keeps the scrollback count
in the app, under Settings › General, not on the daemon (`storage.ts`
lines 54–56, read: default 10,000, 0 through 1,000,000). In Vessel it is
one field of `view::prefs::Preferences`, per device, default
`ScrollbackLines::DEFAULT`, and a missing field is that default. The
Settings row that edits it is Paseo's (a number field 112 wide, digits
only, kept on blur; "Terminal scrollback" / "Lines kept in the built-in
terminal buffer", `terminal-section.tsx` 44–65, `en.ts` 2561); it comes
with find (slice T4), drawn with track E3, who owns Settings.

The grid reaches nothing outside the process it is called in. It does
not start a program, listen, or read a credential. The bytes are the
ones the link already received. A path paste returns text and does not
upload. Tests are `#[cfg(test)]` byte fixtures; no fake answers in a
product build. This track does not use signals, systemd or launchd, so
there is no Windows path to add for them. Quoting a Windows-shaped path
is string shaping only.

## Alternatives considered

**Who parses.**

1. `alacritty_terminal` 0.26.0 (published 2026-04-06, 411,445 bytes,
   8,625 lines of Rust, licence Apache-2.0 only, rustc 1.85, edition
   2024). It is a terminal library, and it already contains `vte`. It
   also contains a PTY: `rustix-openpty`, `polling`, `signal-hook`,
   `home`, `libc`, and on Windows `miow`, `piper` and `windows-sys`,
   plus `base64`, `parking_lot`, `regex-automata` and `unicode-width`.
   The app has no PTY to open. The crate's public grid is Alacritty's
   release surface (0.24, 0.25 and 0.26 each moved it), so `kit` would
   still need a snapshot of our own or a dependency on that surface.
   `home` reads a home directory, which library code here does not.
2. `vte` 0.15.0 (published 2025-02-02, 57,487 bytes, 2,942 lines,
   licence Apache-2.0 OR MIT, rustc 1.62.1). A parser only: `Parser::advance`
   and a `Perform` implementation. Default feature `std` pulls `memchr`.
   The `ansi` feature (and `serde`) is not taken. Maintained by
   Alacritty's publishers; 0.15.0 is the current release.
3. A parser written here, or copied from Alacritty, xterm.js or WezTerm.
   The state machine is Paul Williams' ANSI parser plus UTF-8 and the
   OSC exceptions `vte` already documents. Copying an editor's terminal
   is refused. Writing the same machine is a second parser to maintain,
   and the standard library has none.

**Who owns the grid.**

1. `alacritty_terminal`'s grid, wrapped. Rejected with option 1: the PTY
   stack comes with it, and the kit still cannot name those types.
2. The grid in `kit`, the parser beside the drawing. Rejected: `kit`
   must not know a session, a mode or a key sequence, and a test of
   Korean width should not start GPUI.
3. The grid and the key encoding in `view`, a snapshot the app copies
   into a kit control that only draws cells. This matches the crate
   table.

**Widths.**

1. `unicode-width` 0.2.3 (published 2026-10-28, 209,503 bytes, the UAX
   #11 tables, licence MIT OR Apache-2.0). Already in the native lock
   through `gpui-component`. Default feature `cjk`. A Hangul syllable
   is width 2. It measures scalars, not grapheme clusters.
2. Paseo's `@xterm/addon-unicode-graphemes` table, copied. Refused: it
   is another terminal's data. No dual-licensed crate in the lock does
   that table's job (one emoji cluster, one Indic conjunct, one cell).

## The choice

`vte` 0.15, without default features, is the parser, and only
`vessel-app-view` may depend on it. Its `std` build keeps an OSC string
in a `Vec` that grows for as long as the program writes one (`lib.rs`
63–64, 547–551); without `std` the parser's own buffer is an `ArrayVec`
of 1,024 bytes and bytes past it are dropped, so an endless OSC costs no
memory (`view::terminal::parse`, `OSC_LIMIT`; `memchr` and `arrayvec`
stay, they are not optional). The grid is new code in `view`:
bytes in, cells out, keys and pastes back to bytes. No source from
Alacritty, xterm or WezTerm is copied in. `unicode-width` 0.2.3, with
its default `cjk` feature, is the width of a scalar; it is not given an
exclusive owner, because `gpui-component` already uses it. Both
dependencies land in the grid's commit, not in this one; the README
reserves `vte` now so that commit is the one the boundary check allows.

The split:

- `view` holds the model and the encoding. It does not draw, and it
  does not read the network, the disk or the clock. The existing
  sender, resize rule and session-log fold stay; the grid is a module
  beside them.
- `kit` gains a control that draws a grid of cells it is given. It does
  not parse, and it does not name a session or a node. It is a GPUI
  element of its own: for each visible row, runs of cells with one style
  are shaped once and placed on the cell grid (a glyph's x is its cell's,
  not the font's advance, so a Hangul syllable from Pretendard still
  covers exactly two cells); backgrounds are merged quads; a row whose
  cells did not change keeps its shaped line. The face is the kit's code
  face (a real code font, then Pretendard, then monospace: TYPE-07, no
  new font), at the code size with a line height of 1.0 (Paseo's
  `lineHeight: 1.0`, `terminal-emulator-runtime.ts` 564). The cursor is
  Paseo's bar (`cursorStyle: "bar"`, `cursorBlink: true`, 560–561),
  blinking only while the terminal has focus. Colours are the kit's
  tokens, taken from Paseo's terminal palette (`styles/theme.ts`:
  `lightTerminalAnsi` 242–256, `darkTerminalAnsi` 373–386, foreground,
  cursor and selection beside them), never a hex literal in a screen.
- The grid is drawn as a view of its own (an entity, drawn with the
  kit's `cached_view`), the rule `app/src/region.rs` and the README's
  "Rules every screen follows" set for heavy parts: a terminal's bytes
  notify that view only, never the shell, so output does not draw the
  sidebar, the conversation or the composer again.
- `link` follows a shell with the client crate's existing
  `follow_terminal` and sends input with the existing `terminal.send`.
  A session that is a terminal keeps `Act::Terminal`. The same grid
  consumes both streams.
- `app` copies a changed snapshot into the kit control. It is the only
  crate that sees both types, because neither may depend on the other.

Bounds:

- Scrollback is that preferences field: default 10,000, allowed 0
  through 1,000,000. Lines are kept as output arrives; the oldest line
  is dropped when the count is passed, which is the bound, not a
  discarded command. History is also bounded by cells: a line keeps no
  trailing unprinted cells, and past 4 Mi cells in all the oldest line
  goes, whichever bound comes first (at the setting's maximum and the
  grid's widest, lines alone would be a billion cells). A cell is 20
  bytes (a glyph, two colours, six attributes; a test pins it), so
  history's cells are at most 80 MiB; each line also costs a 32-byte
  header, about 32 MiB more at the 1,000,000-line setting, so about
  112 MiB at most. At the default a line would have to average over 419
  printed cells for the cell bound to come first. A reflow allocates no
  more than history's own cells and one screen (rows going to history
  are not padded). The node's ring is unchanged (4,096 records, 1
  MiB): a reconnect cannot show bytes the node has already dropped.
- One feed applies at most 16 KiB, the PTY's read size, and reports how
  many bytes it consumed. The rest stays with the caller. The app replays
  live output on the background executor too: no event, including a small
  event in a large queued batch, parses bytes on the UI thread. One model
  is in flight; output arriving meanwhile waits in order for its return.
  A completion paints once, rather than painting after every queued record.
  The cost is an executor handover even for a small echo.
- A scrolled viewport pins a typed physical history-row address, including
  the history epoch and front evictions. Live output leaves that address
  still until it is evicted; then the viewport clamps to the oldest row.
  Zero follows output. Alternate output has no history and preserves the
  primary anchor. This follows the pinned xterm's `BufferService.ts`
  99–149. The last presented range also accepts bounded wheel/pan intent
  while the executor owns the model, so parsing cannot drop gestures.
  Opening replacement discards that range and intent. History clear and
  RIS change the address epoch. These are changes to the existing history
  and viewport, with constant-sized metadata and no second renderer or
  input queue. Width reflow still resolves the physical row index in the
  resulting history; its logical-position mapping remains a stated
  difference in the terminal plan, because reflow currently maps only the
  cursor. This guarantee concerns live output at existing geometry.
- History is not fed frame by frame. On open, the terminal's ring (a
  shell) or its log (a session that is a terminal, which can be many
  megabytes) is read page by page (`terminal.read` / `session.read`, 512
  records) and replayed into a model on a background executor; the
  screen shows a loading row until that model is handed over, and live
  records that arrived meanwhile follow it. The model is plain data, so
  it can be built off the UI thread; `view` still does no I/O (the app
  reads, `view` parses).
- The empty size is refused, and so is a size over 1,024 columns or
  512 rows (`TerminalError::TooLarge`): two screens are kept, so the
  largest size bounds a terminal's memory before any byte is read (a 4K
  screen at the smallest code face is about 550 × 150). A size the node
  reports past the bounds (the wire holds `u16`) is drawn at the bounds
  (`GridSize::within`); the wire's 80×24 when nobody has resized is a
  size. Resize of
  the model is immediate. The screen waits 100 ms of an unchanged
  measurement before asking, and only a view that has focus and has
  typed sends it (`ResizeRule`, already).

What the model answers, in the grid's commit, is fixed by recorded
bytes rather than by a second copy of xterm's tables: SGR colour,
a Hangul syllable of width 2 and its continuation cell, the alternate
screen, bracketed paste, and a UTF-8 sequence split across two feeds.
An unknown sequence is ignored and the rest of the feed is kept (that
is the parser's rule, not a dropped command).

Who answers a query is a difference from Paseo. Paseo's daemon writes
the reply itself, and no viewer is part of that decision
(`packages/server/src/terminal/terminal.ts` lines 883–922, read). A
primary-device query (`CSI c` or `CSI 0 c`) becomes
`ESC [ ? 62 ; 4 ; 22 c`. A status query (`CSI 5 n`) becomes
`ESC [ 0 n`. A cursor query (`CSI 6 n`) becomes the cursor's cell,
1-based, `ESC [ row ; col R`, and the private form (`CSI ? 6 n`)
becomes `ESC [ ? row ; col R`. An OSC colour query (`OSC 10`, `11` or
`12` with `?`) becomes the three fixed `rgb:` replies in the map at
lines 45–49 of the same file (read), dark ones whatever Paseo's theme.
Vessel answers those three with the colours the grid is drawn in
(foreground, background, cursor of the scheme shown), given to the model
as values: a program that asks whether the background is light then
gets the truth in the light scheme. That is a difference from Paseo,
stated. The app's note says the same
thing: the daemon answers cursor reports from its own buffer
(`packages/app/src/terminal/character-widths.ts` lines 3–5, read). A
program that asked and got nothing waits; Paseo's helper then writes
`DA_TIMEOUT` or `DSR_TIMEOUT` and exits 2
(`packages/server/src/terminal/terminal.posix.test.ts` lines 135–163,
read). Vessel's node does not parse, so it cannot answer. The app
answers a live query from one viewer, or from none, and the reply
bytes are those above:

- `replay` applies bytes and returns no replies. Attach, resume and any
  fold of the ring use only `replay`. A cursor-position query in that
  history moves the grid and types nothing. The grid's tests feed
  `ESC [ 6 n` through `replay` and assert the reply is empty.
- The hand-over from history to live output is `replay` too. Records
  that arrive while the history is being read and replayed are kept and
  applied with `replay` after it; only records received after the
  hand-over go through `feed`. A query that arrived during a load is
  therefore never answered late, as a missed query is never answered
  late. A sequence whose introducer was consumed by `replay` produces no
  reply even when it completes in live bytes (history ending in
  `ESC [ 6`, a live record starting with `n`): the model remembers that
  the sequence under way began in replay. Tests:
  `records_buffered_during_load_are_not_answered`,
  `a_query_split_across_the_handover_is_not_answered`.
- The answered set is closed: exactly the five replies above (primary
  device attributes for `CSI c` and `CSI 0 c`; status for `CSI 5 n`;
  the cursor for `CSI 6 n` and `CSI ? 6 n`, each with exactly one
  parameter, as Paseo's handlers ask (`terminal.ts` 892, 907:
  `params.length !== 1` is not answered); the colour for `OSC 10`, `11`
  or `12` whose argument is exactly `?`). Paseo trims the OSC argument
  first (line 916), so `OSC 10 ; ? BEL` with a space is answered there
  and refused here, a stated difference on the refusing side. A reply's bytes come only
  from the fixed DA and status strings, the cursor's position and the
  colours drawn, never from text the program sent. Every other query
  produces no reply. Refused by name, each with a test that feeds it
  through `feed` and asserts an empty reply: window and icon title
  reports (`CSI 20 t`, `CSI 21 t`), size reports (`CSI 14 t`, `16 t`,
  `18 t`, `19 t`), DECRQSS (`DCS $ q … ST`), XTGETTCAP (`DCS + q … ST`),
  the clipboard read (`OSC 52` with `?`), ENQ (answerback, `0x05`),
  DECRQM (`CSI Ps $ p`, `CSI ? Ps $ p`, which Paseo's client also
  swallows), secondary and tertiary device attributes (`CSI > c`,
  `CSI = c`), an `OSC 10`–`12` whose argument is anything but `?`, a
  status query with more than one parameter (`CSI 6 ; 1 n`), and the
  kitty keyboard query `CSI ? u`. Paseo's daemon answers that last one
  with its tracked flags (`packages/protocol/src/terminal-input-mode.ts`
  105, 189–193, written at `terminal.ts` 1041–1042) and tracks push,
  pop and set (`CSI > f u`, `CSI < n u`, `CSI = f ; m u`). Vessel
  refuses the query and ignores the three: the input slice encodes keys
  as xterm does (normal and application cursor modes), never with
  kitty's flags, and any answer, `CSI ? 0 u` included, would tell a
  program the protocol is there. A stated difference, tested
  (`kittys_keyboard_query_push_pop_and_set_neither_answer_nor_move_the_cursor`).
- `feed` may return those reply bytes. The link sends them from one
  viewer. Once any viewer qualifies under `ResizeRule` (it has focus
  and the person has typed), that viewer is the one. Every other
  viewer applies the same live bytes with `replay`.
- Until any viewer qualifies, the first viewer of a terminal this app
  created is the one that answers. It answers before anyone has typed,
  and it answers while its window is not focused, so a shell that asks
  for device attributes as it starts, and a TUI that asks where the
  cursor is before it draws, do not sit there. A second window does
  not answer: only the view that created the terminal is that first
  viewer. The grid's slice tests this as
  `the_opener_answers_before_anyone_qualifies`.
- When no viewer qualifies and there is no such first viewer, nobody
  answers. That is an attach to a terminal this app did not create, or
  the view that created it has closed. Every viewer uses `replay`. The
  program that asked waits. Paseo's helper calls that wait a timeout:
  the shell sits on the device-attributes read, the TUI on the cursor
  report, until the program gives up or a later query arrives after a
  viewer has focus and the person has typed. The missed query is not
  answered late. The app does not invent a reply and does not show an
  error of its own. The grid's slice tests this as
  `nobody_answers_a_live_query_when_no_viewer_may`.
- The page's xterm.js also answers. This track does not change the
  page, so a page that is open beside the answering native viewer can
  still answer the same live query. That remaining double answer is
  stated, not hidden.

## Why this and not the others

The PTY already exists on the node. `alacritty_terminal` would put
another one, and a home-directory read, in the UI process, for a grid
the kit cannot name. `vte` is the maintained parser those libraries
share, two orders of magnitude smaller, and dual-licensed; the grid is
the part this app has to own so `view` can test it without GPUI.
Writing or copying the parser fails the reason `vte` exists and the
rule against taking an editor's terminal. Scalar width is enough to
place Hangul; Paseo's cluster table is a different, larger choice and
is not smuggled in as a copy.

## Advantages, costs and effects

- The app can draw a session's terminal and a shell from one model.
  Create, rename, kill and the follow are calls the node already has
  (ADR 0031). Nothing new is numbered at merge.
- `view` grows a `vte` dependency (57,487 bytes, plus `memchr` through
  its default feature) and a direct `unicode-width` dependency the lock
  already has. Neither crate is vendored. Their `unsafe`, if any, stays
  inside them.
- History is replayed off the UI thread (above), so a long log costs a
  loading row, not frames. Every read and replay completion belongs to a
  typed opening identity and its typed book name; reopening the same
  session cannot accept work from an earlier opening. Cancellation alone
  cannot provide this guarantee for a result already delivered.
- Paseo's 5 ms output coalescer is not copied. The node already chunks
  at 16 KiB; a timer in `view` would be I/O-shaped.
- Paseo's refit ladders (0, 48, 144, 320 ms, and the emulator's waits up
  to 2 s) are not copied. They paper over a layout that has not measured
  yet. GPUI measures; the 100 ms debounce is the one wait, and it is a
  debounce.
- A live query with nobody entitled to answer is not answered.
  Paseo's daemon always writes the reply (the handlers above). Until a
  viewer has focus and has typed, the view that created the terminal
  answers, including while that window is unfocused, so a new shell
  does not sit on its first query. An attach, or a creating view that
  has closed, with nobody qualified, leaves the program waiting. The
  app adds no error of its own. The reply that is sent is Paseo's
  bytes, including the three OSC colour replies; it goes out as
  `TerminalInput` on the terminal that viewer already follows.
- Cluster width differs from Paseo. Paseo gives one cell of width 2 to
  an emoji cluster (VS16, a skin tone, a flag, a ZWJ sequence) and one
  column to an Indic conjunct (`docs/terminal-rendering.md` in Paseo).
  `unicode-width` counts scalars. A program that asks the cursor where
  it is will disagree with Paseo on those clusters. Hangul syllables
  agree (width 2). The difference is recorded in the plan; it is not
  papered over.
- A path paste. Paseo's whole rule (`terminal-path.ts` lines 1–36,
  read): the path's shape picks the branch (`A:\`, `A:/` or `\\` is
  Windows). A Windows path is unchanged, or wrapped in double quotes
  when it contains a space. Any other path doubles each backslash,
  deletes `` ` $ | & > ~ # ! ^ * ; < ``, then single-quotes the result,
  or uses `$'...'` with `'` written as `\'` when both `'` and `"`
  occur, or the same `\'` inside single quotes when only `'` occurs.
  Vessel does not delete. On a POSIX node those characters are
  `TerminalError::Path` (a different path would be pasted). An accepted
  path is then quoted as Paseo quotes after the deletion: backslashes
  doubled, then the same three quote forms. On a Windows node there is
  no deletion in Paseo, so Vessel does not refuse those characters
  either: a space is wrapped in double quotes and every other path is
  unchanged. Which branch runs is the node's platform, passed in by the
  link, not the path's shape and not the app's operating system: the
  shell that receives the paste is the node's, and a shape can lie.
- Inline images (Sixel, iTerm) are not cells. Paseo loads an image
  addon to draw them. A pasted image is a file whose path is pasted,
  which is Paseo's paste, not its drawing. There is no terminal
  file-upload call on the wire: a path on the node is pasted through
  `TerminalInput`; bytes that still have to be uploaded wait for a call
  the coordinator assigns. This ADR does not add one.
- Mouse reporting is not on Vessel's `TerminalInput` (Paseo's terminal
  input has a mouse message). Selection stays in the app. No field is
  added to get it.
- The web page keeps a scrollback of 5,000 and has no setting. The app
  follows Paseo's 10,000, not the page's 5,000.
- The node's shell choice is unchanged (ADR 0031). Paseo's saved
  profiles (a name, a command, arguments, per host) are not added.
- Find walks the grid in `view` (case-insensitive, not a regular
  expression, incremental while typing, next and previous, at most
  20,000 highlights and a `limited` flag at that count:
  `terminal-emulator-runtime.ts` 53, 273–286, 516). Next and previous
  wrap at the ends, as Paseo's do: its search is xterm's add-on, whose
  engine wraps both ways (`addons/addon-search/src/SearchEngine.ts`, "If
  we hit the bottom and didn't search from the very top wrap back up").
  The bar is Paseo's pane find (`terminal/find/index.tsx`, the
  `paneFind` words: "No matches", "{current} of {total}", "{total}
  matches", a `+` after the total when limited). Terminal activity
  hooks, the tablet key bar and the phone renderer are out of this
  decision.
- The bytes a key becomes are Paseo's, read from its encoder
  (`packages/protocol/src/terminal-key-input.ts`): arrows `ESC [ A`…, or
  `ESC O A`… in application cursor mode, `ESC [ 1 ; m X` with modifiers
  (m = 1, +1 shift, +2 alt, +4 ctrl, +8 meta, 19–26); Home and End
  `ESC [ H` / `ESC [ F` in both modes (194, 196); Insert, Delete, Page
  Up, Page Down `ESC [ 2 ~`, `3 ~`, `5 ~`, `6 ~`; F1–F4 `ESC O P`…`S`,
  F5–F12 `ESC [ 15 ~`…`24 ~` (148–162); Ctrl+letter is the letter minus
  64 (99), Alt prefixes ESC (62–64); Enter `\r`, Backspace `0x7f`, Tab,
  Shift+Tab `ESC [ Z`. The grid's tests pin these bytes.
- A send carries at most 64 KiB (`SendTerminal::MAX_BYTES`, refused
  above it by the node): keys, a paste and a reply are split at a
  character boundary (never inside a UTF-8 sequence) into sends that
  size, in order.
- A paste is one item of the ordered sender: keys typed while it goes
  wait behind it. When the program has turned bracketed paste on, the
  start marker `ESC [ 200 ~` goes before the paste's first chunk and the
  end marker `ESC [ 201 ~` after its last, once each, never a pair per
  chunk. An end marker inside the pasted text loses its ESC, Paseo's
  rule (`terminal-paste.ts` 18–28: `input.text.replaceAll(BRACKETED_PASTE_END,
  "[201~")`), so a pasted `201~` cannot end paste mode early and run the
  rest as typed input. Outside bracketed paste, Vessel removes every ESC
  byte from pasted text (a paste is text, not keys); Paseo sends that
  text as it is (21–23), a stated difference. xterm's own paste
  preparation was not read.
- When the node refuses a chunk or the connection drops in the middle
  of a paste, the rest of it is not sent, and nothing already sent is
  sent again. If the start marker went out, the end marker is sent once
  so the shell leaves paste mode. The person is told, beside the
  terminal, how much went: "Paste stopped after {sent} of {total}
  bytes" (a typed failure, `PasteStopped { sent, total }`). Tests in the
  input slice: markers around the whole paste, an embedded end marker
  neutralized, a refused middle chunk followed by exactly one end marker
  and no resend.
- Closing a shell's tab is Paseo's: a confirm ("Close terminal?", "Any
  running process in this terminal will be stopped immediately.",
  `en.ts` 712–713), then the terminal is gone (`terminal.close`). Stop
  (`terminal.kill`) keeps the ended terminal and its output. A script's
  run and a service are listed beside the shells and take no input.
- Resize: Paseo's daemon has a size owner and a pane claims it on focus
  (`terminal-size-ownership.ts`, `terminal-pane-focus-claim.ts`); our
  node has no owner, so the app keeps the page's rule (a view that has
  focus and has typed sends sizes, `ResizeRule`). Stated.
- No call switches a session between its conversation and a terminal,
  so that switch is not offered.
