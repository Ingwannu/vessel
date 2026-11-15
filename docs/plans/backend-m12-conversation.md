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
| Search logic | `view/src/search.rs` | every machine's messages searched for the palette's Messages group: each machine's pages and state, the rows newest first with the ones shown kept in place, More, and the notes under the group |
| Terminal logic | `view/src/terminal/` | the input sender and its queue (`sender`), the resize rule and the replay log (`frame`) |
| Words | `view/src/text/conversation/` | the area's sentences, English and Korean (`lang.conversation()`) |
| Node calls | `link/src/session.rs`, `search.rs` | a session's acts (`Act`: rename, mark, start again, delete, kill, move profile, terminal input, a change to the queue, an answer to a question), each on its own task; a machine's page of a search (`Command::Search`, answered by `Event::Searched`; a machine before protocol 12 answers `Refusal::Older` without being asked) |
| Kit controls | `kit/src/virtual_list.rs`, `menu.rs`, `dialog.rs`, `popup.rs` | the long list, Paseo's menu and dialog (a sheet on a phone), the popup surfaces |
| Kit, GOAT's chat surfaces | `kit/src/chat_markdown.rs`, `output.rs`, `copy.rs` | a reply's Markdown (code blocks that scroll sideways with a language and a copy button, GOAT's tables, links in the theme's blue); a tool's text and diffs in a box capped at 192 that draws only the lines in view; the copy button both use |
| Screens | `app/src/areas/conversation/` | the title bar (`bar`) and its rename in place (`rename`), find (`find`), the outline rail (`outline`), a permission's card (`permission`), a question's card (`question`) and the queue (`queue`), all in the lead's slot above the composer card (`dock`), the timeline (`timeline`), the entries (`entries`, `tool`, and `plan`: an agent's plan as Paseo's task list), the context meter left of the send button (`meter`, in the slot the lead's composer keeps for it), read marks (`read`: the session shown marked read while it is in front), an ended or archived session's banner first in the slot over the composer (`ended`), the palette's Messages group (`search`: its seams, and an example host until the palette draws it), the look (`look`), the scroll bench (`bench`) |

Other areas use the conversation through seams, each one call:

