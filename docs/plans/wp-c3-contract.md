# Git comparison values producer (ADR0087)

Status: V3 row196 correction prepared on the announced82ae7c91 staged-row base;
changed-source narrow/review pending. No method is admitted or implemented here.

[Decision Log]
- Purpose: native and node consumers must share one canonical comparison contract,
  without importing a node runtime into the app or relabelling legacy Base as
  committed-only.
- Existing assets: Quality B's single method table and typed Request association,
  existing DiffSummary/FileDiff/WorkspaceRoot/WorktreeBase values and client call.
- Alternatives: an app-local DTO or raw method string duplicates protocol rules;
  assigning a provisional number would admit incomplete runtime work.
- Choice: isolated proto/client values and staged Read/unary table rows for
  git.compare and git.compare_file, with GIT_PANEL=None. The two variants are
  registered in the existing agent-deny policy arm, one line per method. No
  runtime, transport, absolute path, dependency, authority or number is added.
- Effect: the existing client refuses an unassigned method before sending a frame.
  Integration can share these values early; runtime and Quality B admission remain
  separate, and old git.changes/git.diff retain their exact existing semantics.

The contract follows Paseo checkout-git.ts comparison mode selection and the
checkout-diff-cache forced-read policy. All includes working changes, Committed
excludes them, Uncommitted does not depend on a base, and immutable Commit keeps
Vessel's existing explicit commit extension. Whitespace and forced-read reason
are enums. File offsets remain typed hunk continuation, not text patch parsing.
NoBase/TooLarge/Ready and BaseMismatch preserve their distinct facts. A mismatch
retains the recorded and selected identities without declaring either correct.

All new method rows are at the table's end, staged while their runtime is absent.
The shared Handler defaults to typed Unsupported, the guard emits no Target or
NodeService forwarding, and its compile-time assertion requires GIT_PANEL=None.
The runtime wiring commit must change staged to pending before assignment. Tests pin Read/unary/None, path validation, normalized round trips,
refusal facts, and the client returning UnassignedProtocol without a request
frame. Forwarding and legacy field/reply admissions are exhaustively classified;
no wildcard or fake version is introduced. This producer does not claim runtime,
node mutation, observation, whole-package, native capture or real-forge proof.

## Resume point

The source comes from the node package's comparison contract only. The runtime,
cache, history and other C3 work remain on the separate node worktree. Run remote
proto/client all-target Clippy and library tests on this exact isolated source,
then request V3 quick source review. Only that exact reviewed producer may be
provisionally adopted by native code; pending stays Unsupported until admission.

## Review correction before staged registration

V3 returned the two names in WORKSPACE_FAMILIES_LATER: that exception is only
for assigned methods. Both remain outside the exception, so the shared family
test continues to assert None in addition to the package-specific Read/unary/None
fixtures. No admission is assigned. The next integration depends on the announced
node head carrying the shared staged-row seam; the previous nested-vesseld test
attempt stopped before executing library tests and does not prove root readiness.

## Announced staged-row prerequisite installed

Merged82ae7c91 (gate46g counted by the coordinator with one named web unit load
timeout; Rust2389/0/12, e2e68/68, forge4/0, no wire difference). The leaf now
uses staged for both methods and adds exactly GitCompare/GitCompareFile to the
existing refused=>true arm in workspace/policy.rs. There is no other own node
edit, Target impl or NodeService handler. Proto fixtures additionally assert the
staged form, while the shared workspace-family None guard remains active.

The native consumer must have the shared staged-row prerequisite through its
announced package base or an explicitly approved shared producer; merging this
whole node branch into the app is not authorized. Until that prerequisite and
this reviewed leaf are present, existing app scopes stay Unsupported. Earlier
2fc Clippy passes and failed nested-vesseld tests keep their original source;
the following changed-source checks must prove this join before V3 recheck.

## Exact staged leaf check result

Exact19460b8c4a4702f4c30de88a8250bac9c2676de6 passed changed-source remote
Proto/Client/Node all-target strict Clippy0 (run
wp-git-values-narrow-20261115T131237Z-19460b8c). Its independent library run
wp-git-values-tests-20261115T131237Z-19460b8c passed web-ci/fakes/vesseld/cargo,
all0/no skips: Client80+Proto322=402 passed,0 failed,0 ignored. Proto/Client
actually compiled in the library step, and Node compiled in its prerequisite.
No runtime, admission number, TS generation, native or whole-package pass is
inferred. V3 same-row correction review is requested, disposition not assumed.
This result addition changes only this Markdown plan; no unchanged gate rerun.
