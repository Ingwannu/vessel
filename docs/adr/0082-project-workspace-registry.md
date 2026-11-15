# 0082: Explicit project metadata and independent workspaces

Status: proposed

## Purpose and intent

A project is a registered directory. A workspace is an independent, durable
container within a project, with its own identity, title, isolation and lifecycle.
Two workspaces can share a cwd without owning each other's agents or terminals.
Registering a project creates neither a workspace nor an agent.

Reference behaviour: Paseo 0.15.4 `packages/app/src/hooks/open-project.ts:80-114`,
`add-project-flow/model.ts:29-85`, `workspace/workspace-archive.ts:82-135`, and
`packages/server/src/server/session/workspace-provisioning/`.

## Existing implementation and constraints

Projects/worktrees already have typed RPCs, a SQLite-backed node registry,
checked roots and digest-scoped command approvals. They are reused. There is
no independent workspace identity, project metadata mutation or guarded config
editor. A worktree id cannot replace a workspace id: deleting the last managed
backing and archiving one logical workspace have different effects.

## Alternatives considered

- Infer workspaces from cwd or session id: conflates independent owners and
  cannot represent an empty workspace.
- Client-local registry or metadata overrides: diverges across clients and
  cannot authorize filesystem mutation.
- Reuse generic file.write for vessel.toml settings: lacks project-specific
  authorization, full syntax checking and explicit stale revision semantics.

## Decision

Add typed project and workspace registry contracts to vessel-proto, with a
single pending feature constant. The protocol number is assigned at merge after
all consumers handle the new outcomes. No call is reachable while unassigned;
older peers are refused before dispatch, and existing replies remain unchanged.

Stage the node implementation independently until the shared method table can
register pending calls. Store schema numbers are assigned by the integrator.
Keep new SQL in a dedicated migration module rather than rewriting released SQL.
Reuse the store writer and read pool; do not create a second database/file registry.

Project metadata is a separate descriptor keyed by ProjectId; existing
ProjectInfo remains backward compatible. Rename changes only custom display
name; reset derives from the recorded directory. Plain directory creation uses
exclusive nonrecursive mkdir and reports registration/rollback separately.
Suggestions use registered harness history readers with explicit limits and do
not import conversations. Icons are bounded image bytes with content revisions;
URL fetching remains a client responsibility, never a node SSRF endpoint.

Configuration editing targets only active registered roots. Preserve raw TOML
text/unknown keys rather than projecting and rewriting a partial schema. A content
hash revision distinguishes absent file from empty file. Compare immediately
before exclusive-temp-file rename; serialize node writes per project. External
writers can still race stat/check/rename: this is optimistic concurrency, not an
OS-level compare-and-swap guarantee. Validate via the existing vessel.toml reader
before committing; saving never approves or executes commands. Preserve existing
file access modes and refuse symlink targets. New config files are private to the
service account; read/write permission failures keep the original source error.

Workspaces persist a project id plus explicit local or managed-worktree backing,
state, title, revision and resource ownership. Creation receipts use a caller
operation id to reconcile disconnects without duplicating a worktree. The registry
must serialize conflicting lifecycle mutations. Archive operates by workspace id,
archives owned agents and terminals, stops owned setup, and removes managed backing
only after its last active reference, with the existing safety checks. Local
folders remain. Partial cleanup failures are typed and visible, never success by
optimistic UI alone. Bulk archive returns every target's result. Recovery is
inspected by the node, re-inspected before mutation, and does not resume agents
or rerun setup implicitly.

## Why this choice

It extends the existing authority and typed boundaries, permits empty/shared-cwd
workspaces, preserves old clients, and avoids widening automation approval.
Existing worktree removal/snapshot semantics remain distinct from archive.

## Advantages, costs and effects

Additional typed calls, storage rows and lifecycle coordination cost more than a
path alias but make ownership reviewable. No new dependency is introduced.
Parsing, filesystem IO and image inspection run off async executor threads.
Windows uses the same checked filesystem contract; an unavailable atomic replace
or permission operation returns a typed refusal rather than claiming success.

## Security boundary

Only authenticated person callers with the required Read/Operate scopes may use
registry mutation/config/icon/history endpoints; agents are refused before any
filesystem or provider-history access. Delegated callers retain scope intersection.
Recheck stored root identity against canonical filesystem identity on every IO.
No shell command or repository hook is executed by project metadata/settings.
Existing digest approval and environment stripping continue to gate automation.
Debug and errors do not include config/icon/history payloads or credentials.

## Verification and staged state

Contract decoding validates private bounded fields and rejects forged values.
Tests cover absent-vs-empty revisions, stale writes, unknown TOML keys, symlink
and root replacement, directory rollback, concurrent lifecycle actions,
shared-backing last-reference rules and recovery reinspection. Fake harnesses
only. No protocol/schema number, runtime call, gate, capture or parity completion
is claimed by this decision document.
