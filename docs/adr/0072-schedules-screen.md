# ADR 0072: The schedules screen, field by field against the node

Status: accepted for the native schedules screen (the decision this
screen is built from). No protocol change. The node's schedules stay
protocol 11 and the read-only preview stays protocol 13, both as ADR
0033. The plan is `docs/plans/app-schedules.md`.

## Purpose and intent

Paseo has one Schedules screen: a list across hosts, a detail of runs,
and create and edit for a cron or an elapsed interval, a time zone, a
run cap, an expiry, run now, pause and resume, plus a heartbeat (a
recurring prompt into an agent itself). Vessel's node already does
this (`vessel schedule`, the schedule tools, `schedule.preview` and
`NodeInfo.timezone`). The native app has no screen. This decision says
which control calls which existing method, which of Paseo's controls
the screen does not draw because the node has no such behaviour, and
which types in `view` hold the list, the detail, the form and the
preview. Later slices implement this text. They do not invent a second
contract in code.

## Existing implementation and constraints

The node, not the screen, decides when a run happens (ADR 0033): five-field
cron in an IANA zone, or `every` as elapsed seconds from an anchor;
daylight-saving rules and missed due times live in `vessel-node` and
are not reimplemented in the app. A client can refuse a bad expression
before sending because `CronExpr`, `EverySeconds`, `TimeZoneName`,
`PromptText`, `ScheduleName` and `MaxRuns` already parse in
`vessel-proto`.

What a screen may call, and no other schedule method, is the table in
`crates/vessel-proto/src/rpc.rs` (protocol 11 unless noted):

| Call | Scope | Reply |
| --- | --- | --- |
| `schedule.list` | read | `ScheduleList`, newest created first, at most 256 |
| `schedule.get` | read | `ScheduleInfo` |
| `schedule.runs` | read | `RunList`, newest 50 (`RunList::MAX`) |
| `schedule.preview` | read, protocol 13 | `SchedulePreview` |
| `schedule.create` | operate | `ScheduleInfo` |
| `schedule.update` | operate | `ScheduleInfo` |
| `schedule.pause` | operate | `ScheduleInfo` |
| `schedule.resume` | operate | `ScheduleInfo` |
| `schedule.run` | operate | `ScheduleRun` |
| `schedule.delete` | operate | `Ack` |

`schedule.update` replaces the whole `ScheduleDefinition` under a
`revision`. A person who updates becomes `defined_by`, so later runs
act as them. A forwarded caller (`fleet.forward`) may pause, resume
and delete, and may not update or run. The app therefore calls the
node that holds the schedule, as the paired device, and does not
forward these calls.

`NodeInfo.timezone` (protocol 13) is the machine's zone when the node
could name one. Absence is not UTC (ADR 0033). An older handshake omits
it.

`view` already depends on `vessel-proto` and must not do I/O or draw.
`link` already depends on `view`, `vessel-proto` and `vessel-client`.
No new crate edge and no new dependency. The sidebar already has an
empty slot for this screen (`app/src/areas/sidebar.rs`, `schedules`,
returning `None`). Filling that slot is not this decision's code: the
shell's owner draws the row. This track does not edit the sidebar.

An earlier branch contains unreviewed `view` modules for schedules.
They are not imported and not the source of the rules below. The rules
are Paseo 0.15.4 read again, then cut to what this node actually does.

## Alternatives considered

1. **Draw Paseo's form as it stands**, including converting `every` into
   cron, defaulting a missing zone to UTC, and treating an unparsable
   run cap as "unlimited". Rejected: ADR 0033 kept `every` because cron
   cannot say "every 14 days" and because a wall-clock hour is not
   3,600 seconds across a daylight-saving change; a silent UTC is the
   fallback ADR 0033 refuses; a silent "unlimited" drops what the
   person typed.
