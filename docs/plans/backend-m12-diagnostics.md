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
| D2 | Value types and DIAGNOSTICS_PROTOCOL=None | Valid round trips; reject excess agent rows/recent records, malformed units/versions and oversized report; errors never echo supplied text; no Request/method/guard registration | Values gated: root 1975/0/12, TS freshness 2/2, web 417 units and 62 E2E; V1 value-types-only passed at `2cdeea97` / `f1e7e5e0` |
| D3 | Staged owner projections, inventory selection, bounded journal and collection leases | Actual SQLite counts/schema, borrowed runtime/cache/lifecycle facts, metadata-only lookup, exhaustive relay state; privacy, loss/window, bounds and cancellation fixtures; whole gate | Private staged source gated at `fbfd3ad8`: whole root 2004/0/12 includes all 29 new tests; source-reviewed/staged accepted at `5235023a` / `9ef6f31c`; remains private on this branch |
| D4 | After shared accessor/metadata land: compose the collector and wire one node call, client, CLI and bridge admission | Unassigned no-frame/no-dispatch; assigned direct and all nested older-peer refusals; supported call; Read/no-Read/revoked/delegated/agent fixtures; CLI selected remote node text/JSON/refusal | Quality B consumed at `ef4ed750`; product source/CLI wiring checkpoint is unverified and package gate/review remain |
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

Code uses focused package checks while being written, then a whole gate
against an immutable committed tree. The approved remote node-gate runner
now runs that gate, including root and web steps, with one stable track label;
only committed inputs travel. Record its run id, commit, per-step return codes
and load figures and test totals. It prebuilds both
`cargo build -p vessel-testkit --bins` and
`cargo build -p vessel-node --bin vesseld` before the workspace tests. Earlier
local evidence used private mode-0700 SSD runtime temporary data, a unique
data-disk target and front/back uptime/free records and remains separate.
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

D2 value types are implemented and gated without call plumbing. V1 passed
only this value scope at `2cdeea97` / `f1e7e5e0`. The root whole gate passed 1,975 tests,
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

D2's exact values commit has passed V1. D3 owner mechanisms are installed
under `cfg(test)` and gated at `fbfd3ad8`; staged-source review is pending.
Submit the immutable source and remote evidence to V1. Keep all staged code
private and unreachable from a product API. No Request, method, guard,
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


### Committed checkpoints and handoff

- `b6d0e93d`: ADR 0074 and acceptance plan; contract/documents review passed.
- `86fbbb33`: merge of the green node line `a365d295`, without conflicts.
- `2cdeea97`: D2 bounded diagnostic values and canonical declarations; the
  gates above apply to its exact compiled inputs. V1 passed this value scope
  with documents checkpoint `f1e7e5e0`; the ADR remains proposed until its
  runtime implementation is reviewed.

Next is the staged D3 source collectors, failure journal and cancellation
fixtures under their owning modules. Off-tree drafts are not installed or
compiled and supply no implementation evidence. Cross-crate product API and
wire wiring wait for the shared admission accessor and RPC method metadata.
There is no callable diagnostic method for a screen or CLI to consume yet;
the planned call/response and older-peer behaviour remain the D4 contract.


### Staged D3 boundary and retained checks

The node's private diagnostic journal and collection leases are being tested
under `cfg(test)`. They are unreachable from the product and have no public
Node API. Source projections stay under their existing owners; cross-crate
composition and its deliberate visibility changes belong to actual wiring.
No staged mechanism is product evidence before the wiring gate. The actual
collector's cross-crate assembly, five-second response budget, listener bind
publication and default-profile selection are part of that D4 wiring.

Early narrow D3 Clippy runs found an incorrect task-token import, similar
test variable names, a documentation formatting warning and unobserved
tracker-close results; these are corrected. The store preflight also found
an exported error type name, unchecked fixture write results and a redundant
conversion; these are corrected before running its tests. The node mechanisms
passed focused Clippy and ten tests; the store passed focused Clippy and five
actual SQLite tests. The later combined owner checks passed Clippy and 27 tests;
the later whole gate includes all 29 new component cases. These are staged component checks, not product
report or wire evidence. D2's accepted immutable source is not
changed by the working collector code.

