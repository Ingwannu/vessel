# Web schedules

The first packet covers TRACKER 581–583 and Area I4–I6/I29–I31.
The form packet covers TRACKER 584–585 and the web portion of 587
(Area I7/I9/I32–I33/I35). ADR 0033 defines node ownership and execution.
ADR 0072 records the native consumer decisions.

## Decision log

- Purpose. Make a fleet's scheduled work visible and controllable from
  the browser. A person can open `/schedules` directly, select Schedules
  in the sidebar, or find it in the palette.
- Existing implementation. Protocol 11 exposes the list and row actions.
  Protocol 13 exposes node-authoritative cadence preview. The React page
  has a typed Bridge to every machine in the book, a fleet store, and a
  shared UI kit for rows, menus, dialogs and filters.
- Alternatives. A new query dependency would duplicate the page's external
  store and reconnect handling. Deriving availability from a machine's
  session directory would mark schedules ended when that directory is
  still loading.
- Choice. The lazy route reads the fleet store and owns a route-scoped
  schedule controller. The controller fans out typed calls on the bridge,
  tracks each machine's connection and request identity, and adds a
  per-row pending overlay for pause, resume and deletion. The immutable
  node snapshot stays under that overlay so one failed action cannot roll
  back another row. Presentation derives the filters and words from
  the controller snapshots.
- Trade-offs. One slow machine leaves loaded rows from other machines
  visible. Connected machines that refuse the request get a banner.
  Offline machines are skipped until they reconnect, as in Paseo.
  Replies from a dropped connection are ignored and the
  machine is read again after reconnect. Node end states include
  Creator gone and Unknown. The bridge uses each book entry's own
  connection, where the node authorizes every action. As in Paseo
  (`utils/schedule-format.ts:31–33`, `components/schedules/schedule-row.tsx:196,275–277`),
  the row menu calls every existing-session target a heartbeat and offers
  deletion as its only mutation. The target line shows the session title
  (`schedules/schedule-derivation.ts:64–72`). Ownership and session-end
  cleanup use the node's rules in ADR 0033 and ADR 0072. Vessel's Go to group and
  shell are preserved, and the English list copy follows Paseo while
  Korean localizes the route label. Vessel has no schedule docs URL.
- Verification. Run the web gate and the Rust half for route registration
  through the standard remote gate. Use an isolated fake fleet to cover
  direct navigation, mixed answers, filtering and row mutations.

## Paseo 0.15.4 reference

Paths are within `packages/app/src` in the pinned Paseo 0.15.4 reference.

| Area | Source |
| --- | --- |
| Route | `app/schedules.tsx:1–10` |
| Sidebar and command palette | `components/sidebar/sidebar-nav-rows.tsx:168–185`, `sidebar-nav/model.ts:11`, `command-center/root-registration.tsx:230–246` |
| Host and status filters, list and load states | `screens/schedules-screen.tsx:144–348` |
| Per-host aggregation and reconnect | `schedules/aggregated-schedules.ts:61–123`, `hooks/use-schedules.ts:36–74` |
| Rows, actions and delete confirmation | `components/schedules/schedule-row.tsx:99–244,266–385`, `components/schedules/schedules-table.tsx:117–128`, `hooks/use-schedule-mutations.ts:145–216` |
| Forms and cadence, second packet | `components/schedules/schedule-form-sheet.tsx:316–554,699–883`, `components/schedules/cadence-editor.tsx:33–122`, `schedules/schedule-cadence-options.ts:14–51` |

The pinned CLI register in `packages/cli/src/commands/schedule/index.ts:14–99`
covers create, inspect, logs and actions. Its heartbeat commands in
`packages/cli/src/commands/heartbeat/index.ts:35–155` establish the agent
caller's ownership. The web packet covers the list, row actions and
sheets in the UI references above.

## Editing decisions

- Purpose. Create and edit recurring new-agent schedules in the same
  place as their rows, with an explicit read-only view for heartbeats.
- Constraint. `schedule.update` replaces a whole revision and changes
  the authorizing principal to the caller. Old definitions can carry
  elapsed cadence, an expiry, a non-default catch-up policy or choices
  the current harness probe omits. Preserving those values avoids a
  behavior change when editing another field.
- Alternatives. Converting every interval to cron changes its timing
  across daylight-saving transitions. Sending a partial edit is not
  supported by this wire method. Editing an agent-owned heartbeat with
  that method also changes when the node cleans it up.
- Choice. The dialog uses the existing composer catalogue and option
  probe, kit controls and the node's cadence preview. It sets an hourly
  cron in the local device zone for creation. A saved zone or elapsed
  interval stays as stored. Updates preserve unedited fields and carry
  the original revision. The read-only heartbeat sheet displays Target,
  Cadence and Host for every existing-session target. Heartbeats can be
  deleted from the row. Cadence-only editing depends on TRACKER 839.
- Differences. The Model field reuses the composer picker and shows the
  provider and model in one trigger. Thinking and Mode have their own
  fields. A profile appears when the machine offers choices. The node cannot mint a fresh worktree
  for each run, so only an existing project is selectable on creation.
  An existing worktree or CWD stays as stored on edits. Old elapsed
  cadence is displayed as a fixed value. The app shows the next instant
  supplied by the node and validates an invalid run cap inline. A
  missing device zone uses the machine's zone when known. When the
  harness cannot return a model catalogue, the composer can still
  schedule the selected agent with its default model.