2. **Recompute the next runs in the app** with the node's library linked
   into `view`, or with a second cron implementation. Rejected when
   protocol 13 landed: `schedule.preview` is the node's `Due`
   calculation, and a second one can drift. An older peer is told that
   preview is unavailable. Times are not guessed.
3. **Add wire** for a relative expiry, a minted worktree per run, a run
   transcript, or a heartbeat created by a person but owned by the
   agent. Rejected for this screen: each is a node change, and drawing
   the control would claim a behaviour the node does not have. They are
   listed under costs, for the node line, and the screen does not fake
   them.
4. **Leave schedules to the CLI.** Rejected: the owner's definition of
   this version is Paseo's screens as well as its node.

## The choice

The screen is a client of the calls above. Meaning lives in `view`
(list, detail, form, preview) as typed values and typed errors. `link`
performs the calls and maps a `WireError` once. `app` draws with the
kit, in `app/src/areas/schedules/`, and holds no sentence of its own.
Nothing in `view` matches on a protocol number or on a string to decide
behaviour: `link` turns the handshake into a `SchedulePeer` (schedules
offered or not; preview offered or an older peer; the machine zone or
none) and `view` matches that enum exhaustively.

A schedule the node describes with an `Unknown` cadence, target,
`AfterRun`, `CatchUp` or state is shown and is not rewritten. Update
serialises `#[serde(other)]` back as a lossy value, so a definition
that contains any `Unknown` cannot be saved. Pause, resume and run are
not offered for `ScheduleState::Unknown` or `Ended`: the node refuses
them (`calls.rs` treats `Unknown` like an ended schedule). Delete stays
offered.

### What each control calls

Paseo paths are the 0.15.4 application unless a line says CLI or
protocol. "Same" means the screen does what that line does.

