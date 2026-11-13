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

## Notes for later slices

- `notify-rust` 4.19 is already in the dependency tree (through the pinned
  GPUI snapshot): the operating system's notifications may need no new
  crate; its licence is to be checked against ADR 0014.
- Kit stand-ins: none yet.
- The seam with track E (settings): E writes `view/src/settings.rs`
  (`Section`), `view/src/prefs/` (the preferences) and `kit/settings.rs`
  (the rows). The sidebar's view and the shortcuts are E's types now (no
  second store); the app changes them through `Prefs::prefer` and saves
  (`Command::SavePrefs`). Alerts (`prefs::Alerts`) are E's type too.
- Track C's session bar assumes protocol 10 for its marks
  (`conversation/bar.rs`); it can read `Machine::protocol` now.
