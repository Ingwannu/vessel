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


## Node runtime-only port

The node source leaf adds recovery ownership, typed checkpoint persistence,
creation provenance, fresh device fences and exact-run person-stop routing.
It contains no settings lane, disk record, Opening or boot/shutdown replacement.
The common settings implementation owns their actual joint assembly.

Approved semantic joins keep the guarded dispatch boxed future and its
completed diagnostics unchanged around the added Operation lease. All refusal
paths still pass through that diagnostic boundary. Connection initialization
retains both its lifecycle token and the new operation field. Reopen changes
only the Store call to reopen_exact; the existing match, restore_metadata,
subagent metadata and cleanup remain. The models test collector only adapts
JournalEntry to the records-only event it previously collected.

This source includes the exact-run Kill repair and its held real-ACK cases;
the original physical-preparation return remains open until the qualified
shared seam and retained node-attempt consumer join. Recovery fixtures still
require canonical settings test adapter updates; the old settings implementation
is not included merely to make them compile. Checkpoints from previous source
lineages establish no execution claim for this port.


## Client, CLI and fake-provider source join

The client uses explicit capability reads and the existing typed settings
mutation. The CLI adds resume-on-startup and verbatim resume-prompt options;
both return canonical unassigned admission until activation. Their source
port preserves the shared base's other commands. Existing settings e2e literals
only gain default request fields; all assertions and deadlines remain. The
fake provider supplies actual load/prompt-response barriers and explicit test
process termination, without a real agent or a fabricated commit result.

These are still source ports, awaiting canonical settings assembly, qualified
physical preparation, generated declarations and the joint checks. Neither
old execution evidence nor a successful source application proves this union.


## Canonical diagnostics and legacy setter regressions

RecoveryApply's own test module now seeds nondefault policy through the
canonical opaque SavedNodeSettings constructor and writer. It observes actual
server bytes for ordinary Get and parking/driver Set at every supported
settings peer version, retaining the saved policy after each operation. This
ports the original raw setter regression without reviving the removed record
API or bypassing wire authority. It also walks complete Display, Debug and
source chains from both canonical snapshot and compatibility reads of an
invalid private recovery value. Private typed-cause and actual startup logger
canaries remain at their shared codec/Opening owners. These source cases still
need the joint field/engine assembly and execution.
