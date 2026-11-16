# Canonical physical-root borrow producer

Status: producer-only implementation awaiting acceptance. The first exact source
leaf da61a1461 on node ed4cd137 passed its own ordinary three-library Clippy
(run wp-checkpoints-node-20261116T003228Z-da61a146, cargo0/88s) and eight selected
library tests (run wp-checkpoints-node-20261116T012226Z-da61a146, 8/0/0). That test
compile reported two unused-qualification warnings in a test timeout constant;
they are removed using the already imported Duration. The next natural base merge
consumes announced bc9cacc6. This union still needs its own strict test-code check,
tests and remote boundaries before V3; prior-base results are scoped evidence.
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
  nested admission, including unlink/replacement.
- Costs: metadata I/O is bounded by retained worker permits but is not forcibly
  cancellable. Only one ordinary file-write consumer is wired in this leaf.

## Modules and data flow

vessel-proto::checkpoint holds only PhysicalRoot, preserving its canonical
BlobHash-backed representation. There are no checkpoint calls, events, store
schemas or protocol changes. vessel-workspace::physical owns safe opened-FD
identity and bounded parent traversal, with no admission authority.

workspace::metadata retains two blocking metadata permits through actual I/O.
workspace::admission chooses and pins the nearest managed ancestor; it keeps
only the selected ancestor FD rather than one descriptor per registered tree.
workspace::exclusion owns the bounded queue and leases, independent of any
session, logical workspace or terminal registry. Stop refuses new claims and
waits for existing lease owners; deadline expiry does not force-unlock I/O.

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
retention through unlink/replacement, and physical alias/replacement observations.
Run boundary checks on the build machine only. Local static format, size,
test-module ordering and token checks do not establish compilation or runtime.
All filesystem fixtures use private temporary directories. No live node,
permission change, real provider, UI capture or production flow is exercised.

## Resume point

Complete the exact leaf's ordinary strict run, then tests. Keep failed and queued
full-feature records independently. Review/adoption waits for this exact leaf's
green evidence. After V3 acceptance, W17 consumes the exact commit and the full
checkpoint branch merges that same leaf, retaining one canonical implementation.
