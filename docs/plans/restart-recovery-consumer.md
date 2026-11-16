# Startup recovery consumer: receiving plan

## Status

This receiving branch starts at node `ed4cd13776cc32a2da0ffeb4ffa67e2e2548461e`.
It adds this plan only. Automatic recovery is not activated here. The common
settings producer and the prepared-runtime API require their own exact source
and execution qualification before their consumer is assembled.

The earlier recovery implementation at `23cf77eb29ec3ba6b0572082e3126a78831ad0ae`
has a green predecessor gate, but its package review returned lifetime and
private-diagnostic defects. That gate does not validate later corrections or
the receiving union. Its independent disk record and settings lane are excluded
from this branch; only the canonical settings owner may persist this policy.

## Decision log

- Purpose: restore an interrupted committed turn after restart without admitting
  a late process, losing an unresolved operation, or blocking a person's stop.
- Existing assets: the settings owner, Store restart projection, runtime task
  tracker and conversation claim already own the required state and work.
  Runtime preparation has a shared host-admission adapter under development.
- Alternatives: retaining the old parallel settings engine would create two disk
  and mutation owners; dropping preparation on cancellation would detach physical
  work; a second semaphore would obscure the original bound.
- Choice: extend those existing owners. Keep the original attempt and Store
  obligation until physical preparation and admitted effects actually settle.
- Consequences: blocked filesystem work or an unobserved helper root may retain
  capacity and prevent hard drain. This is explicit unresolved ownership rather
  than a successful cancellation or a fabricated stop acknowledgement.

No new dependency, wire method, protocol assignment or public event is planned
by this receiving step. The feature's single optional protocol constant remains
unassigned until integration supplies its value. Store migration numbering is
assigned when the common producer joins the current node line.

## Prerequisite boundaries

The common settings producer owns the opaque record, safe decoder, accepted
intent lane, atomic writer and startup snapshot. Its concrete recovery hook
uses the same Policy and Store. Policy Off cleanup debt and On application
settle within that owner, including identical-value operations and caller loss.
Shutdown seals effects before waiting for settings, drains accepted writes and
SQL work while the runtime remains available, then closes the runtime and Store.
The consumer adds no writer, compatibility parser, settings queue or policy copy.

The runtime's canonical LaunchAdmission observes its actual runtime and node
stop sources. Its final checks cover structured spawn and the terminal closure's
file-write and PTY-spawn admissions. The prepared-runtime view captures that
actual runtime handle internally; a caller cannot supply another runtime's
handle. The consumer must not reconstruct or replace this fence.

The preparation adapter retains the original caller owner and task-tracker token
inside actual lookup work and native version-helper ownership. Typed refusal is
terminal for that unadmitted effect. A temporary policy barrier returns Wait on
an existing watch, with subscription before the same admission check. A failed
Off save can restore admission without inventing a new attempt or permanently
canceling the old one.

Preparing a relaunch can execute a bounded native version helper. It must never
be described as preparation that executes no process. Helper admission and
session-process admission are separate effects with separate fresh checks.

## Consumer assembly order

1. Receive the exact qualified common settings and preparation source leaves;
   preserve their canonical APIs and recorded predecessor results.
2. Extend the same Policy core with engine watch, snapshot and effect-admission
   operations. Keep its existing state, generation, cleanup debt and permanent
   shutdown seal; do not restore a competing Policy implementation.
3. Port the restart journal and Store obligation adapters selectively. Preserve
   current-node collector behavior and migration sequence. Do not import the
   returned implementation's whole ancestry or old settings ownership.
4. Pass an Arc of the original attempt through the prepared-runtime host gate.
   Subscribe before checking the existing Policy/attempt arbiter and check the
   borrowed live cancellation, deadline and lifecycle within that arbiter.
5. Pin actual preparation outside the cancellable stage. A stage cancellation
   signals the original token, then awaits that same preparation's real
   settlement before returning its original reason or releasing ownership.
6. Before session launch and prompt enqueue, re-read stored creator authority
   through the existing Device fence after awaits. No cached principal, Owner
   fallback or synthetic permission probe authorizes these effects.
7. Port exact-run person-Kill admission and its real committed-ACK regressions.
   Enable startup orchestration only after the receiving narrow checks prove
   the joined preparation and stop boundaries.
8. Run the changed package's enclosing gate and submit its exact immutable
   union for independent review. Native and page consumers remain separate.

## Required receiving regressions

Use the fake providers and existing testkit. Coordination observes actual
lookup, journal commit, session record or child settlement; it adds no sleeps,
mutation retries, larger deadlines or weakened assertions.

- Hold physical preparations at the original attempt capacity. Cancel one
  through authenticated Kill, then stop the node. No additional physical
  preparation enters; the original capacity, Store obligations and the same
  runtime hard drain remain held until the actual work settles.
- Keep the preparation waiter alive across a held native lookup. Person Stop,
  committed Off, deadline expiry and node stop each refuse a subsequent helper
  or session effect. Check an actual version-spawn witness as well as session
  state; an empty PID list alone does not prove absence of helper execution.
- Pause PendingOff at the shared writer. The original attempt waits; a failed
  save restores the prior admission and wakes that same attempt. A successful
  Off invalidates its generation and a following On cannot rebase it.
- Retain an already-admitted version helper after waiter loss through actual
  root settlement. Root-unknown failure retains all original ownership and
  cannot acknowledge hard drain. No delayed numeric-PID retry is introduced.
- Hold an actual recovery prompt's committed Arm acknowledgement. Authenticated
  Kill must reach durable suppression and the exact original run while that
  delivery answer remains unresolved. Capacity stays occupied until settlement;
  an old pinned handle cannot kill a later manual replacement. Off after prompt
  admission leaves the already-admitted turn alive.
- Revoke or reduce the creator grant while real load/readiness is held. No next
  launch or prompt is admitted; cleanup affects only the same owned run.
- Check complete Display, Debug and public error-source chains for private
  setting canaries through the canonical full-record and compatibility decoders
  and actual startup diagnostics. Keep typed raw causes private.
- Cover clean shutdown and process kill, parked/archived/person-stopped exclusions,
  failed resume reporting, old-peer Get and ordinary Set reply projection, and
  same-value cleanup debt through the canonical settings owner.

The budgets come from their existing owners: Recovery::MAX_ATTEMPTS,
Recovery::MAX_OPERATIONS, the recovery admission deadline, SettingsAdmission's
queue bound and the recovery test CASE_DEADLINE. This plan introduces no copies
of their numeric values or substitute observation deadlines.

## Evidence scope

The held post-prompt Kill correction has earlier narrow causal and positive
execution evidence and a separate all-target strict check. That is mechanism
proof, not a receiving-package pass. The common producer's checks do not prove
startup orchestration, and a prepared-runtime source review does not prove its
real helper or drain regressions. Keep all predecessor failures and results
separate from the final union. No live provider, service-account deployment,
other operating system or native screen is validated by this plan.
