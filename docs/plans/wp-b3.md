# Per-turn file checkpoints: implementation plan

ADR 0077 passed design review at `a852bde3`; acceptance is a contract decision,
not a product/runtime pass. The node branch consumes Quality B `29eebb8b`
and the shared `Request`/`UnexpectedReply` extraction `1c8d0b93`. The separate
app branch consumes package base `8d5513e4`; no app feature is implemented yet.

## References and existing assets

The exact reference/source mapping and stated differences live in
[ADR 0077](../adr/0077-per-turn-file-checkpoints.md). Paseo's files mode and
Claude SDK rewindFiles define files-only behavior; GOAT supplies turn-linked
changed-files presentation. Existing node snapshots recover a removed worktree
into a fresh one. Existing session.rewind changes conversation and leaves files.
Neither is a fallback for this feature.

Decision: extend session/workspace infrastructure with focused checkpoint
modules. Reuse RelPath, Git probing, regular-file defenses, bounded channels,
existing SHA-256 and SQLite. Strict enumeration refuses invalid paths rather
than reusing an existing lossy listing. Existing rustix supplies Linux fd-based
xattr refusal and rename exchange; the dependency stays dev-only during A10
staging. No provider SDK, tool-call-count inference or new crate edge.

## Current implementation and boundaries

- proto/checkpoint: validating images/metadata/limits/ids, effective permission
  policy, typed changes and errors, unary requests/responses. FILE_CHECKPOINTS
  remains None and PROTOCOL_VERSION is unchanged. The five canonical pending
  rows generate client/guard/envelope plumbing. No independent method table.
- node/checkpoint: test-only bounded capture owner, worker/root/quota leases and
  deadline fencing. Timeout acknowledges without joining blocked I/O; capacity
  remains occupied until actual cleanup. Candidate persistence and durable Ready
  activation are separate phases; uncertain activation retains the archive.
  The real store/coordinator assembly still needs connecting. No real prompt/TurnEnded wiring yet.
- node/service/checkpoints: registered methods explicitly refuse as unavailable.
  Guarded authorization and agent refusal remain in front of the seam. Ordinary
  human forwarding preserves destination checks. Pending calls refuse locally
  and at dispatch; no feature capability is advertised.
- store/checkpoint: test-only SQL operations and one production schema append
  after this base's title-origin schema. The migration runner is unchanged;
  the integrator adjusts its one schema constant in merge order. Restore
  admission reloads the capture's full key/session and frozen effective target.
  Durable per-path intent and compare-and-advance phases constrain terminal
  outcomes; human-principal-bound change pages share the frozen target/expiry, enforce
  bounded offsets and refuse expired or consumed previews; admission enforces
  the five-minute lifetime and four active previews per human principal; restart pins interrupted operations for recovery. Existing writer
  transaction wiring, retention and physical quota reconciliation remain owed.
- workspace/checkpoint: test-only strict enumeration, rooted nofollow reads,
  private immutable raw blobs, bounded streaming and canonical fingerprints.
  Files are fully staged before per-path mutation; rename exchange retains
  original inodes, no-replace creates avoid clobbering, and guarded rollback
  retains conflicting external edits. Apply and undo also compare inode identity,
  refusing an equal-content external replacement. Unsupported metadata on roots
  and intermediate directories refuses before file mutation. The node must
  durably journal each step
  and verify the entire covered scope before claiming success. Linux is staged;
  other platforms remain unavailable. Full enumeration/ignore/root revalidation
  at operation boundaries and crash-restart assembly remain owed.
- session/checkpoint: test-only bounded acknowledgement prototype. Actual queue,
  steer, prompt-delivery and persisted-turn-end integration remains owed.

## Validation and remaining work

Five-crate all-target clippy passed on `fa83036e`; checkpoint-filtered behavior
tests passed on `79b3171b` (27 passed, zero failed, six ignored). These precede
new filesystem restore and pending RPC registration and do not verify them.
New tests cover effective permissions, guarded restore/undo, non-root owner,
execute-only metadata, absent recreation, later-created deletion, path-journal
phases and forged session/worktree/boundary/generation admission. They remain
unverified until their exact compiled run completes. No package gate, code
review, permission/platform/crash recovery or native capture pass is claimed.

Next: compile the new mechanisms, regenerate TypeScript, run real migration
rollback tests, finish writer/coordinator assembly and durable Ready reconciliation,
then authenticated list/preview assembly, physical quota/retention and real session/
workspace exclusion. Replace ignored obligations only after implementation.
After node completion: one whole node gate and V3 review; after consuming the
reviewed node contract, native view/link/words/screen, complete confirmation,
result/status flow, captures, one native gate and V4 review. Reuse kit controls;
shared app/link roots receive only designated registration lines.

The ownership amendment to ADR 0077 persists the complete human principal,
including delegated origin; a delivery DeviceId alone cannot authorize a
plan. Local token/socket owners remain supported; direct/delegated agents
are refused. Cross-origin sharing and local-owner retry tests accompany the
amendment in the node package review.