For D2, the full diff whitespace check returned status 2 with 64 warnings,
all in canonical generated `web/src/wire/proto.ts`. The authored-path check
excluding that generated file returned zero; official lint and both
TypeScript freshness checks passed. Generated whitespace is retained exactly
as emitted, not edited by hand.


The relay projection is deliberately inside the crate that owns `LinkState`:
its existing non-exhaustive public attribute is unchanged, and the local
four-case match is exhaustive. A closed producer becomes unrecognized rather
than repeating its last registered state. Only a test-gated module line and
its private projection/tests are added; no relay API, dependency or variant
changes. The runtime projection borrows per-actor status/parked metadata and
does not claim one global transaction across actors. Default-profile choice,
listener bind publication and cross-crate assembly still need the real D4
caller; component tests do not replace that wiring evidence.


The combined owner preflight also caught documentation code spans and an
unnecessary clone of a Copy key in a process-projection fixture; these were
corrected. The process projection uses the existing lifecycle receiver and
borrows only identity/state; it does not clone the description's names,
hostname, free-form version or key. Its monotonic-clock fixture does not
establish the actual startup capture point, which remains D4 wiring work.


The owner components passed 27 focused library tests (including the actual
fake-session lifecycle and process clock). The complete inventory selector is
also staged in the node: it borrows configured definitions, includes known
builtins and refuses a list exceeding the named report limit before metadata
lookups or row construction. Its Clippy check passed; its separate narrow test
build was stopped at the build lane's time cap (status 124) while linking,
before either test ran. That is not a test pass. The later whole remote gate ran and passed both boundary cases. No report assembly or public service is implied.


The D3 candidate was committed before remote execution so the runner received
only immutable repository inputs. Its recorded complete gate has now passed;
the earlier unexecuted narrow cases remain distinct from that later proof. No untracked asset, dependency folder or local
artifact is part of that commit. The source stays on the track branch until
the wiring commit activates it and passes the product gate.


### D3 whole-gate evidence and resume checkpoint

Source commit `fbfd3ad88c8eed25b1d63535340301c81dbbe57d` was tested by remote
run `diagnostics-20261115T014448Z-fbfd3ad8`. The runner returned zero; all
applicable steps passed. The 18 recorded step entries include two explicitly
skipped forge-fixture steps because this commit has no `forge-fixture`
feature; they are not fixture-test passes. The actual run took 14 minutes
19 seconds after admission, with eight compiler jobs and eight Rust test
threads. Every step records its return code and front/back load.

- Whole workspace: 112 result summaries, 2,004 passed, zero failed, twelve
  ignored. All 29 new D3 component tests passed; the twelve accepted D2
  diagnostic tests passed again. All five changed source crates compiled.
- Formatter, size and boundary checks, full all-target Clippy, TS-enabled
  Clippy and both TypeScript freshness tests passed.
- Bundled binaries and both prescribed nested-build prebuilds passed before
  the workspace tests. Web lint/typecheck/fonts/build, 417 unit tests and
  62 E2E tests passed (E2E 2.4 minutes). No A9 replay or waiver was needed.
- The selected input manifest has 1,738 files and SHA-256
  `df33983422db12af49cbc2c8cd6025810c15f62fa16a892dedbaef71f5ba519e`;
  every selected blob matches the immutable source commit. This covers
  repository inputs, not ignored environment/configuration or the whole
  build machine. The approved runner transported only the commit.

The relay change is exactly the test-gated declaration at `link.rs:29–31`
and its new private 80-line projection/tests file. No existing relay API,
variant, dependency or non-exhaustive attribute changed. Local preflight
failures and the last linking cutoff remain recorded separately; no failed
or unexecuted run is relabelled green. No local owned process identities
remained before remote submission; remote execution uses the approved runner,
without a direct login or an independent live cleanup attestation.

