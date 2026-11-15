# Checked owned process cleanup (W16 prerequisite)

Status: source implementation; narrow execution and V3 review owed. This is
one additive shared mechanism requested by the coordinator, not F6 or desktop
package acceptance. It has no protocol, schema or node method.

## Baseline and ownership

Reviewed node head ba45a6e3 and exact workspace process carrier a7697906 are
merged. The desktop owner supplied the release/observation join 9821de008;
that join's changed execution is still its owner's debt. W16 modifies Child
only after W15 handed off its sole writer role; nodectl identity::spawn follows
this prerequisite. Existing Bound/Releasable, Released, optional inner-handle
ownership and ProcessLifetime revocation points are preserved.

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
  signature, leader return value, logging and detach behavior stay the same.
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

Checks use the remote lanes under the coordinator's 09:2x capacity rule. One
meaningful narrow suite is strict proc all-target Clippy plus proc library
tests; remote test prerequisites follow the installed A5 plan unchanged.
