# ADR 0074: Bounded typed diagnostics from one node

Status: proposed, 2026-11-14. Merged on the node line 2026-11-15 and
assigned protocol 24 (`DIAGNOSTICS_PROTOCOL = Some(24)`); the `None` text
below describes its review checkpoint.

## Context and reference evidence

The app already has an app-side Diagnostics report. It does not yet ask each
node for operational facts. `vessel status`/`node status`, `node.info`,
`agent.status`, the store reader and relay watches own much of the required
information, but returning their existing objects would disclose hostnames,
paths, executable output, sign-in help or keys. This report must be safe for a
read-only device and for copying into a support conversation.

The following are source observations, not runtime verification. Paths in this
section are relative to the Paseo source tree inspected on 2026-11-14.

| Source and lines | Exact excerpt | Behaviour followed |
|---|---|---|
| `packages/protocol/src/messages.ts:1484–1487` | `type: z.literal("diagnostics.request")` and `requestId: z.string()` | A correlated, explicitly requested report, not an unsolicited event |
| same, `5263–5270` | `diagnostic: z.string()` | Paseo returns one text block |
| same, `3794–3795` | `daemonDiagnostics: z.boolean().optional()` | The app checks whether the daemon offers the feature |
| `packages/server/src/server/authorization/operation-permissions.ts:83,318` | `"diagnostics.request": "daemon.read"` / `"diagnostics.response": "daemon.read"` | Read authority, not daemon management |
| `packages/server/src/server/session/daemon/daemon-session.ts:293–315` | `const diagnostic = await collectDaemonDiagnostics({` | The daemon collects its own facts and echoes the request id |
| same, `317–325` | `error instanceof Error ? error.message : String(error)` | A collection failure is visible rather than an empty success |
| `packages/server/src/server/session/daemon/diagnostics.ts:63–107` | `return redactDiagnostic(sections.filter(Boolean).join("\n\n"), options);` | Sections are collected separately, then joined and redacted |
| same, `127–155` | `{ label: "PATH", value: getEnvValue("PATH", "Path") ?? "unset" }` | Paseo includes process environment and home; Vessel deliberately excludes these values |
| same, `216–235` | `provider.error` | Availability includes a reason; Vessel uses closed reasons, not the error text |
| same, `58–61,426–457` | `const LOG_TAIL_LINES = 80;` / `const LOG_TAIL_MAX_BYTES = 64 * 1024;` | Paseo has a bounded recent log tail, not merely a boolean error flag |
| same, `543–567` | `.replace(/paseo:\/\/\S+/gi, "paseo://[redacted]")` | Paseo masks configured endpoints, links and credential patterns after collection |
| `packages/app/src/diagnostics/app-diagnostic-report.ts:15–16,108–142` | `formatDiagnosticSection(title: string, entries: DiagnosticEntry[])` | A copyable section/indented-label report with client-side redaction |
| `packages/app/src/components/app-diagnostic-sheet.tsx:111–123` | `for (const host of hosts)` | Per-machine progress and a combined client report |
| same, `131–151,175–200` | `runIdRef.current += 1;` | Closing/refresh supersedes old results; Copy and Refresh are explicit controls |
| same, `264–309` | `if (serverInfo?.features?.daemonDiagnostics === true)` | Offline, unsupported and failed hosts retain explicit per-host status |

Paseo also reports CPU/memory, disk capacity, workspace counts, websocket
metrics and tool versions (`diagnostics.ts:158–214,239–350,410–423`). Vessel
will not claim parity for metrics it does not collect. The operator's required
node facts below are the first contract. No Paseo UI or real daemon was run.

## Decision

[Decision Log]
- Purpose: a machine supplies a bounded, privacy-preserving operational
  snapshot for the app's existing Diagnostics page and the selected-node CLI.
- Existing assets: `Lifecycle::info`, the store's blocking reader,
  `Lookup`/`Readiness`, `Sessions`, and `Relays`' watch state remain the owners
  of their facts. The diagnostic service only projects them.
- Alternatives: return `NodeInfo`/`Harness` wholesale; run a shell doctor;
  return a server-formatted log/text block with regex masking; require Admin;
  add an OS inspection dependency. Each exports unnecessary data, runs new
  commands, or changes an existing authority boundary for a read operation.
- Choice: a typed allowlist, rendered by each client into Paseo's section and
  indented-label text. Standard-library and existing workspace crates suffice;
  no new dependency, persistent record or schema migration is planned.
- Why: the compiler can expose an added field for review. A bounded response
  containing no raw path/environment/error field is stronger than attempting
  to recognize every possible secret after assembling a text report.
- Consequences: details excluded by this policy stay in local logs. Unknown,
  stale, unavailable and incomplete coverage remain explicit. This report is
  neither a store integrity check nor a fresh agent login/protocol probe.

### Report contents and ownership

The planned `DiagnosticReport` contains only these sections:

| Section | Allowed facts | Source / meaning |
|---|---|---|
| Node | stable node id, collection time, normalized binary version, OS/architecture, Serving/Draining, monotonic uptime | lifecycle and node-instance clock; never hostname/name/key |
| Store | readable/unavailable, expected and observed schema, current stored session counts | one bounded reader transaction; no file path, SQL, file content, repair or integrity guarantee |
| Agents | at most 64 built-in ids/report-local custom slots, executable presence, structured availability, cached numeric version and check age/reason | existing registry/readiness plus metadata-only discovery; no label, program/path, arguments or output |
| Sessions | stored total and status counts, current live/parked runtime counts | count projections; no session ids, titles, prompts, projects, paths or transcripts |
| Listener | configured/bound state and LocalSocket/LocalPipe/DirectTcp kinds | startup/runtime state, never address, port, socket/pipe name or route URL |
| Relay | counts by Connecting/Registered/Refused/Stopped state, configured/callback origin | existing watches; no endpoint, ticket, token, peer key or last error string |
| Recent failures | last 32 retained failed local typed RPC completions within 15 minutes: ErrorCode and time; omission indicator | bounded in-memory journal, explicitly RPC-only coverage; not a log tail |

Node version is its compiled package version, projected into numeric components
and prerelease/build-presence flags without copying arbitrary suffix text.
Agent versions are optional numeric components from already observed facts;
unparseable/free-form words become UnknownVersion. Existing `AgentVersion`
is bounded but permits arbitrary words, so it is not copied as diagnostic text.
Built-ins reuse `HarnessKind`. An added `HarnessRef` contains a person-chosen
`AgentName` (including dots), so custom entries use report-local numeric slots
1–64 instead. Labels, names and profile names are not copied. This trades
custom-name identification for a report that cannot export an address or
secret placed in such a name. The local client knows its selected machine.

No request starts an agent, runs `--version`, a sign-in command, a shell, git,
gh, a network probe, a repair or a migration. Metadata-only executable lookup
uses the node's existing configured search policy, on a blocking worker; it
never exports that search path. Existing cached readiness may be NotChecked,
Stale, InUse, Missing, TooOld, TooNew, Unread, TimedOut or Failed(step).
Unavailable structured support is mapped to a closed reason; its raw reason
string is not copied. A binary being found is not a claim that it can sign in
or complete a turn. Refresh means collect another report, not refresh credentials.

Each unavailable section has a closed reason (not checked, stale, timed out,
read failed, too many items or stopping). A failed section does not turn into
an empty healthy section. The snapshot is collected at one time but is not a
cross-runtime/SQLite global transaction; section timestamps make that clear.

### Recent failure coverage

Paseo reads its last log lines. Vessel instead records only completed typed
RPC failures at the common guarded service boundary. Only the existing error
code and timestamp enter it; messages, method names, arguments, principals and
resource ids do not. Omitting method variants also avoids putting a future
method unknown to an older reporting client into an otherwise supported reply. Envelope wrappers are excluded to avoid counting a
single local inner failure again as its wrapper. Transport/handshake/parser
failures and errors written only by background tasks are outside its coverage.
The renderer says "failed node calls", not "all errors" or "no errors".

One node-owned task keeps 32 records; its nonblocking observation channel holds
64 events and drops new observations when full. A saturating omitted counter
and a limited flag disclose loss/eviction since this node instance started.
Records expire against a monotonic clock after 15 minutes. Restart clears the
journal and changes its observation start time. No log file is opened and no
raw tracing event is captured. Tests must prove observation failure cannot
block or change an RPC's result. This narrow summary intentionally replaces
Paseo's broader log tail to satisfy the no-content/no-secret requirement.

### Privacy boundary

There is no field for an address, hostname, pairing link, executable or home
path, environment name/value, key (public or private), token, password,
secret name/value, sign-in help, raw error, agent output or file content.
Home is represented by a fixed RedactedHome value and rendered as `~`; this
never carries the real home or a custom data-directory suffix. Existing
objects containing these fields are projected, never serialized wholesale.

This applies to JSON, text, Debug, malformed-value errors and partial failures.
Future client copy/save actions use only this allowlist. The report crosses
an ordinary authenticated encrypted connection; the relay retains its existing
traffic-metadata visibility and receives no plaintext report. Collection does
not dial anything. No host or address is inferred from the listener.

### Bounds and failure behaviour

- Entire serialized report: at most 64 KiB, excluding the existing RPC frame.
  A bounded serialization sink enforces this before a reply. No JSON tree is
  built solely to inspect or redact it. Too large is a typed whole-report
  refusal, never silent truncation of a purported complete report.
- Agents: at most 64 rows. Discovering a 65th makes that section unavailable
  with TooManyAgents; it does not allocate/return the complete registry first.
