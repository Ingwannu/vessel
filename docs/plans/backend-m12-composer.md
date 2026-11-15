# M12: the session's composer (track P)

Track P builds the composer's rows of the feature tables in
`docs/plans/backend-m11.md` for the native app, after the lead's plan
`docs/plans/backend-m12-composer-controls.md`. The reference for each is
Paseo's composer (`packages/app/src/composer/` and what it imports), value
for value; GOAT where Paseo has nothing. Every deliberate difference is
named below with its reason.

| Slice | Row | State |
| --- | --- | --- |
| a | Change the model, effort or permissions of a running session | built; permissions rows marked as Paseo's |
| — | An agent's lists kept and refreshed behind the controls (owner word 28) | built |
| b | `/` the agent's commands | built (a running session's composer and the new chat's) |
| c | Attach images and files | built in both composers (c1 rules and uploads, c2 the screen); sent messages' pictures next (c3) |
| d | `@` a file of the project | not started (track F's seam) |
| e | Drafts kept per session; Up and Down recall sent messages | built |

Code: `view/src/composer/` (the rules, with tests), `app/src/areas/composer/`
(the wiring), `view/locales/{en,ko}/composer.ftl` (the words). The pills,
their lists and the effort card stay the draft's (`app/src/areas/draft/`,
the lead's): the session's composer draws the same controls.

## Slice a: options on a running session

Followed: Paseo `composer/agent-controls/index.tsx` (`AgentControls`) and
`composer/agent-controls/mode-control.tsx` (`useLiveAgentModeControl`);
`utils/provider-notice-toast.ts`.

What Paseo does, and so the app:

- The same pills as the new chat, on the session's options as its agent
  last reported them (`options_changed`, folded by the conversation). Only
  for a live structured session whose agent reported options; the pills
  are not drawn otherwise (nothing shown may do nothing).
- A choice is sent at once (`session.set_option`). Nothing is blocked while
  the agent is in a turn: whether a change applies mid-turn is the agent's
  to say, and its refusal is what the person sees. The pill keeps the
  reported value until the agent's own report of the change arrives.
- A refusal is an error toast (Paseo's `toast.error`), wherever the person
  is now, not the conversation's bar: the change was the composer's.
- Each choice is also kept as that agent's default for a new chat on that
  machine (Paseo's `mergeProviderPreferences`), at Paseo's moment: the
  model once the agent took it (`setAgentModel` awaited, then kept), the
  effort, the permissions and any other option as they are sent, whatever
  the answer. The kept list is the one the new chat already remembers
  (`Prefs::keep_choices`), so Settings › Agents shows the same defaults.

Deliberate differences:

- **The toast's words.** Paseo shows the error's own message. The app's
  rule is that the toast's body is the app's sentence for the node's code
  (`view::composer::running::Refusal`, English and Korean), never the
  agent's original words; those stay in the log until the shared mapping
  of codes (the lead's slice) gives them a tooltip or a copy. The link now
  carries a node's refusal as its own message, not the client's wrapping.
- **A refused effort, mode or feature stays the new chat's default**, as in
  Paseo (it keeps those preferences before the change is sent, whatever
  the agent answers); only a model is kept once taken.
- **A read-only device** gets its own sentence (the node answers
  `Forbidden`); Paseo has no read-only devices.
- **Matching answers to changes.** The link answers an act with its kind,
  not its choice, and runs each act on a task of its own: two changes in
  flight on one session are matched to their answers oldest first
  (`InFlight`, at most 16 remembered). Only what is kept as the default
  after a model change can be wrong in that race (the next new chat's pill
  shows it before anything is sent).

The permissions list and its closed control draw each mode's mark, as
Paseo's `ModeComboboxOption` and `getAgentModeIcon` do: the marks are
Paseo's manifest's (`packages/protocol/src/provider-manifest.ts`), by the
agent's mode id (`view::composer::modes`), drawn in the foreground beside
the mode's name, with no description cut beside it; a mode without a mark
in a row has none, and the closed control falls back to the agent's mark.
Two stated differences, both for rows Paseo never shows:

- **Claude Code's `dontAsk`** is not in Paseo's manifest; here it takes
  `ShieldAlert`, in its row and in the closed control, so Claude Code's
  list is marked on every row and the pill never shows the agent's mark
  while its row shows a shield.
- **A list in which any row has a mark keeps the mark's column on every
  row** (the kit's list), so labels share one left edge: Codex's
  `read-only` has no mark beside its marked modes. Paseo's manifest marks
  every mode it lists, so it never meets the case.

Seams in others' files (the owners agreed):

- `app/src/shell.rs` (the lead's): the act answers go through
  `option_acted` after the board's (one expression).
- `app/src/areas/mod.rs` (the lead's): one `mod` line and one field.
- `app/src/areas/draft/mod.rs` (the lead's): the lead's `wip` commit
  `1c05f689` (the controls on a running session) merged; its target type
  and the target's helpers moved to `areas/composer/running.rs`;
  `choose_option` calls `change_running_option` on a session.
- `app/src/areas/draft/pickers.rs`: the permissions rows' and pill's
  marks (`areas::composer::modes`); `effort_card.rs` and `session.rs`: the
  lead's `wip` lines as they are.

Not done in this slice:

- Paseo's provider notice (a warning or info toast a provider returns with
  a successful change): the node's `session.set_option` answers the options
  only; there is no notice on the wire.
- Paseo's profiles on a running agent (apply or edit an agent profile):
  Vessel's profiles are chosen when a session starts.

Checked (fake fleet, the app paired as a person pairs, in both themes and
languages at 1280 and 390): the pills on a running Claude Code session with
three prompts queued, the effort card, the permissions list (a sheet on the
phone), a change shown once the agent reported it, and a refusal mid-turn
(Codex's fake refuses `locked`). Captures outside the repository.

A read-only device's book (`ROOT/readonly-config`) first showed its machine
unreachable in the app. That was the test's setup, not the link: a book
paired through the CLI names its credential under `~`, and the app ran with
another `HOME`. With the fleet's `HOME` it connects; the read-only refusal
toast is checked by the view's tests.

## Slice b: the agent's `/` commands

Followed: Paseo `hooks/use-agent-autocomplete.ts`,
`utils/agent-command-autocomplete.ts` (`findActiveSlashCommand`,
`filterAndRankCommandAutocompleteEntries`,
`applySlashCommandReplacement`), `hooks/use-autocomplete.ts`,
`components/ui/autocomplete.tsx` and `autocomplete-popover.tsx`,
`packages/protocol/src/search/text-match.ts` (`compareMatchScores`).

What Paseo does, and so the app (`view::composer::{slash, matching}`,
`app/src/areas/composer/slash.rs`, the kit's `autocomplete`):

- The menu shows while the prompt starts with `/` and the caret is in the
  command's name (no space, quote or second `/` before it).
- Every command in the agent's order for a bare `/`; otherwise the
  commands whose name matches every word typed, by tier (whole name, whole
  word, start, a word's start, anywhere, in order), then where the match
  starts, then name. Descriptions are not searched.
- The best row is drawn last, nearest the text, and is chosen when the
  menu opens or what is typed changes; Up and Down move the choice and
  wrap; Enter and Tab write `/name ` over what was typed; Escape closes the
  menu and empties a prompt it opened at the start; a click writes the
  row's command. With no row, every key is the text's (Enter sends what
  is typed).
- The chosen command's card above the list: its name, what it does and
  the input it takes; the empty line "No commands found".
- The menu is never taller than the room above the card, less 12 from
  the top (`autocomplete-popover.tsx`'s `maxHeight`): the list scrolls
  within it and the chosen command's card keeps its height. Paseo counts
  from the top of its window; the app counts from under its own title
  bar, so the menu never covers it (at 800 by 600, thirty commands left
  the window's top before this).

The commands are the session's own report (`commands_changed`); until it
arrives, its agent's last session's report, kept in the store and on the
device with the agent's options (`offers`: a session's report wins over a
kept copy and is kept when it differs). The new chat's menu lists the kept
report of the agent it would start on its machine.

Deliberate differences:

- **The new chat lists the kept commands.** Paseo's new chat asks its
  daemon for the commands of the agent it would start; the node reports
  commands only from a running session (`harness.options` carries none),
  so the new chat's menu lists what that agent's last session on that
  machine said, kept on the device, and says so when none has yet
  (`composer-commands-unknown`). Each composer keeps its own choice,
  scroll and glide. The menu is drawn over the draft's card through two
  seam lines in `app/src/areas/draft/mod.rs` (the lead's).
- **At the prompt's start only.** Paseo also completes a skill's `/name`
  in the middle of a prompt; ACP does not say which commands are skills.
- **Motion** (the owner's word 39; Paseo's popover appears at once): the
  menu rises 4 points into place and fades in over the kit's `STANDARD`
  (0.3 s, a surface coming and going), fades out with its last rows, and
  the chosen row's fill comes in over `QUICK` (0.15 s, a control's own
  change) when the choice moves, both on the kit's one curve
  (`vessel_app_kit::motion`); at once when motion is held (reduced motion,
  the app's animations off, a software renderer: `motion::held`)
  (`composer::reveal`; frames are asked only while it moves).

Checked: the fake fleet's Claude Code session (`compact` with a hint,
`review`) at 1280 and 390, both themes and languages, open, filtered, the
choice moved, no match, closed; the kit's example scenes in both themes
(long names and descriptions, no rows, thirty rows past 220 high, the
choice kept in view). Captures outside the repository. Idle cost on the
software renderer, a session's composer focused, with the caret steady
while motion is held: 6 ms of CPU per second on a release build, with
the menu closed and with it open (16 to 19 on a debug build). Before the
caret was steady the window redrew twice a second for its blink (440 to
525 ms a second, debug); the open menu asks for no frames of its own.

Found while capturing, fixed: GPUI keeps a chosen row in view from the
last frame's layout, so a list nothing else redraws stayed at its top on
its first frame; the kit's list now asks for one more frame until the row
has been laid out.

## An agent's lists, kept and refreshed (owner word 28)

The owner: keep the lists, refresh them in the background, update when
they changed, and show them the moment a control is clicked. One store for
what an agent on a machine offers (its models, efforts, permissions modes
and on/off options), read by the draft's controls, the model list's agent
rows and a running session's list.

Followed: Paseo's provider snapshot. `data/provider-snapshot-cache.ts`
keeps the last snapshot per host on the device (format version 2, 4 MiB,
oldest dropped first); `hooks/use-providers-snapshot.ts` hands it to the
query as its first data and refetches when a selector opens
(`selectorOpenRefetchDecision`: always when the selected provider has no
entry, else when stale); `provider-selection/provider-selection.ts`
(`buildEntryModelSelection`) shows models whenever any exist, the loading
state or the error only when none; a remembered choice no longer offered
falls back to what is shown (`resolveEffectiveComposerModeId`; here the
agent's own value, `with_picks`).

How it behaves (`view::offers`, tests in `view/src/offers/tests/`):

- A control opens on the list there is, in the same frame, however old;
  the first time (no list) it is Paseo's loading state, and a first
  failure is Paseo's error with its words.
- A refresh is asked when a control opens or the draft settles on an
  agent, when a machine connects (its kept lists, each asked again), and
  when a machine's agents change. At most one request per agent is in
  flight; an open does not ask again within 30 s of a good answer or 10 s
  of a failed one; a request unanswered for 60 s no longer holds the next
  back. Ten opens are one request.
- An answer that arrives while one of the draft's lists is open is laid
  over the shown list (`arrange`): what stays keeps its place, what is new
  goes at the edge away from the one the list is anchored by, in the
  agent's order (a new value of a group already shown next to that group,
  so a group is drawn once), what is gone leaves; names, descriptions and
  the value in effect are the answer's. The composer's lists open above
  their control, or as a sheet from the bottom, so they grow upward: new
  rows go at the top and every shown row stays where it is on screen. A
  closed list takes the agent's order.
- The keyboard highlight follows its row (by its label) when rows are added
  or removed above it (the kit's list), and is dropped when its row goes.
- A failed refresh keeps the list and says once, quietly, above the rows,
  that it may be out of date (`composer-offers-stale`).

Kept on the device (`link/src/offers.rs`): one JSON file in the app's state
folder (`$XDG_STATE_HOME/vessel/agent-options.json`; `%LOCALAPPDATA%\Vessel`
on Windows; `~/Library/Application Support/Vessel` on macOS), owner-only
(0600 in a 0700 folder, checked by a test), versioned (another version, or
a file that does not read, is dropped and rebuilt), at most 64 lists and
1 MiB, oldest first. Entries are keyed by the node's id from its welcome,
the agent and its profile, never by a book's name, so two machines or two
books never see each other's lists. It holds the agents' option ids, names,
descriptions and values, nothing of a node's address, token or credential.
It is written only when an answer differs, and handed to the app when a
machine connects (`Event::OptionsKept`, before the fleet that shows the
machine online).

Deliberate differences:

- **Order while a list is open.** Paseo replaces the data; its desktop
  model list has a fixed height and keys rows by id, so a new row does not
  move the popover. The kit's list is as tall as its rows and opens above
  its control, so a row added anywhere but the top moves every row above
  it up by one (the first capture of this store showed it: "Fake XL"
  appended at the end lifted the whole list). New rows go at the top
  while the list is open. A value removed, or one added to a group shown,
  still moves the rows on its far side by one.
- **The not-fresh line.** Paseo says nothing when a refetch fails with data
  present; the owner asked for it to be said once, quietly.
- **No timer.** Paseo's snapshot also updates by a pushed event from its
  daemon; Vessel's node has none for an agent's options, so the refresh
  moments are the ones above.
- **No folder in the key.** Paseo keys a snapshot by host and folder (a
  provider's list may follow a project's settings); `harness.options`
  takes no folder, so a list is the agent's on that node whatever the
  folder.
- Opening the effort card refreshes too (the lead's line, at the merge of
  slice b), as opening the model list from it or any other option list
  does.

Seams in others' files: `app/src/areas/draft/mod.rs` and `pickers.rs` (the
lead's) read and ask the store in place of their own answers (`Probe` is
gone); `app/src/shell.rs` takes the kept lists and tells the store of a
fleet whose agents changed; the kit's `ComboHeader` has a `note` (filled
with `None` at each other list); `link/src/prefs.rs` lends its owner-only
file helpers.

## Slice e: drafts and recall

Followed: Paseo `stores/draft-store/` (a draft per composer, kept on the
device), `composer/recall.ts` (`ComposerRecall`) and
`stores/session-store.ts` (`selectAgentSentMessages`); the bounds are the
page's (`data/drafts.ts`) and its bridge's (`keep/drafts_doc.rs`).

What the app does (`view::composer::{drafts, recall}`,
`link/src/drafts.rs`, `app/src/areas/composer/drafts.rs`):

- What is typed in a session's composer stays that session's draft, and
  the new chat's its own; another session opening puts the one open's
  text away and brings the other's back. A sent prompt empties the
  composer, and so its draft: a session's when it is sent, the new
  chat's once its session has started (until then a failed start's text,
  put back in the composer, is still the draft). The emptying is written
  to the draft by the app itself: the field reports only what is typed
  (gpui-base's `set_value` turns its events off), which at first left a
  sent prompt behind as the draft, back after a restart (found in slice
  c's captures).
- Kept on the device a second after the last edit (one write for a burst
  of typing): `drafts.json` beside the agents' lists, owner-only (0600 in
  a 0700 folder, checked by a test), versioned. Read once at start; the
  new chat's draft is in its composer on the first frame. A file of
  another version, or one that does not read, is set aside as
  `drafts.json.unread` (the text is the person's: never overwritten) and
  the app starts with none.
- Bounded as the bridge bounds the page's: at most 64 drafts, none older
  than 30 days, each at most 128 KiB, 1 MiB in all, the oldest dropped
  first. A longer draft is kept for this launch only, and the log says so
  (its size, never its text).
- Up in an empty session composer shows the last message sent in the
  session, Up again the one before, Down the next newer, then the empty
  composer; a message equal to the one after it, or empty, is skipped.
  The caret is at the recalled message's end, as Paseo leaves it, so
  within a recalled message of several lines the arrows move between its
  lines first. The first edit gives the arrows back to the caret. A
  recalled message is not a draft until it is edited, and a `/name` at
  its start does not open the command menu (it is sent as it was). An
  open `/` menu takes Up and Down before recall does.

Deliberate differences:

- **Text only.** Paseo's draft also holds its attachments; attachments are
  slice c, and join the draft when they are built.
- **No lifecycle.** Paseo marks a draft sent or abandoned to tell its
  create flow apart; the app has no such flow, so an emptied composer is
  simply no draft.
- **Bounds.** Paseo's store has none of its own; the app writes its own
  file and takes the page's bounds so it cannot grow without end.
- **The new chat has no recall**: it has no sent messages.

Checked: the view's rules and the file's (round trip per composer,
owner-only modes, a file set aside, a missing file) by tests.

## Slice c: attachments

Followed: Paseo `composer/index.tsx` (`renderAttachmentTray`, the image,
file and labelled pills), `components/attachment-pill.tsx` (a body 48
high, an image 48 by 48, a label 260 wide at most, the remove button 24
across at the top left), `composer/input/input.tsx` (the attach button and
its menu: an image, a file); for what Vessel's node needs, Vessel's own
page (`web/src/data/attach.ts`, `tray.ts`, `upload.ts`), which settled the
same node.

The node already has every call (protocol 10, ADR 0025): `attachment.begin`,
`write` and `finish` send a file in pieces to a live structured session,
`attachment.read` reads one back, and a prompt names a sent file as a
`ContentBlock::Attachment`. Nothing new on the wire.

How it behaves (`view::composer::{attach, tray, trays}`, `link::attachments`):

- A file's type follows its extension, the CLI's table; its kind is the
  node's (`AttachmentKind::of`); its name is made one the node takes (no
  `/ \ : * ? " < > |` or control character, no leading dot or space, not a
  Windows device, at most 255 bytes cut before a short extension).
- A file is checked when it is attached, or once the composer has a session
  whose agent said what it takes: images and audio only for an agent that
  takes them, nothing empty, nothing past the node's size for its kind, at
  most 16 a prompt and 25 MiB of images and audio. A refused file stays as
  a pill with its reason and stops Send until it is removed; removing a
  file looks at the refused ones again.
- Files are uploaded when Send is pressed, two at a time, each a piece at
  a time from what the node says it has; an image whose first bytes are
  not its type's is refused before anything is sent. A failed upload stops
  Send; a retry goes on from what the node has, an upload the node dropped
  is begun again, a file that changed or that the node judged wrong is not
  tried again. The prompt goes, text first and then each file, when every
  file is in.
- Each composer keeps its tray while the app runs (a session's, the new
  chat's), sixteen at most, the least recently used dropped first; the new
  chat's becomes its session's once the session exists.

On screen (`app/src/areas/composer/{attach,files}.rs`, the kit's
`attachment`):

- The attach button (Paseo's `+`, 28 across, 44 on touch) leads the
  composer's left group and opens Paseo's menu: Add image, Upload file
  (the system's file dialog). Files dropped on the card (its edge takes the
  accent while files are dragged over it) and images or files pasted into
  the text join the tray too.
- The tray sits over the text: an image as its 48 by 48 picture, any other
  file as its name over its extension; the remove button at the top left
  shows on hover (always on touch). A file on its way shows its progress
  along its pill's foot; a refused or failed one has a red edge and says
  why in its second line, and a click on a failed one tries it again.
- Send in a session's composer with files takes the text with them: the
  composer's text empties, the message shows in the conversation, and the
  prompt goes once every file is in. Until then the files stay in the
  tray with their progress, and Send takes nothing more (what is typed
  meanwhile stays): a second send would take the waiting prompt's place.
  Nor does Send go while a file is refused (the send button is off).
  Send in the new chat with files starts the
  session without its prompt; once the session exists its tray is the
  session's, once its agent has said what it takes (its first records)
  the files go, and the prompt follows.
- A sent message shows its files as Paseo's user message does
  (`components/message.tsx`: `UserMessageImagePill` in a row, then the
  other files' `AttachmentLabel` in a row, each row 8 apart that wraps,
  8 above the text), in the kit's `attachment_frame`
  (`app/src/areas/composer/sent.rs`, through the rows' `Paint`). An image
  is its picture once its bytes are in, an empty frame until then (Paseo's
  placeholder), and its name over its kind when it could not be read.
- An image's bytes are read back from its node (`attachment.read`, a
  mebibyte at a time, `link/src/attachments/read.rs`) when its message is
  drawn, by the page's rules (`web/src/data/images.ts`, kept in
  `view::composer::sent`): two reads at a time, the most recently drawn
  first; one read per image however often it is drawn; at most 48 pictures
  or 64 MiB kept, the least recently drawn dropped; a failed read not
  tried again while kept. An image pasted here is kept from the bytes the
  composer held when its prompt went, never read back; a file picked on
  this device is read back (it may have changed since).

Deliberate differences:

- **Uploaded at Send.** Paseo keeps a picked file on the device and sends
  it with the message in one call; Vessel's node takes a file only into a
  live session, in pieces, before the prompt names it.
- **One waiting prompt a session.** Paseo empties its composer, files and
  all, at Send and can take the next message at once; here the waiting
  prompt's files are the composer's tray until they are in (the tray is
  the composer's, not the message's), so the next send waits for them.
  Files added in that time go with the waiting prompt.
- **Removing a file looks at the refused ones again** (the page keeps them
  refused until its session changes).
- **Sent images read when drawn.** Paseo's message carries its images'
  data with it; Vessel's record names the attachment, so the app reads it
  back, and only for messages on screen.
- Image viewing is completed by the attachment parity extension
  (`wp-a8.md`): both tray and sent-image frames open the same zoomable
  overlay. Failed upload pills keep their retry action.
- **The menu's rows have no icons** (the kit's menu has none), and both
  rows open the same dialog: GPUI's file dialog takes no type filter.
  Paseo's issue, pull request and plugin rows are not Vessel's yet.
- **The node's limits are this build's.** The app does not read a
  conversation's own limits; the node checks again at `attachment.begin`
  and its refusal shows on the pill.
- An ended session is resumed without a prompt before its attachments
  upload; the pending text follows the files. Resume failure keeps the
  tray and restores the text (`wp-a8.md`).


Not seen on screen: the three ways a file reaches the composer besides an
image pasted (the system's file dialog, a file list pasted, files dropped
on the card). Under the capture display (Xvfb) GPUI's dialog needs the
desktop portal, which is absent; GPUI's X11 clipboard does not read a
file list (`text/uri-list` is not among its formats); and a drop reaches
GPUI (its `XDND_DATA` is read, the drag source is told it succeeded) but
the card's `on_drop` and `drag_over` did not run. They wait for a run on a
real desktop.

### Captures: a screen behind its keys (measured, not a defect)

Two c2 captures showed keys handled but not drawn: typed text missing
from the screen until the next pointer event, and a Return whose effect
showed only after a click. One capture's bubble read its text twice: the
first typing was in the composer but not yet on screen, so it was typed
again, and the app sent what the composer held (one text block; the fake
agent's echo has no space between the two copies). The capture was
retaken with the text typed once.

No state change lacks a notify. The composer's text is its model, kept as
the draft (`drafts.json`), and the model held every key while the screen
did not show them; the next frame showed them all. GPUI draws a dirty
window before it dispatches each key (`Window::dispatch_key_event`) and
presents only when the X events of that wake are handled (the 60 Hz
refresh timer). A burst of keys therefore costs one draw a key before
anything is presented. Measured on the capture display (software
renderer), 24 keys typed in about 0.3 s:

| build | main thread CPU | screen 0.6 s after the last key |
| --- | --- | --- |
| debug | 1.42 s and 1.61 s (about 60 ms a key) | before the burst |
| release | 0.15 s and 0.16 s (about 6 ms a key) | after the burst |

The capture host adds a second stall: the debug binary (over 400 MB) is on
a spinning data disk under heavy I/O pressure (`/proc/pressure/io` "some"
about 55 %), and code run for the first time waits for its pages: the
main thread was seen waiting on the disk (`folio_wait_bit_common`) for
12.5 s while a cold binary started and for 0.6 s when a session first
opened. Replays of both captures' paths, with the binary's pages cached,
drew every key at once.

So a capture waits until the app's main thread has been idle in its event
loop (`ep_poll`, ten samples 50 ms apart) before it is taken.
