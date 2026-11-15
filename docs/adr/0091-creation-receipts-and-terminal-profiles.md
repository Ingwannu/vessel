# 0091: Durable creation operations and terminal launch profiles

Status: contract accepted; receipt/runtime implementation pending review.

## Purpose and intent

Creation survives a route change, reconnect or node restart without blindly
duplicating a workspace, chat or terminal. Empty workspaces and terminal
launches use the same explicit workspace authority as chat creation.

Paseo 0.15.4 references inspected: `packages/server/src/server/creation/index.ts`
lines 64-149 (admission, intent fingerprint, resource claims, subscribe and
interrupted in-flight refusal), `packages/server/src/server/agent/create-agent/create.ts`
lines 156-214 (registration before first prompt), and
`packages/protocol/src/terminal-profiles.ts` lines 11-104 (defaults,
sentinel substitution and launch shape). App behaviors are recorded in the
native package's `docs/plans/wp-a1.md`.

## Existing implementation and constraints

The node has session creation, prompt delivery, shell terminals, project
approval, managed worktrees and the SQLite store's serialized writer.
ADR 0082 owns the sole workspace identity, registry and backing reservation.
There is no durable user creation operation, observation/reconciliation
contract or configurable terminal launch profile contract on the base.
The existing shell terminal call does not accept arbitrary program argv.

## Alternatives considered

- Retry the existing calls after every disconnect: can duplicate worktrees,
  sessions or the first prompt after a lost acknowledgement.
- Implement a client-local receipt or cwd-derived workspace: cannot prove
  node effects or distinguish independent workspaces sharing a directory.
- Reproduce Paseo's JSON receipt/claim files: introduces another storage and
  locking mechanism beside the node's existing SQLite authority.
- Persist receipt state in the existing writer and compose the existing
  registry/session/terminal services at explicit side-effect boundaries.

## Decision

Add typed creation key, revision, phase, stage, target and snapshot values.
One package protocol constant remains `None` until the integrator assigns it.
New rows register only after the central method table supports pending rows.
Older peers receive the standard unsupported result before any side effect.

Admission atomically binds an idempotency key to the validated normalized
intent fingerprint and reserved resource ids. A different intent with the
same key is an explicit conflict. Concurrent identical requests observe one
operation. Completion stores only the resource identities and safe status;
neither prompt text, environment nor configured argv is copied into a receipt.

Persist the next stage before invoking a side effect. Persist its resource
identity/result before proceeding. A restart or lost completion while a stage
is in flight becomes an explicit outcome-unknown receipt; do not replay it.
Subscribe observes revisioned snapshots and recovers this distinction. A
known failure before a side effect can retry the same reserved identities.
Registration commits a new agent: first-prompt failure retains that agent and
does not go back to creating another one. Completed receipts are rechecked
against the registry/resource before being returned as reusable success.

Workspace provision uses the receipt's reserved WorkspaceId and ADR 0082's
registry/backing reservation API. Under the recorded ownership transfer, A1
owns the actual filesystem adapter while the registry, project admission and
fixed-id worktree primitives remain their existing owners. This package does
not create a second workspace table or bypass those admission boundaries.
All project approvals and setup blockers remain explicit, never auto-approved.

Terminal profiles are a bounded ordered set of validated ids, display names,
programs and argv templates. Chat is the default launch target; blank shell
is a distinct target, not a stored profile. The four reference defaults are
Claude Code, Codex, OpenCode and Pi. Substitute every `{{{prompt}}}` occurrence
as literal argv text. With an empty prompt, omit standalone prompt arguments
and option assignments whose entire value is the sentinel. Do not concatenate
shell commands. A profile without a sentinel launches as-is and shows its
resolved command read-only. Blank-shell typed input goes through terminal
input followed by CR, letting the user's shell parse it.

## Why this choice

It reuses node ownership, explicit workspace identity, write serialization,
checked paths, process groups and terminal transport. Typed stages separate
known refusal from uncertain side effects; generated profile argv avoids
shell interpolation of a prompt.

## Advantages, costs and effects

Adds a bounded persistent receipt index and explicit observation lifecycle.
Recovery may require a person to inspect an unknown outcome; this is the
intended cost of refusing a duplicate side effect. SQLite remains the only
receipt authority. No external dependency is added.

Profile ids, counts, command/argument sizes and total payloads have named
bounds. Compared with Paseo's permissive profile schema, validation refuses
empty/NUL-containing programs, duplicate/reserved ids and excessive data
before writing configuration or spawning a process. These are stated safety
differences, not silently modified launch semantics.

