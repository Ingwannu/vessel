# E4: node diagnostics package

## Assigned-wire validation follow-up

The accepted node checkpoint is now integrated on the announced green head
`9c150c6e`. The integrator assigned diagnostics protocol 24 in `729144ff`;
minimum protocol stays 2. Incoming `97a020ef` boxes guarded calls on default
worker stacks and `58defe1d` admits this command's own JSON switch. Those
changes and the four transitioned protocol tests are preserved unchanged.
Earlier unassigned-gate evidence below remains historical evidence, not a
successful assigned-wire result.

[Decision Log]
- Purpose: prove the assigned transport reaches the selected real collector,
  and distinguish transport behavior from the prior direct-guard proof.
- Existing assets: reuse testkit fakes, the actual pairing/relay/fleet helpers,
  the actual generated guard and the RPC memory transport. No new dependency,
  product hook, principal, version override or second collector is needed.
- Alternatives: more direct calls would not prove authenticated wire admission;
  a broad fake handler would duplicate the shared method table.
- Choice: modify the existing diagnostic test areas with real owner/paired
  Read/no-Read and agent calls, two-node identity/count/revocation, fleet refusal,
  selected named-node CLI exports and a previous-protocol CLI responder.
  A dedicated typed responder tests client identity, reply correlation,
  disconnect, caller abort and the actual named client timeout.
- Lifetime: actual RPC partial deadlines and a disconnected caller leave queued
  source leases held until the real blocking pool is released and shutdown
  joins it. The earlier direct-future abort fixture stays separate; the wire
  test does not pretend to identify a running OS read's cancellation point.
- Tradeoff: synthetic responder fixtures isolate client behavior but do not
  prove pairing or collection. Real temporary nodes and paired devices prove
  those boundaries, without claiming physical machines or native adoption.

New follow-up cases are authored and await strict compilation, focused tests
and the one changed-tree package gate. No new execution result is claimed yet.

## Scope and existing assets

This is the node half of the diagnostics package. The native owner owns the
screen, local-app report, translated text and link/view state. Onboarding is
owned by the desktop lifecycle package; this package does not duplicate it.
The provider package owns provider discovery and its diagnostic collector.

References are the supplied survey's `area-5-hosts-devices-desktop.md`
Diagnostics sections (app sheet, daemon diagnostics/status, provider
reports, process/watchdog and desktop logs), plus its cross-area connections.
The original source spans and privacy decisions remain in ADR 0074. This
package uses those decisions; a broader survey item is not silently claimed
by an existing narrower DTO.

Existing assets and reuse decision:

- `vessel-proto::diagnostics`: validated immutable report, bounded collections,
  normalized versions and closed source states. Reuse; no second JSON model.
- Private staged owner projections and bounded admission/journal: reuse the
  accepted D3 source. They are not a public collector or a working RPC.
- Node lifecycle, Store reader, profiles/readiness, runtime and relay watches:
  compose existing owners; do not enumerate private descriptions into text.
- Provider diagnostics belongs to A9, which owns its collector/cache and RPC.
  This host report does not consume its unreviewed getter. `agent.status` is
  not a pure cached alternative: even `refresh=false` probes builtin versions.
- Ordinary node text and CLI cause-chain output include private descriptions;
  they are not diagnostic renderers. Add a bounded allowlist renderer and a
  deliberate output boundary when its actual caller is wired.
- External software: no new dependency is introduced by this plan. Any safe
  process-metrics implementation still needs an owned typed source and the
  repository's dependency decision before adoption.

## Implementation and acceptance rows

| Node responsibility | Implemented behavior and selected evidence | Remaining boundary |
|---|---|---|
| Safe daemon report | Real bounded collector, canonical pending request, typed client and CLI; real-source and authority fixtures passed | Final review and protocol assignment |
| Identity, platform and uptime | Startup clock captured before I/O; lifecycle/private-text canaries and actual node identities tested | Successful assigned wire request after integration |
| Store/schema and session counts | Actual guarded SQLite aggregate reader and runtime projection; source/timeout/cancellation cases tested | Not a store-integrity scan or atomic whole-machine snapshot |
| Agents and cached readiness | Immutable current-profile selection, safe owner metadata, no refresh; cache projection and no-execution traps tested | Provider diagnostics is a separate A9 package |
| Listener and relay state | Actual successful bind publication, weak relay ownership and exhaustive owner-side watch projection | No external relay-service reachability claim |
| Self-process metrics | Canonical ProcessId, bounded fixed Linux self-status source, typed memory and decoder fixtures | Other operating systems explicitly return Unsupported; no other-OS runtime proof |
| Broader system/tool metrics | CPU/heap allocation, disk capacity, workspace/socket counts, latency and tool probes are absent | Explicit difference from Paseo; no full diagnostic parity claim |
| Recent failure summary | Bounded code/time journal; actual guard regression observes scope and collector refusals newest first without loss | Not a raw daemon log viewer |
| Shareable text/JSON | Root and complete export budgets; safe typed errors and CLI debug-output privacy fixtures | Assigned-protocol successful CLI report after integration |
| Bundled web UI | Existing node-hosted web implementation and package regression gate pass | No new Diagnostics screen here |

