# 0091: Durable creation operations and terminal launch profiles

Status: proposed

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
