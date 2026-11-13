# Milestone 12, track C: the conversation in the native app

A structured session's conversation, brought from the page into the app:
the timeline with every entry kind, the title bar and its Session actions,
find, the outline, read marks, the ended banner, the context ring, the
inspector's Summary, and the terminal view (inventory sections 6 and 7);
then what goes beyond the page (copying, a screen-reader buffer) and,
written down only, rewind, fork, cross-session search, export and import.
Branch `backend/m12-app-conversation`, off the desktop lead's
`backend/m12-spike`.

The look is GOAT's chat (`apps/web/src/components/chat/` in the GOAT reference checkout,
the reference notes' Appendix G), the controls Paseo's (Appendix P);
neutral colours come only from `tokens.neutral.*`.

## Where the code is

| Part | Where | What it holds |
| --- | --- | --- |
| Timeline logic | `view/src/timeline/` | records folded into entries (`fold`), runs of tool calls (`runs`), the rows drawn (`rows`), the page's window arithmetic (`window`), find (`find`), the outline (`outline`), approvals (`approval`: the buttons of a request and what each says; `permission`: what the permission card shows, its title, plan or command, primary answer, marks and count), a question's form (`question`: its steps, drafts, checks, the answer and the answered lines, the page's `question-form.ts` and `question-answers.ts`), the context meter (`meter`), diffs (`diff`) |
| Session logic | `view/src/session/` | read marks (`marks`), titles and names (`titles`) |
| Terminal logic | `view/src/terminal/` | the input sender and its queue (`sender`), the resize rule and the replay log (`frame`) |
| Words | `view/src/text/conversation/` | the area's sentences, English and Korean (`lang.conversation()`) |
| Node calls | `link/src/session.rs` | a session's acts (`Act`: rename, mark, start again, delete, kill, move profile, terminal input, a change to the queue, an answer to a question), each on its own task |
| Kit controls | `kit/src/virtual_list.rs`, `menu.rs`, `dialog.rs`, `popup.rs` | the long list, Paseo's menu and dialog (a sheet on a phone), the popup surfaces |
| Kit, GOAT's chat surfaces | `kit/src/chat_markdown.rs`, `output.rs`, `copy.rs` | a reply's Markdown (code blocks that scroll sideways with a language and a copy button, GOAT's tables, links in the theme's blue); a tool's text and diffs in a box capped at 192 that draws only the lines in view; the copy button both use |
| Screens | `app/src/areas/conversation/` | the title bar (`bar`) and its rename in place (`rename`), find (`find`), the outline rail (`outline`), a permission's card (`permission`), a question's card (`question`) and the queue (`queue`), all in the lead's slot above the composer card (`dock`), the timeline (`timeline`), the entries (`entries`, `tool`), the look (`look`), the scroll bench (`bench`) |

Tests: `view/tests/page/` replays the page's own cases (72 in the page
binary at `70e9cf2`), with a 20,000-entry fold among them.

## Real shapes

Screens are checked against fake-acp's `shapes` turn (`200de4c`): a long
thought, Korean first, streamed a few characters at a time (174 chunks,
the first two one syllable each, as finely as a real stream can cut
Hangul); Markdown with a table, a 240-character code line, a list, a
link and Korean; a read of a long path; an edit with a wide diff; a
command on one long line asking for permission with four answers, and
2,400 lines of output. `scripts/fleet.sh start` makes two copies on
`laptop` (`shapes` answered, `shapes-asking` still asking) and fails if
either lacks the shapes; the verb's own tests hold the output's line
count, which `vessel log` does not print.

## Captures

