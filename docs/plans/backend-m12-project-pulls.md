# Project pull requests without a session

Status: contract accepted; bounded value stage validated. ADR 0061 records
the decision before code. Project listing is not callable; `pr.list` still
lists watched sessions.

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
- Contract commit: `5f36714d`; fast document checks passed. Source `1bdc66e7`
  adds `PullCursor`, `ProjectPullCursor` and `OpenPullPage` only, with
  validating constructors/decoders and safe cursor diagnostics. No method,
  existing reply, client, CLI, store or dependency change is present.
- The page decoder retains only an admitted prefix, then refuses an extra
  item without decoding another summary. It does not first allocate the
  whole input array. Empty, complete and continued pages remain distinct.
- Seven focused value tests passed, including actual decoding refusal of
  the unregistered method. The source crate was recompiled. Whole root
  validation passed as recorded below; no callable behavior is claimed. The shared
  unassigned-protocol accessor remains the prerequisite for a method.
  This value-only stage uses the full root Rust/source gate and regenerates
  TypeScript to check that the existing declarations stay byte-identical.
  It changes no web source or callable wire shape; web integration checks
  belong to the following callable stage in ADR 0061.
- Then: provider and node service, exact client/CLI call, generated types,
  fake fleet checks and older-peer admission, after the shared accessor is
  merged. The existing protocol value is not changed by this track.
- Waiting consumers need project-wide results without a live session.
  They must continue labeling the existing `pr.list` as session-derived;
  neither this ADR nor the value stage is a callable replacement.
- No native/page screen, real forge, production process or permissions are
  changed. The value types have TypeScript derives but are unreachable
  from existing calls/replies; declaration regeneration must remain
  unchanged until a callable transport stage deliberately includes them.

## Value-stage validation

On Linux, the complete root Rust/source runner passed: format, file budget
(771 files, no new exception), crate boundaries (22 crates and ten owned
external dependencies), both explicit binary prebuilds, workspace Clippy
and fixture-feature all-target Clippy. The whole workspace run reported
113 summaries: 1,980 passed, zero failed and twelve existing ignores.
There was no failed test or retry. Node unit and E2E suites each passed
220 tests; proto passed 190, including the seven added value tests.

The explicit fixture launcher target passed four tests. TypeScript
regeneration passed both filtered tests and produced zero declaration
diff. No new method, field on an existing reply/event, client/CLI behavior,
store schema, crate edge or external dependency is introduced. Because
there is no web-source or callable-wire change, this stage does not claim
web E2E, UI, real-forge, Windows or macOS execution evidence.

All 1,718 selected source and policy inputs stayed byte-identical across
the run and matched immutable source commit `1bdc66e7`. This comparison is
source evidence, not an attestation of the environment, compiler or caches.
The normal user ran the gate (UID/GID 1000); isolated test storage retained
mode 0700 and was emptied after checking for remaining test processes.
Production processes, files and permissions were unchanged.

Next is the provider and node-service stage: actual repository pages from
the shared fake and loopback GitHub provider, root/remote validation,
credential-associated cache and bounded cancellation-safe admission. A
service prepared before transport registration remains test-only until
wiring is available; its review is separate from product integration.
The following wiring stage must add the actual client/CLI/fleet consumers,
authority and older-peer fixtures, then execute the callable path before
claiming this gap is filled.