The collector and `NodeClient::diagnostics()` are product code. The original
checkpoint was pending and its wire refused before send/dispatch. Protocol 24
is now assigned on the consuming head; new supported-wire cases above require
their own changed-tree execution. Original direct-guard evidence stays scoped.

## Contract and canonical wiring

ADR 0074's proposed call is `node.diagnostics {}` returning
`DiagnosticReport`, requiring Read and initially not fleet-forwardable.
Clients use each selected node's existing authenticated connection; encrypted
relay transport is distinct from a fleet-forwarded call.

The feature constant stays `DIAGNOSTICS_PROTOCOL: Option<ProtocolVersion>`.
Only the integrator assigns its number. After the Quality B head is announced
and consumed, the sole canonical method row uses the pending feature form in
`rpc_methods!` in `rpc/table.rs`. Generated guard/envelope/client registrations are consumed,
not copied into new hand lists. `Method::since()` supplies the nullable requirement; `Call::required_protocol()`
includes the inner call and field requirements. `Method::shape()` classifies
Unary, Stream and Envelope. The journal excludes Envelope through that
metadata, never a local wrapper list.

An unassigned feature refuses locally before a frame and at server admission
before collection. Older assigned peers refuse before decoding unsupported
work. Envelopes propagate the complete inner requirement. No dummy version,
second metadata table, test-only assigned number or alternate handler exists.
Successful wire collection requires the assigned combined head; earlier tests
must state whether they exercised actual service source or wire admission.

## Data flow and resource ownership

1. Use the shared admission and current effective principal before collection.
2. Retain the accepted nonqueued collection leases in queued/running source
   jobs and register work on the same shutdown tracker before the stop check.
3. Capture the monotonic start clock at node startup entry. Publish listener
   state only after actual binding succeeds, without retaining an endpoint in
   the report. Use weak relay ownership to avoid another gateway/service cycle.
4. Take `NodeProfiles::current()` once per collection. Reuse default-profile
   selection only for typed cache keys; never call the execution launcher,
   expand its secrets, refresh credentials or run an agent/version/status probe.
5. Read Store facts through the existing guarded blocking reader. Filesystem
   metadata work stays on blocking workers. Keep each source's observed time.
6. A response deadline can return explicit unavailable sections; it cannot
   forcibly stop arbitrary filesystem work or release its lease prematurely.
7. Validate the root and output budget before returning/rendering. A missing
   section is not zero or healthy. Runtime and durable counts are distinct.

Provider diagnostics belongs to A9. Its owner has committed an unreviewed
cached getter proposal; no provider getter or provider RPC is consumed here.
A future shareable composition must normalize free version text and private
identifiers at this report boundary and preserve cache-miss-as-NotChecked.
It must not refresh a provider during a Read-only host diagnostic request.

## Deliberate differences and remaining gaps

- Typed source allowlists replace Paseo's server-created text and regex-only
  redaction. Keys, addresses, credentials, pairing links, environment values,
  private paths, arbitrary messages and file contents never enter this report.
- Home is the fixed privacy marker. Custom agents use report-local numeric
  slots. Free version suffix text is not exported.
- Recent failures mean completed local typed RPC failures with explicit
  coverage/loss. They do not replace or claim an actual daemon log viewer.
- Paseo's process/system/tool/log sections are broader than current D3. The
  package must record safe implementations or accepted differences before it
  is closed; the old value/component passes do not establish full parity.
- Desktop daemon/app logs and native watchdog data belong to their desktop
  owners, not a new remote filesystem-read route in this package.
- Existing local-app diagnostic text remains local. Test fixtures may model
  absent/partial data; they are not a production remote-success stub.

## Required tests

Use real testkit nodes/fakes and common safety clocks with named phase bounds.
Test Read/no-Read/current revocation, direct agent/delegated policy, per-host
identity and privacy canaries, unassigned/older/no-frame/no-dispatch admission,
real source composition, partial failures, caps, Busy, timeout, caller abort,
shutdown, retained source leases and no agent execution. Exercise assigned
wire behavior on the assigned integration head, without a parallel test gate.

The CLI must test selected remote node text/JSON, typed refusal, output bounds
and hostile remote-error/connection-data canaries on stdout and stderr.
The native owner tests request/connection generations, close/refresh/rename/
forget late results and partial rendering with its existing admitted identity
and ticket mechanism. It does not implement another transport or collector.

## Package workflow and resume

The existing D2/D3 and merge-base evidence in
`backend-m12-diagnostics.md` is retained. No earlier source-only pass becomes
a complete package pass. Consume the announced Quality B head under merge
rules; use narrow checks per commit, one whole root gate and one package
submission at completion. Security changes retain per-commit review. No
unchanged whole-gate rerun is needed for this plan.