Next: obtain staged-source review for this immutable D3 checkpoint. It stays
on this branch until D4's real caller removes staging. The actual collector
assembly and its five-second response budget, bind observation publication,
current-default-profile selection, configured/callback relay aggregation,
authority/compatibility checks, one registered call, client and CLI remain
owed. Registration still waits for the shared compatibility accessor and RPC
method metadata on a consumed node head. Native diagnostics is another
owner's work; these components do not make a machine answer a report yet.


### D3 staged-source liveness correction

The immutable `fbfd3ad8` execution remains green; source review returned the
new concurrent fixtures because a stuck channel or leaked tracker token could
hang the gate. It found no additional validated product/privacy/lease defect
in that staged source. This is a source return, not a failed execution or
permission to integrate the staged modules.

The correction reuses `vessel_testkit::within` with a named safety budget and
named phase deadlines for new admission, journal and Store waits, including
process-fixture shutdown. The single-thread blocking pools, barriers, ordering
and behavioral assertions remain. Release senders live inside the timed test
futures, so failure drops them before custom runtime shutdown waits on blocked
receivers. A receiver accepts sender closure as cleanup; the successful path
still requires its release send to succeed. The Store adds only the existing
workspace testkit as a development dependency, with no product dependency edge
or new crate version. No shared-head tests or product mechanisms change.

Store aggregate validation covers malformed JSON, unknown state tags and
count consistency. It does not deserialize every field of a recognized status
payload; for example an exited tag with an invalid code payload is still
counted ended. This is deliberately not a store integrity check.

The first narrow correction Clippy found two large future layouts in Store
fixtures after adding timeout layers. The private deadline helpers now box
the test and phase futures before constructing their async state, following
the existing node e2e step helper; no lint is suppressed. The failed narrow
run remains separate from later evidence.

The boxed correction `5235023a` passed the second narrow all-target Clippy
check for node/store (17 seconds). Its whole changed-tree gate then passed;
the original gate evidence remains separate and is not substituted for this
correction run.
D4 remains blocked on consuming the announced shared compatibility accessor
and method-metadata head; no wire registration or product assembly is added.


### D3 correction gate and resume checkpoint

The exact correction commit is `5235023a696bc0c3c342d71cdb80cf329098f254`,
following `6410b5bf`. Approved remote run
`diagnostics-20261115T022213Z-5235023a` finished with status zero. Of its 18
listed entries, 16 executed successfully and two forge-fixture entries were
explicitly skipped because this commit does not define that feature. Those
two entries are not fixture passes. Admission-to-finish took ten minutes
36 seconds after a 106-second queue wait, with eight compiler jobs and eight
Rust test threads. Overall front/back one-minute load was 8.78/7.78; the
workspace test step was 12.13/15.38.

- The raw Rust summaries independently total 112 sections, 2,004 passed, zero
  failed and twelve ignored. All 29 D3 component cases and twelve D2 value
  cases passed again. Both changed test-owner crates actually recompiled.
- Formatting, size (774 files, 31 existing exceptions), boundaries (22 crates,
  ten external ownership rules), full all-target Clippy and TS-enabled
  Clippy passed. Two TypeScript freshness tests passed with zero generated
  differences. Binaries and both required nested prebuilds passed before tests.
- Web lint, typecheck, bundle, fonts, 417 unit cases and all 62 E2E cases
  passed; E2E took 2.1 minutes. No shared intermittent replay or waiver was used.
- The selected input manifest has 1,739 files and SHA-256
  `13ced8afe3418cfbc5171bd08e035e7adf98b54ba96e40b86e76b8ed4a8e8629`.
  Every selected blob matches the immutable commit, and pre/post-run inputs
  match. The final result notes change only this plan. This is repository
  input evidence, not a build-machine or historical cleanup attestation.

The original green run, original source return and failed first correction
Clippy remain recorded. The correction changes only fixture liveness, its
existing development dependency and the aggregate-validation description;
it neither demonstrates nor repairs a product permit leak. No deadline,
assertion or shared-head test was weakened.

The corrected source `5235023a` with result notes `9ef6f31c` is accepted
as source-reviewed/staged only; the original liveness return is closed.
The original source, green run and failed narrow check remain separate.