In a local folder, not committed, each beside the GOAT or
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
| `45-renamed-in-place-light-1280`, `46-empty-title-refused-light-1280` | the title renamed where it stands (the bar and the sidebar take the node's answer at once); an empty title refused in the bar | the page's `session-title.tsx` |
| `47-agent-log-folded-full-dark-1280`, `48-agent-log-opened-full-dark-1280`, `49-agent-log-hidden-collapsed-dark-1280` | a Claude Code session's start-up log (the adapter's `[session/create] …` lines): one quiet row "Agent log · 12 lines" at the "Full" display, opening to the lines in a text box; nothing at all while tool calls are collapsed | Paseo and GOAT never show an adapter's log; the page draws every line raw (a defect, below) |
| `50-find-wrapped-to-last-dark-1280`, `51-find-korean-light-390` | find: Mod+F opens the bar over the conversation's top right with the count, previous, next and close in its shell; Shift+Enter past the first match wraps to the last (5/5); on a phone it spans the width | GOAT `ThreadFindBar.tsx` (`top-3 right-3`, `w-[min(17.5rem,100%-1.5rem)]`, "1/…" while counting, the count red with no results) |
| `52-find-marks-current-dark-1280`, `53-find-current-in-last-reply-light-1280`, `54-find-marks-in-table-dark-390` | find's matches marked in the text, the current one amber and scrolled to (inside a reply taller than the view, to its line); a search run again as the session grows keeps the current match (1/1 became 1/2) | GOAT `threadFindHighlights.ts`, `index.css` `::highlight(goat-thread-find…)`; the page's `find-bar.tsx` refresh rule |
| `55-outline-pointed-dark-1280`, `56-outline-jumped-to-prompt-dark-1280`, `57-outline-pointed-korean-light-1280`, `58-outline-hidden-on-phone-dark-390` | the outline rail on the fleet's five-turn session: the lit prompt's pill wider, the pills near the pointer magnified, the prompt's card (two lines, Korean too), a click putting that prompt at the view's top; no rail under 918 | Paseo `agent-stream/chat-outline/rail.web.tsx`, `model.ts`, `hover-intent.ts` |
| `59-queue-dark-1280`, `68-queue-light-1280`, `70-queue-phone-dark-390`, `71-queue-phone-light-390` | the fleet's `queued` session: three prompts waiting behind a running turn above the composer, each a row with a grip, its text on two lines (an ellipsis past them), and round buttons for edit, steer, send now and remove; on a phone the buttons on a line of their own | Paseo `composer/index.tsx` `QueuedMessageRow` (surface1, a hairline, radius 8, 12 by 8, buttons of 32 on surface2, send on the accent); GOAT `QueuedRunsControl.tsx` (grip, steer, remove, edit in place) |
| `60`–`62` (edit), `63-drag`, `64-dropped`, `65-removed`, `66-steered`, `67-sent-now` | a queued prompt edited in its row in the kit's field and saved (the node's revision r0 to r1); a row dragged by its grip onto the first (the drop row outlined, the dragged text following the pointer) and the order the node then holds; a row taken out; a row steered into the running turn (the fake echoes "steered: …"); the last sent now, which stopped the turn ("Turn ended · cancelled") and started its own | GOAT `QueuedRunsControl.tsx`; the node's `session.queue` |
| `72-queue-held-ko-dark-1280` | a held queue (`vessel session queue … hold`): "대기 중 · 3개 · 멈춤" with resume and clear; resume lets it move (`[queue: 3 waiting]`, no longer held) | the node's `QueueHold` |
| `73-before-thought-ko-dark-1280`, `74-after-thought-ko-dark-1280`, `75-thought-open-ko-dark-1280`, `76-thought-ko-light-390` | the folded thought row in Korean: "생가ㄱ" before, "생각" after; the thought opened, its Korean paragraph (streamed in 174 chunks) whole | — |
| `77-questions-dark-1280`, `84-questions-ko-light-1280`, `86-questions-ko-dark-390` | Claude's `AskUserQuestion` with three questions (fake-acp's `ask questions`, the real adapter's form): a tab per question by its header, the question, its options as radio rows with their descriptions, the "Other..." box, Dismiss and Next; the timeline's line "Waiting on an answer"; on a phone the buttons stacked and the options scrolling past 280 | Paseo `components/question-form-card.tsx` (surface1, a hairline, radius 8, 12 inside; tabs 28 high, radius 6; options 12 by 8, controls of 18; buttons 8 by 12, radius 6, a glyph of 14) |
| `78-picked`, `79-step2`, `80-checks-picked`, `81-step3`, `82-answered` | a pick (its row on surface2, the radio filled), Next to the question that chooses several (checkboxes, an Other answer typed beside two picks), the tab of an answered question checked, Submit on the last; the agent's answer `{"question_0":"Start a new entry","question_1":["Unit tests","Clippy with every lint"],"question_1_custom":"and the fleet check","question_2":"한국어"}`; the settled card listing each answer by its question | the page's answered card (`question-entry.tsx`) |
| `83-one-question-dark-1280`, `85-one-question-light-390`, `87-dismissed-dark-1280` | one question (`ask one`): no tabs, the message as the question, Submit at once; Dismiss declining it (the card then says "Skipped") | Paseo's lone question ("a single 1 of 1 tab carries no information") |
| `88-permission-ko-dark-1280`, `89-permission-en-light-390`, `90-skip-ko` | the permission card on a Korean and an English screen: the "once" answers in the app's words (수락, 거부; Accept, Deny), the "always" answers in the agent's own words, their scope; the question card's 건너뛰기 | Paseo's Accept and Deny (`agentStream.permission`) |
| `permit/01-long-command-en-dark-1280`, `permit/02-long-command-ko-light-390` | the permission card rebuilt as Paseo's: `surface1`, a hairline, radius 8; "Permission Required" (the adapter's title was the command itself), the command once, as `$ command` in the fixed-width box; "How would you like to proceed?"; bordered answers with a check or a cross, the primary in the foreground, the agent's "always" scopes in its words; the timeline only says "Waiting on permission" and the tool row stays closed | Paseo `agent-stream/view.tsx` `PermissionRequestCard`, `PermissionActionButton`, `permissionStyles` |
| `permit/03-plan-ko-dark-1280`, `permit/04-plan-en-light-390` | Claude Code leaving plan mode: Paseo's plan card (a chevron of 16, "Plan", the plan's Markdown) over the same footer; five "always" answers in the agent's words, "Accept" and "Deny"; on a phone the card scrolls inside 440 | Paseo `components/plan-card.tsx` |
| `permit/05-codex-en-light-1280`, `permit/06-codex-ko-dark-390` | Codex's five answers for `["bash", "-lc", …]`, read as its script; long answers wrap inside their buttons on a phone | `codex-acp` `commandDecisionOptions` |
| `permit/07-two-ko-light-1280`, `permit/08-two-en-dark-390` | two requests waiting: GOAT's "1/2" at the card's top right | GOAT `ComposerPendingApprovalPanel` |
| `permit/09-sending-en-dark-1280`, `permit/10-failed-en-dark-1280` | an answer going (the node paused): the kit's spinner in the pressed button, the rest not answering; the connection then lost: "Could not send your response. Try again. (the connection to the node closed)" in the card, every answer ready again | Paseo's `isResponding`, `message.question.responseFailed` |
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
- **The outline** is Paseo's rail (the page's is a card at the window's
  side from 1,100 wide): shown in a conversation at least 918 wide, as
  Paseo's. Its pills change size at once (Paseo eases over 140 ms), only
  a pointer entering a slot starts the 150 ms wait (Paseo also starts it
  on a sideways sweep).
