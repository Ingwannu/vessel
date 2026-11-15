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

Workspace provision delegates to ADR 0082's idempotent provision API using the
receipt's reserved WorkspaceId. Backing reservations belong to the registry;
this package does not create a second workspace table or filesystem workflow.
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

Only authenticated person callers with Operate may create; Read observes
safe snapshots; Admin changes profiles. Agents are refused before admission,
receipt lookup, filesystem access or launch. Forwarded requests retain scope
intersection. Bind receipts to their creation authority and recheck current
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
payload or unbounded history. Read scope and person-only policy apply to both
calls, including forwarded agents. Their sole protocol constant stays None;
the method table registers both as pending unary calls, never a dummy version.
