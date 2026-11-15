# Milestone 12, track S: the shell's furniture in the native app

The board (lanes, cards, actions, archive, machine scope, empty states),
the sidebar (search row, New session and Board, the machines or status
groups, the Display menu, the foot; the Projects part is track F's), the
command palette, the shortcut registry and its one listener, notices and
toasts, Settings › Shortcuts and › Notifications, and the operating
system's notifications, brought from the page into the app. Branch
`backend/m12-app-shell`, off the desktop lead's `backend/m12-spike`.

The page's inventory names the area: sections 2, 3 (without Projects), 4
and 11, and the two Settings sections.

## Order of work

1. View logic, without GPUI, in `view`, ported from the page's
   `web/src/data/` with the page's test cases as its tests (same inputs,
   same outputs).
2. The node calls in `link`.
3. Screens in `app/src/areas/`, each working from 360 points wide,
   strings in `view/src/text/{board,sidebar,…}.rs`, captured beside the
   page's same screen and Paseo's.

## 1. View logic

| Module | The page's | What it decides |
| --- | --- | --- |
| `board` (`card.rs`) | `board.ts` `placement` | a session's card: lane, phrase, attention, time; its facts (`CardDetail`: agent, creator, folder, branch, profile, model, marks, failure, created and active times) |
| `board` (`tone.rs`) | `phrase.ts` `tone`, `card-origin.tsx`, `agents.ts` `agentMark` | the status line's colour, the origin line, the agent's letter, the short id, the "profile · model" line |
| `board` (`tree.rs`) | `tree.ts` | sessions nested under the session whose agent started them, across machines |
| `board` (`deal.rs`) | `board.ts` `board`, `clock.ts` | the lanes (needs-you first, then newest) and the archive (newest first); the one 15-second tick |
| `board` (`archive.rs`) | `archive.ts`, `archive.tsx` | the archive's search; 30 a page and "Show N more" |
| `sidebar` | `sidebar.ts` | from the preferences' Display view (`prefs::SidebarView`: group, sort, ended), a row's mark, what is listed, by machine or by status, the order keys walk and the step |
| `shortcuts` (`keys.rs`) | `shortcuts/keys.ts` | a chord from a key press (by key, else by physical code), a binding of one chord or two ("G S"), the platform's key caps and words |
| `shortcuts` (`registry.rs`) | `shortcuts/registry.ts` | the fourteen actions, their groups, default and fixed keys, the jump digit, an action's keys from the preferences' shortcuts (`Preferences::shortcuts`) and what is changed |
| `shortcuts` (`matching.rs`) | `shortcuts/matching.ts` | what a chord runs: an action, the first key of a sequence waiting 1.5 s, or nothing; changed actions first |
| `shortcuts` (`edit.rs`) | `shortcuts/edit.ts`, `shortcut-bridge.ts` | a recorded chord refused (no modifier, kept by the system, a text key), a clash (same, fixed, taken), assign, remove and restore as the preferences' `ShortcutEdit`s, kept through `PreferenceChange::Shortcuts` |
| `shortcuts` (`listing.rs`) | `shortcuts/listing.ts` | Settings › Shortcuts' rows: label, keys, words, changed, fixed, clashes; search by label or words; All or Modified |
| `palette` | `commands.ts`, `palette.ts` | the palette's entries (actions, go to, machines, sessions, theme), their ids, details and keywords, the search and the groups in order |
| `notices` | `notices.ts` | from two reads of the book, which sessions turned to needs-you or finished (never on a first sighting), and the window title's count |

Tests (`view/src/tests/`), each named for the page's case it follows:
`board.rs` (seven of `board.test.ts`, the tone case added), `tree.rs`
(both of `tree.test.ts`, and the origin line), `archive.rs` (the three of
`archive.test.ts`, and the pages), `sidebar.rs` (the seven of
`sidebar.test.ts`), `shortcuts.rs` (the 23 of `shortcuts/*.test.ts`, and
apply and a sequence's timeout), `palette.rs` (the eight cases from
`commands.test.ts`, plus native selection and highlight regressions),
`notices.rs` (the page's cases, a machine first seen, the titles).
`fixture.rs` turns the page's short ids ("3f2a") into
session ids that start with them.

### What differs from the page, and why

- A machine is Connected or Offline only. The link reads the book's
  machines whole on each refresh and keeps no connection open, so the
  page's Connecting, Reconnecting and Draining are not known yet; Offline
  keeps the page's neutral tone (the lead's first sidebar drew it red).
- `Fleet::tree` takes one machine or all, not a list of node keys: the
  board's scope is one machine or every one. The page's second tree case
  (two of three machines listed) is the same input as a book of those two.
- The card's title is its title, else its folder's name, so the archive's
  search tests the title and the folder as the page does, the same.
- `Fleet::lane` and `Fleet::archive` stay, now read through
  `Fleet::board(None)`, for the screens not yet moved to rows.
- The browser's demo builds its cards by hand (`link/src/demo.rs`): it now
  names `detail` with the agent and the rest defaulted (one edit in the
  browser track's file, so a card made by hand still compiles).
- The palette's machine detail is the state word only: the native book
  has no transport yet, so no "Direct", "Over SSH" or "Paired".
- The palette follows Paseo's token matcher rather than the page's
  accent-insensitive Base UI collator. A session's detail stays the
  page's status word and folder; these are Vessel session navigation,
  rather than Paseo's workspace, agent and file-search scopes.
- A refused chord's sentence names the system, not the browser; the rules
  are the page's unchanged, so overrides travel between surfaces (ADR
  0036).
- The edits are the preferences' (`prefs::ShortcutEdit`, track E's one
  model of the bridge's document): `assign`, `remove_key` and `restore`
  make them, `Preferences::changed` keeps them, as the page's bridge
  does when the node answers. The registry reads `Preferences::shortcuts`
  and keeps no map of its own; a test says every registry id and default
  is one the preferences keep (the two grammars are the same).
- `view::Machine` says the protocol the machine answered with
  (`protocol`, none when unreachable): the board offers a card's marks
  (archive, restore, delete) only from protocol 10, as the page does.
- Notices compare two `Fleet` reads, keyed by machine and session; a
  machine absent from the earlier read is a first sighting, so its
  sessions say nothing, as the page's first load.
- The palette is generic over the settings section and the theme, so it
  takes track E's `Section` and the kit's `SchemeChoice` without a second
  enum of either.

## 2. The board's screen

