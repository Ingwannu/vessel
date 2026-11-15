# Session children: node contract and execution plan

The package decision is ADR 0090. Native import is already an existing-contract
client (ADR 0053); it does not require node changes in this worktree.

## Structure

- vessel-proto/subagents: complete identities, current-parentage and durable
  detach result; provider descriptor/event/page types; one pending protocol gate.
- vessel-store/subagents plus store facade and writer registration: atomic
  first/repeat receipt, safe read/projection, one appended metadata migration.
- vessel-node/sessions/subagents and service/subagents: cancellation-safe detach
  and authoritative admission/publication; existing guard/envelope registration
  only after Quality B. Ownership uses current parentage.
- vessel-agent native decoding edges: actual child lifecycle/timeline extraction;
  unrelated main-timeline frames retain their current behaviour.
- Provider projections reuse the stored parent event stream, bounded reads and
  typed page results. No second event transport or raw JSON in domain state.

## Asset/reuse analysis

Modify existing store writer, marks publication, authenticated principal/ownership
and event-page mechanisms. New modules isolate the new responsibilities; existing
APIs do not model current delegation or provider-owned child identities. Standard
collections, serde and existing RPC/store crates suffice. No dependency adoption.

## Delivery order and evidence

1. Types and tests; no method registration while Quality B is absent.
2. Detach persistence/ownership regressions and cancellation-safe service.
3. Provider extraction/projection, with edge fixtures and no real agents.
4. Central Quality B merge, pending table rows, wire compatibility and generated
   TypeScript; one V3 node review and root gate for the complete node package.
5. Native integration consumes only a reviewed, integrated head.

Resume: ADR recorded, implementation not started. Base is the reviewed node head
named in the working brief; later central merges follow A8 without rewriting it.
