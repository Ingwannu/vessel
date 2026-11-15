# Checked owned process cleanup (W16 prerequisite)

Status: source implementation; narrow execution and V3 review owed. This is
one additive shared mechanism requested by the coordinator, not F6 or desktop
package acceptance. It has no protocol, schema or node method.

## Baseline and ownership

This portable slice starts directly at the announced green node head
ede20014b44609d1f333c48b0fa65acad6a977d9. It contains ONLY the checked-wait
mechanism and its tests/docs; no W15 local-node crate, release/detach states,
proto local_launch, store or node bin-support ancestry. The earlier622465a9
mechanism passed scoped V3 source+strict+39 library tests, but that branch could
not land because its parent joined unreviewed W15 work. This rebuild retains
checked.rs, error-source changes and all eight tests, adapting only the normal
owned Child constructor and direct Windows inner handle. Checked wait does not
need Released/Releasable or optional-inner state. Nodectl's identity::spawn
follows this sole-writer prerequisite; W15 adapts its own later release join.

## Decision Log

- Purpose and intent: a management caller must not mistake successful leader
  wait for a successfully requested group/job cleanup when that first sweep
  actually failed.
- Existing asset and constraint: proc already owns Unix groups, Windows jobs,
  the macOS EPERM/reap rule and drop ownership. Legacy Child::wait deliberately
  logs a sweep error and returns leader status. Signal after Reaped does not
  signal again. ProcessLifetime ended is authority revocation, not OS death.
- Alternatives: replace legacy wait semantics (break existing callers), make
  another platform reaper (two cleanup owners), retry a group id later (it may
  have been reused), or retain the original owned result.
- Choice: additive wait_checked returns private-constructor OwnedGroupExit or
  CheckedWaitError with the concrete WaitError/first Arc<SignalError>. Keep the
  first sweep outcome; never run a delayed second sweep. The existing wait
  signature, leader return value, logging behavior stay the same.
- Why: only the owned transition has the original group/job context. The Unix
  refused-signal path may reap then sweep inside signal_with_leader, so that
  result is recorded there too, before a later wait_checked call.
- Tradeoff and evidence scope: receipt means leader reaped plus accepted
  cleanup request (including already absent), not observed death of every
  descendant, escaped process or account process. The pre-existing immediate
  post-reap group-id race remains documented by ProcessGroup; later calls add
  no new signal. Missing evidence refuses with Unrecorded.

## Modules and data flow

child.rs owns the real handle/state transition and invokes checked::FirstSweep
at the first owned sweep. checked.rs retains immutable result and mints the
receipt. error.rs makes SignalError cheaply Clone by privately sharing the
original io::Error; Display and its concrete io::Error source are unchanged.
This lets signal preserve its return type and retain the same cause. No error
is flattened to a string or reconstructed from a subset of an OS error.

wait_checked first performs the ordinary cancel-safe wait, then reads the
retained outcome. It grants no signal capability and is never minted from a
stored PID, a read-only lifetime observer or a detached child.

## Verification

New library cases exercise first-success/failure retention without invoking
another signal, original error-source pointer identity, legacy wait followed
by repeated checked waits, missing-proof refusal and cancellation followed by
the actual owned child exit. Child cases launch only this test executable,
with cleared environment (SystemRoot explicitly on Windows) and an isolated
stdin probe. Synthetic refusal is injected only into a private owned test
instance after reaping; no account, setuid executable, permission or unrelated
process is modified to create EPERM. These fixtures do not claim actual macOS
EPERM or Windows job execution proof. Existing process/desktop/fake-helper
descendant tests and their package gates remain separate evidence.

All Rust checks run on the build VM; no local Cargo/rustc. One
meaningful narrow suite is strict proc all-target Clippy plus proc library
tests; remote test prerequisites follow the installed A5 plan unchanged.
