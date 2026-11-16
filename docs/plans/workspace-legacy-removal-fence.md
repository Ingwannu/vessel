# Legacy cleanup cannot bypass workspace registry ownership

Status: source prerequisite on announced node head 93970acb; VM checks, V3 review
and combined-line proof are owed. Actual provisioning remains unavailable until
its owning package has this checked prerequisite and its other admission facts.

Narrowed by ADR 0106 (2026-11-16, coordinator ruling "option A narrowed"):
- A `Local` descriptor that is Active or Archived owns no directory, so it no
  longer blocks.
- Legacy project unregister archives the project's own `Local` Active
  descriptors in the same store write that forgets the project (Paseo TRACKER
  258), and keeps the records.
- `Managed` backings, every reservation stage, and descriptors that are
  Archiving or Restoring still block.
- W11's pinned test
  `registry_fence_empty_local_and_archived_workspaces_block_legacy_project_unregister`
  was rewritten deliberately into the two tests named under "Authored tests"
  below.

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
  reservation intents for exact ProjectId. Any reservation stage, any `Managed`
  descriptor and any Archiving/Restoring descriptor blocks the legacy path; a
  `Local` Active/Archived descriptor does not (ADR 0106). Project unregister
  checks under its existing metadata lock; worktree remove/retire obtains the same bounded ProjectAdmission before the check,
  rechecks its WorktreeId/project association and moves its own Arc into the
  existing tracked task through teardown, Git removal and Store acknowledgement.
- Reason: the provisioning driver retains that same admission through descriptor
  finalization, so publication cannot enter the gap between check and cleanup.
- Impact: this is intentionally conservative at project scope for owned backings,
  transitions and Reserved/Creating/Uncertain/Ready reservations. An unowned
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

Authored tests (not executed): (ADR 0106) unregister archives Local Active
descriptors and keeps Local Archived ones, while Local Archiving/Restoring still
block (`registry_fence_local_workspaces_are_archived_by_legacy_project_unregister`,
`registry_fence_local_workspace_in_transition_still_blocks_legacy_project_unregister`); Managed ownership blocks Discard before an approved marker-writing
teardown and retains dirty files; an uncertain pre-publication reservation also
blocks; actual retained admission orders descriptor publication ahead of removal;
automatic retire keeps backing while an unrelated unowned worktree still removes;
64-slot saturation refuses before detached cleanup and recovers after release.
Three SQLite tests cover descriptor lifecycle, all four reservation stages and
malformed retained intent. Existing worktree safety tests remain required.

## Why this prerequisite cannot scope the existing hold to one ProjectId

The current ProjectAdmission owns project_registrations: one Arc<Mutex<()>> shared
by all project registration/removal and the admitted backing creator. ProjectEntry
has no per-project metadata guard. Taking another independent per-project lock
only for cleanup would not serialize with those existing publishers and would
reopen the checked-read/publication gap. Converting the canonical admission,
registration, unregister and publisher paths together needs a separate bounded
ownership design and regression set; silently weakening this prerequisite is not
an equivalent local refactor. The existing shared 64-holder/waiter admission cap
and five-minute teardown limit remain unchanged. A C1 TRACKER follow-up records
per-project admission as owed; this prerequisite accepts the global serialization
cost explicitly under the coordinator's bounded-scope ruling. No caller bypass,
new registry, timeout or extra unbounded waiter lane is introduced.