- **The title's rename**: no "Rename" tooltip on the title (the kit has
  no tooltip yet), and the keys do not go back to the title once the
  field leaves (the page returns focus to it, as a dialog to its opener).
- **Find**: the bar is GOAT's width (280), not the page's (320); on a
  phone it stands above the conversation instead of over it (below).
  Matches are marked in a person's messages and in replies (GOAT's
  yellow, the current one amber); in a reply the current one keeps the
  reply's own ink, as its renderer paints backgrounds only (GOAT darkens
  it). A question's text is searched but not marked yet. The wrap is said
  only to a screen reader, as on the page (kept for the screen-reader
  buffer).
- **The queue** is Paseo's row with GOAT's actions as more of Paseo's
  round buttons. Edit happens in the row and keeps the place (GOAT; Paseo
  moves the text back into the composer and loses the place); it is
  checked by the node against the revision it was made from. Steer shows
  only while a turn runs and the agent takes one (`abilities.steer`).
  Send now interrupts a running turn, as its own button, apart from
  steer. On a phone the four 40-point buttons go on a line under the
  text (Paseo keeps one line, having two). Not yet: GOAT's keys on the
  grip and its steer shortcut, a queued message's attachments. The page
  (`backend/m11`, `features/session/queue-dock.tsx`) follows
  agent-orchestrator's `QueuedMessageDock` instead: a count with a
  chevron that folds the list to its first row, 40-pixel rows of one
  line, one button that is Steer or Send now by the agent, edit in the
  composer, and a held queue offering Resume only.