## Retained capacity, atomic admission and key lifetime

The single receipt ledger uses the named limits in `vessel_proto::creation`:
`MAX_RECEIPTS` bounds retained operation count, `MAX_RETAINED_BYTES` bounds
reserved logical bytes, `MAX_RECEIPT_BYTES` bounds any serialized receipt at
every revision, and `RESERVED_BYTES_PER_RECEIPT` reserves a receipt's worst-case
future growth plus its key/operation indexes and resource claims. A receipt
can claim at most its new workspace and one resource. Caller keys are hashed;
the fixed digest and UUID identities cannot grow with caller text.

These are retained logical-data budgets, not a claim about SQLite page, WAL,
filesystem or backup overhead. Their values are defined once beside the types.
No unbounded diagnostic, prompt, command, environment or descriptor snapshot
is permitted in a receipt. Future fields must fit the existing reservation or
introduce a reviewed capacity/migration change before they can be persisted.

Admission first looks up the existing key and compares its normalized intent.
Only a genuinely new key checks capacity. One existing-writer savepoint then
reserves the operation's entire worst-case byte allowance, count slot and all
resource claims atomically, before any filesystem/provision/process/prompt
effect. Concurrent requests for the last slot cannot both succeed. Failed
admission rolls back all claims and capacity changes. A full ledger produces
typed `Full`/Unavailable before effects; it never evicts another receipt.
The accepted slot already covers every later phase/failure/recovery update,
so those updates need no additional admission and remain possible at capacity.
Every write checks the serialized receipt cap before replacing the prior row.

Keys, fingerprints, operation ids and claims are retained indefinitely on this
node. There is no age eviction, automatic release, cleanup-on-resource-delete
or release RPC in this version. Removing a session/workspace does not make its
key reusable: a completed receipt whose resource is gone returns an explicit
missing-resource refusal, and an unknown receipt remains unknown. Same-key
lookup/reconciliation continues at capacity. The cost of this exact retry
guarantee with finite storage is refusal of new keys when full. A future
compaction could keep a permanent refusal tombstone, but cannot treat a retired
key as absent; it needs its own reviewed contract. Resetting a node's whole
data/identity is an explicit administrative destruction outside this guarantee.

Receipt capacity belongs only to this ledger. ADR 0082 independently admits
workspace identity/backing/start leases on the same Store; it must not reserve
another copy of a receipt or count its byte budget again. Registry admission
failure after receipt admission becomes a retained known failure before
provision effects; it does not silently free the caller key. Its backing/start
claims and this receipt's operation/result markers keep their distinct jobs.

Tests required before receipt source is accepted: count and byte saturation,
concurrent last-slot admission, existing-key lookup/conflict at capacity,
worst-case later-phase growth while full, refused oversized mutation retaining
the previous row, restart at capacity, no effect after failed admission,
resource deletion preserving claims and old-key retry/unknown refusal. Shared
registry capacity/start-lease behavior is tested through ADR 0082's API, not a
second workspace admission implementation.

## Security boundary and platforms

Authenticated person callers with Operate may create workspaces/terminals;
the amendment below also permits principal-bound agent creation and own-agent
observation. Read observes safe authorized snapshots; Admin changes profiles.
Agents cannot enter the person workspace/terminal namespaces or use terminal
profile configuration. Forwarded requests retain scope intersection. Bind
receipts to their creation authority and recheck current
authorization when observing or advancing them. No raw prompt, command argv,
credentials or template contents enter logs, Debug or error messages.

Terminal launch runs under the node's existing account and approved workspace
root. Profiles are intentionally code execution configured by an administrator;
there is no download or package installation. Resolve a program on the node,
use argv rather than shell evaluation, preserve the stripped terminal
environment and existing group/Windows job cleanup. Unsupported platform
launch behavior returns a typed refusal. The app never executes the program.

## Verification and integration

Tests cover forged/boundary values, default/profile substitution, duplicate
and conflicting requests, concurrent admission, revision ordering, staged
restart, missing completed resources, route-safe completion, first-prompt
failure and blank-shell input. Use fake harnesses and in-memory transport.
Security-bearing persistence/launch commits receive review individually.
Node and native package gates run at completion; no runtime, protocol/schema
number, review result or parity completion is claimed by this proposed ADR.

## Observation transport