Resume point: D3 stays private under `cfg(test)` on this branch. This source
acceptance does not authorize integration into the product head. The actual D4 collector, five-second
response budget, startup clock/bind/default-profile wiring, relay aggregate,
authority/revocation/compatibility admission, registered call, client, CLI and
fleet workflow remain owed. First consume the coordinator-announced green
head containing the shared compatibility accessor and common method metadata.
`DIAGNOSTICS_PROTOCOL` remains `None`; no native file is changed.


The staged-source acceptance covers the owner mechanisms and their corrected
fixture liveness, not a complete diagnostic feature. Product configuration,
actual wire authority/compatibility and a successful machine report must be
reviewed again with D4/D5. D2's value-only acceptance remains independent.
No code, generated declaration, dependency or wire constant changes in this
acceptance note. The prerequisite head has not been consumed; no alternate
protocol mechanism or temporary assigned number is introduced.


### Shared-accessor head merge checkpoint

The branch merges the supplied green node head
`f453ec2eda51191e73ebd388c3313be2544ed71a` after the staged-source acceptance.
This preserves its assigned protocol, schema and migration order, existing
method/older-peer tables, test expectations and canonical generated output.
There were no merge conflicts. Existing owner files gain only the previously
accepted test-gated declarations and the existing testkit development edge;
the staged diagnostic modules are retained unchanged. No call is registered.

The shared `required_protocol()` accessor is now present. The existing
`rpc_methods!` table is also retained unchanged. Before call plumbing and the
failure observer, the common method-metadata/Quality B requirement recorded
in D1 and the staged-source acceptance still needs to be resolved; there is
no replacement metadata list or compatibility engine in this branch.

Resume point: run the merged-tree gate through the approved remote runner
before adding product code. That result is separate from the earlier D3
gates. `DIAGNOSTICS_PROTOCOL` remains `None`; no diagnostic field, request,
client call, CLI command or native screen is added by this merge.


### Merged-head gate and remaining metadata prerequisite

Merge commit `ec1e41211802ae51724e1214197a079850c95084` has parents
`7ba48530` and `f453ec2e`. The narrow all-target Clippy check of the five
staged owners passed first (42 seconds). Approved whole run
`diagnostics-20261115T034929Z-ec1e4121` then passed all 18 executed steps,
including actual forge-feature checks: no step was skipped.

The raw logs independently total 114 Rust result summaries, 2,234 passed,
zero failed and twelve ignored. All 29 staged D3 cases and twelve D2 cases
passed again, with the same diagnostic test names. All 22 workspace crates
compiled. The separate forge fixture ran four cases successfully; those four
are not added to the workspace total. Web lint/typecheck/build/fonts, 442
unit cases and all 65 E2E cases passed (E2E 2.8 minutes). The TS selection
passed four cases: two declaration-freshness cases and two representation
cases; generated differences were zero. Both prescribed prebuilds preceded
the workspace tests. There was no failed case, replay or deadline waiver.

The whole run used eight compiler jobs and eight Rust test threads, after a
708-second queue wait. Execution from admission took thirteen minutes
35 seconds. Front/back one-minute load was 11.72/11.95; workspace tests were
10.72/4.41. The selected 1,899 input files match immutable Git and pre/post
checks, with manifest SHA-256
`c4ef9ac1f483feb4da90400679ad820054d0bc60707c0218e266005c18d43ad9`.
Result notes change only this plan; environment and historical cleanup are
outside that input attestation.

The full merge whitespace check returned status two with 87 warnings, all
in the node head's unchanged canonical generated declaration file. Authored
paths excluding that file passed. The generator output was not hand-edited;
the official freshness and web checks passed. No head case, migration or
method-table entry was changed to obtain the gate result.

Review confirmed that this head satisfies shared compatibility admission and
the existing scope/since facts, but does not supply common envelope-category
metadata for observation exclusion. D4's common-metadata/Quality B boundary
therefore remains unmet. A local Method match, second table or numeric
placeholder for an unassigned call would violate the single-owner contract.
Only the canonical table owner supplies that fact, with its reviewed gate;
this does not require expanding the task to all unrelated quality work.