| Control | Paseo | Call | Rule here |
| --- | --- | --- | --- |
| List | `schedule/list`, then newest `createdAt` first across hosts (`screens/schedules-screen.tsx` 152) | `schedule.list` on each connected node | Same order. Tie-break is node id, then schedule id. A node that fails does not hide the others (`schedules-screen.tsx` 337–343). A node that has not answered is not an empty list. A node whose handshake is older than 11 is "schedules are not on this machine", not an empty list. A reply of 256 says the node keeps at most that many. |
| Host filter | Shown when there is more than one host (same file, 164) | none | The paired nodes the app already lists. One node: no filter. |
| Active / Ended | Active is derived `active` or `paused`; Ended is expired, finished, target gone (`schedule-derivation.ts` 98–100; filter labels `schedules-screen.tsx` 48–51) | none | Runnable is `Active` or `Paused`. Ended is every `Ended` reason and `Unknown`. `Unknown` is not dropped. |
| Badge | Active, Paused, Expired, Finished, Target gone (`schedule-row.tsx` 78–96) | the `ScheduleState` on the info | Active, Paused, Finished (`EndReason::MaxRuns`), Expired, Target gone, and also Creator gone and Unknown. The badge is the node's state. The screen does not decide expired or target-gone from the clock or from a missing session (Paseo does: `isExpired` at `schedule-derivation.ts` 49–55, `isAgentTargetGone` at 57–63, applied in `deriveState` at 81–96). |
| Row meta | Cadence, created, last run or "Never run", next run only while active (`schedule-row.tsx` 99–122) | fields of `ScheduleInfo` | Same order. Next run only for `Active`. The instant is the node's; the relative phrase ("in 3h", "soon", thresholds in `schedule-format.ts` 173–195) is display only. |
| Title | Name, else the new session's title, else the first prompt line (`schedule-format.ts` 35–48) | `ScheduleName`, `SessionSpec.title`, `PromptText` | Same order. No name and no title and a blank prompt cannot occur (`PromptText` refuses blank); the fallback sentence exists so a newer node's odd record still has a line. |
| Open row | Opens the editor (`schedule-row.tsx` 171–176, 216–220) | `schedule.get` if the list row is not enough | The editor opens on the listed `ScheduleInfo`. Runs are a detail, not a second copy of the form. |
| New | "New schedule" (`schedules-screen.tsx` 286 and 319) | none until save | Opens the form. Does not create a paused schedule (`CreateSchedule.paused` stays false; Paseo's form has no such switch). |
| Empty | "No active schedules" / "No ended schedules" (`schedules-screen.tsx` 315, 331) | none | Same two empties. Paseo's "See docs" link (`schedules-screen.tsx` 317) is omitted: there is no Vessel page to send it to, and a link to Paseo's site would be the wrong product. |
| Load error | "Unable to load schedules" and Try again (`schedules-screen.tsx` 237–241) | `schedule.list` again | Same. The error is the node's, mapped once. Retry does not keep a failed list and call it current. |
| Name | Optional (`schedule-form-sheet.tsx` 493) | `ScheduleName` or absent | Blank means absent. A non-blank name that `ScheduleName` refuses is a typed error, not a trimmed surprise. |
| Prompt | Required for a new session (`schedule-form-model.ts` 548–549; placeholder `schedule-form-sheet.tsx` 513) | `PromptText` | Parsed with `PromptText`. Empty, over 16 KiB, or a forbidden control character is a typed error. |
| Cadence presets | Every minute `* * * * *`, every hour `0 * * * *`, daily 9:00 `0 9 * * *`, weekdays 9:00 `0 9 * * 1-5`, Mondays 9:00 `0 9 * * 1` (`schedule-cadence-options.ts` 10–16) | `Cadence::Cron` | Those five are cron, as Paseo submits them. A new schedule opens on every hour (`0 * * * *`), which is what Paseo's default hour becomes after `normalizeScheduleFormCadence` (`schedule-cadence-options.ts` 42–61, default `schedule-form-model.ts` 143). |
| Custom cron | A text field (`cadence-editor.tsx` 102–116) | `CronExpr` | Parsed with `CronExpr`, not a second grammar. Invalid is a typed error and blocks save. |
| Every | CLI `--every`. The app form rewrites it to cron (`schedule-cadence-options.ts` 50–61) | `Cadence::Every` | The form has a separate elapsed interval, 60 seconds to 366 days (`EverySeconds`). It is not rewritten into cron. Opening an existing `every` does not convert it. The default for a *new* schedule is still the cron preset above, so a person who does not touch cadence gets Paseo's "every hour" wall clock, not an elapsed hour. |
| Time zone | The device zone, stored on the cron (`cadence-editor.tsx` 47–49). A missing zone becomes `UTC` (`schedule-format.ts` 133, `schedule-form-model.ts` 144) | `TimeZoneName` on cron; `schedule.preview` can answer `UnknownZone` | The field starts as `SchedulePeer`'s machine zone when that zone parses. No UTC default. Cron cannot be saved until a zone is chosen. The zone's shape is checked locally; whether the database has it is the preview's or the save's refusal. |
| Preview | A sentence under the cron (`cadence-editor.tsx` 33–37, 88–94) | `schedule.preview`, count 5 (1 to 10) | Shown only from a preview reply: the instants and the cadence the reply returns. A later edit invalidates it. Save reuses that cadence, anchor included, only when it is the form's cadence plus the filled anchor. Otherwise `every` is saved with no anchor and the node fills it. Protocol older than 13: the call is not made; the form says this machine cannot preview; no instants are invented; save still works. Four previews already running comes back `unavailable` and is shown, not retried in a loop. |
| Max runs | Placeholder "Unlimited". `parseMaxRuns` turns 0, blank and garbage into `null` (`schedule-form-sheet.tsx` 68–71, 537–547) | `MaxRuns` or absent | Blank means absent (no cap). Anything else must parse as 1 to 1,000,000. A bad value is a typed error and does not mean unlimited. |
| Expiry | Not on the app form. CLI `--expires-in` is a duration from the device clock; update takes an absolute `expiresAt` (`protocol` `UpdateScheduleInput`) | `ends_at` | An optional absolute UTC instant. No duration is added to the device clock. A value that does not parse is a typed error. A past instant is allowed: the node then ends the schedule by its own rule. |
| Archive on finish | Switch, default on (`schedule-form-model.ts` 682; control `schedule-form-sheet.tsx` 801–809). App only, in the form | `AfterRun` on `ScheduleTarget::NewSession` | Default `Archive`. Off is `Keep`. Only for a new session. The detail shows the same switch: changing it is `schedule.update` of the whole definition with that one field changed, at the revision the screen holds. `AfterRun::Unknown` is shown and the switch does not move. |
| Target, new session | Project, model, thinking, mode (`schedule-form-sheet.tsx` 724–780). Create requires a project (`schedule-form-model.ts` 551–554) | `ScheduleTarget::NewSession` and its `SessionSpec` | The same choices the new-conversation composer can already set (harness, place, profile, model, options), built as a `SessionSpec`. No second catalogue. A place is required. Profile and model and options follow the composer's own "left out means the default" rules. |
| Target, existing session | Protocol and CLI. The app form does not create one; edit of an agent target sends cadence only (`schedule-form-sheet.tsx` 320–331) | `ScheduleTarget::Session` | Create offers it, because the node and Paseo's CLI do and the app form's omission would hide a call the node has. The session is one the node still has. Edit of a *person's* session target may change prompt, cadence, cap and expiry. The target session is not silently swapped. |
| Isolation | Local or worktree (`schedule-form-sheet.tsx` 814) | none for "mint a worktree" | Not drawn. An existing worktree is chosen as a place, which is what a hand-started session can already do. See the node list below. |
| Catch-up | No control | `CatchUp` | Create sends `Once` (the node's default). Edit sends the value the schedule already has, including `Skip`. `Unknown` blocks save, as above. No new control. |
| Pause / Resume | Menu, hidden when the target is an existing agent (`schedule-row.tsx` 275). Pending label, no optimistic badge | `schedule.pause` / `schedule.resume` | Offered for a person's schedule in `Active` (pause) or `Paused` (resume), including a session target. Hidden for a heartbeat (below) and for `Ended` and `Unknown`. The badge changes when the reply arrives, not before. A refusal is shown; Paseo swallows it (`schedules-table.tsx` 84) and this screen does not. |
| Run now | Same menu; enabled only for a new-agent target that is active or paused (`schedule-row.tsx` 195) | `schedule.run` | Offered for a person's schedule that is `Active` or `Paused`, new session or existing session. Hidden for a heartbeat and while this schedule's last known outcome is `Making`, or a run request is outstanding. The node's "a run is being made" stays a shown conflict. |
| Delete | Confirm, then `schedule/delete` (`schedules-table.tsx` 117–128) | `schedule.delete` | Same confirm, then the call. The row stays until the ack. Failure is shown. |
| Edit heartbeat | Title "Edit heartbeat"; save sends cadence only (`schedule-form-sheet.tsx` 316–331 and 404–406) | `schedule.update` of the whole definition, cadence replaced, every other field the loaded one | A heartbeat is a schedule whose `defined_by` is an agent and whose target is that agent's session (`scheduleProductName`, `schedule-format.ts` 31–33, is every agent target; this screen is narrower, on purpose). Menu is edit and delete only. The save copy says runs will afterwards act as the person, because update changes `defined_by` (ADR 0033). Paseo's heartbeat update does not transfer ownership; the sentence is the difference, not a silent transfer. |
| Detail runs | The feature is a run list. The 0.15.4 screen has no run list; CLI `schedule logs` and `schedule/logs` do. A run there has status, output and error text (`protocol` `ScheduleRun`) | `schedule.runs` | The detail lists the reply: number, due time, when it ran, trigger, outcome. A session id is a link to that session, not a transcript. There is no output or error string on Vessel's `ScheduleRun`. At most 50, newest first. |
| Heartbeat create | CLI and MCP, only inside the agent (`commands/heartbeat/index.ts`) | the agent's `create_heartbeat`, not a person's `schedule.create` | The screen lists and edits heartbeats an agent already made. It does not create one. A person's create with a session target is a schedule, labelled Schedule, and is not deleted by the agent's session ending. |

### Models in `view`

One module each, under `view/src/schedules/`. They borrow the proto types
they do not need to reinterpret. They do not format sentences: a phrase
is an enum the catalogs render in both languages (the catalogs arrive
with the screen).

- **List.** Input: per node, either a `ScheduleList` or a typed load
  failure, plus whether that node's session directory has loaded.
  Output: rows (node, info, kind Schedule or Heartbeat, badge, bucket,
  title parts, meta parts, which menu actions are offered) and the
  banner of node failures. Filter and sort are functions of that output.
- **Detail.** Input: `ScheduleInfo` and `RunList`. Output: the run rows
  as an exhaustive match of `RunTrigger` and `RunOutcome`, and the
  archive change as a full `UpdateSchedule` or a refusal (`Unknown`
  after-run, missing revision, not a new session).
- **Form.** Input: create, or edit of one `ScheduleInfo`, plus
  `SchedulePeer` and the composer's existing catalogues. Output: either
  a `ScheduleDefinition` (and, for edit, the `revision`) or one
  `ScheduleFormError`. The error names the field: name, prompt, cron,
  interval, zone, cap, expiry, place, target, unknown definition,
  preview. Wrapping a proto parse error keeps that error as source.
  Several fields can be wrong; the model returns every current one, not
  the first only. `can_save` is false while any remains.
- **Preview.** Input: a validated cadence, an optional `after`, a count.
  Output: the request, or `OlderPeer` without a request. A reply is
  reusable for save only under the rule in the table.

`link` grows one command per call in the table, answered by one event
carrying the proto reply or the mapped error. No command takes a bare
`String` for an id, a zone, a cron or a count.

## Why this and not the others

The screen is the place a person stands. The node is the place time is
decided. Putting due times, ownership and catch-up back into the client
would split ADR 0033 in two. Copying Paseo's form where it drops or
rewrites a value would make the screen a worse client than the CLI of
either product. Adding wire so the picture matches Paseo's extra fields
would claim this screen can change the node; it cannot, and those gaps
are named so nobody fills them with a fake.

## Advantages, costs and effects

Advantages: every control names a call that already exists; a bad field
fails before a call; an older node is a typed peer, not a crash and not
an empty list; `every` stays elapsed time; the preview the person saw
is the cadence that is saved, anchor included.

Costs, listed for the node line and not drawn:

- A run transcript, or a log longer than 50. The detail shows the typed
  outcome and the session.
- A worktree minted for each run (Paseo's isolation "worktree"). ADR
  0033 left it unbuilt. The form can only select an existing place.
- A relative expiry added on the node's clock. The form takes `ends_at`.
- A heartbeat created by the person but owned by the agent, so it dies
  with that session. That remains the agent's `create_heartbeat`.
- A client-derived "expired" or "target gone" while the node still says
  active.
- Webhooks (ADR 0034 is not a screen). Saved prompts (the node has them;
  they are not this screen).

Effects elsewhere: no row in `docs/ARCHITECTURE.md`, no new method, no
generated TypeScript, no sidebar edit. The shell owner turns the
existing `schedules` slot from `None` into the row that opens this
area (`nav_row` with the calendar-clock glyph, as that function's
comment already says). `Route` gains a schedules variant when the
screen exists; that registration is the screen's, the slot is not.

What this decision does not verify: no screen is drawn here, no test
runs against these types yet, and no capture exists. Those are the
later slices in the plan.
