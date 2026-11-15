# Session prompt index

Status: contract, snapshot projection and pending RPC row implemented. Frozen
root code f7c8ba9f8 completed the whole gate as 17/18 steps green with one named
runner-path case; all owned tests passed. The coordinator accepted closure of
that case by the same-head vessel-remote narrow under the corrected runner,
33/0/0. Both original red and closing green receipts are retained below.
V3 final source review and integrator protocol assignment remain pending. The
wire constant stays None; no positive assigned new-RPC runtime, native app or
large-history frame-latency acceptance is claimed.

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

## Validation receipts (2026-11-15)

- `wp-search-node-narrow-20261115T115158Z-18fd8000` stopped while compiling
  tests: two own rewind fixtures used FromStr on HarnessSessionId. Source
  `13149bdaf` switches all three own occurrences, including the integration
  fixture, to the existing canonical constructor. The original red is retained.
- `wp-search-node-narrow-20261115T124346Z-13149bda`, immutable source
  `13149bdafef60369f669bdd913feed0f7920af63`, executed Client 80, Proto 321,
  RPC 54, Store 81 and Web 56 passing tests; one Web test failed (raw 592/1).
  All owned fold/preview/budget/admission/client tests and TS writer tests passed.
- The sole failure is `wire::ts::tests::every_primitive_alias_is_classified_and_every_entry_still_declared`:
  LimitPercent aliases FiniteNumber, but the base's primitive-only registry
  lists it. The coordinator and integrator confirmed that the base gate's TS
  filter had not executed this case. The bundle counts with that named base
  defect. Integrator fix `9dfdf0a11` removes the stale registry entry in batch 12;
  this package does not edit the head's registry entry or its test, and does not
  replay the unchanged bundle.
- Generated TS was retrieved from that verified remote source, not hand edited.
  Commit `59c2e95ce` records the generated ordering change; the bridge is unchanged.
  Proto SHA-256 `96f5f0d76d7654f1bd8cfebd5c6ebeb37c4687fb19cfa2561e46be1cb8547c12`;
  bridge SHA-256 `aac2f47064fb6f343e2c73588f6405f8cb32fc9c864dec3d23f3c03903e5734a`.
- Durable integration and the explicitly ignored 20,001-record measurement run
  once together using `cargo test -p vessel-store prompt_index -- --include-ignored --nocapture`.
  Immutable source `59c2e95ce` finished as raw 8/1: six owned unit tests,
  retention integration and measurement passed; the reopen fixture failed with
  DatabaseBusy because it dropped the facade without awaiting worker shutdown.
  The fixture now awaits the existing Store::close before reopen and closes the
  reopened store, preserving all durability/unknown-session assertions.
- Measurement on the shared VM: 20,001 records, 10,000 prompts, 50 pages;
  first page 494.871783 ms, complete traversal 12.83429935 s; first reply 21,264
  bytes. This is debug-build data-path evidence under load (start39.08/end32.81),
  not an app frame-latency claim. The raw failed suite remains retained with run
  `wp-search-node-narrow-20261115T132404Z-59c2e95c`, store measurement Cargo log;
  no unchanged measurement rerun is needed. The final root gate must verify the corrected reopen case.

These scoped results do not constitute the final root gate, V3 approval,
assigned wire admission or native app/capture approval.

## Frozen whole gate and named runner-case closure

Source `f7c8ba9f82ac0c21db8df42376b05e792355150c` includes exactly the
coordinator/integrator-authorized shared fix `9dfdf0a11` via cherry-pick -x; it
removes one stale LimitPercent primitive-alias entry and changes no generated
wire bytes. Earlier failed receipts remain unchanged.

- Whole run `wp-search-node-narrow-20261115T154618Z-f7c8ba9f` completed all
  18 steps: 17 rc 0, Rust tests raw 2388/1/13. Web unit 464, e2e 68 and forge 4 passed;
  all three Clippy configurations, TS freshness, binaries and other steps rc 0.
  Owned pending/old direct/enveloped admission, bounded scan cancellation,
  retention and the corrected durable reopen test explicitly passed. The ignored
  large-history measurement is retained as its earlier individual receipt.
- The sole failing test was vessel-remote's private SSH control-directory case.
  Its runner TMPDIR was 49 bytes; the default tempfile suffix plus `/ssh` made
  the directory 64 bytes, beyond the product's conservative 63-byte limit. The
  product correctly omitted sharing. SSH source/tests were unchanged. The
  coordinator classified this as a named runner case, not a product failure.
- T installed a short, private, ownership-checked per-label temporary directory.
  Same-head narrow `wp-search-node-narrow-20261115T165918Z-f7c8ba9f` then ran
  only vessel-remote tests (plus the tool's existing build prerequisites). It
  completed 33/0/0, including the exact formerly failing case; Cargo rc 0, digest
  prefix `8072f2eebe830115`. Runner facts record UID/GID 1000 and the 21-byte
  temporary directory with mode 0700. No SSH test or product guard was edited.

The accepted evidence is the original whole 17/18 plus its named runner case
closed by that narrow, with both logs. The original whole remains raw rc1; it
is not rewritten as an uninterrupted green run. This documentation-only receipt
leaf changes no gated code or generated TypeScript and needs no unchanged gate.
V3, wire assignment and native feature acceptance remain separate.
