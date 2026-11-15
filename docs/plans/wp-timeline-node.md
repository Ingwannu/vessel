# Canonical conversation protocol package

Updated: 2026-11-15. Status: contract proposed; implementation not started.
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
TIMELINE_DETAILS, PROMPT_RECEIPTS, DURABLE_QUESTIONS remain None; no provisional
numbers. Whole package root gate only at completion; narrow per-crate checks
while implementing. Schema number and shared method-table head are dependencies.
Node fake fixtures only; no real agent, preview, production config or service.
No native UI files are owned by this package.

## Resume point

Done: fresh worktree from the announced node head, source/survey inventory,
existing assets checked, ADR and contract written.
In progress: contract review and schema allocation requested.
Next: resolve review; send exact approved types to native/CLI consumers; value
modules, adapter fixtures, persistence/state machines, typed client/service
wiring once the shared head is announced; regenerate TS; one complete root gate.
Unverified: all implementation, compilation, runtime and UI behavior.
