# ADR 0072: The schedules screen, field by field against the node

Status: proposed. No protocol change. The node's schedules stay
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
`revision` and sets `defined_by` to the caller
(`crates/vessel-node/src/automation/schedules/calls.rs`, in `update`,
`info.defined_by = asker.by`). ADR 0033 (who a run acts as) is the
reason: whoever last wrote the definition is who a later run acts as,
so a device cannot leave another name on a prompt it wrote. Paseo's
`ScheduleService.update` patches fields on the stored record and does
not replace a creator (`packages/server/src/server/schedule/service.ts`,
the `update` method). Every control that calls `schedule.update` carries
that sentence. A schedule an agent made is not given one: an update
that keeps the agent as `defined_by` is listed for the node line.

Who may ask is `rules.rs` (`Asker::may_create` and `Asker::may`):

- A person on that node (`Via::Person`) may create, update, pause,
  resume, delete and run.
- A delegated caller (`Via::Delegated`, the body of a `fleet.forward`)
  cannot create. `may_create` refuses every delegated asker, and ADR
  0033 says a delegated caller cannot create or update.
- A delegated person (`Delegated { agent: false }`) may pause, resume
  and delete, and may not update or run. That arm's words are the
  refusal: a call passed on may pause, resume and delete, not change or
  run. The "run" half is this arm. ADR 0033 names create and update and
  does not name run.
- A delegated agent (`Delegated { agent: true }`) may not pause either
  (`calls_passed_on_may_only_stop_things`). This app forwards as the
  paired device, so the forward is a person.

`SchedulePeer` carries the reach. `Direct` is this device's own session
on that node. `Forwarded` is a node the app can call only through
`fleet.forward`. The schedule methods are forwardable
(`crates/vessel-proto/src/forward.rs`, the arms from `PreviewSchedule`
through `ListRuns`). The screen lists that node with the others. It does
not pretend the call is local.

On a `Forwarded` peer, list, get, runs and preview are offered, and a
refusal is shown. Create, update and run now are hidden and are not
sent. Pause, resume and delete are offered for a person's schedule; a
refusal is shown. A heartbeat's menu stays delete only, on either
reach, for the reason in the plan.