- **A question's card** is Paseo's, in the slot over the composer (where
  the permission panel also stands) rather than at the end of the
  conversation, where Paseo draws it: its option rows therefore scroll
  inside it past 360 (280 on a phone) instead of with the conversation.
  The question and its options are plain text (Paseo renders their
  Markdown); option previews are not shown (Claude's `preview` travels
  under `_meta`, which the node does not keep); no spinner while the
  answer goes, the buttons dim. As in Claude Code and Paseo, picking a
  single choice clears its Other box and typing there clears the pick
  (the page keeps both, as a note). Next and Submit wait for each of
  Claude's questions to be answered (Paseo's rule, though Claude's form
  requires nothing); another agent's form is held to its own `required`.
  A yes-or-no field is two radio rows (Paseo has none). The node's own
  check (`Question::check`) runs before the answer goes. The page
  (`backend/m11`) walks the same steps after agent-orchestrator's dock.
- **The permission card** is Paseo's, value for value (the owner on the
  old one: "허가 여부 ui 디자인 개떡같네 paseo나 goat 보고 배워"), in the
  conversation's slot over the composer. Kept apart from Paseo: a title
  that is the command itself (Claude Code's adapter titles a shell call
  so) falls back to "Permission Required", so the command is read once,
  in its box; the box is the kit's (GOAT's cap of 192, Paseo's 200); a
  plan scrolls inside past 320 (200 on a phone) and the card past 520
  (440), where Paseo's scroll with the conversation; the primary answer
  is the one Approve would take (ACP marks none, Paseo then marks none).
  The answer buttons are the kit's `card_button`, which the question
  card's Dismiss and Submit now use too.