`creation.observe` resolves the original kind/key after a lost creation reply;
absence remains explicit. `creation.subscribe` is a bounded unary long poll
with an optional committed revision cursor. This differs from Paseo's
connection observer: it reuses Vessel's per-connection/per-origin waiting
shares and ordinary typed reply rather than adding a second server-frame
subscription transport. A saturated share replies immediately. Subscribe
before the initial SQL snapshot, then re-read after a coalesced post-commit
wake-up or deadline. An unchanged revision is not completion, and a cursor
ahead of storage is refused. Notifications hold one unit value, never a
payload or unbounded history. Read scope applies to both calls; the amendment
below confines agent callers, including delegated agents, to their own agent
namespace and kind. Their sole protocol constant stays None;
the method table registers both as pending unary calls, never a dummy version.

## Amendment: principal-bound agent creation receipts

Contract source accepted by V3 row154; implementation security review remains owed.

This amendment extends agent creation for MCP without opening a person's
workspace/terminal creation or operation history to agents. V3 has accepted
the contract. Principal-bound storage and own-agent observation are staged
implementations awaiting source acceptance; the actual agent creation driver
and MCP acknowledgement remain unavailable. Contract acceptance alone does
not approve those implementations or authorize runtime effects.

### Authority and identity

The receipt key is `(owner namespace, operation kind, opaque key)`. The node
constructs the namespace exclusively from the admitted principal:

- Person: the existing person namespace shared by authorized person devices.
- Agent: the origin `NodeId` and exact parent `SessionId` from
  `Principal::agent()`, including delegated ingress. A delivering device, book
  alias, requested child id, raw cwd or request-supplied owner is not identity.

Store lookup hashes a domain-separated canonical encoding of that complete
namespace/kind/key. Fixed owner metadata fits the existing retained reservation;
raw keys and prompt/argv are still absent. An agent cannot select the person
namespace or another parent's namespace. The same opaque key and intent used
by different namespaces yields independent operations; a different intent
within one namespace remains a conflict. Existing person keys retain their
original digest encoding during migration so an upgrade never forgets a key.
All namespaces share the existing count/retained-byte capacity and permanent
key/claim retention. No second receipt database, time window or key eviction
is introduced; exhausted capacity can still refuse new legitimate work.

### Agent entry and admission order

