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
apply and a sequence's timeout), `palette.rs` (the eight of
`commands.test.ts` and `palette.test.ts`, and the search),
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
- The palette's search trims the query and is not accent-insensitive
  (the page's Base UI collator is). A session's detail is its status word
  and folder, as the page's; the card's board phrase is not used there.
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
- Four lanes in one surface divided by hairlines, at least 256 points
  each, scrolling sideways when narrower (vertical wheel stays vertical);
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
  session", the archive's bar and "Show N more") until the kit has one;
  no tooltips on the card's icon buttons (the tooltip is the lead's).
- The "Merged" tone has no token: a local purple (#9333ea light, #c084fc
  dark, the page's values).
- The needs-you pulse is not drawn (the tint and border are).
- GPUI clamps a title to two lines and ends it with "…" when it is cut
  mid-word; when the cut falls between words it can end without the
  ellipsis (seen at 390 points). The page's `line-clamp` always shows it.

### Captures (`/tmp/work/m12-shell/shots/`, fake fleet plus the board's shapes)

The fleet (`scripts/fleet.sh`) with, added by a scratch script, a long
English title and a long Korean one with branches, a terminal session,
five more sessions and two killed ones. `board-dark-en`,
`board-hover-dark-en` (the action), `board-confirm-dark-en` (archive's
confirm), `board-archived-dark-en` (the count to 1), `archive-dark-en`,
`kill-confirm-dark-en`, `kill-toast-dark-en` (the card moved to Exited),
`archive-hover-dark-en`, `delete-confirm-dark-en` (over the archive),
`delete-toast-dark-en`, `board-light-ko`, `phone-board-dark-en`,
`phone-board-light-ko`, `phone-archive-dark-en` (the sheet).

## Run log

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

## Notes for later slices

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
