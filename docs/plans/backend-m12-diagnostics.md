# Milestone 12: diagnostics from each machine

ADR 0074 is proposed. Base: the supplied wholly green node commit `7f87d41c`.
The inbox checkpoint is complete and independent; it is not merged into this
branch. A wire number is assigned only by the integrator. No native files are
owned by this track.

## Reference and pre-implementation decision

Paseo source observations and exact line quotes are in ADR 0074. In particular,
protocol `messages.ts:1484–1487,5263–5270` defines correlated request/text reply;
`3794–3795` gates daemon support. Server `daemon-session.ts:293–327` collects
and returns it; `diagnostics.ts:58–107,127–155,216–235,426–457,543–567` describes
sections, reasons, bounded log tail and redaction. The app's
`diagnostics/app-diagnostic-report.ts:15–16,108–142` formats and redacts;
`components/app-diagnostic-sheet.tsx:76–151,264–309` collects per host, handles
superseded runs, offline/unsupported/failure states and copy/refresh.
These are source-only observations; no real Paseo daemon was exercised.

[Decision Log]
- Existing assets: lifecycle/NodeInfo, Store's guarded reader, Sessions,
  Lookup/Readiness, and relay watches already own the facts.
- Reuse: project their safe fields; add only bounded diagnostic value types,
  a small typed failed-call journal and the collector. Do not reuse objects
  that contain paths, keys, free-form output or sign-in help as wire DTOs.
- External alternatives: a shell doctor or OS inspection crate adds execution,
  privacy and platform concerns. Existing std/serde/chrono/workspace types
  suffice; no new external dependency is proposed.
- Selected approach: modify/reuse the existing owners with a new allowlisted
  projection. Typed Read-scoped fields replace Paseo's server text and raw
  log tail. Clients render the same section/indented-label presentation.

## What a person must be able to do

1. Open Diagnostics and see local app facts plus a separately identified result
   for every selected machine. The node reply has its stable identity; the
   client owns its local machine label and per-machine progress.
2. A connected permitted node supplies version/platform/uptime, store/schema,
   agents with exact availability reasons, session counts, listener kinds and
   relay state counts. A copied report shows `~`, never a home/address/key.
3. Missing/stale facts have a reason. A partially failed report retains its
   available sections without suggesting an empty node or healthy store.
4. Recent failed node calls show bounded code/time entries with
   RPC-only coverage, observation start and loss indicator. They contain no
   raw message, log, request body, caller id or resource id.
5. Refresh recollects without launching an agent, checking a login, running
   a command, repairing storage or probing a network destination.
6. A Read-only paired device gets the same safe report. A device without Read
   is refused before collection; clipping, revocation and delegation retain
   the existing node admission boundary. No new grant is inferred.
7. An offline node remains offline; an older/unassigned node is unsupported.
   Neither becomes an empty successful report or an RPC sent speculatively.
8. Closing or refreshing the page cannot let a late result replace a newer
   machine/report. Copy reflects only the chosen completed run. This client
   behaviour belongs to the app owner, not this node implementation.
9. The selected-node CLI prints the safe report, with opt-in exact JSON and
   nonzero explicit refusal/unavailable errors. It neither mutates the node
   nor substitutes local-machine facts for a remote node.

## Slices and executable acceptance rows

| Row | Scope / deliverable | Required evidence | State |
|---|---|---|---|
| D1 | ADR 0074 and this node plan | Reference quotes, bounds/privacy/authority, unassigned seam, docs checks and immutable review commit | Contract/docs passed at `b6d0e93d`; implementation still proposed |
| D2 | Value types and DIAGNOSTICS_PROTOCOL=None | Valid round trips; reject excess agent rows/recent records, malformed units/versions and oversized report; errors never echo supplied text; no Request/method/guard registration | Values gated: root 1975/0/12, TS freshness 2/2, web 417 units and 62 E2E; V1 implementation review pending |
| D3 | Store snapshot, node collector and bounded typed journal | Actual SQLite aggregate/schema reads, runtime counts/relay/listener fixtures; unknown/read failure/too-many/timeout states; safe cached readiness; zero subprocesses; restart/loss/window fixtures; 2-permit cancellation lifetime | Not implemented |
| D4 | After shared accessor lands: one node call, client, CLI and bridge admission | Unassigned no-frame/no-dispatch; assigned direct and all nested older-peer refusals; supported call; Read/no-Read/revoked/delegated/agent fixtures; CLI selected remote node text/JSON/refusal | Blocked on consuming the shared accessor and common RPC method metadata |
| D5 | Testkit/fake fleet and consumer handoff | Two nodes/devices, one offline/older/refused node, exact safe fields, no addresses/env/keys even in failure; generated TypeScript freshness; whole gate; current docs | Not implemented |
| App handoff | Existing native Diagnostics consumes the gated call | App-owned per-machine load/refresh/copy/stale-result fixtures and screen gates; no node-track native edits | Not implemented by this track |

