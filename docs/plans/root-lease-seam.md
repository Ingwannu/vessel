# Canonical physical-root borrow producer

Status: V3 returned exact bc9cacc6..5cc56ccf for project-subdirectory
admission and async-fixture failure cleanup. Its exact all-target strict check,
quality checks and eight tests passed; those results do not verify this revision.
The first correction passed all-target strict but its tests failed before seven
fixture bodies: a new aggregate setup clock expired. The revision separates
store and workspace initialization into retained named phases, keeping each bound
and the total case budget unchanged. It records a late phase and actual completion
rather than hiding the cause. Current source awaits ordinary checks and V3.
No consumer-ready hash, process quiescence or restore capability is claimed.
The full checkpoint branch's results never count as evidence for this leaf.

[Decision Log]
- Purpose: W17's future terminal quiescence proof must borrow the node's exact
  physical lease rather than create a second root lock or infer one from cwd.
- Assets: existing workspace registry, TaskTracker, safe nix filesystem APIs,
  SHA-256 and typed BlobHash already supply the needed dependencies.
- Alternatives: copying checkpoint restoration would couple the producer to
  unreviewed recovery/store machinery; a second mutex or workspace id as a
  physical key would authorize conflicting aliases.
- Choice: one bounded RootGate owner per Workspaces; opaque non-clone RootLease;
  StartGuard owns both opened cwd and selected ancestor FDs and exposes a borrow.
- Why: physical aliases share admission while logical workspace identity stays
  independent. Keeping the actual ancestor FD prevents inode reuse during a
  nested admission, including unlink/replacement. Project roots and worktree
  roots come from one existing registry snapshot. A project subdirectory must
  not receive independent admission from a writer addressing its project root.
- Costs: metadata I/O is bounded by retained worker permits but is not forcibly
  cancellable. Only one ordinary file-write consumer is wired in this leaf.

## Modules and data flow

vessel-proto::checkpoint holds only PhysicalRoot, preserving its canonical
BlobHash-backed representation. There are no checkpoint calls, events, store
schemas or protocol changes. vessel-workspace::physical owns safe opened-FD
identity and bounded parent traversal, with no admission authority.

workspace::metadata retains two blocking metadata permits through actual I/O.
workspace::admission chooses and pins the nearest registered ancestor; it keeps
only the selected ancestor FD rather than one descriptor per registered tree.
workspace::exclusion owns the bounded queue and leases, independent of any
session, logical workspace or terminal registry. Stop refuses new claims and
waits for existing lease owners; deadline expiry does not force-unlock I/O.

Nested/overlapping registrations keep their own nearest selected root and logical
identity, but cannot divide physical exclusion: the same gate compares bounded
opened-FD ancestor observations in both directions. A held parent refuses a child
and a held child refuses a parent, including a parent registered after the child
lease was issued. Sibling scopes sharing ancestors remain independent. This uses
no pathname prefix key, registry generation as authority, or second lock. Each
active observation is shared, bounded by the existing ancestry limit, and removed
with its lease. Selection retains only cwd plus the selected root FDs, not one FD
per registry record. Relative inputs refuse before ambient-cwd canonicalization
or metadata admission. Mutable mounts, concurrent external root moves and
race-free descriptor-relative mutation remain outside this producer's claim.

workspace::starting binds opened directories to the exact lease. The returned
StartGuard::root_lease() is borrowed, opaque and cannot outlive its guard. The
physical key is annotation, not a public constructor for lease authority.
workspace::writing retains the ordinary file.write operation in the existing
TaskTracker through actual I/O and directory postconditions after RPC loss.
It releases admission before acknowledging completion.

No terminal, agent, service or descendant cleanup proof is implemented. Registry
absence, an ended flag, PID, cwd, copied fingerprint or ProcessLifetime revocation
must never substitute for such proof. W17 owns terminal root binding/debt; W11's
logical identity remains canonical. Lifecycle consumers take their durable claim
before physical admission and never wait for a logical claim while holding this
lease. Other writers, duplicate, script/start/cleanup integration and any restore
controller remain separate consumer work; this leaf is no full-root safety claim.

## Validation owed

Run clippy for vessel-proto/workspace/node including test targets with warnings denied, then
checkpoint-filtered tests on vessel-node and vessel-workspace. The leaf tests
cover queue saturation/expiry, aliases/drop/stop, actual ordinary file denial,
retained I/O after RPC loss, metadata admission capacity, managed ancestor FD
retention through unlink/replacement, ordinary project subdirectories/parent
aliases versus the actual Project writer, project FD replacement, nested
registration in either order, relative refusal under saturated metadata permits,
and physical alias/replacement observations.

New async fixtures retain setup and actual owner handles outside caught/timed
assertions. Named shorter setup, admission, worker-entry, caller-close,
owner-close and drain phases sit inside the unchanged case budget. Cleanup always
releases the same held channels, requests normal stop, joins callers/owner,
drains TaskTracker and closes Store before propagating the original failure.
A cleanup phase deadline is diagnostic failure: the same future is then awaited
to actual completion, never aborted or counted as success. Therefore blocked
kernel I/O may extend cleanup past a test deadline; it cannot be made cancellable
by this harness.
Run boundary checks on the build machine only. Local static format, size,
test-module ordering and token checks do not establish compilation or runtime.
All filesystem fixtures use private temporary directories. No live node,
permission change, real provider, UI capture or production flow is exercised.

## Resume point

Complete revised exact-leaf ordinary strict checks, then its actual tests. Keep
all returned-source and full-feature records independently. After V3 acceptance
and normal integrator announcement, W17 consumes the exact commit and the full
checkpoint branch merges that same leaf, retaining one canonical implementation.


## Quota-stop continuation (2026-11-16)

The current source also retains partially initialized fixture owners outside the
caught setup future: workspace-start panic after opening Store must still close
that actual writer, drain the tracker and retain its private temp directory until
cleanup ends. This is test-only lifetime work; the project/root admission code is
unchanged. Named setup phases and the product/case limits are not increased.

No current runtime pass or consumer-ready producer exists. The prior twelve-unit
attempt failed before seven fixture bodies, so its green compilation does not
prove the ordinary-project counterexample. The current correction requires its
own all-target strict and actual tests before V3 revision. No new build is started
at the quota warning; resume from the recorded queued-run disposition and exact
commits. Production restoration stays Busy and downstream adoption stays closed.
