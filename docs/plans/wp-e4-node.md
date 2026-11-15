# E4: node diagnostics package

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
- Provider diagnostics: consume the provider package's one collector/cache
  contract. `agent.status` is not a pure cached diagnostic alternative:
  even `refresh=false` reaches builtin version discovery.
- Ordinary node text and CLI cause-chain output include private descriptions;
  they are not diagnostic renderers. Add a bounded allowlist renderer and a
  deliberate output boundary when its actual caller is wired.
- External software: no new dependency is introduced by this plan. Any safe
  process-metrics implementation still needs an owned typed source and the
  repository's dependency decision before adoption.

## Audit and acceptance rows

| Node responsibility | Existing evidence | Work still required |
|---|---|---|
| Safe daemon report and partial source states | D2 value-only acceptance; D3 private staged-source acceptance | Actual collector, one request, client, CLI and admission tests |
| Process identity/version/monotonic uptime | Private lifecycle projection | Capture startup clock before I/O; actual caller and bind observations |
| Store/schema and durable/runtime counts | Real SQLite/runtime component tests | Compose source timestamps, timeout and shutdown ownership |
| Agent metadata/default-profile cached readiness | Metadata-only source tests; no agent invocation | Current immutable profile snapshot, bounded selection and actual caller |
| Listener and configured/callback relay states | Owner-side watch projection | Publish actual bind state and aggregate without endpoint copies/cycles |
| Provider diagnostics | Provider owner reports a proposed cached/refresh contract | Consume its reviewed API; do not duplicate or claim an existing endpoint |
| Process/system metrics from the survey | Not implemented by the narrow report | Audit safe typed sources or obtain an explicit accepted difference; do not rename session counts as process diagnostics |
| Daemon status and local daemon logs | Existing node description; desktop/controller owners have separate responsibilities | Reuse those owners; no arbitrary remote log/path endpoint |
| Recent failure summary | Private bounded typed-RPC journal tests | Common generated observation boundary; never claim a raw log tail |
| Bundled web UI | Existing node-hosted web package and its gate | Preserve it; audit any advertised capability separately from report readiness |
| Shareable rendering and failure output | Validated values only | Bound the complete export; exclude raw connection/error cause text |

The pending diagnostic method and `NodeClient::diagnostics()` are now wired
as an unverified product checkpoint. The number remains unassigned, so no
successful wire request is claimed. CLI rendering and refusal/privacy fixtures are present but unverified;
package-level validation remains.

## Contract and canonical wiring

ADR 0074's proposed call is `node.diagnostics {}` returning
`DiagnosticReport`, requiring Read and initially not fleet-forwardable.
Clients use each selected node's existing authenticated connection; encrypted
relay transport is distinct from a fleet-forwarded call.

The feature constant stays `DIAGNOSTICS_PROTOCOL: Option<ProtocolVersion>`.
Only the integrator assigns its number. After the Quality B head is announced
and consumed, the sole canonical method row uses the pending feature form in
`method_table!`. Generated guard/envelope/client registrations are consumed,
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

Provider integration uses the other package's proposed `DiagnosticRead`
separation: Cached reads only prior observations with Read; explicit Refresh
requires Operate and may execute checks. A cache miss never becomes Refresh.
The concrete request/response and cache getter are not committed yet, so these
are coordination requirements rather than an available API. A bounded free
`AgentVersion` still needs normalization before a shareable report; custom
provider identifiers likewise cannot become arbitrary export labels.

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