D2/D3 may form one coherent node-only slice before registration, with value
types written first. The collector is staged under `cfg(test)` and remains unreachable over the
wire until D4. Tests drive its actual code. The wiring commit removes staging
after the shared admission accessor and common RPC method metadata exist;
its whole gate supplies the product-build evidence. No public service API
or dead-code suppression is added to make an absent caller compile. D4 does not start merely because a compatible-looking version
exists: the shared accessor and common RPC method metadata must actually be
on the node head. The integrator
assigns Some(version); this track never substitutes the current version or a
sentinel for None. A source dependency is not an invitation to add another gate.

### Named adversarial fixtures

- Put fake secret values, URLs, pairing links, custom agent names, labels, program paths,
  free-form versions and raw error text in the underlying sources. Serialize
  and render the report; assert every canary is absent, including Debug and
  malformed-value failures. Home is the fixed redacted value.
- Agent metadata lookup points at testkit fakes that record invocation. Fetch
  and refresh diagnostics; invocation count stays zero. Cached known/missing/
  too-old/too-new/unread facts stay distinct; unparseable version words are
  unknown, not copied or guessed.
- Two actual stores have different schema/session counts. The selected node's
  reply reflects its own store/runtime. Query/store failure is unavailable,
  not zero. No transcript, project content or secret store is opened.
- Hold two blocking readers, cancel their callers and attempt a third report:
  it remains Busy until the original blocking work releases its guards. A
  deterministic pool barrier replaces sleeps/retries or longer deadlines.
- Record typed RPC failures with hostile raw messages and method parameters. Retain only 32 from
  15 minutes, disclose loss/eviction, exclude envelope duplicate observations,
  and clear on a new node instance. Observation overload never changes a
  method's original result; transport-only errors are not claimed observed.
- Verify body/text bounds before writing a frame/stdout/copy. A 65th agent or
  excess reply refuses a section/report explicitly rather than truncating
  choices or presenting a complete inventory.
- Once registered, use exact old/unassigned client no-frame tests and forced
  raw welcomed-id server tests on direct/Forward/Relay/Deliver. Read denial
  precedes collection; supported current peers get the same bounded report.

## Wire/client proposal (not registered)

`node.diagnostics` has no request fields and returns `DiagnosticReport`.
It is Read scoped and initially not fleet-forwardable; encrypted relay
transport is supported. `NodeClient::diagnostics()` and
`vessel [--node NAME] node diagnostics [--json]` are the planned entry points.
The app renders typed sections into its existing report. Its connection,
permission and unsupported statuses remain separate from returned node facts.

No added fields ride an existing reply/event while the feature is unassigned.
No protocol maximum/minimum, store schema, RPC table, client or CLI surface is
changed by D1/D2. D3 adds no persisted data; its error journal is in memory.

## Gates and report boundaries

D1 changes documents only: rustfmt check and whitespace/boundary inspection;
no compilation or tests are claimed. Root size/boundary scripts are absent on
the supplied base; native token checks are inapplicable without native changes.

Code uses focused package checks while being written, then one detached whole
root gate per changed slice. In the same environment and slot, prebuild
`cargo build -p vessel-testkit --bins` and
`cargo build -p vessel-node --bin vesseld`; then run the root fmt, full clippy
and workspace tests with `--no-fail-fast`. Use private mode-0700 SSD runtime
temporary data, a unique data-disk target and front/back uptime/free records.
The merged head fixes the initial protocol-expectation failure. Its known
side-question completion-race cases are recorded with their original results
and one exact rerun where authorized; no other failure is waived. Timeout endpoint load above 32 is
saturation; all other timeouts go to their owner. Only a saturated test/crate
may run once more when load is below 24; deadlines/assertions stay unchanged.