`app/src/areas/board/` (`mod.rs` the area, the scope, the lanes, the
archive's bar and the empty state; `card.rs` a card; `acts.rs` what a
card asks of its node and the answers), `app/src/areas/archive.rs` (the
archive's dialog), `app/src/areas/toasts.rs` (the shell's toasts), on
`view::board` (`CardActions`, `Card::actions`, `Fleet::put_session`,
`drop_session`, `protocol_of`) and `view::toasts` (the stack, four
seconds each). The kit gains `toast.rs` (a toast and the stack, under
the title bar), `spinner.rs` and twelve glyphs (plus, terminal, message,
archive, branch, trash, pin, reopen, board, keyboard, bell, spinner).

What is drawn, as the page's (`board-page.tsx`, `lane.tsx`,
`session-card.tsx`, `card-actions.tsx`, `archive.tsx`,
`use-mark-actions.tsx`):

- The machine scope (All machines, then each machine) when the book has
  more than one; a scope whose machine left the book is All again.
- Four lanes in one surface divided by hairlines, at least 240 points
  each (the page's 256 would cut the fourth lane in the default 1280
  window beside the 264 sidebar: 1016 left; 240 lets the four share it,
  as the page's grid does), scrolling sideways when narrower (vertical wheel stays vertical);
  each lane's head (dot, name in the lane's colour, count) and its own
  scroll. Under 768 points (a phone) the lanes stack and the page
  scrolls as one, every card's actions shown.
