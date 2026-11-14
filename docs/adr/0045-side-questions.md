# 0045: Side questions in a separate conversation

Date: 2026-11-13. Status: contract accepted; implementation under verification.

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
Protocol 19 adds side_open, side_prompt and side_reset. Read plus operate is needed;
agent principals are refused. Every side question gets a bounded, labelled
partial parent snapshot. Neither its question nor its answer is sent to
the parent. Existing secure transport, grants and fleet forwarding apply.

The track plan specifies wire types, limits, failures and older peers.
The store already persists the parent link in the spec. Schema 14 adds a
bounded latest-plan projection so that transcript paging does not erase
the parent's current plan. General prompt and queue ingress refuse side
sessions; only side_prompt refreshes context and starts their turns.
Opening reuses ended/interrupted sides; restarting is explicit. Parent
deletion does not cascade to side logs and prevents new questions or resume.
Side logs remain subject to normal independent retention and deletion.
The complete prompt is capped in UTF-8 bytes, including all metadata.

## Alternatives

- Inject a question into the running agent: violates the isolation rule.
- Resume the parent's native id in another process: providers may share
  history on disk; neither process nor context is reliably isolated.
- Fork at a completed turn: GOAT's preferred path. The integrated base now
  has session.fork, but it registers the new session before the provider
  accepts or refuses the fork. A transparent failed-fork/fresh-start
  fallback would need to coordinate that failed session's retention and
  deletion with concurrent open/reset. Keep the fresh-thread path explicit
  for this slice, with only the bounded live snapshot advertised; do not
  imply that the parent's complete native history was inherited.
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


## Provider-only context and native commands

A question and its prepared context travel in one actor command. The
provider request receives a context prefix; TurnStarted and the persisted
user message contain only the person's question, like GOAT SideChatNote.
Context is consumed by that one request even on busy, cancellation or send
failure, never set as sticky state. The driver checks the final serialized
ACP request against 48 KiB before sending it. Its prefix, escaping and
request envelope count too: the driver reserves the actual request id
before measuring and publishes only after the check. Ordinary prompt and
attachment limits stay as-is.
The context's Debug representation contains only its size.

GOAT bypasses its prefix for provider-native slash commands so they remain
at the start. Vessel instead refuses slash questions with a typed reason:
its current side contract requires fresh parent context on every question.
Silently turning a slash command into prose is not acceptable. Run a native
command in a normal conversation. This deliberate restriction is tested;
it creates no turn. Replay during resume of an existing Vessel session or a profile move
remains suppressed until Ready, including UserMessageChunk containing
provider-stored generated context.
An agent can still echo its input: provider-only describes request/event
placement, not secrecy from that agent.

Vessel's event log, log reads, subscriptions and CLI event output retain
only the visible question as user input. The existing session.export is a
different boundary ([ADR 0012](0012-fleet-sync.md)): it transfers the provider's native conversation files
unchanged. A provider may persist the generated context there, and that
context can travel with a native export/import or resume. Vessel neither
promises context-free native exports nor rewrites an opaque provider
transcript. Imported native sessions lose their node-local side link;
they become ordinary sessions, as the existing import contract requires.
A new native import can replay that provider history, including its context;
the question-only guarantee does not extend to those opaque imports.

A side call or a session template carrying its parent link needs protocol
19 on every fleet decoding hop, including envelopes. A protocol-18 home
would discard the optional parent link before forwarding session.create,
changing it into an ordinary session. Clients therefore refuse that route
before sending; the older-hop behavior for pre-19 calls stays unchanged.

The integrated node baseline speaks protocol 18 (including the agent-driver seam).
Side calls and optional parent links therefore require protocol 19, including
all forwarding envelopes; minimum version remains 2. Schema 14 follows the
search index migration. A parked side is live and its dedicated prompt wakes
it through the existing actor path with replay suppressed. A failed wake
fails the one command and does not preserve context for a later prompt.


## Integration with rewind and fork

A rewind leaves the old records in the audit log. A side snapshot therefore
stops its newest-first tail at the latest rewind, and its independent plan
projection is cleared transactionally on that event. Until the provider
reports another plan it is explicitly unavailable, rather than showing a
plan from a discarded future. This bounded choice also omits valid earlier
turns before that rewind; the snapshot remains labelled partial. The generic
session.fork refuses a side source with unsupported and points to side_reset,
so it cannot duplicate the parent link outside serialized side creation.
Ordinary session forks and rewinds retain their existing behaviour.

The transient context is an optional part of the shared driver's atomic
Prompt command. Every new direct driver must either implement the same
question-only event placement and serialized request bound, or refuse that
context-bearing command before sending; dropping the context is not valid.
The integrated driver-seam baseline supplies ACP and unbuilt direct stubs;
only fake ACP processes are exercised by this slice.
