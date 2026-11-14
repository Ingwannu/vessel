# ADR 0068: Plugins as a versioned helper protocol; hooks first, and no UI

Status: proposed, 2026-11-14. Not accepted. No code comes with this
record. The study is `docs/plans/plugins-inventory.md` and
`docs/plans/plugins-hosts.md`; the plan is `docs/plans/plugins.md`.

Paseo 0.15.4 is the reference. Where this record differs from it, the
difference is stated.

## Purpose and intent

Paseo's plugins are how a person adds a lifecycle rule, a tool, a
provider, a usage reading and a screen without changing Paseo. The
owner's definition of this version is that Paseo's behaviour comes
across. A plugin layer that invents its own hook names, or that
pretends a GPUI window can mount a React component, would not be
that.

The intent of this record is to fix the host, the names and the
first slice, so a later change can be gated. The first slice is the
hook machine on the node. It runs no plugin code, draws nothing and
adds no wire method.

## Existing implementation and constraints

Vessel has no plugin host. What it does have:

- Automation hooks (protocol 11): an HTTP delivery with a secret.
  Not a request rewrite, not a UI, and not to be extended into one.
  The names `subject.operation.before` and `.after` are free; the
  automation type keeps the name it has.
- `session.create`, in `vessel-node`'s session service. The spec is
  a harness, an optional directory, a profile, a title, a place and
  a mode (`vessel-proto` `SessionSpec`). Paseo's create request is
  a provider configuration, an optional environment and a directory
  that the hook cannot change (`packages/plugin/src/server/lifecycle.ts`
  lines 117 to 122; the directory check is
  `packages/server/src/server/plugins/lifecycle/index.ts` lines 260
  to 261).
- A GPUI app that draws kit controls from view models (ADR 0035).
  No web view.
- The lesson already written: out-of-process plugins, versioned
  JSON-RPC, those hook names (`docs/LESSONS.md`).

Paseo's own host, which this follows and does not copy:

- Fourteen hooks, a wait bit per hook, plugin-id order, 30 seconds
  on a waiting hook, a typed failure that names the plugin and the
  hook, notifications that log and continue
  (`packages/plugin/src/server/lifecycle.ts` lines 174 to 190;
  `packages/server/src/server/plugins/runtime.ts` lines 41 to 42
  and 546 to 571). The inventory has every name.
- Installed plugins in a forked child; built-ins in process; both
  speak one message union. The trust model is unsandboxed code, off
  until `pluginsEnabled` is set, default false.
- Client contributions are React components. That is the part a
  GPUI app cannot host. See the hosts note.

Constraints that bind the slice: types rather than strings, a typed
error, no new dependency without a paragraph this record does not
yet have, no protocol number until a client can see the feature,
validation before any work, and a refusal that is not a silent
drop.

## Alternatives considered

Three hosts, compared in `docs/plans/plugins-hosts.md`.

- **A helper process over a versioned protocol.** Paseo's installed
  plugins are already this. The node decodes typed messages. No new
  dependency. A Paseo server bundle runs only if the helper is Node,
  and that adapter is not the first slice. The app still cannot draw
  the React entry.
- **WebAssembly components.** A real sandbox, if the node grants
  little. No existing Paseo plugin is a component. A runtime
  dependency is not measured and is not added. The app draws the
  same data, or nothing.
- **An embedded JavaScript runtime.** Does not run Paseo's bundles
  unless the runtime is Node, which is the helper process. In
  process, a plugin fault is a node fault. Does not draw React in
  GPUI.

Also refused:

- **Reusing the automation hook type.** Different lifetime, different
  secret, different peers. A plugin id is not a `HookId`.
- **Loading the client bundle in a web view.** ADR 0035.
- **Offering Paseo's deprecated hook names** (`before` / `on`,
  removed after 2027-04-30). Vessel starts at the new names. A
  plugin written to the old names does not load; the error names
  the hook. That is a stated difference, not a silent miss.
- **Giving the plugin `PaseoApi`.** That value is the whole daemon
  SDK, including `config.patch` (`packages/client/src/index.ts`
  lines 499 to 508). A later slice can expose a narrowed typed API
  under the caller's grant. Slice 1 exposes none.

## The choice

The host, when a plugin runs, is a helper process speaking a
versioned protocol the node owns (design A). The protocol's first
and only messages in the first slice are not sent to a process at
all.

**Slice 1, the contract.** An in-process registry. No child, no
JavaScript, no install, no UI, no new wire method, no new
`ErrorCode`, no new external crate, no change to `PROTOCOL_VERSION`
(it stays 20). An empty registry leaves `session.create` as it is.

The registry knows all fourteen hook names and the wait bit from
Paseo's table, so a fifteenth name does not compile and a match
cannot use `_`. Slice 1 *calls* two of them, and only from the
RPC `session.create` path (`create_session`), before it calls the
shared create:

- `agent.create.before`, waiting.
- `agent.create.after`, not waiting.

