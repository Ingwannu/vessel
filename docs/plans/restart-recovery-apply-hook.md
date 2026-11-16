# Recovery settlement hook for the shared settings owner

The concrete hook is assembled into the common settings producer. Focused Node
settings cases execute the actual Store settlement and failure paths; complete
source review and the enclosing gate remain owed. Recovery engine/runtime
activation is a separate unqualified consumer and remains unwired.

[Decision Log]
- Purpose: preserve durable Off cleanup, same-value retry and a shutdown effect
  fence inside the canonical accepted settings operation.
- Existing assets: the recovery Policy's final-enqueue mutex and the Store's
  typed restart-policy operation; the shared settings owner retains its opaque
  document, writer, queue and saved guard.
- Choice: add one concrete RecoveryApply over those two assets. A second
  writer, callback framework, parser or quota would split ownership.
- Consequences: actual Store failure remains typed debt and can delay drain;
  no caller cancellation or synthetic completion erases that obligation.

## Supplied files and integration edge

`crates/vessel-node/src/settings/recovery_apply.rs` implements the exact
shared contract methods. The owning settings module registers it and calls
settle_before before preparation, begin_write immediately before real I/O,
write_failed on actual failure, saved after persistence, and settle_saved
after publication, including the unchanged-value path.

`crates/vessel-node/src/recovery/policy.rs` is the recovery-owned policy port.
Its shutdown seal shares the same short mutex as actual effect enqueue and
is irreversible across later apply, cleaned and write_failed calls. The
common producer registers this core; a policy default or hidden wire field
must never be interpreted as a capability acknowledgement or confirmed Off.

The hook requires the recovery package's existing
`Store::set_restart_policy(enabled) -> Result<(), StoreError>`; it introduces
no new Store method or alternate SQL writer. The shared settings implementation
owns file decoding and the opaque saved model. This leaf deliberately supplies
no public SavedNodeSettings, settings_record writer, compatibility parser,
queue, Opening adapter or shutdown orchestration.

The hook tests cover real Store closure failures, retained cleanup/apply debt,
unchanged-value failure, provisional Off rollback and irreversible final effect
closure. The registered producer hook tests execute real Store ACK/failure/debt
paths; engine-only admission assertions stay with the later consumer extension.
Held real file I/O, lane saturation/caller loss, startup snapshot and shutdown
sequencing also pass in the focused common-owner settings suite. These scoped
tests do not substitute for SQL integration or the final joint package gate.

## Resume point

The real Policy and passive Store projection are assembled without the old
parallel settings model. The canonical safe decoder is reused. Finish the common
producer's remaining checks and enclosing gate before publishing its seam.
The later recovery runtime must separately consume qualified physical
preparation; no producer result or unassigned protocol field proves that join.


## SQL and optional request-field port

The next source leaf ports the existing recovery projection and its original
regressions onto the agreed joint base. It excludes settings_file, settings_record,
the old saved model, settings lane and Opening. The SQL operation remains inside
the existing Store writer; checkpoint, creation provenance, reopen fencing and
indexed startup intent reads share its transaction and post-commit notifications.

The migration follows the current base's background/listing migration through the one named
RESTART_SCHEMA constant. Existing migration order and version facts remain.
Mechanical port conflicts retain registry, subagent and diagnostic registrations,
all writer Answer variants, and head rollback statements. The inbox rollback and
title-origin rollback only gain undo for the recovery tables before their original
statements; no head assertion, count or deadline changes.

The wire input port adds the optional SetNodeSettings.restart_recovery patch and
GetNodeSettings.include_restart_recovery opt-in, with Set becoming Clone. Both
fields use the canonical recursive Call requirement and the same unassigned
feature constant; there is no local version or new method. Existing response
projection for both Get and Set remains the joint base's implementation. The
explicit request fields and all forwarding wrappers remain refused while the
feature is unassigned. Existing callers/literals must be completed during joint
assembly; those mechanical adapters are now present in the common producer.