Quality B head `29eebb8bcc285fb04bf9cb4c7a38a10d0622f6c1` is consumed by
a conflict-free merge. It preserves protocol 23, minimum 2 and schema 18.
The canonical pending row and MethodShape now allow wiring; the pending diagnostic
row is registered, with execution refused until the integrator assigns it. Provider diagnostics is A9/W22, including tracker row 568; E4 does not
implement its collector or RPC. No native sources changed and the package remains incomplete.
Under A14 this merge receives narrow checks; the whole gate is at package end.

## Product narrow checkpoint

The product checkpoint after the current shared method-table move passed
strict all-target Clippy for the eleven affected owner/client/CLI crates.
This establishes compilation and lint acceptance on Linux, not runtime or
whole-package acceptance. Focused tests, generated TypeScript freshness and
the package gate remain pending. The pending method still refuses on the wire;
an assigned-protocol successful request requires the integrator's assignment.

The focused product checkpoint passed 53 selected tests (52 in diagnostic
namespaces and one existing diagnostic-named dictation test), including the
real collector, actual generated authority guard, two-node source separation,
queued source cancellation/deadline lifetime, CLI privacy and pending wire
refusal. This run predates the subsequent shared node-head merge and canonical
PID reuse; it is not evidence for the later merged tree. The standalone Proc
decoder cases and full package gate remain to run on that tree.

The newer shared node head preserves protocol 23, minimum 2 and schema 18.
Its process owner now supplies the canonical ProcessId, so the diagnostic
self-process edge removes its duplicate identifier and reuses the checked
root value. No lifetime/guard, process carrier or signal behavior changes.

The first product TypeScript generation attempt found the diagnostic PID alias
row before DeviceId, outside the canonical sorted order. The generator's
binary search therefore reported it unclassified and stopped before writing
new declarations. Only the new row moves into its correct sorted position;
existing shared classifications and their order are unchanged. The failed
generation remains separate from the subsequent corrected source check.

The generated-guard regression also checks the actual collected recent-error
window: both pre-handler scope refusals and the three saturated-collector
refusals must be present in order without loss. This verifies the journal's
real caller rather than relying only on its actor component tests.

The first expanded node/Proc narrow execution stopped after the node crate:
273 tests passed and the added guard-window assertion failed because it
expected oldest-first rather than the journal's existing newest-first order.
The product returned all five expected codes; only the test's expected order
is corrected. Proc tests were not reached in that fail-fast invocation.
The subsequent run keeps all assertions and uses no-fail-fast to report both
selected crates even if one fails. No deadline or product policy changes.

The corrected expanded node/Proc run passed all 322 selected library tests,
including the canonical PID decoder and actual newest-first guard-error
window. The latest shared node head is consumed with its protocol, schema and
existing cases unchanged. These results do not replace the full package gate.
The CLI distinguishes missing Read scope from missing authentication using
closed messages, while keeping private connection/source chains internal.

## Package gate outcome

The frozen product source `95e4505f` passed the complete root package gate:
all 18 executed steps returned zero, including both testkit prebuilds and
normal/TypeScript/forge strict Clippy. Rust has 2,422 passed, zero failed and
12 ignored across 116 summaries; TypeScript generation/freshness has four
passed and no generated diff. Web lint/types/build, 464 units and 68 E2E cases
passed; the feature-specific forge fixture has four passed. All 22 workspace
crates were actually compiled in the Rust test log.

The selected 2,000 input hashes match both the frozen Git source and the
post-run worktree. The generated file retains eight canonical whitespace
warnings in the complete diff check; authored paths excluding that file pass.
It is not manually reformatted. Earlier failed/interrupted/narrow runs remain
separate evidence and are not relabelled as this whole execution.

This outcome establishes the gated node package on Linux. Final implementation
review and integration are still separate. The pending number stays None;
successful assigned-wire, native consumer, other-OS and deployment behavior
are not claimed. The integrator assigns the version and validates the combined
line, including supported/older-wire cases after assignment. No live service
or permissions were changed.

Pre-execution source check caught a responder fixture naming its reply after
the response type rather than the canonical method variant. It now uses
`Reply::Diagnostics`; no product method/DTO/number changes. The first queued
strict attempt remains pinned to its original test source; no result is
claimed from the correction before its own remote check.

The obsolete first strict ticket was still queued and was cancelled before
slot admission under the standing supersession rule (terminal 143, no compiler
result). Its source had the known reply-variant error. One changed-source
replacement follows; the previous-protocol CLI responder also advertises its
actual reduced maximum rather than claiming the current range.

The no-Read agent fixture creates its session through the actual Operate-only
creator, then observes credential output through the owner connection, as the
existing scope-clipping fixture does. It does not grant the creator Read merely
to make its follower work. A raw authenticated websocket case additionally
checks exact correlated report/refusal ids at the assigned and previous
versions and sanitized unknown-field rejection without breaking the connection.
These additions still await their execution.

The wire lifetime fixture reuses the existing test-only authority node setup
instead of repeating startup policy. Its release-sender helper preserves
timeout cleanup. Paired-reader and clipped-agent assertions are named fixture
phases so each test history remains readable without a large function. This
changes test structure only; the accepted guard test and product API stay intact.