A new typed agent-operation entry accepts a caller key, an existing workspace
(or the caller's authoritative registry membership), an allowed structured
session specification and bounded first input. It does not accept a namespace,
creator, parentage, reserved resource id, arbitrary shell/profile command or
permission override. An unbound legacy caller receives a typed placement
refusal for this new workspace-bound entry; its existing authorized legacy
CreateSession call remains unchanged. No cwd-derived or fabricated workspace
is supplied as a fallback.

Scope, origin/parentage, descendant/depth policy and requested configuration
are checked before durable admission. Under one bounded intake gate (not an
unbounded mutex-per-key map), existing-key lookup/conflict precedes any new
count reservation. A fresh operation takes the existing agent count/creation
lease before ledger admission; failure rolls it back. The gate's finite wait
refuses overload before effects, so identical concurrent requests cannot both
reserve a child or fail merely because each reserved the last count slot.
The intake uses named `MAX_ACTIVE_CREATIONS = 64` retained actor permits and
`CREATION_ADMISSION_WAIT = 5 seconds` for the one admission mutex. Existing-key
lookup/reconciliation happens before taking a new permit and again under the
mutex; an already active operation never allocates a second actor. A fresh
permit remains with its driver until settlement, not merely the first reply.
No resource/count reservation is held while waiting for the mutex. The count
adapter wraps existing `Agents::reserve`/`forget` (agents/mod.rs:112-182 and
service/sessions.rs:155-188); it does not introduce another child counter.
The receipt writer then reserves ids, count/bytes and claims atomically.
The background driver owns that count lease across startup; it uses the same
parentage/count/runtime registration path as authorized legacy creation.

Before a new spawn or a known-refusal retry, recheck current parent authority,
count admission and ADR0082's Active registry start lease. A parent archive,
revocation or workspace archive cannot be bypassed by an older receipt.
Only exact reserved child registration settles the lease; a count/refusal or
proven no-spawn releases it. A restart/in-flight ambiguity remains unknown
and never grants a replay/count lease. Existing registered or completed keys
never create another child or consume another child-count slot. First-input
retry keeps the same reserved child and the canonical message-ledger id; it
never returns to agent creation. No parallel message-id type is introduced.

Workspace and terminal creation stay person-only, including terminal-mode
session specifications. Cross-workspace agent placement does not detach or
change its parent: the admitted origin remains the parent through the existing
ownership checks. Scheduled/internal callers must carry their validated
creator and cannot construct an owner namespace from request data.

### Observation and privacy

The existing typed kind/key observation requests need no client-supplied owner.
A person resolves the person namespace. An agent may resolve only its own
namespace and agent-operation kind; workspace/terminal kinds are refused
before lookup, even when the key is absent. Delegated agent calls derive the
same origin-bound namespace and the effective scope intersection. There is no
wire lookup by guessed operation UUID that bypasses this binding. Internal
operation-id reads remain trusted driver APIs and do not confer wire authority.
Read scope is still required; a read cannot create, retry, release a claim or
answer a permission. Snapshots remain safe historical lifecycle records, not
proof that a deleted resource currently exists. Reusable create success must
separately revalidate the exact reserved registry/resource ownership.

### Required regression obligations

Test person/agent and agent/agent identical keys, distinct origin nodes with
the same session UUID, same-principal reconnect/delegation, forged owner and
parent fields, own-kind restrictions, Read/Operate scope intersection and
revoked/archived parents before spawn. Test simultaneous identical requests
at the last child-count slot, count/ledger-capacity refusal with no effect or
leaked lease, restart/unknown non-replay, and completed/deleted child retries
without count inflation or replacement. Cover first-message failure/retry to
the same child/message id, retained capacity/growth accounting including owner
metadata, old person-key migration and failed namespace decoding as corruption
rather than an absent key. MCP consumes this one typed path only after source
review and checks; it does not construct a second receipt pipeline.


## Terminal profile settings persistence and scope

Profiles use one bounded row in the existing protected SQLite store and the
same serialized writer/savepoint, not a parallel settings file. Absence means
the four reference defaults; an explicit empty list stays empty across restart.
A distinct ProfileRevision provides CAS for full-list edits. Identical current
settings do not increment it; stale edits cannot overwrite a newer list.
Corrupt settings are refused and retained, including when an administrator
tries to write, rather than silently resetting or exposing parse contents.

The validated profile set has a named aggregate textual bound (1 MiB), and
its serialized storage row has an 8 MiB limit below the RPC frame limit. This
is an additional stated safety difference from Paseo's permissive settings.
Read-scope terminal_profile.list returns only ids, names, icons and prompt mode;
program/argv templates are absent. Admin get/set return the full configuration
and stay node-local administration. All three calls refuse agents, including
delegated agents, and remain pending under the sole creation protocol constant.
The configuration does not itself launch a process or approve repository setup.

### Accepted-before-Begin restart continuation

V3's 2026-11-15 contract clarification distinguishes a provably unstarted
Accepted operation from an orphan committed in-flight marker. A same-key
Operate request may continue the former using the original reserved child;
Read/observation cannot start it. Existing key/intent/namespace lookup and
permanent accounting precede the bounded gate and all count admission.

This continuation requires authoritative startup recovery with no old driver
or blocking task able to commit a late marker/effect. Every external effect
must follow the durable Begin commit acknowledgement; a lost acknowledgement
is resolved through actual stored state, never inferred from cancellation.
If ownership/generation fencing cannot establish that proof, refuse as
uncertain. Attach to a current driver rather than creating a second one.
Recheck current effective Operate, parent/depth/grants/count and workspace
Active/start lease. The exact child must have no registered, live, stored or
restored count entry before re-acquisition. Inconsistency is reconciliation
or typed refusal, never overwrite, another UUID or reminted credentials.
Rollback releases only the lease acquired by this attempt.

A committed marker without authoritative completion remains Unknown even if
the process crashed before calling the effect. Unknown, registered and
completed operations never return to child creation. Known pre-effect refusal
may retry the same ids after current checks; first-message retry stays on its
existing child and canonical message id. Required regressions include idle
Accepted restart and read-only observation; same-id/count continuation under
last-slot concurrent retries; commit-before-effect crash; lost Begin ack and
stale actor; changed parent/workspace authority; restored count/resource
non-overwrite; and all existing deleted/completed/Unknown retention cases.

The existing Agents count owner now refuses duplicate ids while holding its
one lock, before credential minting. This is a prerequisite for a future
adapter, not proof that the unimplemented creation driver supplies the
remaining persistence, ownership and startup fencing requirements.