| Seam | For | The call |
| --- | --- | --- |
| Another area's conversation | a side question's panel (track N5), drawn as GOAT's `SideChatSurface` embeds its chat view | keep a `ConversationArea` of your own; `open(machine, session)`, then `take(session, records)` for its records; draw `conversation::timeline::rows(shell, seat, tokens, window, cx)` inside an element with an id of its own, where `seat` is a `Seat { get, get_mut }` of two functions from the shell to that state (the session area's is `Seat::SESSION`); the rows' toggles change the state at `seat` |
| A session's requests answered elsewhere | the inbox (track N1) answering in place | keep a `ConversationArea` at a `Place` of your own (a type with `const SEAT: Seat`; the question card's text box takes plain functions, so each place gets its own copy of them); open it, feed it the session's records and what the node says waits (`Act::Conversation`'s answer into `live`); draw `conversation::dock::requests::<P>(shell, tokens, window, cx)` (the permission card, then the question card) inside an element with an id of its own; a place sends the answers its own way (`Place::answer`, by default the session's act; the inbox through its own asking) and may hold every answer back while one of its own goes (`Place::busy`); a place holding a request without its session's log gives the tool call it keeps (`Place::tool`, by default the conversation's fold), and a live request is named by that call's title as the fold names one |
| What waits when its record is gone | a session whose log was pruned before the first record read | on opening a session the area reads `session.conversation` (`Act::Conversation`) and keeps its requests and questions (`view::timeline::Live`); a card draws one the log holds no record of (`Timeline::unseen_permissions`, `unseen_questions`), with no entry made for it; read again after an answer goes |
| Find from the shortcuts | the app's rebindable shortcuts (track S2) | `Shell::conversation_find(window, cx)`: opens the find bar in the session shown, or gives its field the keys again; nothing when no session shows. The area's own Mod+F observer goes once the shortcuts bind Find (one press opening twice does no harm meanwhile) |
| A machine renamed or forgotten | Settings › Machines (track E2), once the node book took the edit; a failed edit calls nothing | `Shell::conversation_machine_renamed(from, to, cx)`: the session shown on `from` follows the new name, so its acts go to it; `Shell::conversation_machine_forgotten(name, cx)`: a session shown on it is closed here, its state dropped, nothing stopped or deleted on the machine. Another area's own conversation (a side question, the inbox) is that area's to follow |
| A session's usage line and a limit notice | usage and limits (track N2), drawn by its own renderer | two lines it adds first in the slot over the composer (`dock.rs`'s `dock`, before the requests): its session line (the node's totals for the session shown: tokens, cache, the cost the agent reported), then its limit notice; both read the session shown (`shell.areas.conversation.shown`: machine and session) and return `Option<AnyElement>`, and the slot shows when either does. A turn's own token count stays on this area's turn divider, and the context meter stays this area's |
| A session marked unread elsewhere | a sidebar row's or a board card's "Mark as unread" (track S2) | `Shell::conversation_marked_unread(machine, session)`, called as the mark is sent: when it is the session shown, the open screen does not mark it read again until a record comes after what it shows; another session, nothing. The Session actions menu here is its first caller |
| Whether an ended session can go on | the composer (the lead's), to offer no send when it cannot | `Shell::conversation_cannot_resume()`: the session shown has ended and its agent cannot pick up where it left off (the banner says so and has no Start again); `false` while it runs, can start again, or is not yet described |
| The palette's Messages group | the command palette (track S3): its group after Paseo's command groups, entered by its shortcut and the sidebar's Search row | `Shell::message_search_query(text, cx)` as the words change (empty when it closes; searched once they rest 150 ms); `Shell::message_search_rows()`, the rows top to bottom as typed ids (`view::search::Row::Hit(HitId)` or `Row::More`); `conversation::search::message_row(shell, row, active, compact, tokens)` draws one (no handlers: the palette's keys and pointer choose); `message_search_note(shell, tokens)` the lines under them; choosing a hit calls `Shell::open_search_hit(hit, window, cx)` (the palette closes first), More `Shell::message_search_more(cx)`. Until the palette lands, `VESSEL_APP_MESSAGE_SEARCH=<words>` draws the group in a card over the session shown (`search::example`) |
| Open at a record | the inbox opening a session at what it lists | after the session area opened the session, `Shell::conversation_reveal(seq, cx)`: the record's entry is brought to the top once it is read, the first row when the log starts after it |
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

## Slice M: the conversation as a message exchange (2026-11-14)

The owner on 0.0.0.0.13 (words 33, 36–39 of the common brief): the
session view as Paseo's, sending as sending a message, the reply a
bubble beside the agent's mark, quick motion. Read for it: Paseo's
`components/message.tsx` (`UserMessage`, `AssistantMessage`,
`AssistantTurnFooter`), `assistant-turn-footer-label.ts`,
`assistant-fork-menu.tsx`, `agent-stream/turn-footer.tsx` and
`agent-stream/view.tsx` (the stream, its pending message), the
composer's `submit.ts` (a failed send), `types/stream.ts` (what a stream
holds) and `docs/agent-lifecycle.md` (an agent is closed, never ended
for the person: the next prompt resumes its provider session).

Sending: a message that starts a turn (no turn running, or a session
that ended, was interrupted or released) shows at once as the person's
message after the entries (`tail`), with Paseo's working footer under
it (a 14-point spinner, the elapsed time once the turn's prompt is
in); an ended session is started again underneath. When the node
records the prompt the pending row gives way to it. A failed send puts
the words back in the composer with the error under the conversation
(Paseo's send error). A send into a running turn goes to the queue or
joins the turn, as before.

What left the timeline, and where each fact is now:

| Line | Now |
| --- | --- |
| "Session starting / running / exited / interrupted / failed · time" | not drawn; the session's state is the header's chip (nothing while it runs) |
| "Agent ready" | not drawn (Paseo has nothing); the working footer ends when the agent answers |
| "Agent released while idle", "Agent resumed" | not drawn; the header's "Released" chip and its tooltip ("Send a message to resume.") |
| "Model call · route · model" | not drawn (gateway accounting) |
| "Turn ended · reason · tokens · in · out · cache" | Paseo's turn footer: the reply's copy button and "Worked for 14s" ("작업 시간 14s"), the end time while pointed at; a turn that did not end normally adds its reason ("· cancelled"); the session's tokens and cost are the context meter's card; a turn's own tokens are no longer shown |
| the ended card ("Session ended … Start again") | gone for a session that can go on (the next message starts it again); kept for an archived session (Paseo's archived callout) and for one whose agent cannot go on |
| the held message box and "Restarting the session…" (the composer's) | the message in the conversation and the working footer |
| "released" on a sidebar row and a board card | gone: an idle session reads idle, as in Paseo; the header keeps it |
| "Agent log · N lines" rows between turns (the adapter's stderr) | the Session actions' "Agent log · N lines" (shown once the agent wrote a line), opening a dialog with every line in one box, copyable; Paseo shows no adapter log in its stream |
| the rule under a folded run's summary (it read as a rule between turns) | gone; Paseo's summary row (`agent-stream/turn-collapse/summary-line.tsx`, its `ExpandableBadge`) has no rule, and its stream draws none between turns |

Kept in the timeline: messages, thoughts, tool rows, plans, questions,
permissions, a turn that failed (its words: the person must see them),
a profile change (the person's own act).

A failed turn (the coordinator's finding on 0.0.0.0.15, after Paseo's
`formatTurnFailedMessage` and its Claude provider's `withheldErrorReply`)
is one sentence in the kit's error alert under the replies: the agent's
own words without the JSON-RPC wrapper ("Internal error: ") or its data
as JSON (`view::timeline::turn_failure`). A usage limit (`errorKind`
`rate_limit`, or the limit's sentence) is its sentence alone; any other
failure reads "Turn failed · <sentence>". The reply Claude writes with
the same words just before the failure is not drawn, so it is said
once. The full text stays in the session's log (`vessel log`).

The reply: the owner's bubble (Paseo's reply has none): Paseo's user
bubble mirrored (16 inside, radius 16 with the corner toward the agent
at 4) on `surface1` edged with a hairline. It hugs its words as the
person's bubble does (a two-word reply is a two-word bubble) and grows
to the column at most: a wide table takes the column, its cells wrap,
and what still does not fit scrolls sideways inside the bubble (800 ×
600: the five-column table of the fleet's `shapes` turn); a long code
line scrolls sideways inside its block; the bubble never passes the
column. Every reply takes it the same way (the first
after a session is started again, a replayed conversation, a reply
still streaming: all are the timeline's message entries); beside a turn's first reply a 28-point circle
with the session's agent's mark (worker B's star-to-circle morph is to
be mounted in that circle). The person's bubble takes Paseo's values:
`surface3`, 16 inside, the corner toward the writer at 4, as wide as
the column needs.

Motion (`conversation::motion`, on the kit's tokens
`vessel_app_kit::motion`): two one-shot transitions, both `STANDARD`
(300 ms on `cubic-bezier(.23,1,.32,1)`: a message arriving is a
surface, by the lead's rule). A message sent is drawn whole on its
first frame and rises 6 points into place (a fade would leave Enter's
next frame nearly empty); a reply fades and rises in. Only what just
arrived moves, so nothing replays on scrolling and an idle session
draws nothing for it; while motion is held (Animations off, reduced
motion, a software renderer) GPUI draws both at their end at once. The
working footer's elapsed time is drawn again once a second while a turn
runs, and not otherwise.

## Send while a turn runs: Paseo's rules (2026-11-14, the owner's word 45)

`view::session::send_delivery` decides how a plain send goes, from
Settings' Default send and three facts (a turn runs, the agent takes a
message mid-turn, a permission waits). Paseo's rules, its cases as the
tests in `view/src/tests/delivery.rs`:

| Default send | No turn running | Turn running | Permission waiting |
| --- | --- | --- | --- |
| Steer (the default now, Paseo's `storage.ts`) | starts a turn | joins the turn; replaces it when the agent cannot steer (Paseo's server, `steerOrReplaceActiveTurn`) | joins the turn |
| Queue | starts a turn | waits in the queue | interrupts (Paseo's `resolveActiveSendBehavior`: a queued message would be stranded behind the request) |
| Interrupt | starts a turn | stops the turn, starts the next | stops the turn, starts the next |

Differences from before: the default was Queue, and a Steer the agent
could not take waited in the queue. A send that does not wait in the
queue shows in the conversation at once (the tail; a steered message
settles on its steer record). Settings' notes under Default send say
the new rules; the setting's page is track E3's. An install that saved
"queue" before keeps it (the preferences store every field, so a saved
default cannot be told from a choice).

## Rewind and fork (protocol 14, ADR 0052; started 2026-11-14)

Plan: Paseo's controls on our node's calls. Paseo puts **rewind** in
the trailing row under the person's message (its time, an undo mark,
copy; `components/message.tsx` `UserMessage`, `rewind/rewind-menu.tsx`):
a menu headed "This action cannot be undone" with one row per mode the
agent supports, and the rewound message goes back into the composer
(`rewind/composer-restore.tsx`). It puts **fork** in the reply's turn
footer between copy and "Worked for" (`assistant-fork-menu.tsx`). Our
node rewinds and forks only at a turn's end the agent named (a point,
`TurnPoint`), rewinds the conversation only (files are not put back;
the record says how many tool calls changed them), and forks into a new
session (no tabs or workspaces).

| Slice | What | Reference |
| --- | --- | --- |
| R1 | the person's message's trailing row (time, rewind, copy); rewind to the turn end before it (`session.rewind`), the rewound message back in the composer; the entries rewound away drawn no more (they stay in the fold and the log: entries are never removed) | Paseo `UserMessage`, `RewindMenu` (conversation mode) |
| F1 | the turn footer's fork menu; `session.fork` at that turn's end; the new session opened; its first line says where it came from | Paseo `AssistantForkMenu` (one target: a new session) |
| F1b | the fork shows its origin's conversation up to the fork point above its first line, read-only, the line their divider; a removed origin leaves the line with a plain sentence | Paseo carries the context as a draft attachment only ("Chat history" / "Previous conversation"); ours draws the origin's real turns |
| F2 | a fork that never had a conversation (`empty_fork`): its reason and Remove | ADR 0052 addendum (track A2) |

Each slice: rules in `view` with tests, captures at 1280, 390 and
800 × 600, the gate, a report.

R1 done (2026-11-14; 9b8eadfb, 92ef3ed9, ba96e64a): on the fake fleet a
rewind to the first turn's end left the second turn behind (no longer
drawn, nor on the outline rail, nor found), the node logged "rewound to
record 21", and the message went back into the empty composer.
Differences from Paseo, kept: one mode (the conversation); the row's
height is kept while it is hidden (Paseo's trailing row also keeps its
place). The undo mark's tooltip ("Rewind to this message", the kit's
tooltip; none on a phone, as Paseo's) followed R1 on 2026-11-14. The
fake agent names a point only for its `plain` reply, so the fleet's
other sessions offer no rewind.

F1 done (2026-11-14): a turn's footer whose end the agent named shows
Paseo's `Split` mark between copy and "Worked for" (tooltip "Fork chat
from here"); its menu's one row "Fork in a new session" asks
`session.fork` at that turn's end; the new session is listed at once
(`Fleet::add_session`) and opened on the next frame, its first line
"Forked from “plain” · today 18:55" (the source's title from the list,
`Fleet::title_of`); a refusal is the notice's ("Failed to fork chat:
…"). Rules in `view::timeline::fork_point` with tests
(`view/tests/page/fork.rs`). On the fake fleet at 1280, 390 and
800 × 600 the fork opened as "plain (fork)". Differences from Paseo,
kept: one target (a new session; Paseo's are a tab or a workspace);
the menu closes when chosen and the mark stays locked until the answer
(Paseo keeps the row pending); no fork from the working turn (the node
refuses a whole-conversation fork while a turn runs). The fork showed
only its own records until F1b.

F1b (2026-11-14, the coordinator: "a forked session that opens empty
hides from the person exactly what the agent remembers"): once the
fork's `forked` line is folded, the origin's log is read from its start
with `session.read` (one act, `Act::Log`, page by page off the screen's
thread) and kept up to the fork's record (`view::timeline::Until`; a
whole-conversation fork, up to when it was made). It is folded as its
own conversation (`origin.rs`: its own rows, runs and disclosures,
reached through `Seat::ORIGIN`) and drawn as the list's first rows,
the "Forked from" line under them as a divider. Inherited turns carry
no rewind or fork mark; copy stays. The session area's list ops add
the origin's row count (`ConversationArea::at`); find and the outline
rail cover the fork's own turns. A removed origin leaves the line and
"The conversation it was forked from was removed." What Paseo shows:
no earlier turns; its fork is a draft whose composer holds one
attachment pill, title "Chat history" (`activity-curator.ts`), subtitle
"Previous conversation" (`attachment-pill-content.tsx`), the text the
agent receives. Ours are the origin's real turns, as the agent has
them. An archived origin's log is still on the node (`session.read`
answers it), so its turns show as any other origin's; only a removed one
(NotFound) leaves the line alone. Measured on the fake fleet (debug
build): a long origin of 162 records (122 rows) read in 281 ms off the
screen's thread, folded in 0.5 ms; the fork's own rows show before it.
Captures: `shots/m/100`–`109` (1280, 800 × 600, 390; both themes; en
and ko; long, removed and archived origins).

While the origin is read, a quiet loading row (the kit's skeleton, one
line) sits above the "Forked from" line, so the turns do not appear
without a cue on a slow link (`shots/m/110`, the node held with
SIGSTOP). A read that fails for any reason other than NotFound (the
connection lost, the node refusing) leaves the line and "The
conversation it was forked from could not be read: … Open the session
again to try again."; opening the session again, or another session
and then this one, reads it again.

F2 done (2026-11-14): a fork that never held a conversation (its
status `failed` with `empty_fork`, ADR 0052's addendum) has a banner
over the composer in the ended banner's callout: "The fork did not
start", why in the person's words (`view::timeline::empty_fork`: the
node's "the agent did not fork at the chosen point" → "The agent did not
fork at the chosen point. Nothing here can go on; remove it and fork
again."; "cannot fork" likewise; any other start failure in the node's
words), and Remove (`session.delete`, no confirm: nothing is lost). Once
removed, the session it was forked from opens (where the person forked
from), when the list holds it; a refused Remove is said under the
banner. An empty fork reads no origin, and drops one read before it
failed; its composer sends nothing. Its records decide (the timeline's
last status), before the node's description is read again. On the fake
fleet (`FAKE_ACP_FORK_IGNORES_POINT=1` on the laptop node for one fork,
then back): `shots/m/112` (en dark 1280), `113` (ko light 800 × 600),
`114` (en light 390), `115-*` (Remove, then the origin open). Paseo has
no empty fork: its fork is a draft until sent.

## Module layout (2026-11-14, the owner's word 44)

`app/src/areas/conversation/` after the split (no behaviour change;
every file under 460 lines). Rules with tests live in `view`; `app`
draws and wires.

| File | Holds |
| --- | --- |
| `mod.rs` | the module's Decision Logs, its file list and re-exports |
| `area.rs` | `ConversationArea` (the state), opening, clearing, revealing, the rows |
| `intake.rs` | an act's answer and a batch of records, applied only for the session shown |
| `place.rs` | `Seat` and `Place`: where a conversation's state is kept, how a card's answer is sent |
| `actions.rs` | the Session actions menu, its confirms, the agent's log, the acts sent |
| `follow.rs` | what the shell tells the conversation (opened, records, answers, lost, machine book) |
| `entries.rs` / `bubbles.rs` | an entry drawn by kind / the message exchange (the two bubbles, the circle) |
| `question.rs` / `question/card.rs` | the answer's state and controls / the card's drawing; the step's rules are `view::timeline::question_step` |
| `queue.rs` / `queue/rows.rs` | the queue's edit and acts / its rows, grip and buttons |
| `find.rs` / `find/bar.rs` | find's state, steps and keys / its bar; where it starts is `view::timeline::start_at` |
| `rewind.rs` | the person's message's trailing row (time, undo, copy), the rewind menu, the act, the message back into the composer; where it can go is `view::timeline::rewind_point` |
| `fork.rs` | the turn footer's fork mark and menu, the act, the new session listed and opened; where it can go is `view::timeline::fork_point` |
| `origin.rs` | a fork's origin: read, folded as its own conversation, drawn as the list's first rows; the "Forked from" divider; where the read stops is `view::timeline::Until` |

Moved to `view` with tests: `question_step` (choices, pick, typed,
header, prompt, text target), `start_at`, `first_reply`.

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
| `permit/q-moved`, `permit/q-end-crop`, `permit/q-up-crop`, `permit/q-grip-zoom` | the queue's keys: the first row's grip clicked, Down moves it one place, Down again to the end, Up back; the grip keeps the keys, its mark in the foreground on `surface2` | GOAT `QueuedRunsControl.tsx` (the grip's `ArrowUp`/`ArrowDown`) |
| `60`–`62` (edit), `63-drag`, `64-dropped`, `65-removed`, `66-steered`, `67-sent-now` | a queued prompt edited in its row in the kit's field and saved (the node's revision r0 to r1); a row dragged by its grip onto the first (the drop row outlined, the dragged text following the pointer) and the order the node then holds; a row taken out; a row steered into the running turn (the fake echoes "steered: …"); the last sent now, which stopped the turn ("Turn ended · cancelled") and started its own | GOAT `QueuedRunsControl.tsx`; the node's `session.queue` |
| `72-queue-held-ko-dark-1280` | a held queue (`vessel session queue … hold`): "대기 중 · 3개 · 멈춤" with resume and clear; resume lets it move (`[queue: 3 waiting]`, no longer held) | the node's `QueueHold` |
| `73-before-thought-ko-dark-1280`, `74-after-thought-ko-dark-1280`, `75-thought-open-ko-dark-1280`, `76-thought-ko-light-390` | the folded thought row in Korean: "생가ㄱ" before, "생각" after; the thought opened, its Korean paragraph (streamed in 174 chunks) whole | — |
| `77-questions-dark-1280`, `84-questions-ko-light-1280`, `86-questions-ko-dark-390` | Claude's `AskUserQuestion` with three questions (fake-acp's `ask questions`, the real adapter's form): a tab per question by its header, the question, its options as radio rows with their descriptions, the "Other..." box, Dismiss and Next; the timeline's line "Waiting on an answer"; on a phone the buttons stacked and the options scrolling past 280 | Paseo `components/question-form-card.tsx` (surface1, a hairline, radius 8, 12 inside; tabs 28 high, radius 6; options 12 by 8, controls of 18; buttons 8 by 12, radius 6, a glyph of 14) |
| `78-picked`, `79-step2`, `80-checks-picked`, `81-step3`, `82-answered` | a pick (its row on surface2, the radio filled), Next to the question that chooses several (checkboxes, an Other answer typed beside two picks), the tab of an answered question checked, Submit on the last; the agent's answer `{"question_0":"Start a new entry","question_1":["Unit tests","Clippy with every lint"],"question_1_custom":"and the fleet check","question_2":"한국어"}`; the settled card listing each answer by its question | the page's answered card (`question-entry.tsx`) |
| `83-one-question-dark-1280`, `85-one-question-light-390`, `87-dismissed-dark-1280` | one question (`ask one`): no tabs, the message as the question, Submit at once; Dismiss declining it (the card then says "Skipped") | Paseo's lone question ("a single 1 of 1 tab carries no information") |
| `88-permission-ko-dark-1280`, `89-permission-en-light-390`, `90-skip-ko` | the permission card on a Korean and an English screen: the "once" answers in the app's words (수락, 거부; Accept, Deny), the "always" answers in the agent's own words, their scope; the question card's 건너뛰기 | Paseo's Accept and Deny (`agentStream.permission`) |
| `permit/01-long-command-en-dark-1280`, `permit/02-long-command-ko-light-390` | the permission card rebuilt as Paseo's: `surface1`, a hairline, radius 8; "Permission Required" (the adapter's title was the command itself), the command once, as `$ command` in the fixed-width box; "How would you like to proceed?"; bordered answers with a check or a cross, the primary in the foreground, the agent's "always" scopes in its words; the timeline only says "Waiting on permission" and the tool row stays closed | Paseo `agent-stream/view.tsx` `PermissionRequestCard`, `PermissionActionButton`, `permissionStyles` |
| `permit/03-plan-ko-dark-1280`, `permit/04-plan-en-light-390` | Claude Code leaving plan mode: Paseo's plan card (a chevron of 16, "Plan", the plan's Markdown) over the same footer; five "always" answers in the agent's words (then "Accept" and "Deny" for the rest; since refined, `permit/13`); on a phone the card scrolls inside 440 | Paseo `components/plan-card.tsx` |
| `permit/05-codex-en-light-1280`, `permit/06-codex-ko-dark-390` | Codex's five answers for `["bash", "-lc", …]`, read as its script; long answers wrap inside their buttons on a phone | `codex-acp` `commandDecisionOptions` |
| `permit/07-two-ko-light-1280`, `permit/08-two-en-dark-390` | two requests waiting: GOAT's "1/2" at the card's top right | GOAT `ComposerPendingApprovalPanel` |
| `permit/11-long-command-en-dark-1280`, `permit/12-long-command-ko-light-390` | the long command ends before the copy button: a box with a copy button keeps the button's column, no line enters it (before, the command showed on both sides of the button) | GOAT `CommandOutputPanel.tsx` `pe-7` |
| `permit/13-plan-ko-dark-1280`, `permit/14-codex-ko-dark-1280` | a direction with three answers or more keeps the agent's names: the plan exit's six allows ("Yes, manually approve edits" primary), Codex's "Yes, proceed" beside its two "always" allows; a lone reject is still 거부 | Paseo's agent labels on ACP |
| `plan/01-plans-en-dark-1280`, `plan/04-plans-ko-dark-1280`, `plan/03-plans-ko-light-390` | the fleet's `plans` session (fake-acp's `plans`, Claude Code's shape): a row per change, "Created 4 tasks", "Started", "Completed" as the next starts, "Added" once work began, each with Paseo's mark and the step's words after; on a phone the step cut to a line | Paseo `types/stream.ts` `deriveTaskActivities`; `components/message.tsx` `TodoListCard` |
| `plan/02-plans-open-en-dark-1280`, `plan/06-plans-open-en-light-1280`, `plan/07-plans-open-en-dark-390`, `plan/05-created-open-ko-dark-1280` | a row opened: the plan as it stood then, Paseo's `TaskListRow`s (done struck through, extra muted; under way in the running blue; to do muted rings); the "Created" row holds the four steps it was made with | Paseo `components/task-list-row.tsx` |
| `plan/m-01-amber-en-dark-1280`, `plan/m-02-card-amber-en-dark-1280`, `plan/m-04-red-card-ko-light-1280`, `plan/m-05-room-en-dark-390` | the context meter left of the send button (the fleet's `context-*` sessions, fake-acp's `usage <tokens>`): Paseo's ring at 75% in amber, at 95% in red, at 0.6% in the muted ink; its card: "Context window", the share used, used and maximum tokens, the session's cost, GOAT's "Compact context" | Paseo `components/context-window-meter.tsx`, `context-window-details.tsx`; GOAT `chat/ContextWindowMeter.tsx` |
| `plan/m-03-compact-sent-running-en-dark-1280`, `plan/m-06-room-card-en-dark-390`, `plan/m-07-closed-by-press-outside-390` | Compact sent as the agent's `/compact`; while that turn runs the card holds the button, faded, with its reason; on a phone the card opens on a press and closes on a press outside | GOAT's disabled compact with its reason; Paseo's press-to-open on touch |
| `ended/01-ended-en-dark-1280`, `ended/05-ended-ko-light-1280`, `ended/06-archived-ko-light-1280`, `ended/07-ended-en-light-390` | an ended session (the fleet's `context-ended`, stopped after its turn) and an archived one: "Session ended" or "Session archived", whether the agent can go on, Start again at the right, first over the composer card; on a phone the line wraps beside the button. The lead's own line above it ("This session has ended. Sending starts it again…") goes in its merge | the page's `ended-banner.tsx`; Paseo `components/archived-agent-callout.tsx` (surface1, a hairline of borderAccent, radius 16, 12 by 16 on a phone and 16 by 24 wider, a secondary button) |
| `ended/03-starting-en-dark-1280`, `ended/04-started-en-dark-1280` | Start again pressed: the session runs again and the conversation goes on with its new records (the node ends a follow at the final status, so the session area follows it afresh); the banner goes | the page's `marks.run("start-again")` |
| `ended/02-meter-ended-en-dark-1280`, `ended/08-meter-ended-ko-dark-390` | the meter card on an ended session: "Compact context" disabled with "Compacting needs the session running." (a prompt there would start the session again, not compact it) | GOAT `ChatView.tsx` `compactDisabledReason` |
| `search/01-plans-en-dark-1280` | the palette's Messages group (drawn by `search::example` until the palette lands) for "plans": two messages on `laptop`, "Agent:" in green and "You:" in blue, the matched word semibold in the foreground, the session's title with its machine after it, when the message was written at the right | GOAT `CommandPaletteThreadItems.tsx`, `ThreadSearchMatch.tsx`, `CommandPaletteResults.tsx` |
| `search/02-opened-at-hit-en-dark-1280` | the first hit chosen: its session opens at that message with find on the words, the current match (2/2, amber) the one in the message that matched | GOAT `openThreadFindOnQuery` (find on the word at the matched message) |
| `search/03-more-ko-light-1280`, `search/04-more-row-ko-light-1280`, `search/05-after-more-ko-light-1280` | "paragraph" on both machines: ten rows (build-server's answered first, laptop's below them: a row once shown keeps its place), then "메시지 더 보기"; pressed, the rest show and More goes | GOAT `PALETTE_MESSAGE_LIMIT` (10), `nextShownOrder` |
| `search/06-nothing-en-dark-390` | a phone's width, words nothing matches: "No messages match." | the app's own (GOAT shows an empty group) |
| `permit/09-sending-en-dark-1280`, `permit/10-failed-en-dark-1280` | an answer going (the node paused): the kit's spinner in the pressed button, the rest not answering; the connection then lost: "Could not send your response. Try again. (the connection to the node closed)" in the card, every answer ready again | Paseo's `isResponding`, `message.question.responseFailed` |
| `43-table-scrolls-not-breaks-light-390`, `44-code-block-and-korean-dark-390` | a phone's width: the table keeps its words and scrolls sideways; the code block, the link, Korean | GOAT `.chat-markdown table` ("the min column width is the longest word, and only wider content scrolls") |

## Differences from the references, and why

- **The lane.** GOAT's chat lane is 672 wide; the app uses the lead's one
  column (820, inset 16) for the timeline and the composer alike, so the
  two never disagree.
- **The phone's width.** The kit's breakpoint is 768 (the lead's); Paseo's
  compact layout starts under 600.
- **Message search.** One place, the palette's Messages group, entered
  by its shortcut and the sidebar's Search row (Paseo's command center
  row); GOAT also searches from a field at the top of its sidebar, not
  built here. Rows are ranked by when the message was written (the
  node's order); GOAT also weighs how recently a thread was used, which
  the app does not record. A snippet is the message's own text, Markdown
  marks and all, as GOAT's is. A machine before protocol 12, and one
  that refused, are named in a line under the rows; an unreachable
  machine is left out (the sidebar already says it is offline).
- **The ended banner.** The page's words and Start again in Paseo's
  archived-agent callout. Paseo centres one line ("This agent is
  archived" and Unarchive); this keeps the page's heading and line at
  the left and its Start again at the right. The page's button has a
  restore mark; the kit's button takes words only, so it has none.
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
  it). An answered question's text is marked too; a waiting one is
  answered over the composer and its timeline row holds no mark. The wrap is said
  only to a screen reader, as on the page (kept for the screen-reader
  buffer).
- **The queue** is Paseo's row with GOAT's actions as more of Paseo's
  round buttons. Edit happens in the row and keeps the place (GOAT; Paseo
  moves the text back into the composer and loses the place); it is
  checked by the node against the revision it was made from. Steer shows
  only while a turn runs and the agent takes one (`abilities.steer`).
  Send now interrupts a running turn, as its own button, apart from
  steer. On a phone the four 40-point buttons go on a line under the
  text (Paseo keeps one line, having two). GOAT's keys on the grip: a
  click gives the grip the keys and the arrows move its row one place,
  the grip keeping them as the row moves (shown in the foreground on
  `surface2`). Not yet: a queued message's attachments as thumbnails. The page
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
  Accept and Deny, 수락 and 거부) when its direction (allow, or reject)
  has at most two options; with three or more of one direction every
  option of it keeps the agent's name (a bare 수락 among Claude Code's
  five named ways to leave plan mode did not say it means approving edits
  by hand; Codex's "Yes, proceed" sits beside two "always" allows). An
  "always" option keeps the agent's name, which is the scope granted
  ("`cargo test` commands", "this session"); a "once" kind offered
  several times keeps the agent's names (Codex's two rejects differ:
  carry on, or say what to do instead); and if two buttons would still
  say the same, all keep the agent's names. Paseo shows the agent's names
  on ACP throughout.
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

- Read marks seemed never to go on the fleet: Xvfb with no window
  manager never makes the app's window active, and a mark goes only
  while it is (`Window::is_window_active`, the page's focus rule). To
  check them there, give the window focus first (`xdotool windowfocus
  --sync <window>`); the debug log then says "the session shown marked
  read" and the CLI no longer lists it `[unread]`.
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

In the order the coordinator set (2026-11-13), what this track has not
done yet:

- **The queue**: a queued message's attachments show as their names in
  its text (`[name]`), not as GOAT's 16-point thumbnails, which need the
  node's files read; GOAT's steer and edit-latest shortcuts wait for the
  app's shortcuts (track S2) to bind `Shell::conversation_steer_next` and
  `Shell::conversation_edit_latest_queued`; the area's own Mod+F
  observer goes once they bind Find.
- **The turn's usage line** and a limit notice are track N2's, in the
  two lines it adds over the composer (the seams above); the turn's own
  token count stays on its divider.
- **Chat settings** (Settings › Chat, track E2 draws the page): always
  expand reasoning, the outline shown or not, collapse whole turns; each
  needs a preference and its rule here.
- **What A2 landed** (protocols 12 to 14): message search is done on
  this side, the palette's group waits for the palette (track S3, the
  seam above; the example host goes then). Left: parking an idle agent
  (parked wherever a session shows, a wake button when waking failed,
  the `parked` and `woken` marks in the timeline); then rewind and fork
  (an empty fork offers to be removed: "the agent did not fork at the
  chosen point"), import from an agent's own history and handoff as A2
  sends them.
- Then: the branch chip rail, the
  inspector's Summary, the terminal view (§7), stream banners; copying a
  message and a conversation (a tool's text and a code block have their
  copy button); a screen-reader buffer; export.
