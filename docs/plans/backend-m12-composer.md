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
| b | `/` the agent's commands | kit hooks (`5f2fda69`) and list (`46932bc9`) in; view and screen next |
| c | Attach images and files | not started |
| d | `@` a file of the project | not started (track F's seam) |
| e | Drafts kept per session; Up and Down recall sent messages | not started |

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

How it behaves (`view::offers`, tests in `view/src/offers/tests.rs`):

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
- **Opening the effort card** does not refresh by itself (its file is the
  lead's); opening the model list from it, or any other option list,
  does.

Seams in others' files: `app/src/areas/draft/mod.rs` and `pickers.rs` (the
lead's) read and ask the store in place of their own answers (`Probe` is
gone); `app/src/shell.rs` takes the kept lists and tells the store of a
fleet whose agents changed; the kit's `ComboHeader` has a `note` (filled
with `None` at each other list); `link/src/prefs.rs` lends its owner-only
file helpers.