Resume point: the merged tree is gated. Retain the accepted private D3 code
and wait for the required metadata head before product assembly, observation
or call registration. The diagnostic feature number remains `None`. The
actual report, five-second response budget and other D4/D5 obligations remain
unimplemented; this merge gate is not diagnostic feature completion.


### Merged staged-source review accepted

The merge base/staged-source and its exact gate are accepted at `ec1e4121`
with result notes `39c664c9`. Review verified the twelve component files
against the prior accepted checkpoint, the private owner overlays, all
1,899 selected input blobs and the recorded whole-gate results. This is
merge-base/staged-source acceptance only; it does not authorize product
integration or approve a working diagnostic report. The canonical generated
whitespace and remote-environment attestation limits remain as recorded.

Resume point: keep D3 private under `cfg(test)` on this branch. The shared
accessor is consumed, but canonical envelope-category metadata and the
single-table representation of an unassigned requirement are still owed by
their owner. Do not add a dummy version, local Method classification or
second table. Product collection, observation and call/CLI wiring continue
to wait for that reviewed prerequisite. No source or generated file changes
in this acceptance note; no unchanged whole-gate rerun is needed.


### Preparation for the announced canonical metadata shape

The canonical owner is preparing two required facts in the one RPC table:
`[Scope, pending FEATURE]` uses a `method_protocol!` helper and generates
`Method::protocol() -> Option<ProtocolVersion>`. The shared compatibility
accessor reads that result. Diagnostics will use the existing
`DIAGNOSTICS_PROTOCOL` constant through this pending-row form, with `Read`;
no provisional integer, fallback birth version or second requirement table
is introduced. The integrator remains the only owner of assignment.

The same table generates `Method::observation()`, whose categories are Call
and Envelope. Forward, Relay and Deliver are classified by their table rows.
The journal will consume that classification exhaustively at the generated
guard's common completion boundary. It will not name wrapper methods in a
local match. The guard, envelope and RPC client hand lists are generated by
the canonical owner; diagnostics must consume that result rather than add
the new call separately to each old hand list. Exact exported enum names
and the full row syntax are read from the actual approved head before use.

The preparation reuses the accepted D3 mechanisms rather than replacing them.
The wiring checklist is:

1. Capture the monotonic process clock at node startup entry, before startup
   I/O. Construct collection admission and the bounded journal with the same
   shutdown tracker, retaining leases in actual blocking source jobs.
2. Publish listener binding only after the existing bind returns successfully;
   retain the observation time and omit every address and local endpoint.
3. Use `NodeProfiles::current()` for an immutable profile/agent snapshot and
   its existing default-profile selection for readiness keys. Do not call
   `Launcher::added()`, which expands secrets for execution, or refresh status.
   Validate inventory limits before metadata work; do not clone definitions.
4. Promote each owner projection only with its real collector caller. Retain
   Store source errors and queued-read guard ownership, metadata-only harness
   discovery, borrowed runtime facts and the relay owner's exhaustive match.
   Add no public API simply to make staged code compile.
5. Aggregate configured/callback relay watches inside their owner without
   cloning endpoints. The collector uses weak relay ownership to avoid
   adding a service/gateway retention cycle.
6. Preserve each source's observation time and expose partial unavailable
   states. A five-second response budget cannot cancel arbitrary filesystem
   I/O; outstanding work retains its permits until completion.
7. After the approved metadata head is consumed, connect the one pending
   request, generated dispatch, typed client and selected-node CLI. Generate
   TypeScript through the existing generator. The native page remains outside
   this track.

While assignment is `None`, genuine client and server admission tests must
prove typed refusal before frames/dispatch, including nested envelopes.
A successful diagnostic wire report cannot be claimed before assignment.
Actual collector and guarded-service tests exercise product source directly;
the assigned combined head must provide the corresponding successful wire
proof. There is no test-only feature number or alternate callable handler.

Preparation also identified the CLI output boundary: ordinary node rendering
includes hostname fields, and the current CLI prints an error's entire cause
chain. Neither is a diagnostic renderer. D4 must render only the immutable
allowlist, enforce the named text cap before output, and retain typed internal
causes without printing private connection/remote-message data. Existing
commands' behavior is outside this change.