New value/wire types regenerate TypeScript and run its checks. Native screens,
real-agent turns, live preview nodes, production accounts, actual service
permissions and other operating systems are not verified by local fake tests.

## Current status and next action

D1 contract/docs passed at `b6d0e93d`; ADR remains proposed until the
implementation review. The green node head `a365d295` was merged at
`86fbbb33` without conflicts. Its protocol/schema/tables/tests/documents
were retained unchanged; no protocol number is assigned to diagnostics.

D2 value types are implemented and gated without call plumbing; V1's
implementation review is pending. The root whole gate passed 1,975 tests,
with zero failures and twelve ignored, across 112 result summaries. Both
TypeScript freshness tests passed without writing declarations. The web
lint, typecheck, 417 unit tests, build and all 62 E2E tests passed. These are
value/compatibility regression checks, not a live diagnostic report or a
registered diagnostic call.

The earlier focused proto suite passed 200 tests, including twelve
specific diagnostic cases, sorted alias classification and declaration
freshness. This covers the saturation-certainty regression and the distinct
agent protocol domain. An intervening two-crate narrow check was cut off
during compilation; it remains a non-result, separate from the later full
two-crate freshness pass.

Early preflight failures are retained outside the repository: a duration
constructor in a test, documentation/test formatting lints, the generator's
reserved `unknown` spelling, and an incorrectly positioned alias entry.
They are corrected. A source-only whitespace check excludes canonical wire
output, whose formatting is owned by the generator; generated files are
never edited by hand. No whole root gate had begun during those preflights.

The cached sign-in field covers only each agent's default profile, matching
`agent.status` without a selected profile. It contains no sign-in help;
non-default profiles are outside this report's coverage. Counts preserve
exact JSON integers and explicitly mark saturation. Numeric version parsing
uses checked decimal accumulation against its named ceiling rather than
creating and discarding a parser error.

Resume: submit the gated immutable D2 commit to V1, then continue D3.
D3 drafts are not installed or compiled. Stage its real collector and actor
under `cfg(test)` with tests, without a public node API. No Request, method, guard,
client, CLI or bridge registration is added before consuming the shared
admission accessor and common RPC method metadata. No runtime diagnostic
result or native integration is claimed.


### D2 implementation boundary

`vessel-proto::diagnostics` owns immutable `DiagnosticReport` plus bounded
agent/failure lists, normalized versions, exact/saturated counts and closed
section states. Only the root report is the public untrusted decoding edge:
malformed serde errors are replaced there because their messages and Debug
chains can contain supplied field/variant strings. Serialization writes into
a nonallocating byte-budget sink before the root becomes a valid report.
The TypeScript generator starts from this value root explicitly, since it is
not reachable from any Call/Reply. No request, method, scope, version number,
client, CLI, bridge, store schema or native file changes in D2.


### D2 gate provenance (2026-11-14 to 2026-11-15 UTC)

Both prescribed binary prebuilds preceded the root formatter, size and
boundary scripts, full all-target Clippy and workspace tests; every step
returned zero. The test run used three Rust test threads, private mode-0700
SSD temporary data and front/back uptime and memory records. Twenty-two
workspace crates compiled, including the changed protocol crate. The frozen
input manifest has 1,727 files and SHA-256
`0ab5e96d3c2e8ab435cc407c205b68a4caaa0a1c2aef3f8dee43bcf965024026`.
The same bytes were checked before the remaining web pipeline and after it.
Only this plan's result text was updated after testing.

The first combined runner ended with status 2 after its successful root and
freshness stages: web lint could not resolve a dependency because the local
copied dependency directory lacked the conventional `node_modules` basename.
Its package files existed. Correcting only that local directory layout and
symlink made resolution succeed; no package, lock file or source bytes
changed, and no packages were reinstalled. The original failed log is retained.
The separate complete web pipeline then returned zero, including a fresh
bundled-binary build before the 62-test E2E run (3.4 minutes). The unchanged
root suite was not rerun.

All recorded owned process identities and the fleet's five process IDs were
absent afterward. The fixture left its temporary root, which was manually
removed along with other owned temporary entries; the private SSD directory
was verified empty. No live service or service-account permissions were
changed or verified. No native screen, real agent, real node diagnostic
response, phone, other OS or production behaviour is covered by this slice.
