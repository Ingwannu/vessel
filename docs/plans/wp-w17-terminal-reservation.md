# W17 terminal reservation prerequisite

Status: exact product/test source bf4dcd2a17aea87f9ef2fc9ae8c536139864e6e6 is
narrow-green on announced gate58b base d5bfde87. This plan's final evidence edit
changes documentation only. V3 acceptance, integrator landing and receiving-head
checks remain owed. No full-package gate or consumer readiness is claimed.

## Scope and ownership

Only the terminal registry, its registration/tests submodules, the two existing
terminal.create/script.run callers, ADR0031 and this plan change. No RPC, wire,
protocol/schema number, root binding, cleanup-debt ledger or quiescence proof.
Manifests, lock, proto and session are byte-identical to the d5 base.

Before starting, node-line history from ed4 through bc9 contained no registry.rs
change; C2 explicitly confirmed no committed or work-in-progress edits. The same
log check through d5 also found none. W17 is the single writer. W3 waits for the
accepted, landed hash and must not consume this source as an announced contract.

## Why and boundaries

The registry already owns ID/limit admission. Its registration submodule owns
synchronous reservation-to-handle handoff and retained refusal cleanup. Reuse
that mutex and the existing TaskTracker instead of adding a second registry,
numeric wrapping reservation IDs, or a detached cleanup owner.

A non-cloneable Reservation carries expected TerminalId, the original registry
Arc incarnation, a private non-reusable allocation identity and its Start/Owner.
Duplicate existing or starting IDs refuse under the same mutex before spawn.
Registration validates that binding and inserts only into a vacant entry; no
incumbent handle is replaced. Successful registration remains synchronous before
the next await, preserving existing watcher/start ordering.

A RegistrationRejected observes cleanup already spawned in the original tracker.
A fresh rejected actor is killed and retained with its reservation/plan. A known
incumbent is retained without killing it. Dropping the caller's observer does not
cancel tracked cleanup. into_wire observes the canonical CLOSE_WAIT budget;
expiry reports Unavailable and leaves the cleanup task owned.

Live uncertain starts remain retained. During node shutdown, a lost end-watch
cannot block the tracker forever: the same close budget yields the explicit
ShutdownUnconfirmed observation and warning. It never means cleanup success.
Base end/status, ProcessLifetime and tracker drain are not descendant, I/O,
physical-root or quiescence proof. GKS lifecycle owns future agent-run and
forgotten-agent evidence; W17's later root composition receives those inputs
alongside its terminal/debt evidence under W9's separately reviewed RootLease.
This prerequisite unlocks no production restore or StartupHooks path.

## Executed checks

Exact bf4d product/test source:

- wp-cli-narrow-20261116T032040Z-bf4dcd2a: locked vessel-node alltargets Clippy
  with -D warnings, Cargo0,108s. Actual Checking vessel-node and Finished retained.
- wp-cli-narrow-20261116T032330Z-bf4dcd2a: locked vessel-node lib plus e2e target,
  terminals:: filter. All four web-ci/fakes/vesseld/cargo steps0; Cargo194s;
  15library and6wire cases passed,0failed,0ignored. Library runtime13.30s and
  wire runtime3.30s; the rest is compilation/preparation, not test runtime.

All nine new cases were checked individually in the actual log: starting and
existing duplicates, concurrent admission, wrong ID, foreign registry, dropped
refusal observer, incumbent preservation, bounded missing-end shutdown and
already-ended/shutdown readiness. Existing wire cases cover real shell typing,
missing shell, approved scripts/service ownership, agent access, port discovery
and terminal cleanup on session deletion.

The fake actors have isolated environment/storage. Canonical within and named
SAFETY_NET/OBSERVATION_WAIT/TRACKER_DRAIN_WAIT/PHASE_WAIT constants bound each
fixture and phase. Fixture drop signals the real runner's stop token; successful
tracker drain is awaited and dropping a JoinHandle is never called settlement.

Direct format/size/diff and source-only preflight passed (cargo_run=false).
These static checks are distinct from the executed evidence above. No real
provider, service account, other OS or full root gate was exercised by this slice.

## Retained correction history

Original88ed priority strict020812 returned Cargo101 on drop_non_drop for an
explicit OccupiedEntry drop; no runtime cases ran. Corrected9e3 ended the lexical
entry scope before retaining its payload; ordinary strict021539 passed67s. The
later test-budget extraction and clean d5 merge were rechecked at exactbf4d by
both current runs above. The failed priority was not repeated or erased.
