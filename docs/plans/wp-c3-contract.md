# Git comparison values producer (ADR0087)

Status: pending source/narrow review; no method is admitted or implemented here.

[Decision Log]
- Purpose: native and node consumers must share one canonical comparison contract,
  without importing a node runtime into the app or relabelling legacy Base as
  committed-only.
- Existing assets: Quality B's single method table and typed Request association,
  existing DiffSummary/FileDiff/WorkspaceRoot/WorktreeBase values and client call.
- Alternatives: an app-local DTO or raw method string duplicates protocol rules;
  assigning a provisional number would admit incomplete runtime work.
- Choice: isolated proto/client values and pending Read/unary table rows for
  git.compare and git.compare_file, with GIT_PANEL=None. No runtime, transport,
  absolute path, dependency, authority or protocol number is added.
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

All new method rows are at the table's end, pending until the integrator assigns
admission. Tests pin Read/unary/None, path validation, normalized round trips,
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
