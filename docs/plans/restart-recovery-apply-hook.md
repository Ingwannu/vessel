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