- **A permission's buttons** (the coordinator's rule, `timeline::answer_words`):
  a "once" allow or reject offered once says the app's words (Paseo's
  Accept and Deny, 수락 and 거부); an "always" option keeps the agent's
  name, which is the scope granted ("`cargo test` commands", "this
  session"); a "once" kind offered several times keeps the agent's names
  (Codex's two rejects differ: carry on, or say what to do instead); and
  if two buttons would still say the same, all keep the agent's names.
  Paseo shows the agent's names on ACP throughout. The card itself is the
  lead's (`areas/session.rs`, one line changed to ask `Asked::buttons`).
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
  longest frame. On 2026-11-13, debug build, Xvfb with Mesa's software
  Vulkan (lavapipe) at 1280x800, load average about 8:

  | Session | Entries (rows) | Median | p95 | Longest |
  | --- | --- | --- | --- | --- |
  | `entries 20000` | 20,007 (13,341) | 63.9 ms | 79.9 ms | 152.8 ms |
  | `entries 40` | 47 (34) | 64.0 ms | 79.6 ms | 208.1 ms |

  The frame costs the same at 47 entries as at 20,007: the length of the
  conversation adds nothing measurable, and the 64 ms is the software
  renderer's and the debug build's floor on this machine, not the list's.
  A frame time to hold the app to needs a release build on a real GPU
  (the release track's machines); this measurement only shows that the
  long list does not add to it.

## What the page gets wrong

For the page's worker; each found while bringing the same thing to the
app, and checked against the page as `backend/m11` has it (`67bf243`).

- **An adapter's start-up log drawn raw.** A real Claude Code session's
  ACP adapter writes a dozen `[session/create] sessionId=… phase=…`
  lines to its error output before the first prompt; the page draws each
  as a line of the conversation at both display settings
  (`features/session/timeline-item.tsx`, `case "log"`). Paseo and GOAT
  never show it. The app folds a run of them into one row at "Full" and
  leaves them out while tool calls are collapsed (`view::timeline::logs`).
- **The find bar hides its own match on a phone.** `.find-bar` lies over
  the conversation's top at every width (`find-bar.css`); at a phone's
  width it spans the column, and a match revealed at the top of the view
  sits under it. The app puts the bar above the conversation on a phone.

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
- A real Claude Code session opened with a dozen raw lines of its
  adapter's start-up log (`[session/create] sessionId=… phase=settings
  …`, the owner on build 0.0.0.0.7): the node keeps an adapter's error
  output as `HarnessLog` records, and each was drawn as a line of the
  conversation. The page has the same defect: it draws every log line
  raw at both display settings (`timeline-item.tsx`, `case "log"`). In
  the app a run of them is now one folded row at "Full" and nothing
  while tool calls are collapsed (`view::timeline::logs`); the fake
  agent writes the same start-up log (`FAKE_ACP_STARTUP_LOG`, on for the
  fleet's Claude Code) and the fleet checks its ten lines come before the
  first prompt. The fakes had never written such lines, which is why the
  real shapes missed it.
- The folded thought row read "생가ㄱ" for "생각" (the lead, in the fake
  fleet). The text was whole: the label is the area's own word, and a
  streamed thought is whole at every chunk (ACP's text is JSON, so a
  chunk never ends inside a character, and a Hangul syllable is one).
  The font was not: the work row, a run's header and the meta lines set
  a size but no face, and with no face above them in the window they
  had no Pretendard behind Geist; the system's fallback drew the Hangul,
  and at 12.5 points it split "각" into "가" and "ㄱ". The kit gains
  `TypeExt::face` (a type's family and weight with Pretendard behind it,
  the size left to the element); the timeline, those rows, the outline's
  card and the kit's mono boxes (`output`, a code block's language) now
  set it, where `font_family(MONO)` had set the family alone. The shapes
  turn now streams its thought, Korean first, a syllable alone in its
  first chunks; the fleet puts it back together from `vessel log` and
  fails on a chunk count under 100, a sentence not whole, or a Hangul
  letter loose from its syllable; a view test folds the same stream in
  batches of 1, 2 and 7 and checks the text at every step. The window's
  root sets no face either (the lead's `shell.rs`): any area's text
  without one still has no Pretendard.
- Opening a run of tool calls reset the list and threw the view to the
  session's start; it now splices the run's rows where they are.
- A permission request with a long command widened the whole main area
  past the window: GPUI measures a text's least width unwrapped, and the
  area next to the sidebar had no `min_w(0)` (`app/src/shell.rs`, the
  lead's; one line). Its answers now wrap on a phone; the longest answer
  is still one line wider than the panel there (the lead's button does
  not wrap its words).

## Still to do

The branch chip
rail, read marks drawn, the ended banner, the context ring, the
inspector's Summary, the terminal view (§7), stream banners; copying a
message and a conversation (a tool's text and a code block have their
copy button); a screen-reader buffer;
then rewind, fork, cross-session search, export and import, written
down.
