# Recovery settlement hook for the shared settings owner

Source-only handoff on the agreed shared-owner contract. This leaf adds no
module registration or active settings behavior and has not compiled by itself.
The recovery runtime and Store projection remain a separate unmerged package;
the joint assembly and changed-tree checks must precede any runtime claim.

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
recovery runtime registers this module; a policy default or hidden wire field
must never be interpreted as a capability acknowledgement or confirmed Off.

The hook requires the recovery package's existing
`Store::set_restart_policy(enabled) -> Result<(), StoreError>`; it introduces
no new Store method or alternate SQL writer. The shared settings implementation
owns file decoding and the opaque saved model. This leaf deliberately supplies
no public SavedNodeSettings, settings_record writer, compatibility parser,
queue, Opening adapter or shutdown orchestration.

The hook tests cover real Store closure failures, retained cleanup/apply debt,
unchanged-value failure, provisional Off rollback and irreversible final effect
closure. They are currently unregistered source; they require the joint Store
and recovery assembly before strict checks or execution. Held real file I/O,
FIFO saturation/caller loss, actual startup snapshot and shutdown sequencing
remain tests of the common owner and the final joint package.

## Resume point

Port the recovery runtime and Store projection into the agreed joint assembly
without its old parallel settings model. Register this hook there. Consume the
reviewed common safe decoder edge and qualified physical preparation seam;
then run one combined narrow set and enclosing package gate. No standalone
source leaf, old green gate or unassigned protocol field proves that join.


## SQL and optional request-field port

The next source leaf ports the existing recovery projection and its original
regressions onto the agreed joint base. It excludes settings_file, settings_record,
the old saved model, settings lane and Opening. The SQL operation remains inside
the existing Store writer; checkpoint, creation provenance, reopen fencing and
indexed startup intent reads share its transaction and post-commit notifications.

The migration follows the joint base's registry migration through the one named
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
assembly; this source leaf alone is not a compilable runtime handoff.


## Session journal port

The session leaf ports the existing single ordered JournalEntry persistence
stream, durable person-stop barrier, restart checkpoint admission and prepared
resume split. Records-only runtimes keep their existing behavior. Head process
lifetime, stop failure and provider fields are retained. The structured and
terminal test collectors and private-directory drain only adapt the receiver
to JournalEntry and extract records with a test-only records-mode expectation;
no assertion, ordering or deadline changes.

This leaf does not qualify blocking preparation. Its prepared-resume consumer
will be adapted to the common physically retained ownership seam after that
producer's checks and source review. The node must then retain its original
attempt through actual preparation settlement and recheck authority/policy at
final admission. No second claim registry, tracker or quota is introduced.
The source port remains uncompiled until the complete joint assembly.
