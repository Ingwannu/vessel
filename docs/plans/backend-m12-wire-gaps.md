# Track N3: wire gaps

Status: in progress. Each row is a node slice, reviewed separately.

## Row 1: machine time zone and schedule preview — contract

[Decision Log]
- Purpose: a schedule editor must show the target machine's zone and the
  same due times that its scheduler will save.
- Existing assets and constraints: ADR 0033 already decides cron, intervals,
  gaps and folds; `automation::time::Due` implements them on the node.
- Alternatives: repeat the calculation in clients, or move I/O and the zone
  database into the protocol crate. Both add a second policy or dependency.
- Choice: extend the existing node implementation with a read-only RPC.
- Why: one calculation, one set of DST rules, no session or saved schedule.
- Effects: protocol 12 adds the call; protocol 2 remains the minimum.

`NodeInfo.timezone` is an optional `TimeZoneName`, captured at start from
  the system's IANA name. Unknown stays absent; old nodes omit it and old
  clients ignore it. No fallback claim that the machine is in UTC.

`schedule.preview` (read, since 12) takes `cadence`, optional `after` and
`count: PreviewCount` (1–10, default 5). It returns `after` and `runs` (UTC
instants strictly after it). Without `after`, the node reads its clock once;
an interval without an anchor uses that instant. It calls the scheduler's
`Due`, on the blocking pool, with four node-wide concurrent previews at
most. Each search keeps ADR 0033's eight-year bound. It writes nothing.

Refusals: `invalid_request` with typed `SchedulePreviewError` data for an
unknown zone, unknown cadence, or no future run; `unavailable` when the
preview slots are held. No partial list. Read-only devices and agents may
call; a delegated caller retains the effective read scope. Old peers refuse
this method by its declared version before execution.

`NodeClient::preview_schedule` and `vessel schedule preview` expose it.
The CLI's omitted `--tz` uses the target machine's reported zone, and asks
for an explicit zone when it is unknown.

References: Paseo `packages/server/src/server/schedule/cron.ts` and its
`cron.test.ts`, schedule service and CLI schedule commands. Vessel retains
ADR 0033's intentional gap/fold semantics rather than changing scheduling
policy to match a different implementation.

## Checks and progress

Not built yet. No preview service or real agent is used.