`NodeInfo.timezone` (protocol 13) is the machine's zone when the node
could name one. Absence is not UTC (ADR 0033). An older handshake omits
it. A new cron's zone is the device's, below. The machine zone is the
fallback when the device zone does not parse.

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
behaviour: `link` turns the handshake and the path into a `SchedulePeer`
(schedules offered or not; preview offered or an older peer; the
machine zone or none; `Direct` or `Forwarded`) and `view` matches that
enum exhaustively. `app` reads the device's IANA zone name and hands
the form a parsed `TimeZoneName`, or none when the name does not parse.
`view` does not read the clock or the zone database. A fixed offset is
not a zone name. The same read is the path on Windows: the platform's
IANA name, then the same parse. A name that does not parse leaves the
field on the machine-zone fallback. The read uses no credential and
opens nothing.

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
| Host filter | Shown when there is more than one host (same file, 164) | none | The nodes the app already shows, each `Direct` or `Forwarded`. One node: no filter. |
| Active / Ended | Active is derived `active` or `paused`; Ended is expired, finished, target gone (`schedule-derivation.ts` 98–100; filter labels `schedules-screen.tsx` 48–51) | none | Runnable is `Active` or `Paused`. Ended is every `Ended` reason and `Unknown`. `Unknown` is not dropped. |
| Badge | Active, Paused, Expired, Finished, Target gone (`schedule-row.tsx` 78–96) | the `ScheduleState` on the info | Active, Paused, Finished (`EndReason::MaxRuns`), Expired, Target gone, and also Creator gone and Unknown. The badge is the node's state. The screen does not decide expired or target-gone from the clock or from a missing session (Paseo does: `isExpired` at `schedule-derivation.ts` 49–55, `isAgentTargetGone` at 57–63, applied in `deriveState` at 81–96). |
| Row meta | Cadence, created, last run or "Never run", next run only while active (`schedule-row.tsx` 99–122) | fields of `ScheduleInfo` | Same order. Next run only for `Active`. The instant is the node's; the relative phrase ("in 3h", "soon", thresholds in `schedule-format.ts` 173–195) is display only. |
| Title | Name, else the new session's title, else the first prompt line (`schedule-format.ts` 35–48) | `ScheduleName`, `SessionSpec.title`, `PromptText` | Same order. No name and no title and a blank prompt cannot occur (`PromptText` refuses blank); the fallback sentence exists so a newer node's odd record still has a line. |
| Open row | Opens the editor (`schedule-row.tsx` 171–176, 216–220) | `schedule.get` if the list row is not enough | A person's schedule on a `Direct` peer opens the editor on the listed `ScheduleInfo`. A heartbeat, any other schedule an agent made, and any schedule on a `Forwarded` peer open the detail: the fields are shown and there is no save. Runs are that detail. |
| New | "New schedule" (`schedules-screen.tsx` 287 and 320) | none until save | Opens the form on a `Direct` peer. Hidden while the chosen host is `Forwarded`: create is not sent. Does not create a paused schedule (`CreateSchedule.paused` stays false; Paseo's form has no such switch). |
| Empty | "No active schedules" / "No ended schedules" (`schedules-screen.tsx` 315, 331) | none | Same two empties. Paseo's "See docs" link (`schedules-screen.tsx` 317) is omitted: there is no Vessel page to send it to, and a link to Paseo's site would be the wrong product. |
| Load error | "Unable to load schedules" and Try again (`schedules-screen.tsx` 237–241) | `schedule.list` again | Same. The error is the node's, mapped once. Retry does not keep a failed list and call it current. |
| Name | Optional (`schedule-form-sheet.tsx` 493) | `ScheduleName` or absent | Blank means absent. A non-blank name that `ScheduleName` refuses is a typed error, not a trimmed surprise. |
| Prompt | Required for a new session (`schedule-form-model.ts` 548–549; placeholder `schedule-form-sheet.tsx` 513) | `PromptText` | Parsed with `PromptText`. Empty, over 16 KiB, or a forbidden control character is a typed error. |
| Cadence presets | Every minute `* * * * *`, every hour `0 * * * *`, daily 9:00 `0 9 * * *`, weekdays 9:00 `0 9 * * 1-5`, Mondays 9:00 `0 9 * * 1` (`schedule-cadence-options.ts` 14–20) | `Cadence::Cron` | Those five are cron, as Paseo submits them. A new schedule opens on every hour (`0 * * * *`), which is what Paseo's default hour becomes after `normalizeScheduleFormCadence` (`schedule-cadence-options.ts` 38–51, default `schedule-form-model.ts` 143). |
| Custom cron | A text field (`cadence-editor.tsx` 102–116) | `CronExpr` | Parsed with `CronExpr`, not a second grammar. Invalid is a typed error and blocks save. |
| Every | CLI `--every`. The app form rewrites it to cron (`schedule-cadence-options.ts` 46–61) | `Cadence::Every` | The form has a separate elapsed interval, 60 seconds to 366 days (`EverySeconds`). It is not rewritten into cron. Opening an existing `every` does not convert it. The default for a *new* schedule is still the cron preset above, so a person who does not touch cadence gets Paseo's "every hour" wall clock, not an elapsed hour. |
| Time zone | The device zone, stored on the cron (`cadence-editor.tsx` 45–49: `cronTimeZone = normalizedValue.timezone ?? deviceTimeZone`, and `device-timezone.ts` reads `Intl.DateTimeFormat().resolvedOptions().timeZone`). A missing zone becomes `UTC` (`schedule-format.ts` 133, `schedule-form-model.ts` 144 `DEFAULT_TIMEZONE`) | `TimeZoneName` on cron; `schedule.preview` can answer `UnknownZone` | A new cron starts as the device zone `app` parsed. An existing cron opens on its stored zone. When the device name does not parse, the field starts as `SchedulePeer`'s machine zone when that parses. When neither parses, the field is empty and cron cannot be saved until a zone is chosen. No UTC default. The zone's shape is checked locally; whether the database has it is the preview's or the save's refusal. When the device zone and the machine zone differ, the field shows the device zone, and the preview lists only the instants `schedule.preview` returned. Those instants are the node's. Both zone names are shown, so a daily 9:00 is 9:00 in the device zone. The form does not replace those instants with a sentence computed on the device. |
| Preview | A sentence under the cron (`cadence-editor.tsx` 33–37, 88–94) | `schedule.preview`, count 5 (1 to 10) | Shown only from a preview reply: the instants and the cadence the reply returns. A later edit invalidates it. Save reuses that cadence, anchor included, only when it is the form's cadence plus the filled anchor. Otherwise `every` is saved with no anchor and the node fills it. Protocol older than 13: the call is not made; the form says this machine cannot preview; no instants are invented; save still works. Four previews already running comes back `unavailable` and is shown, not retried in a loop. |
| Max runs | Placeholder "Unlimited". `parseMaxRuns` turns 0, blank and garbage into `null` (`schedule-form-sheet.tsx` 68–71, 537–547) | `MaxRuns` or absent | Blank means absent (no cap). Anything else must parse as 1 to 1,000,000. A bad value is a typed error and does not mean unlimited. |
| Expiry | Not on the app form. CLI `--expires-in` is a duration from the device clock; update takes an absolute `expiresAt` (`protocol` `UpdateScheduleInput`) | `ends_at` | An optional absolute UTC instant. No duration is added to the device clock. A value that does not parse is a typed error. A past instant is allowed: the node then ends the schedule by its own rule. |
| Archive on finish | Switch, default on (`schedule-form-model.ts` 682; control `schedule-form-sheet.tsx` 801–809). App only, in the form | `AfterRun` on `ScheduleTarget::NewSession` | Default `Archive`. Off is `Keep`. Only for a new session a person owns, on a `Direct` peer. Changing it is `schedule.update` of the whole definition with that one field changed, at the revision the screen holds, and `defined_by` becomes the person who toggles it. The switch's copy says later runs act as them. Hidden when an agent owns the schedule, and hidden on a `Forwarded` peer: the call is not sent. `AfterRun::Unknown` is shown and the switch does not move. |
| Target, new session | Project, model, thinking, mode (`schedule-form-sheet.tsx` 724–780). Create requires a project (`schedule-form-model.ts` 551–554) | `ScheduleTarget::NewSession` and its `SessionSpec` | The same choices the new-conversation composer can already set (harness, place, profile, model, options), built as a `SessionSpec`. No second catalogue. A place is required. Profile and model and options follow the composer's own "left out means the default" rules. Saving an edit of a person's new-session schedule is one `schedule.update` of the whole definition, and the copy says later runs act as the person who saves. That save is hidden when an agent owns the schedule, and hidden on a `Forwarded` peer. |
| Target, existing session | Protocol and CLI. The app form does not create one; edit of an agent target sends cadence only (`schedule-form-sheet.tsx` 316–324) | `ScheduleTarget::Session` | Create offers it on a `Direct` peer, because the node and Paseo's CLI do. The session is one the node still has. Edit of a person's session target may change prompt, cadence, cap and expiry, as one `schedule.update` of the whole definition. The copy says later runs act as the person who saves. The target session is not swapped. Edit of an agent-owned target is not drawn. |
| Isolation | Local or worktree (`schedule-form-sheet.tsx` 814) | none for "mint a worktree" | Not drawn. An existing worktree is chosen as a place, which is what a hand-started session can already do. See the node list below. |
| Catch-up | No control | `CatchUp` | Create sends `Once` (the node's default). Edit sends the value the schedule already has, including `Skip`. `Unknown` blocks save, as above. No new control. |
| Pause / Resume | Menu, hidden when the target is an existing agent (`schedule-row.tsx` 275–277 returns no execution menu). Pending label, no optimistic badge | `schedule.pause` / `schedule.resume` | Offered for a person's schedule in `Active` (pause) or `Paused` (resume), including a session target, on either reach. Hidden for a heartbeat and for `Ended` and `Unknown`. The reason a person's session target keeps them, while Paseo hides them for every agent target, is in the plan. The badge changes when the reply arrives, not before. A refusal is shown; Paseo swallows it (`schedules-table.tsx` 84) and this screen does not. |
| Run now | Same menu; enabled only for a new-agent target that is active or paused (`schedule-row.tsx` 195, `canRun`) | `schedule.run` | Offered for a person's schedule on a `Direct` peer that is `Active` or `Paused`, new session or existing session. Hidden for a heartbeat, on a `Forwarded` peer (the call is not sent), and while this schedule's last known outcome is `Making` or a run request is outstanding. The node's "a run is being made" stays a shown conflict. The reason a person's session target has this control is in the plan. |
| Delete | Confirm, then `schedule/delete` (`schedules-table.tsx` 117–128) | `schedule.delete` | Same confirm, then the call, on either reach. Delete does not change `defined_by`. The row stays until the ack. Failure is shown. |
| Edit heartbeat | Title "Edit heartbeat"; save sends cadence only (`schedule-form-sheet.tsx` 316–324 and 406) | none on this screen | A heartbeat is a schedule whose `defined_by` is an agent and whose target is that agent's session. Paseo calls every agent target a Heartbeat (`schedule-format.ts` 31–33). This label is narrower because only an agent-made schedule is deleted when that session ends (`calls.rs` `session_ended`, ADR 0033). The menu is delete only. Save is not drawn. `schedule.update` would set `defined_by` to the person, and the record would become a Schedule: pause, resume and run now would appear, and it would no longer be deleted when the agent's session ends (`session_ended` forgets schedules that agent defined; a person's `defined_by` stays). Paseo's save keeps the stored target (`service.ts` `update` patches the same record). Keeping the agent as owner is the node line below. The per-control reason for a person's session target is in the plan. |
| Detail runs | The feature is a run list. The 0.15.4 screen has no run list; CLI `schedule logs` and `schedule/logs` do. A run there has status, output and error text (`protocol` `ScheduleRun`) | `schedule.runs` | The detail lists the reply: number, due time, when it ran, trigger, outcome. A session id is a link to that session, not a transcript. There is no output or error string on Vessel's `ScheduleRun`. At most 50, newest first. |
| Heartbeat create | CLI and MCP, only inside the agent (`commands/heartbeat/index.ts`) | the agent's `create_heartbeat`, not a person's `schedule.create` | The screen lists heartbeats an agent already made and can delete one. It does not create one and it does not update one. A person's create with a session target is a schedule, labelled Schedule. It is not deleted when a session ends, because `session_ended` forgets schedules that agent defined. |

### Models in `view`

One module each, under `view/src/schedules/`. They borrow the proto types
they do not need to reinterpret. They do not format sentences: a phrase
is an enum the catalogs render in both languages (the catalogs arrive
with the screen).

- **List.** Input: per node, either a `ScheduleList` or a typed load
  failure, that node's `SchedulePeer` (including `Direct` or
  `Forwarded`), plus whether that node's session directory has loaded.
  Output: rows (node, info, kind Schedule or Heartbeat, badge, bucket,
  title parts, meta parts, which menu actions are offered) and the
  banner of node failures. Filter and sort are functions of that output.
  The menu follows the table: a heartbeat offers delete only; a
  `Forwarded` person's schedule offers pause, resume and delete.
- **Detail.** Input: `ScheduleInfo`, `RunList`, and the peer. Output:
  the run rows as an exhaustive match of `RunTrigger` and `RunOutcome`,
  and the archive change as a full `UpdateSchedule` or a refusal
  (`Unknown` after-run, missing revision, not a new session, an agent
  owns it, or the peer is `Forwarded`).
- **Form.** Input: create, or edit of one `ScheduleInfo`, plus
  `SchedulePeer`, the device zone as a parsed `TimeZoneName` or none,
  and the composer's existing catalogues. A new cron uses the device
  zone when it parsed, otherwise the machine zone when that parsed,
  otherwise an empty zone. Output: either a `ScheduleDefinition` (and,
  for edit, the `revision`) or the current `ScheduleFormError`s. The
  error names the field: name, prompt, cron, interval, zone, cap,
  expiry, place, target, unknown definition, preview, owner, reach.
  An agent-owned schedule and a `Forwarded` peer produce `owner` or
  `reach` and no update. A `Forwarded` peer produces `reach` and no
  create. Wrapping a proto parse error keeps that error as source.
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
- A `schedule.update` that keeps `defined_by`. Paseo's heartbeat save
  changes cadence and leaves the owner. This node's update makes the
  caller the definer, so the screen does not draw that save. With the
  missing call, a cadence edit would leave the agent as owner: the
  kind, the delete-only menu, and deletion when the session ends would
  stay.
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