Not called in this slice, though the names exist: the workspace
hooks, both worktree hooks, `agent.open.before`, close, archive,
turn and permission. Import, a schedule's run, a fork, a rewind
and a side question do not go through `create_session`, so they
do not fire these two. Paseo already skips create-after on import
and resume. Skipping schedules and side questions is narrower than
"every create"; it is stated, and a later slice decides each one
after reading that call site. It is not verified here whether
Paseo's scheduler fires `agent.create.before`.

`agent.create.before` receives a snapshot of the spec the caller
sent, after nothing else has been reserved: no session id yet
(Paseo's create-before has none either), no credential, no model
token. The handler may replace `title` and `harness`. `cwd`,
`place`, `profile` and `mode` are read-only; a result that changes
one is a refusal, and later handlers of this hook do not run.
Returning nothing keeps the request. There is no deep merge.

The replaced spec is then the spec `create` already accepts, so
the install check, the place, the grant and the credential run on
what the hook returned. Slice 1 adds no second authorization. A
hook cannot grant a harness the create path would refuse, and it
cannot take one the path would allow: the caller could have passed
that harness. Fields Paseo can change and this request does not
have (model, saved environment, MCP servers, system prompt, tool
policy, provider options) are absent from the type, not dropped
on the floor. Adding one later adds a field.

Order is the plugin id's UTF-8 bytes, then registration order.
Paseo sorts with `localeCompare` (`runtime.ts` lines 569 to 571),
which follows the process locale. Plugin ids are
`^[a-z][a-z0-9-]*$` (`packages/protocol/src/plugin-config.ts` line
3). On that alphabet, byte order and code-point order agree, and
they agree with `localeCompare` for those characters. Vessel uses
the bytes so the order does not depend on a locale. Stated, and
compatible for every id the schema accepts.

A waiting hook has 30 seconds, counted by the node around the
call, not by the registry sleeping. The deadline is the same
number as `REQUEST_TIMEOUT_MS` (`runtime.ts` line 41). On a
timeout the signal is cancelled, the operation fails, and later
handlers do not run. A handler error fails the same way. The wire
error is the existing `ErrorCode::Conflict` and a message that
names the plugin id and the hook. It does not include the spec's
directory, an environment value, or a credential. No new
`ErrorDetail` until a client has to branch on the reason; the
message is for a person.

`agent.create.after` runs only after the session exists. It is not
awaited by the RPC. Its error is logged with the plugin id and the
hook, and the session stays. The log line does not include the
spec. The test waits on a signal the handler sends, not on a
sleep. A handler still running at shutdown is cancelled.

Handlers in this slice are Rust functions in the test. There is no
dynamic plugin, so the registry is generic over the handler rather
than a trait object. The open extension point is the later process
boundary, and the comment that says so lands with that code, not
before it exists.

The crate, when the code is written, is `vessel-plugin`: the names,
the registry, the validation, the error. It depends on
`vessel-proto` for the ids and the spec fields it speaks, and on
nothing else from the workspace. It does not depend on a new
external crate. `vessel-node` is the only caller. The
`ARCHITECTURE.md` row and the ADR 0001 paragraph are part of the
commit that adds the manifest, not part of this record: a row
without a crate would fail the boundary check. No other crate
gains a dependency.

## Why this and not the others

Design A is the one `docs/LESSONS.md` already records, the one
Paseo uses for installed plugins, and the one that needs no
dependency to start. Design B is the right answer to a question
this record does not ask, which is "how do we run an untrusted
plugin". Paseo's answer is that the person trusts it. Changing
that is a later decision with its own record, not a silent
tightening. Design C does not deliver the compatibility it
suggests, and it puts plugin code in the node's address space.

The slice is two hooks on one call site because that is small
enough to test without a child process and without a wire change,
and because create-before is the hook Paseo's own examples use to
change what gets saved. Calling all fourteen on day one would edit
worktrees, turns and permissions in one change, and those files
are not this decision's to take.

## Advantages, costs and effects

Advantages. The hook names and the wait bits match Paseo, so a
later helper does not rename them. An empty registry is invisible
to today's clients and to older peers. A refusal names what
failed. The crate boundary keeps the rules out of `vessel-node`'s
session file except for the one call.

Costs. A Paseo plugin does not run. A screen, a theme, a slash
command, a provider and a usage source do not exist yet. Create
hooks do not see model, environment or MCP servers, so the
examples that set a Codex sandbox option or inject a server have
nothing to set. `agent.open.before` is not called, so a launch-only
environment cannot be added. Schedules and side questions are not
covered. `Conflict` is a coarser code than a dedicated reason; a
client cannot branch on "a plugin refused" until a later additive
detail.

Effects on other work. The code commit touches `vessel-node`'s
`create_session` and adds a crate. It must not reorder the checks
inside the shared create path. It must not log the spec. Reviews
of that commit gate the tests listed in the plan, not this record.

What this record does not decide: the helper's framing, the
install switch, whether the helper may be Node, the sandbox
question, and any contribution the app would draw. Each of those
is a later slice with its own review. No dependency paragraph is
owed until one of them wants a crate.
