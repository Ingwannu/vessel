# Plugins

Status: study committed, no code. ADR 0068 is proposed, not
accepted. The inventory is `docs/plans/plugins-inventory.md`. The
three hosts are `docs/plans/plugins-hosts.md`.

## Goal

Carry Paseo's plugin behaviour across, under its names, and stop
where a GPUI app cannot draw a React component or where a security
floor is stricter than a silent drop. The owner reviews the ADR
before any code.

## What was read

Paseo 0.15.4, the files and lines in the inventory. Vessel: the
crate table, `SessionSpec`, `session.create` (`create_session` and
the shared `create_starting`), automation hooks, `docs/LESSONS.md`
on plugins, ADR 0035, and the workspace manifests for an embedded
engine (none).

## Existing asset, reuse, decision

| | |
| --- | --- |
| Existing | Automation hooks (HTTP, protocol 11). `session.create`. No plugin crate, no JS engine, no Wasm runtime |
| Reuse | The create path, unchanged when the registry is empty. The hook names and wait bits from Paseo. The lesson's host: a helper process, later |
| Not an extension of | The automation hook type. A plugin id is not a `HookId`, and a rewrite of a create spec is not an HTTP delivery |
| Outside | Node, as a possible later helper; a Wasm runtime, as a possible later sandbox. Neither is adopted. No licence or size is claimed, because nothing is added |
| Decision | New, when the ADR is accepted: crate `vessel-plugin`, called by `vessel-node`. Insert the two calls in `create_starting`. Do not reorder the checks already in that function |

## Slice 1, after the ADR is accepted

The contract is the "Slice 1" section of ADR 0068. Short form:

- Fourteen hook names, Paseo's wait bits, byte order of the plugin
  id then registration order, 30 seconds on a waiting hook.
- Only `agent.create.before` and `agent.create.after` are called,
  from `create_starting`. The RPC, a schedule, a fork and a new
  side question pass through them. An import (an origin is set)
  and a rewind do not.
- Before runs after the draining refusal and before
  `refuse_installing`. It may replace `title` and `harness`. A
  change to `cwd`, `place`, `profile` or `mode`, or a handler
  error, is `InvalidRequest`. The message names the plugin and
  the hook, and later handlers do not run.
- A 30-second timeout is `Unavailable`, with the same names in
  the message. It is not `InvalidRequest`.
- After is not awaited. Its failure is logged and the session
  stays.
- Empty registry: create is unchanged. No new wire method, so no
  protocol number. A later call takes the number given at merge
  and is gated on one named constant.
- Tests beside the rules, under `cfg(test)` only, no sleep. A
  release build's registry is empty. The node test that proves
  the deadline uses the paused clock.

No install command, no provider, no UI. No `node.settings` key in
this record. A later enable switch is one top-level key, named in
that slice's ADR, absent meaning off.

## Later slices, not started

Each waits on the review of the one before it.

1. `agent.open.before` with launch-only environment, still no
   child. Tools and MCP servers stay absent until a process can
   own a callback.
2. Worktree create before and after, on the worktree call that
   already exists, with the path refusal Paseo makes.
3. Turn and permission notifications, wired to the session events
   that already exist, not awaited.
4. The helper process and its framed protocol, a fake child only
   behind a test-only feature, still no JavaScript. A wire call, if
   a client must see a plugin, takes the number given at merge.
5. Anything the app draws, as data the kit can render, in a native
   track. React components are out of scope until a record says
   otherwise.

A Node helper that evaluates a Paseo server bundle, and a sandbox,
are separate decisions. They are not implied by slice 4.

## Resume point

- Done: the inventory, the host comparison, ADR 0068, this plan.
  Corrected in place: no protocol number is taken (a later call
  gets the number given at merge), no settings key is listed, the
  helper's reach and the Windows path are written, test handlers
  are `cfg(test)` only. Corrected again: both create hooks sit
  on `create_starting`. The before hook follows the draining
  refusal and precedes the checks that read the spec. Import and
  rewind are the skips, with the reasons in the ADR.
  A refusal is `InvalidRequest`. A timeout is `Unavailable`. No
  code. No crate.
- Half-done: nothing in code. The ADR is still proposed.
- Next: wait for the review of this revision. Do not start the
  crate. A returned document is fixed before any code.
- Not verified: Paseo's scheduler implementation and the provider
  `forkSession` body; the field-by-field provider schemas; the
  app's client-bundle loader; whether `localeCompare` and byte
  order differ for any id the schema accepts (they are not
  compared here). The body of `agents.reserve` was not read.
