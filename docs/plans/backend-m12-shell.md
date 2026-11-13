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
| `sidebar` | `sidebar.ts` | the Display view (group, sort, ended), a row's mark, what is listed, by machine or by status, the order keys walk and the step |

Tests (`view/src/tests/`), each named for the page's case it follows:
`board.rs` (seven of `board.test.ts`, the tone case added), `tree.rs`
(both of `tree.test.ts`, and the origin line), `archive.rs` (the three of
`archive.test.ts`, and the pages), `sidebar.rs` (the seven of
`sidebar.test.ts`). `fixture.rs` turns the page's short ids ("3f2a") into
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

## Run log

- Merged `backend/m12-spike` at edd22cd (fast-forward).
- The kit embeds the page's fonts (`scripts/fonts.sh`, not committed);
  copied the generated `assets/fonts/` from the lead's worktree.
- Slice 1 (board and sidebar view logic): `cargo fmt --all --check` clean;
  `cargo clippy --all-targets -- -D warnings` clean; `cargo test`: 0, 3,
  8, 112, 69, 0, 0, 0 passed, none failed.

## Notes for later slices

- `notify-rust` 4.19 is already in the dependency tree (through the pinned
  GPUI snapshot): the operating system's notifications may need no new
  crate; its licence is to be checked against ADR 0014.
- Kit stand-ins: none yet.
