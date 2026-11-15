# Legacy cleanup cannot bypass workspace registry ownership

Status: source prerequisite on announced node head 93970acb; VM checks, V3 review
and combined-line proof are owed. Actual provisioning remains unavailable until
its owning package has this checked prerequisite and its other admission facts.

[Decision Log]
- Purpose: an empty independent workspace or interrupted backing reservation is
  not represented by the legacy live-session/cwd checks. Legacy unregister,
  worktree removal and automatic session tidying must not destroy that state.
- Existing assets: canonical registry descriptors/reservations, bounded retained
  ProjectAdmission and tracked legacy removal already exist. No parallel registry,
  physical-root identity, process tracker or permission map is required here.
- Alternatives: path-based ownership inference, a snapshot followed by unlocked
  deletion, ignoring unpublished reservations, or a caller-supplied bypass.
- Choice: one Store read transaction scans canonical typed descriptors AND retained
  reservation intents for exact ProjectId. Any lifecycle/stage blocks the legacy
  path. Project unregister checks under its existing metadata lock; worktree
  remove/retire obtains the same bounded ProjectAdmission before the check,
  rechecks its WorktreeId/project association and moves its own Arc into the
  existing tracked task through teardown, Git removal and Store acknowledgement.
- Reason: the provisioning driver retains that same admission through descriptor
  finalization, so publication cannot enter the gap between check and cleanup.
- Impact: this is intentionally conservative at project scope, including Archived
  descriptors and Reserved/Creating/Uncertain/Ready reservations. An unowned
  project still follows the original legacy safety/approval/live-resource path.
  There is no boolean bypass or newly activated create/archive/delete method.

All trusted writers that introduce/rebind placement or backing references must
retain canonical project admission through their commit. Metadata rename/pin do
not rebind placement. A Store None is meaningful only under that retained hold;
it does not itself authorize deletion, attest exit, lifecycle generation or
physical root continuity. The query returns an existing WorkspaceId as evidence
of retention, never a newly minted claim/token. Read/decode errors fail closed.

The global metadata serialization can delay other project operations during
legacy cleanup. Approved teardown still uses its existing bounded five-minute
command limit; a teardown that re-enters project admission can wait until that
limit. This tradeoff needs source review with the prerequisite. No new timeout,
command approval, forced cleanup policy or shell execution rule is introduced.
Root exclusion, aliasing, resource quiescence and a dedicated W21 exclusive
claim-bearing cleanup remain separate incomplete authority; this fence does not
qualify them or let legacy routes stand in for workspace archive/restore.

Authored tests (not executed): empty Local Active/Archived ownership blocks
unregister; Managed ownership blocks Discard before an approved marker-writing
teardown and retains dirty files; an uncertain pre-publication reservation also
blocks; actual retained admission orders descriptor publication ahead of removal;
automatic retire keeps backing while an unrelated unowned worktree still removes;
64-slot saturation refuses before detached cleanup and recovers after release.
Three SQLite tests cover descriptor lifecycle, all four reservation stages and
malformed retained intent. Existing worktree safety tests remain required.
