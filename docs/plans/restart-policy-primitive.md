# Recovery policy primitive in the common settings producer

This leaf is source-only and awaits the shared producer's strict/tests/gate.
It registers no startup coordinator, session journal, agent, helper, prompt
or CLI capability. The feature number remains unassigned.

[Decision Log]
- Purpose: common saved settings must retain real policy/debt/seal state
  without importing an unqualified recovery engine or dead admission facade.
- Existing assets: the same mutex-owned Policy and concrete RecoveryApply
  over the canonical settings operation and Store writer.
- Choice: retain effective current value, provisional Off, durable generation,
  cleanup/apply debt and irreversible stopping seal in this primitive.
  Engine-only watch/snapshot/check/admit methods belong to the later consumer
  extension of this same type/state/mutex. No second Policy or cfg fallback.
- Consequences: the common settings read uses current() from the actual
  primitive. It reports saved values, not a capability acknowledgement.
  Request/reply version admission and redaction remain the canonical owners.

The producer needs no change-notification channel until an engine actually
subscribes. The consumer adds its watch notifications and admission impl when
it is qualified; it cannot infer readiness from the producer's gate. The
original complete admission assertions stay on the preserved consumer source
branch. They are not replaced by test-only engine methods here.

Producer tests observe private primitive transitions and actual Store debt:
failed Off save preserves value and generation; saved Off advances generation;
a late saved On and settlement cannot reset the private stopping seal; actual
On acknowledgement clears synchronization debt; actual Store failure retains
cleanup/apply debt and makes same-value retry fail explicitly. These tests
make no helper/process admission or restart execution claim.

The common owner consumes RecoveryApply.current() in Settings.current() and
retains all existing candidate/file/queue/boot/shutdown assembly. This leaf
edits only the policy, concrete hook, their tests and this note. Joint checks
are still owed; the recovery consumer remains unwired and source-returned
until its exact-run Kill and physical/native-version ownership fixes qualify.