Resume point: this is a design preparation only. The announced metadata
implementation is not yet an approved, consumed head. No request, method
row, guard/client list, public projection API or CLI command is registered
by this preparation; `DIAGNOSTICS_PROTOCOL` remains `None`, and all staged
modules remain private under `cfg(test)`. Wait for the Quality B head in the
shared brief, merge under the existing rules and gate that tree before
installing the actual product wiring.


### Native consumer handoff: values available, producer still pending

The native consumer can use the accepted immutable `DiagnosticReport` and
its per-source `DiagnosticSection` values. There is still no registered
request, `NodeClient::diagnostics()` implementation or native link/view
producer. The staged collector is not such an API. The native owner retains
the existing local-app report and translated UI text, and must not replace
the missing node source with another native collector or a successful stub.

The proposed native ticket semantics use the app's existing typed request
mechanism rather than introduce wire fields: associate results with machine
identity/book key, connection generation and refresh generation; reject a
report whose node id differs from the expected connected node. Superseded,
closed, disconnected or forgotten requests cannot replace newer rows or
revive removed state. After a successful book rename, move cached local state
and invalidate the old request ticket; a failed edit changes nothing. Exact
native types and completion hooks remain the native owner's contract. These
semantics are a consumer proposal, not an implemented or reviewed native API.

Machine connection/request failures remain distinct from a received report's
partial source failures. Missing sections never become healthy zeros, an
empty failed-RPC window never means all errors are absent, and retained old
reports remain visibly stale. Copy/export uses only validated values, with
the existing client text bound checked on the complete output. A multi-node
export must not multiply bounded reports into unbounded output. Raw remote
error messages and connection details are outside the shareable report.

Resume point: align the existing native typed-ticket/completion seam when its
owner supplies the concrete type, and hand off the actual client method and
gated hash only after canonical metadata and real wiring exist. This note
changes no producer, field, method, protocol number or native source.


The native owner reports the current seam as
`app/src/areas/settings/diagnostics.rs`: state contains a local text report
and dialog state, populated synchronously by
`view::diagnostics::report(AppFacts, &fleet.machines)`. There is no remote
request ticket or loaded-event seam yet. The existing local report remains
local. At actual API handoff, the native owner plans to adapt the existing
link ticket/admitted-identity mechanism with connection/request generations
and node-id checks, rather than add a second transport. This is a reported
integration boundary and future plan, not verified native runtime behavior.

## E4 correction checkpoint after Quality B

The product narrow run `diagnostics-20261115T065500Z-19fb9fe9` is red,
not a pass: Cargo rc 101, runner rc 1, 12 seconds after 982 seconds queued.
It found three Proc lints (similar pid/parent names, redundant qualification,
unused read byte count) and the required explicit Diagnostics arm in the
older-agent reply projection. The nonexistent Busy error variant was also
confirmed by source review before execution, but compilation stopped earlier.
All red logs are retained separately with the verification artifacts.

The correction reuses Unavailable plus a closed Diagnostics ErrorDetail with
Busy/Stopping and the single MAX_DIAGNOSTIC_COLLECTIONS constant. Existing
workspace policy already permits direct Read agents (its false arm); no
policy arm is moved. The earlier statement that they were refused was wrong.
The generated guard is tested with no-Read calls while all collection leases
are occupied, and direct agent Read is tested against actual source collection.

Blocking metadata, runtime and relay observations share a retained source job.
Store and journal source timestamps are recorded at completion, before the
whole report joins. Source cancellation and response timeout retain queued
real work; shutdown still waits for those leases. No deadline or assertion
in an existing shared test is weakened. New pure client/RPC fixtures use the
existing testkit only as dev dependencies and named safety/phase clocks.

The CLI entry is `vessel [--node NAME] node diagnostics [--json]`. It uses the
existing connection and checks report identity against the admitted node.
Complete text/JSON export has MAX_EXPORT_BYTES beside the renderer. This
command disables debug logging and prints only the safe root Display of its
typed error wrapper; source chains remain available internally, while other
commands keep their established output. Hostile source messages never enter
shareable output. Assigned wire success and complete package validation are
still pending; the feature constant remains None.

