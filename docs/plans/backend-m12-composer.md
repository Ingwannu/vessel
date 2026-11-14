# M12: the session's composer (track P)

Track P builds the composer's rows of the feature tables in
`docs/plans/backend-m11.md` for the native app, after the lead's plan
`docs/plans/backend-m12-composer-controls.md`. The reference for each is
Paseo's composer (`packages/app/src/composer/` and what it imports), value
for value; GOAT where Paseo has nothing. Every deliberate difference is
named below with its reason.

| Slice | Row | State |
| --- | --- | --- |
| a | Change the model, effort or permissions of a running session | built |
| b | `/` the agent's commands | view logic written; needs the composer hooks |
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
  rule is that a person reads the app's words in their language: each
  refusal has a whole sentence of its own by the node's code
  (`view::composer::running::Refusal`), in English and Korean. Where the
  agent's own words carry the reason (it refused in its state; the
  connection failed), they follow the sentence after a colon, as the
  board's refused restart does; they may be English in a Korean sentence.
- **A read-only device** gets its own sentence (the node answers
  `Forbidden`); Paseo has no read-only devices.
- **Matching answers to changes.** The link answers an act with its kind,
  not its choice, and runs each act on a task of its own: two changes in
  flight on one session are matched to their answers oldest first
  (`InFlight`, at most 16 remembered). Only what is kept as the default
  after a model change can be wrong in that race (the next new chat's pill
  shows it before anything is sent).

Seams in others' files (the owners agreed):

- `app/src/shell.rs` (the lead's): the act answers go through
  `option_acted` after the board's (one expression).
- `app/src/areas/mod.rs` (the lead's): one `mod` line and one field.
- `app/src/areas/draft/mod.rs` (the lead's): the lead's `wip` commit
  `1c05f689` (the controls on a running session) merged; its target type
  and the target's helpers moved to `areas/composer/running.rs`;
  `choose_option` calls `change_running_option` on a session.
- `app/src/areas/draft/pickers.rs`, `effort_card.rs` and `session.rs`: the
  lead's `wip` lines as they are; no further line of P's.

Not done in this slice:

- Paseo's provider notice (a warning or info toast a provider returns with
  a successful change): the node's `session.set_option` answers the options
  only; there is no notice on the wire.
- Paseo's profiles on a running agent (apply or edit an agent profile):
  Vessel's profiles are chosen when a session starts.
