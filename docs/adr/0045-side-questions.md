# 0045: Side questions in a separate conversation

Date: 2026-11-13. Status: proposed for contract review.

## Context

A person needs to ask about work while an agent keeps running. Sending
that question into the active session, including steering, changes its
conversation and context. GOAT SideChatSurface and SideChatParentContext
provide the reference: a separate thread with refreshed parent context,
a parent-status header, start-over and open-as-full-thread actions.
Paseo has no equivalent side surface. Agent-orchestrator also isolates
its side provider host.

## Decision

Reuse Vessel's structured sessions with an optional parent link in the
stored SessionSpec. A side never resumes the parent's native conversation.
Protocol 12 adds side_open, side_prompt and side_reset. Read plus operate is needed;
agent principals are refused. Every side question gets a bounded, labelled
partial parent snapshot. Neither its question nor its answer is sent to
the parent. Existing secure transport, grants and fleet forwarding apply.

The track plan specifies wire types, limits, failures and older peers.
The store already persists the parent link in the spec. Schema 13 adds a
bounded latest-plan projection so that transcript paging does not erase
the parent's current plan. General prompt and queue ingress refuse side
sessions; only side_prompt refreshes context and starts their turns.
Opening reuses ended/interrupted sides; restarting is explicit. Parent
deletion leaves side logs readable but prevents new questions or resume.
The complete prompt is capped in UTF-8 bytes, including all metadata.

## Alternatives

- Inject a question into the running agent: violates the isolation rule.
- Resume the parent's native id in another process: providers may share
  history on disk; neither process nor context is reliably isolated.
- Fork at a completed turn: GOAT's preferred path, but unavailable on this
  base. Use its documented fresh-thread fallback until the fork contract
  is integrated; do not imitate a fork with shared files.
- A second provider integration: duplicates session launch, credential,
  approval and lifecycle rules without improving the boundary.

## Why and consequences

A separate native session gives an inspectable isolation boundary and
uses the same tested process and permission lifecycle. A durable link
survives reconnects and restart without another registry or migration.
Bounded recent context keeps reads and prompts finite, but is not a full
history and is labelled accordingly. The same working directory means
normal project tools can still change files: this feature isolates the
conversation, not the filesystem. Reset runs stop/delete/create under the same creation gate with an expected
old-side id. A stale id is conflict. If deletion fails, no replacement is
created. If creation fails after deletion, the old side is gone; opening
again retries creation. This is serialized, not an all-or-nothing rollback.
The screen follows GOAT's structure using Vessel's shared kit; text and
pure decisions remain in view, node calls in link.