Provider diagnostics row 568 belongs to A9/W22. The separate process-lifetime
owner will supply the canonical root Proc ProcessId; after its reviewed
integration hash arrives, self-process facts reuse that observation type.
No unchecked carrier branch is consumed and no process-control authority is
obtained from a diagnostic pid.

The next narrow run `diagnostics-20261115T071731Z-65537b1f` also remains
red: Cargo rc 101 after 25 seconds; the public Store ReadAdmission enum
lacked Debug. It stopped before a complete product check. The follow-up adds
Debug, passes the actual bound TCP result instead of a boolean, and separates
CLI section renderers into short functions with numeric schema/uptime and
normalized version text. The guard fixture owns its transport in a separate
helper; checks/assertions and all named deadlines remain unchanged. These
changes still require compilation and focused tests, and generated types
remain stale. No red run is described as a pass.

Narrow `diagnostics-20261115T073758Z-b7c806e8` is also retained red:
Cargo rc 101 after 16 seconds, with a missing readiness Key import and three
new-test warnings (ignored hello frame and two redundant qualifications).
The correction fixes these together. A newly authored supported-read assertion
also incorrectly expected the shared Fake's separately random welcome and
node.info identities to match. Only that new assertion now checks the typed
NodeInfo/Serving reply plus its request id; forced wrapper targets still use
the welcome id, and refusal/no-dispatch and observed supported-call checks
remain. The shared fake and all older tests are unchanged.

The shell marker setup is Unix-only. The additional two-node fixture creates
one fake ACP session through the existing authenticated session-create call,
then directly collects each node's own store/runtime projection. This is an
in-process source/identity test while diagnostics remains unassigned, not an
assigned-wire, physical-machine or other-OS claim. Its transport helper is
shared only inside cfg(test).

The exact owner correction c5da5305 is merged without conflicts before the
package gate. All five incoming web test/docs blobs are preserved. No local
edit to the shared security test or deadline/assertion relaxation is made.

Narrow `diagnostics-20261115T081036Z-b563198e` remains red (Cargo 101,
19 seconds). Its full diagnostics were read: one redundant test qualification,
getter placement after the Proc test module, private actor types exposed by
journal error fields, unchecked Instant addition, needless error ownership,
implicit Weak cloning, and two function-length warnings. The correction keeps
journal actor types private behind a source-preserving opaque error, checks
the deadline addition, borrows refusal causes, explicitly clones Weak, moves
getters before tests and extracts fixture/startup setup without changing their
order or assertions. Node assembly is now 695 product lines against 709 before
the startup extraction. No existing deadline or validation assertion is reduced.

Shared package base 3, exact 6c782cb6, is consumed as instructed. Its sole
conflict was docs/PROTOCOL.md: the incoming project-port paragraph stays intact
first, followed by the pending diagnostics paragraph. Native sources are only
incoming base material; this node track authors no native screen changes.

Narrow `diagnostics-20261115T082635Z-bef3f5a9` reached the CLI and remained
red (Cargo 101, 58 seconds): redundant args qualification and a redundant
process-id getter closure. No other owned-library diagnostic is reported in
that run. Both CLI warnings are corrected next; this is not yet an aggregate
strict-Clippy pass or any execution/whole-package evidence.

## E4 package checkpoint

The complete node package is gated at `95e4505f`, on the designated node head
with protocol 23, minimum 2 and schema 18 unchanged. The full root run returned
zero for all 18 steps: Rust 2,422/0/12, TypeScript four/zero with no stale diff,
web 464 units and 68 E2E passes, forge fixture four/zero. The final package plan
records scope and source evidence. The result record adds only documentation;
no unchanged-tree gate is rerun.

Implementation review, protocol assignment and successful assigned-wire
integration remain distinct from this gate. The native bounded node.info
consumer has separate evidence and does not adopt the new full producer yet.
No full Paseo diagnostic parity, provider/raw-log, other-OS or live deployment
claim is made.
