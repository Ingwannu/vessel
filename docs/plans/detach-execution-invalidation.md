# Detach invalidates the current execution before its first effect

[Decision Log]
- Purpose: a captured pre-detach execution must not survive the actual detach
  transition. The already-adopted Agents execution watch owns invalidation;
  process liveness and serialized UUIDs are not admission proof.
- Existing assets: Sessions::detach already owns the marking lock through
  durable receipt commit and live publication, in a cancellation-safe tracked
  task. Parentage validates the immutable audit against its durable receipt.
- Alternatives: invalidating after the store write leaves an effect-before-fence
  window; invalidating every retry revokes a newer execution without a new
  transition. A second execution registry or capture API would duplicate owners.
- Choice: under that same marking lock, read authoritative committed metadata,
  refuse Root/corruption, invalidate Attached with ExecutionInvalidation::Detach
  immediately before store.detach, and leave Detached repeats unchanged. The
  writer still owns the atomic first/repeat receipt decision and publication.
- Tradeoff: an admitted first detach whose write subsequently fails still
  invalidates its former execution. This conservative fence must precede the
  effect; an error does not retroactively authorize an older captured intent.
  A session without an Agents entry creates no synthetic execution/watch.

The diagnostic-only cfg(test) observer subscribes to the producer's existing
watch under its registry lock. It exposes no capture, grant, admission, serialized
stamp or new channel. The producer owner explicitly delegated this tiny observer
for the detach regression; it is not a replacement for future queue admission.

Two real fake-backed tests exercise successful commit/publication, repeat and
never-child refusal, missing execution, and both sides of the actual marking
lock. Closing only the Store writer leaves authoritative reads available and
forces the first write to fail: the test must observe invalidation despite no
receipt commit. This distinguishes before-effect ordering from a late call.
A test-local owner retains each created child before the next fallible store
phase. Scenario assertion failures/timeouts are caught so the actual fake
shutdown is joined before the original failure is resumed. The40-second outer
budget includes5-second setup,25-second scenario and5-second cleanup phases;
each launch/store/transition/shutdown phase still has its5-second budget.
This is cleanup ownership, not a longer product or per-operation deadline.
Named launch/store/transition/shutdown phases sit within that outer budget;
no database polling, sleeps or second synchronization registry are used.

This leaf starts on the announced invalidation producer d5bfde87, independently
of the frozen provider/history package. No new protocol, schema, process stop,
credential, workspace, native handle or history identity is introduced. Source
and tests remain unexecuted until the receiving narrow checks complete.

The first exact1ff strict run reported one unnecessary qualification in the
existing parentage mapper after importing Parentage for the new callsite. Its
correction uses that same type directly, with no policy change. The test-owner
cleanup strengthening above is included in the same ordinary correction; no
runtime result is inferred from the first strict failure.
