# Schedules in the native app

The screen is ADR 0072. This file is the reading notes, the reuse
decision, and where the work stopped. If the two disagree, ADR 0072
wins and this file is corrected in the same commit.

## Resume point

- **Done:** ADR 0072 and this plan. No code. No protocol change. No
  sidebar edit. The slot that opens the screen stays `None`
  (`spikes/native/app/src/areas/sidebar.rs`, `schedules`).
- **Half-done:** nothing. The models are specified and not written.
- **Next:** slice 2, the `link` calls and the `view` models with their
  tests, as ADR 0072's "Models in `view`". Do not start slice 3 in that
  commit.

## What Paseo does (0.15.4)

Read before changing a rule. Line numbers are that tree.

List and filters:

- `packages/app/src/screens/schedules-screen.tsx` — title, host filter
  when there is more than one host (164), Active/Ended (48–51), newest
  created first (152), load error and retry (237–241), empty copy
  (315, 331) and the docs link (317), per-host failure banner (337–343).
  An unfocused screen draws nothing (58–62).
- `packages/app/src/screens/schedules-screen-state.ts` — loading, error,
  empty, ready. A loaded empty list is empty, not an error.
- `packages/app/src/schedules/schedule-derivation.ts` — badge order:
  past `expiresAt`, then a missing agent, then completed, paused,
  active (`deriveState`, 81–96). Bucket: active and paused are runnable
  (98–100). While the agent directory has never loaded, a missing agent
  is not "gone" (`isAgentTargetGone`, 57–63).
- `packages/app/src/components/schedules/schedule-row.tsx` — badge
  labels (78–96), meta order (99–122), row opens the editor (216–220),
  run now only for a new-agent target that is active or paused (195),
  pause/resume/run hidden for an agent target (276–278).
- `packages/app/src/components/schedules/schedules-table.tsx` — pending
  labels; a failed action is swallowed (84); delete confirms (117–128).
- `packages/app/src/utils/schedule-format.ts` — "Heartbeat" for every
  agent target (31–33), title (35–48), cadence sentence (86–96), cron
  phrases (104–154) including a missing zone shown as UTC (133), next
  run thresholds (173–195).

Form:

- `packages/app/src/schedules/schedule-cadence-options.ts` — five cron
  presets (10–16). `every` is rewritten to cron before submit (42–61),
  and an interval that is not a cron-shaped step is clamped (`*/59`,
  `*/23`, `*/31`).
- `packages/app/src/components/schedules/cadence-editor.tsx` — presets
  plus a cron field; the zone is the device zone; preview is
  `describeCron`, computed on the device.
- `packages/app/src/schedules/schedule-form-model.ts` — default cadence
  is every hour (143), which the editor then turns into cron; missing
  zone becomes UTC (144, 649, 862); archive defaults on (682); a new
  session can be saved with a prompt, a project and a model (545–560);
  an agent target can be saved with a cadence alone (545–547).
- `packages/app/src/components/schedules/schedule-form-sheet.tsx` —
  fields Name (493), Prompt (506), target, Cadence, Max runs (537),
  Archive on finish (801–809). `parseMaxRuns` maps blank, zero and
  garbage to no cap (68–71). Edit of an agent target sends cadence only
  (316–331). Create and edit of a new agent send the provider block
  (333–391). Titles "New schedule", "Edit schedule", "Edit heartbeat"
  (404–406). No expiry field. No elapsed-interval field.

Protocol and CLI, where the app is thinner than the product:

- `packages/protocol/src/schedule/types.ts` — status active, paused,
  completed; cadence `every` (milliseconds) or cron; target agent or
  new-agent (`archiveOnFinish`, isolation local or worktree); runs with
  output and error strings; `maxRuns`; `expiresAt`.
- `packages/protocol/src/schedule/rpc-schemas.ts` — create (also target
  `self`), list, inspect (schedule plus every run), logs, pause,
  resume, delete, run-once, update. Create's `runOnCreate`. No revision.
- `packages/protocol/src/schedule/cadence.ts` — `every` to cron only
  when the interval divides an hour or a day; otherwise null. The form
  does not use this; it clamps instead.
- `packages/cli/src/commands/heartbeat/index.ts` — create, update,
  delete, and only while the caller is that agent. Cron is required.
  Update changes cron and zone, not the prompt. A heartbeat is an
  agent target whose agent is the caller.
