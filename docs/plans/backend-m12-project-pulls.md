# Project pull requests without a session

Status: contract proposed, implementation not started. ADR 0061 records
the decision before code. `pr.list` still lists watched sessions.

## Reuse analysis

Decision: modify existing assets. Reuse `vessel-proto::forge` summaries,
repository identity and errors; `vessel-forge` credential/HTTP/provider
boundaries; the registered-root checks and hardened remote lookup. The
session watcher's default-branch/head restrictions do not fit a repository
read, and its session-keyed reply cannot represent this page honestly.
GitHub's cursor pagination and existing serde/std machinery suffice; no
external crate or new dependency edge is proposed.

Reference: Paseo `packages/server/src/services/github-service.ts:104`,
`:1166–1215`, `:2027–2050`, and `github-service.test.ts:3980–4085`.
It defaults to an empty search and a limit of 20, caches for thirty
seconds, shares identical reads and invalidates by working directory.
Vessel's declared differences are a registered project selector, fixed
bounded open-only pages, explicit pagination and typed unavailable states.
ADR 0061 gives the reasons and the later service's cache/cancellation rules.

## Resume point

- Completed before this slice: the separate seeded forge fixture and its
  root-gate/CI coverage. This branch starts at `fba50eb0`, whose parent is
  the integrated fixture commit `4660f784`.
- Current: ADR 0061 and this plan, contract only. Next add bounded cursor
  and open-page value types with decoding tests; no registered call while
  the shared unassigned-protocol accessor is unavailable.
- Then: provider and node service, exact client/CLI call, generated types,
  fake fleet checks and older-peer admission, after the shared accessor is
  merged. The existing protocol value is not changed by this track.
- Waiting consumers need project-wide results without a live session.
  They must continue labeling the existing `pr.list` as session-derived;
  neither this ADR nor the value stage is a callable replacement.
- No native/page screen, real forge, production process or permissions are
  changed. Contract validation uses only the fast document checks.