- A card: the origin line (another device, or the agent that started it
  with its parent's short id to open it), the agent's letter, the title
  clamped to two lines, the pin and the unread dot, the folder (or the
  conversation's glyph), the branch, the short id and who; the foot with
  a spinner while working, the phrase in its tone and "machine · 6m ago"
  (the machine left out when scoped) on the one 15-second tick; the
  failure's line; the needs-you tint and border.
- Its actions on hover: Kill (live; a confirm), Archive (ended; the
  conversation's confirm, which says what ends beside it from protocol
  11); in the archive, Restore (start again when it can, else back to
  its lane) and Delete (a confirm). Each answer is put on the board (the
  session's new state, or gone) and said in a toast: "Killed 78e7" and a
  reload, "Session deleted" and the worktree's line, the node's refusal
  as an error toast; a refused start again in the page's words.
- The archive's bar under the lanes (glyph, "Archive", count; quiet at
  zero) opens the archive: a wide dialog (a sheet on a phone) with the
  search, three cards a row (one on a phone) in 60% of the window's
  height, thirty at a time and "Show N more", "No archived sessions" or
  "No results.".
- The empty board: the page's title and words and New session, which
  opens the draft on the scoped machine.

The shell's hooks (the lead's files, one line each): `routes.rs` draws
Board and Archive with `areas::board::render` and arrives at the archive
with `areas::archive::arrive`; `shell.rs` gives an act's answer to
`board_acted` first (it keeps the answers to the acts the board asked,
by machine, session and kind, and hands any other to the conversation's
`acted`) and draws `areas::toasts::render` over the content.

### What differs from the page, and why

- The look is Paseo's neutrals (surface1 cards on the surface0 board,
  `neutral.border` hairlines, 8-point radius as Paseo's popups): neither
  Paseo nor GOAT has a board, so the page's structure and spacing are
  kept and its colours are the kit's.
- The archive is a route (`Route::Archive`, as the sidebar's Archive row)
  drawn as a dialog over the board, not the page's local state; closing
  it goes back to Board.
- The toast stack is the page's behaviour (four seconds, newest last,
  close button, error tone) at the top centre under the title bar,
  Paseo's place for it.
- Stand-ins inside the area: a ghost button (`board::ghost`, for "+ New
  session", the archive's bar and "Show N more") until the kit has one.
- The card's icon buttons say what they do when pointed at, by the kit's
  tooltip (Paseo's: above the button, at once, none on a phone), in the
  page's words (Kill, Archive session, Restore session or Unarchive's
  sentence, Delete permanently).
- The "Merged" tone has no token: a local purple (#9333ea light, #c084fc
  dark, the page's values).
- The needs-you pulse is not drawn (the tint and border are).
- GPUI clamps a title to two lines and ends it with "…" when it is cut
  mid-word; when the cut falls between words it can end without the
  ellipsis (seen at 390 points). The page's `line-clamp` always shows it.

### Captures (the scratch directory's `shots/`, fake fleet plus the board's shapes)

The fleet (`scripts/fleet.sh`) with, added by a scratch script, a long
English title and a long Korean one with branches, a terminal session,
five more sessions and two killed ones. `board-dark-en`,
`board-hover-dark-en` (the action), `board-confirm-dark-en` (archive's
confirm), `board-archived-dark-en` (the count to 1), `archive-dark-en`,
`kill-confirm-dark-en`, `kill-toast-dark-en` (the card moved to Exited),
`archive-hover-dark-en`, `delete-confirm-dark-en` (over the archive),
`delete-toast-dark-en`, `board-light-ko`, `phone-board-dark-en`,
`phone-board-light-ko`, `phone-archive-dark-en` (the sheet).

## 3. The palette and the listener

`kit/command.rs` draws Paseo's command center (`command-center.tsx`, its
`list-metrics.ts`, `components/ui/shortcut.tsx`): the panel 640 by 560
(at most 92% by 80% of the window) 48 under the window's top over black
at 50%, a hairline, 8 round, Paseo's large shadow; the search in a header
over a rule; each group under a muted heading after a rule; a row with a
16-point glyph, its title (and a muted second line), and the badges of
its key, one per chord; the search's place marked `surface3` behind the
foreground; a theme's check; the empty line. At a phone's width it is a
sheet from the bottom with its handle.

`app/areas/palette.rs` lists the page's commands and navigation targets
for the current fleet, with Paseo's token matching, relevance bands and
five recent sessions before a query. Archived sessions are excluded.
Paseo's keyboard (`preserveActiveResultId`, `moveActiveResultId` and
`interaction.ts`) supplies selection identity: Up and Down move the
active row and scroll it into sight, Enter or a click runs it, Escape or a
press on the dimmed page closes it and the keys go back where they were.
Its glyphs are the page's `iconOf` (Plus, Machine, Board, the Settings
section's, Message or Terminal, Monitor, Sun, Moon).

`app/areas/shortcuts.rs` is the one listener (the page's `listener.ts`):
GPUI's keystroke interceptor hears every key before the focused field
and the controls; the press becomes a chord (`chord_of`, its position by
`code_of`), the chord an action with the person's overrides (`matched`,
a sequence's first press kept for the next), and the shell answers
Palette, New session (on the machine in view: the open session's, or the
board's scope), Settings, Go to Board, Machines, Sync and Devices,
Previous, Next and session 1 to 9 in the sidebar's order. A chord
without Mod, Ctrl, Alt or Meta stands down while a field has the keys;
nothing fires while `ShortcutListener::recording` is set (the seam for
Settings › Shortcuts' recorder). An action that acts stops the key (the
page's `preventDefault`): the field neither types it nor runs its own
action.

The shell's hooks (the lead's file, one line each): `Shell::new` calls
`areas::shortcuts::listen`; the render draws `areas::palette::render`
after the toasts. Settings' module exports `section_glyph` (one word in
`settings/nav.rs` and its `use`, for track E) so the palette shows the
same glyph as Settings' list.

### What differs from the page, and why

- Paseo's look, not the page's: no foot with the keys (Paseo's command
  center has none), the second line under the title, the keys of a row's
  shortcut on its right (the page's rows have none).
- Settings sections are listed only when Settings draws them, and
  machines only once Settings › Machines is drawn: a placeholder would
  be a dead end. Go to Machines,
  Sync and Devices by key open Settings at its first section until then
  (`open_settings`' own rule).
- A row the pointer is over is tinted as the active one but does not
  become it (Paseo's hover); the keyboard's row stays.
- Each matching query token is marked, including the scattered runs of
  a subsequence. A query selects the best result afresh; a passive fleet
  refresh preserves identity without scrolling, as Paseo does.
- The keys GPUI's field would take first (its Up, Down, Enter and Escape
  actions) are caught by the panel before the field (`capture_action`),
  as the page's list keeps them from its input.
- Find, the composer's focus and the two panels' keys have no handler
  here: their keys go on to the controls (track C's find bar keeps its
  own Mod+F observer; the panels are the lead's and track F's). They
  should move into the registry so the person's overrides apply.
- The interceptor runs before the field's actions, where the page's
  listener runs after other handlers (`defaultPrevented`): a key a field
  binds and a shortcut also binds goes to the shortcut. The registry's
  default keys with a modifier overlap none of the field's bindings
  (checked against them: the field's are Ctrl or Cmd with `[`, `]`, A,
  arrows, Home, End, Backspace, Delete); a key the person binds may.
- Input-method suppression is not yet proved by this listener. The
  input owner has a focused-field marked-text query; a window-wide seam
  is requested because other areas' private fields must also be covered.
  GPUI's character-input preference must independently suppress AltGr
  shortcuts. Source/API tests and a live X11 input-method check are
  separate evidence; neither is claimed by the palette captures.
- The sidebar's search row (DESIGN.md §2.2) is the lead's sidebar: asked
  of the lead (`Shell::open_palette` opens it).

### Inherited captures (before the strict takeover; not review approval)

`palette-dark-en` (opened by Ctrl+K), `palette-query-dark-en` ("ses":
marks in title and second line), `palette-down-light-ko` (Korean, light,
two Downs), `palette-theme-dark-en` (the check on System),
`palette-slash-dark-en` (`/` from the board; the slash not typed into
the search), `palette-escape-dark-en` (Escape, then typing goes back to
the composer), `palette-enter-dark-en` (Enter on "board"),
`palette-phone-dark-en` (the sheet at 390), `seq-gs-dark-en` (`G S` from
Settings to the board), `key-n-dark-en` (`N` from the board to a new
session, the letter not typed).

## Inherited run log (before the strict takeover)

These are the stopped worker's records, retained as history. The
takeover audit below records the new gates and runtime evidence; these
older claims are not substituted for independent review.

- Merged `backend/m12-spike` at edd22cd (fast-forward).
- The kit embeds the page's fonts (`scripts/fonts.sh`, not committed);
  copied the generated `assets/fonts/` from the lead's worktree.
- Slice 1 (board and sidebar view logic): `cargo fmt --all --check` clean;
  `cargo clippy --all-targets -- -D warnings` clean; `cargo test`: 0, 3,
  8, 112, 69, 0, 0, 0 passed, none failed.
- Merged `backend/m12-spike` at f1787fb (ea13910), clean.
- Slice 2 (shortcuts, palette, notices view logic): fmt clean; clippy
  clean (one `clone_from` fix); `cargo test`: 0, 5, 8, 151, 69, 0, 0, 0
  passed, none failed.
- Merged `backend/m12-spike` at 7638b83 (fast-forward; track E's
  preferences in it).
- Slice 3 (the sidebar's view and the shortcut keys as the preferences';
  a machine's protocol): fmt clean; clippy clean; `cargo test`: 3, 5, 10,
  200, 72, 0, 0, 0 passed, none failed.
- Merged `backend/m12-spike` at 2c8dcef (a94daff), clean.
- Slice 4 (the board's screen, toasts; 057a672): fmt clean; clippy clean
  (a long function split, a redundant closure, a float sum through a
  const); `cargo test`: 3, 7, 10, 205, 72, 0, 0, 0 passed, none failed.
  A fake fleet with the board's shapes: lanes, hover actions, archive,
  kill, delete and their toasts tried by clicks on an Xvfb, desktop and
  phone, dark and light, English and Korean.
- The title's ellipsis and the archive card's foot at the bottom, after
  the captures: same gate.
- Merged `backend/m12-spike` at 777b2da (b5ec1e4): conflicts in the
  kit's glyphs and modules and in the areas' wiring. Kept the lead's
  spinner (`slider::spinner`) and glyphs (its `Reset` for the card's
  restore), added only the board's own (Terminal, Message, Archive,
  Trash, Pin); `Route::Archive` arrives beside the lead's Settings. fmt
  clean; clippy clean; `cargo test`: 3, 10, 10, 215, 73, 0, 0, 0 passed,
  none failed.

- Merged `backend/m12-spike` at 5b934da (640f9b0), clean.
- The coordinator's look at the board (brief, "Added the same
  afternoon"): the fourth lane was cut at 1280 (lane minimum 240 now) and
  the card's buttons had no tooltip (the kit's now). Captured
  `board-fit-dark-en` (four lanes at 1280) and `board-tip-dark-en` (Kill
  over its button). Gate with the palette's view additions below: fmt
  clean; clippy clean; `cargo test`: 3, 10, 10, 222, 73, 0, 0, 0 passed,
  none failed.

- Merged `backend/m12-spike` at 4532c7e (a37d768), clean.
- The palette and the one listener (section 3): the kit's command frame,
  row, heading and key badges; the app's palette and shortcuts areas;
  two one-line hooks in the shell. Built and tried on an Xvfb against
  the fake fleet: Ctrl+K, `/` from the board, `/` in the composer
  standing down, Down, Enter, Escape giving focus back, the theme
  group's check, `G S` from Settings, `N` from the board, the phone
  sheet at 390; dark and light, English and Korean. fmt clean; clippy
  clean; `cargo test`: 3, 10, 10, 222, 73, 0, 0, 0 passed, none failed.

## Notes for later slices

### Takeover audit (2026-11-13)

The matcher is now `view::search::{Query, Score}`, shared with the
provider catalog instead of copied into it. `Query::new`, `is_empty`,
`score(visible, hidden)` and `marks` are public; score fields stay opaque
and ordered, with the palette's recency-tier comparison crate-private.
The existing matcher tests move unchanged, and an external consumer
contract uses the six entries and queries from Paseo's
`components/provider-catalog-search.test.ts`. Alphabetical tie breaking
belongs to the catalog; no locale comparator or new matching rule is
introduced here. Foreign seam: one `pub mod search` registration in
`view/src/lib.rs`; palette imports now point to this shared module.
Shared-source gate: native `cargo fmt --check` and whole-workspace
`cargo clippy --all-targets -- -D warnings` clean; 433 tests and two
compile-fail documentation tests pass. This extraction changes no UI;
it does not resolve or approve the pending palette path/motion work.

Follow-up integration takes lead `d050e7e9`, including the always-visible
desktop sidebar toggle, the focused-window composition query, the X11
input-context fix, shared form/action controls and localization
foundation. The README conflict keeps both control descriptions; the
asset inventory keeps all 66 unique glyph registrations from both
parents. The first attempted integration exposed ambiguous empty-vector
assertions after the regex dependencies added range comparisons. The
expected vectors now explicitly name `Range<usize>`; assertions are
unchanged. The refreshed integration passes native `cargo fmt --check`,
whole-workspace `cargo clippy --all-targets -- -D warnings`, all 430
tests and two compile-fail documentation tests. No root-workspace gate
or new runtime approval is claimed by this integration.

Control-only follow-up at `6d29e29`: Command Center spacing now names the shared
scale; the compact sheet starts at Paseo's 60% snap with its zinc-600
handle. Rows accept multiple highlight ranges through a new additive
`command_row_matched` entry point. The existing single-mark entry point
remains usable until callers migrate. `command/row.rs` owns row drawing;
`command/marks.rs` rejects invalid UTF-8 offsets and merges overlapping
ranges, with Korean and scattered-match tests. No screen or policy moves
into the kit. Source: Paseo's `command-center/command-center.tsx`,
`list-metrics.ts`, `components/ui/shortcut.tsx` and
`components/ui/highlighted-text-segments.ts`. At that hash the sheet has no drag
gesture or animation; V1 subsequently returned this missing behavior.
Control gate: native `cargo fmt --check` clean;
`cargo clippy --all-targets -- -D warnings` clean; `cargo test`:
3, 21, 14, 247, 89 passed (374 total), zero failures, doc tests empty.
Captures and visual review accompany the consuming palette slice; this
control-only change is not claimed as visually approved on its own.

Palette follow-up at `08382d9`: `view/palette/{search,ranking,cursor}.rs` now follows
Paseo's `command-center/results.ts`, `interaction.ts`, their tests and
the protocol's `search/text-match.ts`. Every query token must land in a
field; greedy subsequences cannot cross whitespace. Visible fields beat
hidden keywords at every prefix, section/row ties keep registry order,
and the session section stays below commands. Sessions of an empty query
are the five most recently active, with archived sessions excluded as
`hooks/use-aggregated-agents.ts` excludes them. Every matching token is
highlighted at original UTF-8 boundaries, including scattered matches;
ranking offsets use the reference's UTF-16 unit. No edit-distance typo
tolerance is enabled, as in the reference Command Center.

A query resets keyboard identity and reveals its best result. Passive
refreshes preserve identity (or commit the first remaining row when one
vanishes) without scrolling. The compact breakpoint is Paseo's 600,
not the shell sidebar's 768. Entry presentation is in
`app/areas/palette/rows.rs`; the main palette file remains below 400
lines. New-session commands use the machine actually in view, rather
than a stale draft choice.

Fixture seam: one invocation in `scripts/fleet.sh` calls this track's
`shell-fixtures.py`. It adds long English/Korean titles, long paths,
pinned and archived sessions via the CLI and asserts their durable
projections. The helper was seeded and checked against a fresh fake
fleet. No root workspace source or protocol changed.

The first palette clippy run rejected four test-fixture `vec![range]`
expressions as ambiguous single-range vector initializers. Replaced
those with an explicitly named one-span constructor; no lint suppression
or weaker assertion. Subsequent build commands use the shared build-slot
runner. The fixture bootstrap, its CLI changes and durable-shape checks
also passed as one complete `fleet.sh start` run.

Final palette gate: native `cargo fmt --check` and whole-workspace
`cargo clippy --all-targets -- -D warnings` clean; `cargo test` passed
3, 21, 14, 266 and 89 tests (393 total), zero failures. The final clippy
also removed one import left behind by extracting row presentation.
Matching uses ECMAScript's whitespace set, including BOM and excluding
the Unicode next-line character; a regression keeps this boundary from
drifting to Rust's different `is_whitespace` set. Runtime captures and
the consuming screen's visual approval remain separate from this gate.

Runtime audit at `08382d9`: 70 original captures in the
scratch `palette-captures` folder, all inspected against the reference
components named above. `live` covers six states in each of the eight
size/theme/language combinations: initial, long mixed-script titles and
paths, arrow selection, changed query, reopened and no matches.
`empty` and `offline` add eight captures each; the latter is a cold
connection failure, not a warm disconnect. Commands remain available
without a session section. `actions` adds three desktop English and
three compact Korean captures: selecting Dark, opening the board and
opening General. The saved theme preference was also read back. These
last two destination screens demonstrate navigation, not approval of
the board or the settings frame.

The actual capture canvases are 1280×800 and 390×844. A scratch manifest
records every original image's dimensions and SHA-256 and the debug
executable's build id; labelled QA sheets are separate from the
originals. The app window was explicitly focused before inputs. The
fresh executable followed the 393-test gate. The final evidence-only
notes correction changes no Rust or fixture code, so the unchanged
native gate is reused rather than repeated. All capture processes and
the disposable fleet were stopped afterwards by their recorded PIDs.

At that checkpoint the compact sheet stays at the initial
60% snap without the reference's drag or fade; the registry names
Vessel's available commands and sessions instead of Paseo's workspace,
agent and file scopes. There is no asynchronous file-search loading or
error row to capture in this local registry. Settings commands are
registered as their destination areas become drawn. Composition,
AltGr, terminal ownership and privately observed shortcuts still belong
to the following registry slice; the palette's captures do not prove
those boundaries.

- The inherited notification draft was preserved outside the repository
  as a patch and its untracked area file, then its paths were restored.
  It omitted sound, had no capture or reference comparison, and kept
  notification policy in the drawing crate. It is not a delivered slice.
- The unreviewed changes are `f7364ce` (board fit/tooltips), `bf14de1`
  (palette navigation and key positions), `1ee11c9` (palette/listener and
  kit pieces), and `04a46f7` (board confirmations and errors). None has
  takeover review approval yet. The other commits unique to the branch
  are merges, not additional feature slices.
- Read Paseo's `command-center/command-center.tsx`, `list-metrics.ts`,
  `results.ts`, `results.test.ts`, `components/ui/shortcut.tsx`,
  `components/ui/highlighted-text-segments.ts`, and the protocol's
  `search/text-match.ts` and tests. The inherited frame values agreed;
  the substring-only search and single highlight did not. The follow-up
  above replaces those with token matching, subsequences within words,
  visible-field priority, relevance ordering and matching highlights.
- Read GOAT's `components/ConfirmDialogHost.tsx`, `ui/alert-dialog.tsx`
  and `confirmDialog.test.ts`. GOAT has no session-status board (also
  recorded by the milestone 11 feature table), so board structure still
  follows Vessel's page; confirmation structure follows the shared kit's
  Paseo dialog and GOAT's cancellation/destructive-action semantics.
- Integrated lead head `b3c0f93`. The only conflict was area state:
  retained both the palette/listener and the lead's sidebar state.
  Reviewed kit controls, including QR, badge and alert updates, remain.
  Native integration gate: `cargo fmt --check` and
  `cargo clippy --all-targets -- -D warnings` clean; `cargo test`:
  3, 19, 14, 247, 89 passed (372 total), zero failures; doc tests empty.
  This checks the integrated native workspace, not the root workspace
  or final UI parity. The gate was run once on the integrated code.
- Shared-file seam: `app/src/areas/mod.rs` keeps the three area state
  fields and their constructors; all other incoming changes are the
  lead's reviewed work. No runtime configuration or preview was changed.
- N1 owns inbox and notification policy; this track consumes its view
  decisions for the waiting lane, sidebar count, desktop delivery and
  sound. The Shortcuts page plugs into the settings frame's existing
  `content` and `drawn` slots and reuses its preferences model.
- Keyboard seams still open: conversation's find area observes Mod+F
  privately; its owner is replacing that observer with a registry entry
  point. Composer focus has no registry handler yet. Desktop sidebar
  visibility needs the lead's render to honor `SidebarPanel`; the
  inspector action needs its owning area's reachable entry point.
  Defaults and overrides are not considered verified for these actions.
  The input control owner is adding a focused-field marked-text query;
  that API and a real X11 input-method check are separate evidence.
- The newer fake-fleet check correctly rejected the inherited fake
  executable: it lacked the ten startup lines and fully streamed thought.
  Rebuilt the CLI, node and fake binaries in the configured root target;
  the updated fleet then passed. The command runner reaps detached
  descendants on exit, so capture fixtures are kept under a foreground
  runner and stopped by their recorded PIDs after checks.

- `notify-rust` 4.19 is already in the dependency tree (through the pinned
  GPUI snapshot): the operating system's notifications may need no new
  crate; its licence is to be checked against ADR 0014.
- Kit stand-ins: the board's ghost button (`board::ghost`).
- The seam with track E (settings): E writes `view/src/settings.rs`
  (`Section`), `view/src/prefs/` (the preferences) and `kit/settings.rs`
  (the rows). The sidebar's view and the shortcuts are E's types now (no
  second store); the app changes them through `Prefs::prefer` and saves
  (`Command::SavePrefs`). Alerts (`prefs::Alerts`) are E's type too.
- Track C's session bar assumes protocol 10 for its marks
  (`conversation/bar.rs`); it can read `Machine::protocol` now.


### Palette return: shared controls gated, consuming UI pending

[Decision Log]
- Purpose: V1 returned the palette because a session's path used the prose
  face and the compact handle did not move the sheet. The preceding 70
  original captures remain evidence for the earlier immutable hashes only.
- Existing assets: reuse command row/mark validation, popup geometry,
  native ScrollHandle/pointer capture and the edge-sheet module. No new
  dependency, copied UI kit, area-local gestures or font classification.
- Choice: one canonical typed command-row renderer; opt-in CommandState
  plus additive sheet::snap state/listener/motion. Compatibility entry
  points delegate to the same renderer and existing edge sheets stay fixed.
- Reference: Paseo command-center.tsx and isolated-bottom-sheet-modal,
  Gorhom snapPoint/constants/BottomSheetHandle/useGestureEventsHandlersDefault,
  installed Reanimated spring.ts/springUtils.ts/springConfigs.ts, RNW
  Modal/ModalAnimation.js. Full local source paths are in the review report.
- Why: whole-line search stays in view while semantic parts let the kit
  own prose/code typography; shared state prevents a palette-only gesture
  implementation and gives the scrollable an explicit handoff.
- Costs: native two-pixel pointer slop and bounded velocity sampling replace
  the platform recognizer. GPUI lacks subtree pointer-events:none, so
  exiting frames briefly consume pointer input instead of passing it to
  the page. Prior focus is restored by the host at close, once. The
  application reduced-motion flag is honored; OS startup/change wiring
  remains lead-owned and must not be claimed as covered by flag tests.
- Verification still pending: consuming typed-view/app seam,
  actual 60→90→60/down-close, scroll/click separation, fade, reopen/focus,
  reduced flag and the full path/language/theme/size capture matrix.


Shared-control gate: native `cargo fmt --all -- --check` and whole
`cargo clippy --all-targets -- -D warnings` pass. Whole `cargo test`:
3, 40, 15, 286, 9, 89, 3 tests pass (445 total), plus two compile-fail
documentation tests; zero failures. The logs explicitly compile
both changed kit and dependent app, so prepared source was not mistaken
for a fresh old artifact. The twelve added cases include numeric samples
from the installed reference helper, snap/reopen/content-offset rules,
fade lifetime, and actual GPUI pointer dispatch with App reduced-motion
true: handle expansion, content scrolling without row activation, a
stationary click that still works, and pan-down close. This is a GPUI
flag/hit-test result, not an OS preference or real-desktop capture pass.

The first two lint runs failed (deprecated event alias, arithmetic
operators, unconsumed test results and test signatures). The final code
uses the existing kit's `mul_add`/checked-time conventions and consumes
results with assertions; no lint suppression or weaker assertion was
introduced. Screens still use the static compatibility entry point at
this control-only commit. Typed view/app consumption and new executable
captures are the next separate slice. The old captures are not reused as
proof for the new control behavior.


### Integration checkpoint before the next session

Merged lead target `b6bdd43ce6894ffe9ba484a142f7d1a0d5de4dce` after
`babeeb16`, preserving the shared caption/snap controls. The automatic
merge had no conflicts. It includes protocol12's search/parking line,
the Light faces already provided for embedding, machine-book actor
boundaries, permission-card seams, and the Linux/FreeBSD portal motion
follower with its execution-only capture override. No font fetch ran.

The first target `64b5a3d0` failed native clippy with E0502 in C's act
answer handler. That gate ended before any tests. The lead had already
fixed the exact borrow in `b6bdd43c`; the clean merge was refreshed to
that target. Final native fmt/clippy are clean; whole tests pass:
7,40,29,291,9,99,3 (478 total), plus two compile-fail documentation tests,
zero failures. Logs are `lead-b6bdd-{fmt,clippy,test}.log` in the assigned
scratch. The compiler log names the changed protocol, kit, view, link and
app crates. This is native integration evidence; no root/web gate or new
UI capture is claimed. Incoming generated TypeScript whitespace remains
as its generator produced it.

A self-review of `command-center.tsx`945–949 found that the desktop
Modal is conditionally unmounted at close, so its250ms fade is entering
only. `babeeb16` incorrectly retained an outgoing desktop fade. V1 was
told before any final UI review request. A separate correction is
prepared outside the tree; it is not installed or gated. The compact
sheet keeps its spring and backdrop target through dismissal, as
`sheet-backdrop.web.tsx` says. The earlier generic RNW exit pass-through
comparison does not describe this Command Center and must not justify
its outgoing desktop fade.

No further control, consumer or merge starts in this session. See
`backend-m12-shell-resume.md` for immutable hashes, prepared-only patch
artifacts, review ownership, dependencies and the next work order. The
old70 captures remain evidence only for `08382d9`/`6d29e29`; there is no
new executable or real-shape capture for `babeeb16` or this merge.

### Palette finished (track S3, 2026-11-14)

Commits: `e2a4ada3` (merge of the lead's line, 135 commits; two additive
conflicts: the kit's glyph list and the README), `0cf43664` (kit: the
desktop panel closes at once, only its entrance fades), `153da701` (the
palette consumes the typed caption and the motion; the sidebar's Search
row and the title bar's search mark open it).

Reference files followed: Paseo `command-center/command-center.tsx`
(panel 640 by 560, max 92% by 80%; the desktop Modal unmounted at close,
`if (!state.open) return null`; the compact `IsolatedBottomSheetModal`
with pan-down close and backdrop 0.45), `components/ui/highlighted-text-segments.ts`
(`findHighlightRanges`: one mark per query token), `components/sidebar/sidebar-nav-rows.tsx`
(`SidebarSearchRow`) and `sidebar-nav/model.ts` (header rows: new,
history, search, schedules).

What was checked, on the fake fleet paired as a person (the app's own
configuration directory, the pairing modal reached through the palette,
the link typed by the capture driver from a variable and never shown or
kept; `--token-file` owner tokens were not used):
- Both themes, both languages, 1280 by 800 and 390 by 844: the empty
  query (commands, Settings sections, themes), a query matching eight
  long English and Korean titles with long paths (the path in the code
  face, its match marked inside the path), a Korean query, no match
  (Paseo's fixed 560 panel with its one line), keyboard selection, close.
- Motion, measured from frame strips (an X11 grabber at about 60 Hz;
  software Vulkan draws far slower, so a strip shows few app frames):
  the desktop panel's band brightness goes 103 → 119 → 122 on opening
  (one frame inside the 250 ms fade) and 122 → 103 in one step on
  closing; with the app's reduced-motion flag it goes 103 → 122 in one
  step. The phone sheet settles at its 60% stop with the spring's tail
  (its top at 369, 357, 352, 350, 349, 348 points) and straight to 348
  with reduced motion; dragging the handle to 90% follows the pointer
  (303, 280, 269 … 94) and pan-down follows it until the sheet is gone.

Differences from Paseo, named:
- A session row leads with the Message or Terminal glyph and says
  "status · path". Paseo's agent row leads with the provider's mark with
  a status badge on it, says "host · path" (the host only with several
  hosts) and the time since its last activity after it, outside the
  search. The kit's command row takes a glyph only; the agent mark and
  status badge need a control change in `kit/command`, asked of the lead.
- The title bar's search mark has no tooltip: the kit's tooltip opens
  above its trigger, which is outside the window at the title bar.
- The Messages group (every session's messages, track C's search) is
  not drawn yet: its seam is on C's branch (`69d2003f`), not yet on the
  lead's; the group goes after the command groups once it is merged.
- xdotool typed only the first syllable of a two-syllable Korean query
  on Xvfb; the Korean query captures show that syllable's marks.

Problems met: the prepared kit correction failed clippy once
(`match_same_arms`: two arms both 0.0; merged into one), and one prepared
test expected every occurrence of a token marked, which neither the
shared matcher nor Paseo does (corrected to one match per token). Two
capture runs typed a pairing link into the new chat's composer (a click
on "Add a machine" did not open the modal in time); the captures were
deleted, nothing kept the text, and the link was never used. A capture
display number was used at the same time by another worker's client;
the driver now starts its Xvfb with an authority cookie of its own.

Captures: the scratch directory's `s3/shots/palette/` (matrix) and
`s3/shots/palette-motion/` (strips, with `times.txt` per strip).

### Settings › Shortcuts finished (track S3, 2026-11-14)

Commits: `62f80a87` (every app-wide key goes through the registry: GOAT's
queue keys, Mod+Shift+Enter sends the first queued message and Alt+Up
edits the latest one; a held key does not repeat an action), `b0818ba5`
(the page), and the follow-up that writes an action's two keys as two
("G M or G F") and shortens the Korean label that wrapped at 390.

Reference files followed: Paseo `screens/settings/keyboard-shortcuts-section.tsx`
(one section per group, "Reset all" at the first section's head while a
key is changed, the row's 32-point menu slot kept empty for a fixed row,
recording in the row on `surface2` with Done and Cancel),
`keyboard/keyboard-shortcuts.ts` (the bindings), GOAT
`packages/shared/src/keybindings.ts` (the queue's keys).

Checked on the CLI-paired fake fleet: both themes, both languages, 1280
by 800 and 390 by 844, 800 by 600 and 1024 by 700, scales 1, 1.25, 1.5
and 2 (the menu's dots and the key badges stay sharp): the page, its
scroll, the row menu, recording, a refused key (no modifier), a key
another action has, the "Shortcut already in use" question, and the page
after reassigning ("Not set" on the action that lost it, "Reset all" at
the head). Captures: the scratch directory's `s3/shots/shortcuts/`.

Not done, named: the page is drawn at a phone's width too (Settings'
frame hides desktop-only sections once its section table lands); a
modifier held alone is not shown before the key (GPUI's interceptor
hears keys, not modifier changes); the row menu has no leading glyphs
(the kit's menu has none yet).

### Word 39: Animations setting and the shell's transitions (track S3, 2026-11-14)

Commits: `17bfe083` (Settings › Appearance › Animations, and the setting
holds every motion), `9f237519` (the kit's surfaces arrive on the motion
tokens), `a1fa3c4c` (the combobox's placement in its own function),
`49ac8dae` (a page fades in; the palette's keyboard row fills in).

[Decision Log]
- Purpose: the owner's word 39: every change a person causes has a short
  transition; a setting turns them off and then every change is instant;
  nothing animates while the screen is still.
- Tokens: the lead's `kit::motion` (QUICK 150 ms, STANDARD 300 ms, the
  curve .23,1,.32,1). Popovers, menus, the combobox and tooltips arrive
  over QUICK, as Paseo's menu does (`menu-overlay.tsx`, 150 ms, opacity
  and a 0.97 scale); dialogs, sheets, toasts, the palette and a page over
  STANDARD. GPUI draws no scale on a box, so a surface rises 4 points
  instead (`popup::appear`).
- Leaving: Paseo's desktop popovers, menus, dialogs and its command
  center unmount at once (no exiting animation on the web); so do ours.
  A tooltip appears over QUICK and goes at once. The phone's palette
  sheet keeps its spring out.
- Selection moving: the row the keyboard arrives on fills in over QUICK
  (`popup::fill_in`: the menu's and the palette's rows); the row it
  leaves clears at once.
- A page: the frame is wrapped in one box keyed by the route and the
  session shown (`app/src/shell/page.rs`) and fades in over STANDARD; a
  Settings section is the same page, as Paseo's settings sections switch
  in place.
- Per frame: every transition is GPUI's `with_animation`, which asks for
  the next frame while it runs and stops when it ends; no timer drives
  motion, and nothing repeats.
- Held motion: one flag (`app::motion`): the system's reduced motion, a
  software renderer, and the Animations setting; GPUI's `with_animation`
  draws the end at once under it. `VESSEL_APP_REDUCE_MOTION=0` fixes only
  the system's and the renderer's reasons, so the setting can be shown on
  and off on a virtual display (agreed with the lead).
- The setting: `prefs::Appearance::animations` (`Animations::{On, Off}`,
  lenient, default on) with `PreferenceChange::Animations`; applied at
  once through `motion::animations` and at start in `main.rs`. Kept with
  the app's preferences only: the page has no such setting yet and its
  document ignores the field (the contract test pins both documents).
- Not done: the sidebar's width and the right panel showing and hiding
  wait for the lead's resize work (no transition while a drag resizes);
  a toast's strip needs an action on a reachable machine, which the
  capture fleet was not (its entrance is the menu's `popup::appear` over
  STANDARD).

Measured, release build on a virtual display (software Vulkan, so the app
draws a frame every 35 to 45 ms; the grabber reads the screen every 16):
the mean brightness of each surface's crop, frame by frame after the
press, dark theme.
- Menu: on 29.6, 31.5, 32.5, 32.8, 32.9 (127 to 251 ms); off 29.6 to
  32.9 in one frame.
- Dialog (scrim and card): on 28.3, 19.6, 16.0, 15.4, 14.6 (132 to
  274 ms); off 28.7 to 14.6 in one frame.
- A page (to the board): on 28.1, 26.3, 26.6, 26.8, 26.9, 27.0; off 28.1
  to 27.0 in one frame. To Settings: on 27.0, 28.0, 29.0, 29.3, 29.4,
  29.6, 29.7; off in one frame.
- The phone's sidebar sheet: on 28.9, 23.4, 21.1, 20.6, 20.4, 20.3 as it
  slides in from the left (about 200 ms); off 28.9 to 20.3 in one frame.
- The menu's row moving: on 32.9, 32.8 (the old row cleared), 33.8; off
  32.9 to 33.8 in one frame.
- The palette, opened once before (a fresh process's first open is
  slower than the strip): on 29.9, 27.6, 29.8, 30.7, 30.8 (about 150 ms
  after the panel's first frame); off 29.9 to 30.8 in one frame. Its
  keyboard row: on 30.0, 28.8, 29.8, 30.0 (the new row filling); off no
  step at all (the highlight moves in one frame). Closing: one frame,
  on and off.
- Tooltip (the model pill's): on 30.9, 31.2, 32.1, 32.6; off 30.9 to
  32.6 in one frame.
- Combobox (Settings › General › Language): on one frame with the list
  at part opacity and part of its rise (the row under it shows through),
  then whole; off whole in one frame.
- Both themes were captured; the light strips step the same way.

Idle CPU, release build, 30 s with the screen open and the pointer still
(one core): Settings 0.57% with Animations on and 0.47% off. With a
focused field (the new-session page's composer, the palette's search)
29% and 48% on, 0.5% off: each caret blink redraws the whole window,
which in software costs a large part of a core. On a real software
renderer the app holds every motion, the caret's blink with it, so this
cost appears only when a capture forces motion on; on a GPU a blink is
one ordinary frame.

Captures: the scratch directory's `s3/shots/motion/` (frames with
`times.txt` per strip; `strips/` on above off, and the numbers beside
each).

### The sidebar's list and its Display menu (track S3, 2026-11-14)

Commit: `ad667abe`. The sidebar draws through the view's model of the
page's sidebar (`Fleet::sidebar_machines`, `sidebar_statuses`; the same
list Alt+[ and Alt+] walk): grouped by machine (the default) or by
status, sorted by creation or last activity, ended sessions listed
unless hidden (on by default since the 0.0.0.0.17 smoke test; the page
and the bridge still default to hidden until the node line moves them,
`prefs::SidebarView::default`), pinned first, five to a machine and ten
to a status group, then
"N more", which opens the board (on that machine when grouped by
machine). A row leads with the agent's mark and the page's mark of what
the session is doing (spinner, needs-you dot, hollow dot, dash, the muted
fill for unread); an unread title is primary at medium weight; grouped
by status, the machine's name is at the right.

Reference files followed: Paseo
`components/sidebar/display-preferences/{model.ts,menu.tsx}` (the
Settings2 mark at the list heading, a 232-wide surface aligned to the
trigger's end, root rows naming each decision with its value, one page
per decision, options with marks on the grouping page only, a choice
closing the menu) and `components/left-sidebar.tsx`
(`WorkspacesSectionHeader`); the page's `features/shell/{sidebar-machines,
sidebar-statuses,sidebar-view-menu,session-mark}.tsx` and `sidebar.css`
(the decisions, the marks' sizes and colours, "N more").

Differences, named: a page replaces the root in place with a way back on
a wide window too (Paseo opens a flyout by hover intent; the kit's list
has no flyout); "Show ended sessions" is a switch on the root (Paseo has
no such decision; the page's menu had it); a session row has no branch
chip yet (the workspace track's to draw); a machine's row is the native
sidebar's (name, state, the unreachable line) rather than the page's
dot, name and live count.

Structure: `app/src/areas/sidebar/` is one file per part (`mod.rs` the
frame and its rows, `machines.rs`, `statuses.rs`, `session_row.rs`,
`display.rs`, `foot.rs`), spacing on the kit's steps (the quality audit's
S3 rows 1 and 6); what each Display row is and does is
`view::sidebar::display`, tested without a window.

Capture note: a book paired by the CLI under `HOME=<person>` names its
credential `~/config/…`; an app run with another HOME reports the
machine unreachable ("the device credential … No such file"). The
capture driver rewrites that path in its per-run copy.

### The sidebar's row menu, ended sessions, all projects (track S3, 2026-11-15)

Row menu, commit `f7e91461`: a session row has Paseo's menu
(`components/sidebar/sidebar-workspace-menu.tsx`), opened from the row's
dots (shown while the row is hovered, always on touch, kept while its
menu is up) or a right press on the row. Items in Paseo's order: Copy
path (when the session has a folder), Mark as read or Mark as unread,
Pin or Unpin, Archive. The marks need the node's conversation protocol;
an older node's row offers Copy path only. Archive asks the board's own
question first, drawn by the sidebar while the board is not on screen.
Differences, named in `app/src/areas/sidebar/row_menu.rs`: horizontal
dots (the kit has no vertical mark), no Rename, Copy branch name, Labels
or Open in file manager (no native seam yet), no leading glyphs on the
menu's rows (the kit's menu has none).

How the board's agreement was checked: one run of the release build on
the board route with the sidebar beside it (the capture folder's
`shots/rowmenu/sync-dark-en` and `archive`). Mark read removes the
board card's unread dot; mark unread brings it back; unpin removes the
card's pin and moves the row down the sidebar; Archive, after Confirm,
moves the board's Building count from 28 to 27 and Archive from 4 to 5,
and the machine's "22 more" becomes "21 more". No reload in between:
the node answers a mark with the session's new state, and the board's
bookkeeping of acts puts it in the fleet that both draw from.

Ended sessions, commit `8366a2b4`: "Show ended sessions" is on by
default (the coordinator's ruling after the 0.0.0.0.17 smoke test). A
stored document without the field reads as on; a stored `false` stays
off. The page and the bridge still default to off until the node line
moves them; `view/src/tests/prefs.rs` pins the shared document's shape
and names this one default as the stated temporary difference (Decision
Log at `prefs::SidebarView`).

All projects: the row counted every machine's projects but opened
Settings › Projects, which shows one machine (on a fleet of two, the
other machine's "No projects yet" while the fourteen were on the
first). Paseo's sidebar has no "all projects" screen: it lists every
host's projects in the sidebar and filters them through its Display
menu. The row now unfolds the rest in place, "Show fewer" folds them
back, so the count and what the press shows are the same rows.

### Resume point (track S3, 2026-11-14 15:40 UTC)

- Done and reported: palette (`153da701`, accepted); `view::matcher`
  (`b89e4528`); app-wide keys (`62f80a87`); Settings › Shortcuts
  (`b0818ba5`, `d3bb4f53`, captures passed by the coordinator; handed to
  the lead for 0.0.0.0.16 up to `d3bb4f53`); word 39 (`17bfe083`,
  `9f237519`, `a1fa3c4c`, `49ac8dae`, notes above).
- Waiting on others: the sidebar's width and the right panel's
  transitions (the lead's resize work); the palette's Messages group (the
  lead merging `b89e4528`; C's seams `message_search_query`,
  `message_search_rows`, `message_row`, `message_search_note`,
  `open_search_hit`, `message_search_more`; then tell C to remove
  `example` and `hear_find`); a toast strip (a reachable capture fleet).
- Next: the sidebar slice. `view/src/sidebar.rs` already has the page's
  model (`Fleet::sidebar_machines`, `sidebar_statuses`, `listed`: group by
  machine or status, sort by created or activity, ended hidden, pinned
  first), and `Fleet::sidebar_order` already drives Alt+[ and Alt+]; the
  app's `areas/sidebar.rs` does not draw through it yet. Paseo's
  references: `components/sidebar/display-preferences/{model.ts,menu.tsx}`
  (the Display menu at the sidebar's head, one page per decision) and
  `components/sidebar/sidebar-workspace-menu.tsx` (the row menu: pin,
  mark as read or unread, archive, copy path). With it, the quality
  audit's S3 rows 1 (split `sidebar.rs`, add tests) and 6 (its `px`
  literals to `spacing`). Then notifications (E3 waits for the delivery
  function's name for its Test button). Settings › Sidebar stays hidden
  (E3) until the sidebar reads its items.