- `packages/cli/src/commands/schedule/` — create and update also take
  `--every`, `--expires-in`, `--max-runs`, `--run-now`.

There is no run-history view in the 0.15.4 screen. Inspect and logs
are the CLI and the protocol. ADR 0072 puts the run list on the detail
because that is the node's `schedule.runs` and the behaviour the screen
is for, and it does not invent Paseo's output text.

Server bounds worth not copying by accident: Paseo keeps every run;
Vessel keeps 50. Paseo's app derives expired and target-gone; Vessel's
node records `EndReason`. Paseo's form rewrites `every`.

## What Vessel already has

- ADR 0033 and `crates/vessel-proto/src/automation/`: the definition,
  the info, runs, the preview (protocol 13), the text bounds.
- The call table: `crates/vessel-proto/src/rpc.rs` (`schedule.preview`
  at 13, the other `schedule.*` at 11).
- The node: `crates/vessel-node/src/automation/schedules/` (`calls.rs`
  update, pause, run now; `rules.rs` an agent cannot run now, a
  forwarded caller cannot update or run).
- The CLI: `crates/vessel-cli/src/args/automation.rs`.
- The client: `crates/vessel-client/src/schedules.rs`.
- The agent's tools, including `create_heartbeat`:
  `crates/vessel-mcp/src/tools/automation.rs`.
- The empty sidebar slot: `spikes/native/app/src/areas/sidebar.rs`.
- No `view` or `link` module for schedules on this branch. No route.

`docs/ARCHITECTURE.md` does not change: `view` may already depend on
`vessel-proto`, and `link` may already depend on `view` and
`vessel-client`. The native README's crate table does not change for
the same reason.

## Reuse

| Need | Where it already is | Decision |
| --- | --- | --- |
| Ids, cadence, definition, runs, preview errors | `vessel-proto` | Use them. Do not wrap a parsed cron back into a `String` to match on. |
| The calls | `vessel-client` | `link` calls these. No second client. |
| Due times | the node, via `schedule.preview` | Do not link the node's time walker into `view`, and do not add a cron crate. |
| New-session choices | the composer's catalogues and `SessionSpec` | The form builds a `SessionSpec` the composer could build. No schedule-only model list, thinking list or provider list. |
| Rows, menus, fields, badges, confirm | the kit | Slice 3 and 4 draw only with those. A missing confirm is a kit request, not a private dialog. |
| Words | `view/locales/{en,ko}` | Phrase enums in slice 2; both catalogs when the screen first renders them. |
| Earlier `view` code on `backend/m12-app-schedules` | unreviewed, not on this branch | Do not merge it and do not copy it. The notes above were read from Paseo again. |

No new dependency. Nothing in the standard library or the crates above
fails to do this.

## Slices

1. **This commit.** ADR 0072 and this plan.
2. **`link` and `view`, with tests.** Commands and events for the ten
   calls. Models as ADR 0072. Tests beside them: badge and bucket for
   every state and end reason; `Unknown` kept; sort and tie-break; one
   node's failure keeps the other's rows; a missing session does not
   change the badge; title order; cron presets stay cron; `every` is
   not rewritten; an interval that is not a whole minute is not rounded
   into a minute; blank cap is absent and `0` is an error; empty prompt,
   bad cron, missing zone, cap out of range; preview reuse only with
   the same cadence plus anchor; older peer builds no preview request;
   heartbeat save changes cadence only and keeps the other loaded
   fields; archive toggle is a whole update; a definition with any
   `Unknown` does not produce an update. No `sleep`. No real agent.
3. **List and detail** in `app/src/areas/schedules/`, kit controls,
   both languages, both themes, the sizes the native gate captures.
   Fake fleet, schedules made through the CLI. The sidebar slot stays
   the shell owner's.
4. **Create and edit**, including the preview and the typed field
   errors.
5. **Heartbeats:** the narrower menu, cadence-only edit, the sentence
   that saving makes the runs act as the person. No create control.

## Not in this track

Saved prompts. Webhooks. A new worktree per run. A run transcript.
Relative expiry on the node's clock. A person-created heartbeat that
the agent owns. Filling `sidebar::schedules`. Any edit under `web/`.

## Verification

Slice 1 has no behaviour to run. The gate on the docs commit is
recorded with the report of that commit, not in this file's prose as a
claim that outlives the log. Later slices append their own gate line
and move the resume point in the same commit.
