# Milestone 12, track C: the conversation in the native app

A structured session's conversation, brought from the page into the app:
the timeline with every entry kind, the title bar and its Session actions,
find, the outline, read marks, the ended banner, the context ring, the
inspector's Summary, and the terminal view (inventory sections 6 and 7);
then what goes beyond the page (copying, a screen-reader buffer) and,
written down only, rewind, fork, cross-session search, export and import.
Branch `backend/m12-app-conversation`, off the desktop lead's
`backend/m12-spike`.

The look is GOAT's chat (`/home/dev/refs/goat/apps/web/src/components/chat/`,
the reference notes' Appendix G), the controls Paseo's (Appendix P);
neutral colours come only from `tokens.neutral.*`.

## Where the code is

| Part | Where | What it holds |
| --- | --- | --- |
| Timeline logic | `view/src/timeline/` | records folded into entries (`fold`), runs of tool calls (`runs`), the rows drawn (`rows`), the page's window arithmetic (`window`), find (`find`), the outline (`outline`), approvals (`approval`), the context meter (`meter`), diffs (`diff`) |
| Session logic | `view/src/session/` | read marks (`marks`), titles and names (`titles`) |
| Terminal logic | `view/src/terminal/` | the input sender and its queue (`sender`), the resize rule and the replay log (`frame`) |
| Words | `view/src/text/conversation/` | the area's sentences, English and Korean (`lang.conversation()`) |
| Node calls | `link/src/session.rs` | a session's acts (`Act`: rename, mark, start again, delete, kill, move profile, terminal input), each on its own task |
| Kit controls | `kit/src/virtual_list.rs`, `menu.rs`, `dialog.rs`, `popup.rs` | the long list, Paseo's menu and dialog (a sheet on a phone), the popup surfaces |
| Kit, GOAT's chat surfaces | `kit/src/chat_markdown.rs`, `output.rs`, `copy.rs` | a reply's Markdown (code blocks that scroll sideways with a language and a copy button, GOAT's tables, links in the theme's blue); a tool's text and diffs in a box capped at 192 that draws only the lines in view; the copy button both use |
| Screens | `app/src/areas/conversation/` | the title bar (`bar`), the timeline (`timeline`), the entries (`entries`, `tool`), the look (`look`), the scroll bench (`bench`) |

Tests: `view/tests/page/` replays the page's own cases (72 in the page
binary at `70e9cf2`), with a 20,000-entry fold among them.

## Real shapes

Screens are checked against fake-acp's `shapes` turn (`200de4c`): a long
thought; Markdown with a table, a 240-character code line, a list, a
link and Korean; a read of a long path; an edit with a wide diff; a
command on one long line asking for permission with four answers, and
2,400 lines of output. `scripts/fleet.sh start` makes two copies on
`laptop` (`shapes` answered, `shapes-asking` still asking) and fails if
either lacks the shapes; the verb's own tests hold the output's line
count, which `vessel log` does not print.

## Captures

In `/tmp/work/m12-conv/shots/` (not committed), each beside the GOAT or
Paseo screen it follows.

