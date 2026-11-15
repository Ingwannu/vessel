# Session prompt index

Status: contract, snapshot projection and pending RPC row implemented; validation
is pending. Node head 40 (ba45a6e3), including Quality B, is merged. The pending
row is appended to rpc/table.rs. No protocol number is assigned here.

## Reference and asset check

Paseo 0.15.4 `packages/server/src/server/agent/timeline-prompt-index.ts:16-40`
indexes every projected user message as sequence, timestamp and a whitespace-
collapsed preview. Its preview is bounded by UTF-16 units, including ellipsis.
`packages/app/src/agent-stream/chat-outline/use-chat-outline.ts` accepts only
its newest request and the same timeline epoch; its model resolves the active
prompt by sequence. The native client already derives an outline from received
records, but has no metadata-only way to discover prompts outside that window.

Existing assets: the store owns record decoding, read transactions and the
blocking read pool; the node's search semaphore bounds simultaneous reads;
protocol sequence/session/time types already exist. Reuse these. No migration,
dependency or second timeline store is needed. The standard library and the
existing serde/SQLite stack suffice.

## Decision log

- Purpose and intent: allow outline navigation before a complete transcript has
  arrived, without returning message bodies or starting a harness.
- Existing constraints: records are durable and rewinds keep superseded records;
  the existing search index deliberately keeps historical text. It cannot be
  used as the live prompt index without interpreting rewind boundaries.
- Alternatives: download every log into the client (delays navigation); add an
  eager persistent index/migration (writer cost and duplicated projection);
  reuse text search (requires query text and includes superseded prompts).
- Choice: a bounded read-only snapshot of one session, folding only user prompts
  and rewind boundaries into metadata. The store streams decoded records inside
  one SQLite read transaction, retaining bounded previews rather than bodies.
  Pages share a typed revision derived from retention start and latest rewind.
- Why: uses the existing authoritative log, keeps appends unchanged and makes
  rewind/prune changes visible. Appends alone do not invalidate an older page.
- Advantages, costs and effects: no schema write; each request scans retained
  records. Explicit scan/index bounds refuse excess work rather than silently
  hiding prompts. Measurements must be recorded before claiming acceptable
  large-history latency; a future incremental index can preserve this contract.

## Proposed wire contract

`session.prompts` is a Read request, new and pending. Request: session, exclusive
sequence cursor, optional expected revision, validated page limit. Reply:
matching session, revision, ordered entries (record sequence, timestamp,
validated preview), and optional continuation. Missing session is NotFound;
changed retention/rewind revision is Conflict; work/index bound is Unavailable.
A structured empty history returns an empty page. Rewinds remove superseded
prompts from the index. Chunked history prompts coalesce across metadata/log
records; submitted/steered prompts remain separate entries.

One named constant owns each bound. Preview truncation counts UTF-16 units as
Paseo does, but never splits a Unicode scalar (a deliberate Unicode safety
floor). Pages bound the reply unlike Paseo's single complete array. Incomplete
or malformed peers are refused at the typed decode boundary. Unassigned calls
are refused by both client and node; older peers never receive them. Direct and
nested forwarding admission are tested via the common method-table policy.

## Structure and validation

`vessel-proto::prompt_index`: validated values and pending protocol constant.
`vessel-store::prompt_index`: snapshot read and pure folding, no harness access.
`vessel-node::service::prompt_index`: typed delegation through the existing store
handle. The method table owns Read scope and pending protocol admission; the store
owns a two-scan semaphore whose permit stays inside blocking work after cancellation.
Native `link` consumes it after integration; its view state retains session,
revision and correlation identity. No crate dependency changes.

Tests: whitespace/Unicode/limit and page decoders, chunk boundaries, rewinds,
retention and missing sessions, stable paging versus append, conflict after
rewind/prune, scan limits, pending/old/direct/forwarded admission. Narrow checks
per commit; complete root gate and TS freshness once for the node package.

## Resume point

Done: typed values, pure fold/store tests, pending registration and service, raw
client/server refusal tests for direct calls and all three fleet envelopes, and
deterministic abandoned-running/queued-scan admission regression. Narrow clippy
reported two client-test helper warnings, corrected in fa1a126e9. The combined
library tests and generated TypeScript refresh are queued; no final gate claim.

Preview text retains resource-link labels/URIs in the existing timeline's
Markdown form. Submitted attachment chips stay outside the text; replayed file
names use the timeline's bracketed caption. Each part streams into the bounded
preview builder rather than first allocating a full body or URI. A projection
regression covers both sources. Store integration/measurement test names share
the `prompt_index` prefix so one targeted suite covers all A3 store boundaries.
Next: narrow checks, TS generation, scan latency measurement, root gate and node
review; then integrator protocol assignment and native consumption.
