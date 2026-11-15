# Schedule list state and reply ownership

[Decision Log]
- Purpose: keep the schedules list useful when only some machines answer, and
  prevent a delayed read or an uncertain write from fabricating current state.
- Existing implementation and constraints: the accepted schedule rules already
  own state buckets, action ordering, authority evidence and node next_due. They
  remain the authority. Other areas have their own caches; visiting them must
  not be a prerequisite for this list's project/worktree target facts.
- Alternatives: optimistic mutation and rollback, one fleet-wide error, or a
  display clock deciding whether a row has finished.
- Choice: one cache per validated book key; typed request tickets and operations;
  a single named four-call bound; an outstanding-request ledger that includes
  invalidated reads until their actual replies arrive; node-authored snapshots
  with independently fallible target facts. No display or transport I/O here.
- Why and cost: stale replies cannot restore a deleted row or release a newer
  request's capacity. Lost writes block repetition until a fresh read; reads
  never automatically retry writes. Cached rows survive refresh failures. The
  display clock and prepared Fluent text are separate presentation concerns.

Paseo references, read whole before this change:
- schedules/aggregated-schedules.test.ts: its four connecting/empty/content
  inputs are the first four portable aggregate.json cases. The fifth comes
  from screens/schedules-screen-state.test.ts's failed-load retry decision.
- hooks/use-schedules.ts: named five-second stale interval, host identity and
  connection-sensitive queries; components/schedules/schedules-table.ts:34–53
  and59–160: one card, independently pending rows and mutation ownership.
- components/schedules/schedule-row.tsx:269–327: keep the original operation's
  pending label if a concurrent read changes the cadence action beneath it.
- hooks/use-schedule-mutations.ts: settle invalidates the list. Vessel retains
  the previously accepted deliberate difference: state only changes on the
  node's reply, without optimistic pause/delete. ADR0033 also permits execution
  actions for recurring prompts, which Paseo's execution menu hides.

The read state and mutation state each own their correlation rule. A fresh
connection clears a previous Forbidden refusal, but does not pretend an
in-flight write never happened. An uncertain run has only a read recovery path;
only an explicit later action may issue another run. Wrong reply kind/identity
is also uncertain. Duplicate schedule ids reject that read and preserve the
previous snapshot. Filters/search use the generated catalog introduced in the
preceding words commit; no dynamic formatting happens during row drawing.

The coordinator's three small accepted follow-ups are included: Korean machine
copy uses 머신 (including exact old expectations), each schedule_action branch
occurs once with its output unchanged, and the interval picker offers minutes,
hours and days unless the opened value requires seconds to remain exact. New
rules have tests. Accepted cadence/editor error tables were not migrated into
Fluent or duplicated.

Narrow validation:38 schedule tests pass, zero failed,833 filtered out; View
all-target Clippy returns zero and actually checks View. The preceding attempt
retained two failing old Korean expectations before their explicitly requested
machine terminology update. No test was removed to address those failures.
Formatting and native size/boundary/token checks also pass. The complete merged
native A13 gate remains owed at this checkpoint.

This is a view-model prerequisite, not a delivered list screen. Link calls,
route/sidebar wiring, target/time preparation, fake-fleet additions and every
product capture remain pending. No new control or area-local banner is drawn;
the partial-host banner profile still awaits its shared owner assignment. No
other area or node crate is edited by this prerequisite.
