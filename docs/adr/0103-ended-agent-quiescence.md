# ADR 0103: evidence for an ended agent run to be quiescent

Status: proposed

## Purpose and intent

A destructive file restore must know that the agent runs which could still
write its root have actually settled. An ended session, a revoked process
observation, a successful signal, and a registry entry marked ready are different
facts. None alone proves that a writer and its descendants have stopped or that
its remaining output has been drained.

This decision supplies an agent-run proof to the root-bound restore boundary.
It does not authorize a restore, select its root, or replace that boundary's
terminal proof, admission fence and forgotten-work accounting. Production restore
stays Busy or Unknown while either required proof is absent.

## Existing implementation and reference

Paseo's `packages/server/src/server/agent/agent-manager.ts:1699-1705` awaits the
old session's close and event drain before preparing its closed state and starting
a replacement. `:1764-1797` retains the same pending close across request timeouts;
a retry awaits that release, and a timeout or failed close refuses replacement.
These are lifecycle ordering contracts. They are not a typed certificate that
every operating-system descendant died, and this ADR claims no such reference
behavior.

Existing Vessel assets:

- `vessel-proc::Child::wait` returns a concrete wait error, reaps its owned leader
  and records the first group/job cleanup request. `wait_checked` preserves that
  first outcome without sending another signal to a possibly reused numeric id.
- `vessel-proc::OwnedGroupExit` binds that observation to the original
  `ProcessLifetime`. Its documented scope is leader reap plus an accepted cleanup
  request, not observed death of every descendant or an escaped process.
- `vessel-agent::AgentProcess::wait` currently logs a wait failure and returns
  `None`, which also denotes an exit caused by a signal. Its caller loses the
  distinction between a known exit and an unknown wait outcome.
- `vessel-session` owns the actor, stop sequencing, native output consumption and
  process replacement. `moving::stop_agent` currently ignores the wait result
  and answers success even when its output-drain deadline wins.
- Existing process-group cleanup, process observations and terminal session scans
  have narrower scopes than complete descendant settlement. A scan or a count
  of accepted kill requests is not a substitute for that missing fact.

These existing owners are extended. Standard child waiting and the existing
Tokio/OS supervision are reused; no new dependency, subprocess inspector, shell
command, elevated permission or external service is introduced by this contract.

## Alternatives considered

1. Accept final session status or an empty live registry. This loses failed,
   forgotten and still-draining writers and is refused.
2. Treat `OwnedGroupExit`, revoked `ProcessLifetime` or a root-only PID signal as
   complete cleanup. Their declared evidence is narrower, so this is refused.
3. Rescan numeric PIDs and signal until nothing appears. Incomplete membership,
   identifier reuse and escaped descendants make this neither ownership nor
   complete settlement. No such fallback is added.
4. Preserve typed observations and compose a private proof only after every
   required owner reports completion. This leaves unsupported or uncertain
   backends unavailable, but keeps restore from inventing safety.

## Decision

### Typed wait, settlement and drain are separate results

`AgentProcess::wait` will preserve a typed wait/cleanup failure, with its original
source, instead of mapping it to the exit-code `None` case. Existing callers must
handle the distinction explicitly. A signalled but successfully reaped exit can
still have no numeric exit code; that is not a wait error.

The session actor retains three distinct outcomes for the same actual run:

- leader exit and the original owned cleanup outcome;
- complete descendant settlement from the process-supervision owner;
- complete native-output drain, or its concrete failure/cancellation/deadline.

The exact error and result names are implementation details to review before
wiring. They are enums and validating types, not booleans, strings or a nullable
exit code used as a success sentinel. A drain deadline remains an incomplete
drain. A closed reader channel is complete only when its owning adapter attests
that it ended normally; cancellation or a failed reader cannot mint EOF proof.
All awaits involved in draining must respect the existing absolute budget,
including publication backpressure. No deadline is extended to obtain success.

### A private ended-run proof

A proposed `EndedAgentQuiesced` value has private fields and no public constructor,
Default, wire decoder or timestamp-based conversion. Only the lifecycle owner
may produce it after all three outcomes above are complete. It identifies the
session and original actual run using the existing opaque run observation;
numeric PID equality cannot associate a receipt with a later run.

The value is evidence about one ended run, not an execution grant, a process
revocation capability, a session mutation clock, or permission to restore a root.
Cloning an old receipt cannot make a replacement run quiescent. A consumer must
hold its existing root/admission boundary, bind the receipt to its exact run set,
and reject stale or incomplete inventory before any file mutation. New starts
must stay excluded for that consumer's protected operation; the receipt alone
does not provide that exclusion.