- Recent failures: 32 records / 64 queued observations, with explicit loss.
- Collections: two try-acquired permits per node, no waiting request queue;
  a third receives a typed Busy refusal. Collection deadline: 5 seconds;
  client deadline: 10 seconds. Permits remain owned by outstanding blocking
  store/metadata work after caller timeout/disconnect, following the existing
  guarded-reader pattern. A node shutdown cancels collection/journal tasks.
- Clients render at most 128 KiB of text, checked before stdout/copy/save.
  JSON remains the exact bounded typed report. Numeric counters saturate and
  disclose that state rather than wrapping or panicking.
- Unavailable platform facts use an explicit Unsupported state on any OS;
  no Linux-only diagnostic command is executed as a fallback on Windows/macOS.

### Authority, wire assignment and CLI

Paseo maps both diagnostics operations to `daemon.read` (quoted above).
Vessel uses the existing authenticated `Read` scope, `Some(Read)` when the
method is eventually registered. Owner, paired, agent and delegated effective
scopes keep existing admission/intersection rules; no Admin is needed for
this allowlist. A caller without Read is refused before any collection.
No higher authority is obtained by asking for JSON or refreshing. The first
call is not fleet-forwardable: the app reads each node over its own connection,
as Paseo iterates hosts. Encrypted relay transport remains supported.

The reserved method is `node.diagnostics` (no request fields), returning the
typed report; `NodeClient::diagnostics()` and
`vessel [--node NAME] node diagnostics [--json]` will use the same call.
Text prints the same section/indented-label style; JSON is opt-in. Connection,
unsupported, unassigned, forbidden and busy failures are explicit and nonzero
in the CLI. There is no offline fallback pretending to be a node report.

No wire number is assigned by this track. The value slice declares
`pub const DIAGNOSTICS_PROTOCOL: Option<ProtocolVersion> = None;`, documented
with the node, typed client, CLI and native Diagnostics consumer obligations.
It leaves PROTOCOL_VERSION and MIN_PROTOCOL_VERSION at the supplied base.
There is no Request implementation, method-table/guard registration, client
call, CLI command or bridge allowlist entry until the single shared
`required_protocol()` accessor is merged on the node head and consumed.
No sentinel or alternate gate is introduced. After that seam exists, None
must cause typed unassigned/Unsupported refusal before client transmission
and before server dispatch, including nested envelopes; an assigned number
must refuse older peers by the same shared rule. Existing node.info replies,
lists/events and older clients gain no new fields or unsolicited frames.
The integrator assigns the number only with the consumers/gates in place.

## Verification and delivery

Slice 1 is this proposal and plan only. No diagnostic collector, wire call,
CLI or native integration has been implemented or runtime-verified here.
The node plan owns executable acceptance rows and explicit handoff to the app
owner; this track does not edit `spikes/native` or register a native area.

## E4 product wiring decisions (Quality B)

The pending `node.diagnostics` row is now registered through Quality B's one
`method_table!`, with Read, unary shape and `DIAGNOSTICS_PROTOCOL = None`.
No assigned number is invented. `Method::shape()` excludes Envelope from the
code/time-only failure journal; the generated guard observes the complete
local outcome including authority refusal. Diagnostics is not fleet-forwardable
and direct agents retain effective Read admission, like `node.info`. The node's startup clock
is captured before configuration/store I/O and listener state is published only
after successful binding. A weak relay reference prevents a service cycle.

Self-process metrics belong to `vessel-proc`, as a process primitive, not a
second decoder in the node. Linux reads only `/proc/self/status`, capped by
`MAX_STATUS_BYTES` beside that source. It decodes this process's pid, optional
parent and checked RSS/virtual bytes. Raw names, group lists, paths and lines
are discarded at the edge. Missing memory stays unavailable; other operating
systems explicitly return Unsupported. No new dependency, external command,
arbitrary process enumeration or environment access is introduced. The wire
uses a separate checked process identity and JSON-safe count saturation.

The response sources share `RESPONSE_BUDGET`; caller timeout does not kill
blocking filesystem work. Existing D3 admission leases remain in queued and
running jobs and keep shutdown tracking/capacity until completion. The typed
client has a separate end-to-end `DIAGNOSTIC_REQUEST_TIMEOUT`. Typed source
errors retain their causes internally; only closed reasons enter the report.

The CLI now has a bounded text/JSON renderer and a safe error boundary.
Assigned-protocol successful wire execution and complete package acceptance
are tracked separately from selected source/runtime checks. Provider diagnostics belongs to
A9/W22 and is not implemented or registered by E4.

The self-process decoder reuses the canonical root `vessel_proc::ProcessId`
from the process-observation owner. It keeps the checked kernel u64-to-u32
conversion cause and the canonical nonzero constructor cause. A numeric PID
is an observation, not a lifetime or signalling authority; collection neither
issues nor changes a ProcessLifetime guard. The wire process id remains a
separate validated protocol value.