| Capture | What it shows | Its reference |
| --- | --- | --- |
| `01-session-light-1280`, `10-session-dark-1280` | a conversation: a person's bubble at the right, replies on the lane, runs of tool calls folded into one work-log row, the turn's end as a divider | GOAT `MessagesTimeline.tsx`, the person's bubble at 70% of the lane on `--message-surface`, `rounded-2xl`; the work log at 12.5/20 in `--secondary-label` |
| `02-menu-light-1280`, `11-menu-dark-1280`, `03-menu-keyboard` | the Session actions menu, by pointer and by keys | Paseo's menu: surface1, borderAccent, radius 8, rows 28 with a surface2 highlight |
| `04-archive-dialog-light-1280`, `12-archive-dialog-dark-1280`, `05-after-escape` | the archive confirm, and Escape closing it | Paseo's dialog: at most 520 wide, radius 12, a scrim at 55% |
| `22-session-light-390`, `23-menu-light-390`, `24-archive-sheet-light-390` | the same at a phone's width: the menu and the confirm as bottom sheets | Paseo's sheet: radius 16, a handle, a scrim at 45% |
| `31-shapes-answered-end-dark-1280`, `32-shapes-table-thought-dark-1280` | before the fixes below: the code line wrapped into four ragged rows, the table framed and filled (the component layer's default) | — |
| `34-table-code-link-dark-1280`, `37-table-code-link-light-1280` | the table with rules under its rows only and a semibold header; the code line whole, its language and copy button at the top; the link in blue | GOAT `index.css` `.chat-markdown table`, `thead th`, `.chat-markdown pre`, `.chat-markdown-codeblock`; `MarkdownCodeBlock.tsx` |
| `35-code-scrolled-right-dark-1280`, `36-wheel-over-code-scrolls-timeline-dark-1280` | the code block scrolled sideways with its corner in place; the wheel's vertical turn over it moving the timeline | GOAT `.chat-markdown pre { overflow-x: auto }` |
| `38-command-output-opens-at-end-light-1280`, `39-output-box-scrolled-up-light-1280` | 2,400 lines of output in a box of 192, opened at its end, scrolled up by the wheel while the timeline stays | GOAT `CommandOutputPanel.tsx` (`max-h-48 overflow-auto`, `followRef`, `CopyRegion`) |
| `40-diff-in-a-capped-box-light-1280` | a 75-line diff of 240-character lines in the same box, numbered, washed red and green | GOAT's diffs (`@pierre/diffs`, `bars` indicators) |
| `41-permission-long-command-dark-1280`, `42-permission-long-command-light-390` | a permission request whose command is one long line: it wraps, the window keeps its width | the page's permission panel |
| `43-table-scrolls-not-breaks-light-390`, `44-code-block-and-korean-dark-390` | a phone's width: the table keeps its words and scrolls sideways; the code block, the link, Korean | GOAT `.chat-markdown table` ("the min column width is the longest word, and only wider content scrolls") |

## Differences from the references, and why

- **The lane.** GOAT's chat lane is 672 wide; the app uses the lead's one
  column (820, inset 16) for the timeline and the composer alike, so the
  two never disagree.
- **The phone's width.** The kit's breakpoint is 768 (the lead's); Paseo's
  compact layout starts under 600.
- **Korean line breaks.** A Hangul word may break between syllables, as
  in GOAT (its chat Markdown sets no `word-break: keep-all`).
- **Code blocks** (`kit/src/chat_markdown.rs`): the language sits beside
  the copy button at the right, not alone at the left, and the button is
  always shown rather than on hover; the renderer gives a caller one
  corner of the block, not a header row. No wrap toggle, no "run in
  terminal" yet, and no syntax colours (the renderer's highlighter needs
  a grammar feature the kit does not build).
- **Tables**: a table's outer cells keep their inset; GOAT's sit flush
  with the prose.
- **A diff's sign** is the bar's colour at the line's left, not a `+` or
  `-` (marks are drawn, never typed); GOAT's diff renderer offers the same
  `bars` style.

## Measured

Debug builds on this machine (dev-host, ten workers sharing it):

- Folding 20,001 records in batches of 256: 311 ms in all, the longest
  batch 7.7 ms (`twenty_thousand_entries_fold_in_batches`).
- Find over 20,000 entries: the first search 1.11 s (it builds the
  index), later ones a median of 15.5 ms.
- Scrolling: `VESSEL_APP_SCROLL_BENCH=<entries>` scrolls the timeline by
  itself, 40 pixels a frame for 600 frames, and logs the median, p95 and
  longest frame. Not run yet.

## Gates

At each slice, in `spikes/native`: `cargo fmt --all --check`, `cargo clippy --all-targets
-- -D warnings`, `cargo test`, over the whole workspace. For the testkit
change in the main workspace: `cargo fmt --all --check`, `cargo clippy
--workspace --all-targets -- -D warnings`, `cargo test -p vessel-testkit`
only.

## Problems met

- Real replies broke screens that short fake texts never did (the
  owner's report on build 0.0.0.0.5): hence the `shapes` turn and the
  fleet's check, so the shapes cannot quietly shrink again.
- The component layer's Markdown wrapped wide code into ragged rows and
  had no copy: replaced by GOAT's block over the same renderer. Making
  the block scroll took two tries: a block that only scrolls sideways
  measures its text at the block's width, so it had nothing to scroll
  (five pixels) and its corner moved with the code; scrolling both ways
  takes the renderer's scrolled branch, which pins the corner, and a row
  lets the unwrapped text be as wide as its longest line.
- Passing a style to the renderer made tables white and links dark in the
  dark theme: the renderer folds the style onto one it derives from the
  component layer's theme each layout, and that theme's table, code and
  link colours had never been set from the tokens. Every colour a reply
  shows is now refined in `chat_markdown` or set from the tokens
  (`field::apply_tokens` sets the link's).
- A command's output was cut at 240 pixels and could not scroll; a diff
  ran past the window's edge. Both are now in `output`'s box.
- Opening a run of tool calls reset the list and threw the view to the
  session's start; it now splices the run's rows where they are.
- A permission request with a long command widened the whole main area
  past the window: GPUI measures a text's least width unwrapped, and the
  area next to the sidebar had no `min_w(0)` (`app/src/shell.rs`, the
  lead's; one line). Its answers now wrap on a phone; the longest answer
  is still one line wider than the panel there (the lead's button does
  not wrap its words).

## Still to do

The title's rename in place, the branch chip, find's bar, the outline
rail, read marks drawn, the ended banner, the context ring, the
inspector's Summary, the terminal view (§7), stream banners; copying a
message and a conversation (a tool's text and a code block have their
copy button); a screen-reader buffer; the in-app scroll bench run on
20,000 entries;
then rewind, fork, cross-session search, export and import, written
down.
