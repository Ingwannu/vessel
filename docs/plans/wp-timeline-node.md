# Canonical conversation protocol package

Updated: 2026-11-15. Status: contract accepted; standalone value skeleton prepared, runtime not wired.
ADR: [0084](../adr/0084-canonical-timeline-and-prompt-receipts.md).

## Assets, reuse and alternatives

Decision: modify/reuse. Extend vessel-proto agent/conversation, vessel-agent
ACP/direct decoder edges, vessel-session delivery/projection, vessel-store
conversation persistence, vessel-node authorized service and vessel-client.
No new crate, external API, dependency or architecture edge. Existing typed
ids/answers, RawJson, serde, SHA-256, SQLite and actor sequencing suffice.
A client-only parser/cache duplicates semantics and cannot provide restart
idempotency. Provider-specific arbitrary JSON stays at its decoding edge.

## Reference behavior read

Paseo 0.15.4, paths relative to packages:

- protocol/src/agent-types.ts:215-364: all eleven detail kinds, provenance,
  running/completed/failed/canceled states; :367-409 canonical items;
  :447-487 permission kinds, actions and response payloads.
- server/src/server/message-receipts/index.ts:8-76: canonical sorted-key hash,
  per-agent/message receipt serialization, pending-before-send, completed replay,
  conflict and uncertain refusal. Our own store does not read Paseo receipt files.
- server/src/server/agent/providers/codex/async-questions.ts:8-179: async item
  schema, stable identity, pending independent of completed tool, response
  preparation/commit, serialization and history-retention cleanup.
- server/src/server/agent/agent-timeline-content.ts:4-28: plugin payload limit is
  serialized UTF-8 bytes; the wider timeline truncation policy is separate.
- Surveys read: parts/C-timeline.md, parts/D-messaging.md, gaps.md first sections;
  CATALOGUE.md and CONNECTIONS.md links inspected for cross-area ownership.

- protocol/src/agent-message.ts:4-14: provenance event/sender/text, no separate
  message id in this structure.
- server/src/server/session.ts:1360-1370 and :5181-5220: capability filtering
  and direct modern permission acknowledgements.
- server/src/server/agent/providers/codex-app-server-agent.ts:4797-4823:
  async answer dispatch targets exact active foreground turn, otherwise follow-up.
- app/src/components/question-form-card-core.ts:30-177: custom/option pairing,
  preview, allow-empty and dismiss behavior.

These pointers were opened after the initial contract draft. The local driver
schema is pinned to Codex 0.166.0; unsupported incoming kinds must not be invented.

## Current ownership and planned modules

- proto: timeline/{item,detail,permission,identity,compat}, conversation receipts
  and question presentation/lifetime; existing event and RPC adapters delegate.
- agent: typed ACP/provider decoding modules; common external detail normalization;
  Codex async questions independent of the blocking approval responder collection.
- session: identity through delivery tickets/queue, durable question projection
  and serialized answer routing; never parse provider JSON here.
- store: schema migration allocated by integrator; atomic receipts and durable
  question records. Keep data under existing private store ownership/modes.
- node: authorized service plumbing only after the shared method-table head;
  no bypass of ownership, scope intersection or existing input validation.
- client: identified prompt and rich permission response calls with capability
  refusal. Consumers adopt these types after contract review.

## Differences and activation

The ADR lists every deliberate difference and bidirectional old-peer behavior.
The single TIMELINE_PARITY constant remains None; no provisional protocol
number. One migration holds all tables, newest schema plus one locally and
re-pointed by integration. New events use the shared optional required-protocol
shape from the lifecycle prerequisite. Whole package root gate only at completion; narrow per-crate checks
while implementing. Schema number and shared method-table head are dependencies.
Node fake fixtures only; no real agent, preview, production config or service.
No native UI files are owned by this package.

## Resume point

Done: fresh worktree from the announced node head, source/survey inventory,
existing assets checked, ADR and contract written.
Done: contract revision passed after retained-state capacity finding.
ADR now fixes finite per-session/node count+byte reservations, capacity refusal
before effects, replay-at-capacity and session-removal-only accounting release.
Standalone timeline value modules now define canonical tool/item/provenance,
permission presentation, question lifetime/presentation, admission budgets and
identified prompt request/reply. Existing events/calls/DTOs are unchanged.
 Integration ruling: one optional protocol constant;
one newest-plus-one migration, re-pointed at merge; shared event accessor.
Quality B head is still awaited.
Next: narrow value checks; share exact types/hash with consumers; event/field
adapters, driver fixtures, persistence/state machines, typed client/service
wiring once the shared head is announced; regenerate TS; one complete root gate.
Checks: first narrow clippy found two documentation-markup errors, corrected.
Narrow clippy/tests pending on current skeleton. Runtime, complete gate and UI
behavior remain unverified.

## Staged scalar choices

Typed detail URLs reuse HttpUrl, rejecting credential-bearing/non-HTTP(S) targets;
a provider lacking a valid target retains the opaque Unknown body. Plugin schema
versions are positive typed integers, a deliberate resource/typing floor beyond
Paseo's permissive number payload. Search durations use the existing DurationMs
unit; the provider decoder will normalize seconds rather than expose two units.
Compaction retains Paseo's Loading/Completed contract; ACP failed/canceled
compaction needs an explicit failure-notice mapping before advertising support.

## Retained receipt service staging

`vessel-store/src/timeline_admission` is registered under cfg(test) until the
shared pending-method wiring exists. Its actual SQLite operations enforce
writer-transaction admission, fingerprint canonicalization, exact-key replay
before capacity, fixed envelope reservation and accepted-result CAS. SQL
triggers own per-session/node accounting; startup checks it against retained
rows and encoded envelopes. The SQL is not in production migrations yet.
Tests are written for replay/conflict, authenticated creator and nested JSON
canonicalization, session byte saturation, each count/byte predicate, retained
state after log pruning, committed session deletion, restart/corruption and
concurrent same-key/final-slot admissions. They have not yet run. Durable
question and node/provider dispatch work remain unimplemented.

The staged question store now preserves provider-source deduplication separately
from node-minted request ids, validates typed answers before a durable claim,
retains the exact foreground target, and only settles a claim after an explicit
accepted result. A known-not-sent refusal can restore Pending; restart turns
unacknowledged admitted answers into OutcomeUnknown. Tests are written for
deduplication, resolution/tombstones, invalid/concurrent answers, refusal retry
and uncertain recovery, but are not yet executed. This is storage behavior
only: no actor or real provider steering is connected or claimed.

## Quality B and value follow-up

Merged the announced Quality B node head under the standard merge policy;
no conflicts. Method since/shape and pending-row registration now come from
the shared table. The shared AgentEvent optional accessor is owned elsewhere
and will be consumed rather than duplicated.

Standalone value clippy passed on 96ed8b0b. Its queued local unit test was
canceled before execution under the updated build-line ruling; rc143 is a
cancellation, not passing or failing test evidence. Narrow checks now move to
the remote runner. The follow-up adds safe caller UUID allocation, typed
admission bytes, a stable task-id value and known-detail-preserving updates.