A successful signal or leader reap may be retained as partial evidence while
settlement is pending. It cannot be promoted to `EndedAgentQuiesced`. A backend
without complete owned-descendant observation produces typed Unavailable or
Unknown, never an empty successful set. In particular, the current group-cleanup
receipt cannot by itself enable a positive production path on any platform.

### Ownership and the shared process boundary

| Owner | Responsibility | Must not claim |
|---|---|---|
| `vessel-proc` | Actual owned leader/group/job outcomes and a reviewed OS-specific complete-descendant settlement capability | Accepted signal equals every descendant's observed exit |
| `vessel-agent` | Preserve those typed outcomes through the concrete driver process and keep the actual run association | A failed wait equals signalled exit |
| `vessel-session` | Common stop/replacement ordering, recorded output drain and the private ended-agent proof | Final status alone is cleanup or restore authority |
| `vessel-node` lifecycle registry | Retain active, ended-unsettled and forgotten-agent cleanup debt for the protected run inventory | Removing a handle means its cleanup debt vanished |
| Root-bound terminal/restore boundary | Compose agent-run evidence and forgotten-agent debt with terminal proof and its root/admission lease | Manufacture the agent proof from status, cwd, PID or registry readiness |

Complete descendant settlement is a shared `vessel-proc` backend/API prerequisite.
Its implementation and OS ownership model need review with that crate's owner
before lifecycle can depend on a positive result. No competing descendant scanner
is added in an actor, driver, terminal registry or restore handler. The lower
crate owns physical process facts; the higher lifecycle owner composes them.

On Unix, a group request does not prove that an escaped descendant settled.
On Windows, a job termination request is likewise distinct from observing all
owned members settled. An unsupported backend returns a typed unavailable result;
no platform silently reports zero remaining writers. A new containment mechanism
or privilege requirement would need its own reviewed decision before use.

### Cancellation, retry and forgotten debt

Before operation admission, cancellation must leave no retained cleanup task.
After admission, the existing bounded owner retains the actual cleanup operation,
its run association and its first outcomes across caller loss. Accounting by a
TaskTracker alone is not a queue limit. A retry observes the same owned operation;
it does not signal a reused id or mint a replacement receipt.

An ended actor cannot be forgotten as clean while exit, descendants or drain are
unknown. The lifecycle registry retains that debt until exact settlement, or
reports it as Unknown to the root-bound consumer. A restart or missing historical
observation must not manufacture a clean run. No durable PID or session-status
backfill is a substitute for original ownership evidence. The eventual registry
representation and restart behavior are reviewed with the producer implementation.

### Restore integration remains closed

A file-restore caller consumes both the agent-run proof/debt inventory and the
separate terminal proof under its root-bound guard. It cannot produce either
from an ended session or a registry flag. No StartupHooks, filesystem restore,
new RPC, protocol assignment, migration or native activation is added by this
ADR. Missing proof keeps the existing Busy/Unknown result.

## Required forcing tests before implementation is accepted

- Preserve the concrete wait failure and source; no signalled-exit fallback or
  positive proof is returned. Test-only fault seams require their own approval.
- Keep proof pending when the owned leader exited but a descendant has not
  settled. Exercise an escaped/unavailable case without signalling any unrelated
  process or using a real agent.
- Hold output/drain publication and force the existing deadline; leader success
  must not hide incomplete output. Actual normal EOF produces the drain result.
- Cancel before admission and after accepted cleanup begins. Require no abandoned
  pre-admission task growth and retained bounded settlement after caller loss.
- Repeat waiting after success/refusal/cancellation and retain the first owned
  outcome without a second delayed signal. A different run or reused numeric PID
  cannot consume an old proof.
- Forget or restart with unsettled agent debt, and refuse a root-bound positive
  restore result. Status/Ready alone must be negative controls.
- Cover each supported native backend with its actual owned fake processes and
  acknowledge typed unavailable platforms. A pure fake's proof is not native
  operating-system acceptance.

## Advantages, costs and deliberate differences

The lifecycle, terminal and restore owners exchange typed evidence instead of
reinterpreting one another's status. Wait errors remain actionable and the stop
path records incomplete drain rather than concealing it. The cost is explicit
retained debt and a stronger physical supervision prerequisite: some restores
remain unavailable until their backend can prove settlement.

Compared with the read Paseo close/reload path, Vessel requires explicit
process-descendant and drain evidence before destructive root restore. This is a
stated safety difference, not a claim that Paseo provides the same certificate.
The existing reload history/log differences remain ADR 0063's responsibility.
No implementation, runtime acceptance, consumer readiness, actual OS failure,
real provider behavior or cross-platform success is asserted by this document.
