# Ended-agent quiescence prerequisite

## Resume point

ADR 0103 is proposed. Only this contract and plan exist: no process, wait, stop,
drain, cleanup-debt registry, proof constructor or restore behavior is changed.
Implementation starts after the transferable marking prerequisite and canonical
mutation-clock package close. A source/gate review of those packages does not
qualify this new proof.

## Existing assets and decision

Modify the existing `vessel-proc` owned wait/first-cleanup evidence, the concrete
`vessel-agent::AgentProcess` result and the common `vessel-session` actor stop
path. Reuse the existing actual-run observation and bounded owner; no new
inspector, dependency, process-name matching, numeric-PID authority or second
metadata lock is proposed. Complete descendant settlement is an explicit shared
process-backend prerequisite, not a property of the current OwnedGroupExit.

Read reference: Paseo agent-manager.ts 1699–1705 and 1764–1797 waits old close,
drains events, preserves pending close and refuses failed/timed-out close. It
does not supply the stronger typed descendant certificate proposed here.

The new difference is deliberate: destructive root restore requires complete
owned agent settlement and output drain, rather than status or an accepted kill
request. Unsupported or missing physical evidence remains Busy/Unknown.

## Implementation order and owners

1. Review ADR 0103 and the shared `vessel-proc` complete-settlement capability.
   Keep existing OwnedGroupExit's narrower guarantee and first-attempt behavior.
2. Preserve typed errors through AgentProcess::wait and all actual callers,
   without an Option success fallback. The process and driver owners retain
   native physical facts; the lifecycle actor records and composes them.
3. Record output-drain completion/failure/deadline with the existing absolute
   budget, including blocked publication. Tests first for false-success paths.
4. Add the private exact-run proof and retained ended/forgotten agent debt.
   Caller cancellation and retry keep the same bounded accepted operation.
5. Give the root-bound terminal/restore composition owner typed agent inputs;
   its own root/admission guard and terminal proof remain separate. Restore
   stays unavailable until both producers are checked, reviewed and integrated.
6. One narrow sequence then one package gate, no real agents or production
   restores. New shared backend APIs or test fault seams require prior review.

## Unverified / deliberately absent

No descendant backend, proof API, retained-debt representation, wire/schema
assignment or production consumer exists in this package. Windows/macOS/Linux
physical settlement, restart debt, actual wait failure, escaped processes,
backpressured drain and caller-loss behavior all remain unexecuted. The current
status/registry view cannot be used as a positive restore adapter.
