# ADR 0068: Plugins as a versioned helper protocol; hooks first, and no UI

Status: accepted for the hook machinery slice; the host (helper
protocol, what may be installed) awaits the owner. The hook
machinery is the crate that follows this record. The study is `docs/plans/plugins-inventory.md` and
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
`ErrorCode`, no new external crate. This slice takes no protocol
number. `PROTOCOL_VERSION` stays at the base this branch was cut
from; this record does not set it. A later slice that adds a call
or a field takes the number given at merge, and gates that call on
one named constant of this track so the integrator changes one
line. An empty registry leaves `create_starting` as it is.

The registry knows all fourteen hook names and the wait bit from
Paseo's table, so a fifteenth name does not compile and a match
cannot use `_`. Slice 1 *calls* two of them, from the shared
create, `create_starting`
(`crates/vessel-node/src/service/sessions.rs` lines 114 to 125).

- `agent.create.before`, waiting. It runs after the draining
  refusal and before `refuse_installing`.
- `agent.create.after`, not waiting. It runs only after
  `sessions.create` has returned the session and the claim and
  the watch that already follow a successful create have run.
  The RPC does not wait for it. A failed create does not start it.

Not called in this slice, though the names exist: the workspace
hooks, both worktree hooks, `agent.open.before`, close, archive,
turn and permission.

Paseo fires both hooks inside `createAgentInternal`. That is the
only path `createAgent` takes (`agent-manager.ts` lines 1348 to
1353). The before hook runs for every config that is not
`internal` (lines 1363 to 1370). The after hook runs for every
agent that is not `internal` (lines 1408 to 1411). Vessel's
`create_starting` is the same kind of shared path. The RPC
`create_session` already reaches it (`create`, then `create_as`,
then `create_core_as`, lines 53 to 106). A schedule's run reaches
it through `create_as` (`service/scheduled.rs` line 138). A fork
reaches it directly (`service/forking.rs` lines 12 to 16 and
line 43). A side question that has to create a session reaches it
through `create_core_as` (`service/side_questions.rs` line 114).
Calling the hooks only from `create_session` left those out. The
call moves onto `create_starting`.

`agent.create.before` runs after the draining refusal and before
the checks that read the spec. The draining refusal stays the
first statement of `create_starting` (`sessions.rs` lines 122 to
124). Paseo calls `assertAcceptingAgentRegistrations` and then
the hook (`agent-manager.ts` lines 1361 and 1364). The same
order: refuse a create the node is not accepting, then the hook.
The hook is inserted after that refusal and before
`refuse_installing`, and it moves no existing check. A waiting
hook is up to 30 seconds, so it does not run for a create the
draining check is about to refuse.

The checks that read the spec stay in the order they have now,
after the hook: `refuse_installing` on `spec.harness` (line 125),
`place`, the grant, and the credential (`agents.reserve`). A
harness the hook replaced is what `refuse_installing` reads. The
body of `agents.reserve` was not read, so this record does not
say the grant inspects the harness. Slice 1 adds no second
authorization.

These creates do not fire either hook:

- An import that starts a session passes an origin
  (`service/transfers.rs` lines 126 to 128) and so enters
  `create_starting`. The hooks do not run when that origin is
  set. Paseo's import path, `importProviderSessionInternal`,
  never calls the lifecycle (`agent-manager.ts` lines 1555 to
  1612). The hook document says import and resume do not fire
  `agent.create.after` (`public-docs/plugins/hooks.md` line 252).
- A rewind does not create a session (`service/forking.rs` lines
  48 to 59). That is the resume case in the same sentence.

Paseo also skips `config.internal`. Those agents are ephemeral
system tasks and are not persisted (`agent-manager.ts` lines
4263 to 4265). The generators that set `internal: true` are the
branch-name generator (`worktree-branch-name-generator.ts` lines
126 to 129) and the git-metadata generator
(`git-metadata-generator.ts` lines 224 to 227). Both reach
`createAgent` through `generateStructuredAgentResponse`
(`agent-response-loop.ts` line 359), so the skip inside
`createAgentInternal` covers them. Vessel has no
`config.internal` and no generator session. The only callers of
`create_starting`, `create_as` and `create_core_as` in this tree
are the RPC, the import, the schedule, the fork and the side
question. A side question is not one of those generators. It is
a person's persisted session (ADR 0045). Paseo has no side
surface (that record). A new side is created with no origin, so
both hooks run. Reusing an existing side does not create
(`service/side_questions.rs` lines 71 to 73) and does not fire
them. A later generator session, if one is added, skips both
hooks for the reason Paseo skips `internal`: it is the node's
own ephemeral task, and a plugin must not rewrite it.

A fork fires both hooks. Vessel's fork is a new session on this
path. Paseo's provider `forkSession` was not read. A search of
the server files whose names contain `fork` found no
`createAgent(` call. If Paseo's fork does not go through
`createAgent`, Vessel's fork runs a hook Paseo's does not. The
reason to run it here is the one in `forking.rs`: a fork is a
session its caller creates, with the same refusals as any
session. Paseo's scheduler implementation was not read. Any
Paseo create that does call `createAgent` still passes the hook
above, unless the config is `internal`. Vessel's schedule run
calls `create_as`, so it fires.

`agent.create.before` receives a snapshot of the spec the caller
sent, after nothing else has been reserved: no session id yet
(Paseo's create-before has none either), no credential, no model
token. The handler may replace `title` and `harness`. `cwd`,
`place`, `profile` and `mode` are read-only; a result that changes
one is a refusal (`InvalidRequest`, below), and later handlers of
this hook do not run. Returning nothing keeps the request. There
is no deep merge.

The replaced spec is what the rest of `create_starting` receives.
Fields Paseo can change and this request does not have (model,
saved environment, MCP servers, system prompt, tool policy,
provider options) are absent from the type, not dropped on the
floor. Adding one later adds a field.

Order is the plugin id's UTF-8 bytes, then registration order.
Paseo sorts with `localeCompare` and no locale argument
(`runtime.ts` lines 569 to 571). That call follows the process
locale. Plugin ids match `^[a-z][a-z0-9-]*$`
(`packages/protocol/src/plugin-config.ts` line 3). Vessel uses
bytes so the order does not follow a locale. Whether those bytes
sort as `localeCompare` does for every id the schema accepts was
not measured. The difference is the stable order, not a claim
that the two orders match.

A waiting hook has 30 seconds, counted by the node around the
call, not by the registry sleeping. The deadline is the same
number as `REQUEST_TIMEOUT_MS` (`runtime.ts` line 41). On a
timeout the handler's cancellation token is cancelled, the
operation fails, and later handlers do not run. That token is
not a POSIX signal. A handler error also fails the operation
and stops later handlers. The two failures are not the same code.

A refusal uses `ErrorCode::InvalidRequest`. A refusal is a
handler that returns an error, or a result that changes `cwd`,
`place`, `profile` or `mode`. `InvalidRequest` is a malformed
request, or arguments that were invalid
(`crates/vessel-proto/src/error.rs` lines 17 to 18). A client
must not retry it. `Forbidden` means the caller lacks
the method's scope (lines 21 to 22). A plugin rule is not that
scope, and a client that treats `Forbidden` as a missing grant
would ask for a grant change. `Conflict` means the target is in
the wrong state (lines 27 to 28). No session exists yet, so
there is no such target.

A timeout is `ErrorCode::Unavailable`. That code's comment says
retrying later may work (lines 36 to 37). A hook that did not
answer is that case, not a conflict with state. Paseo's timeout
rejects with a string that names the plugin and the RPC type
(`runtime.ts` line 730). A waiting handler's throw is rethrown
(`lifecycle/index.ts` lines 313 to 317). Vessel does not copy
the handler's text onto the wire. The message names the plugin
id and the hook, and, when a read-only field changed, that
field's name. It does not include the spec's directory, an
environment value, a credential, or the handler's own text. A
handler's text can carry a path or a secret. No new `ErrorCode`
and no new `ErrorDetail`. The code tells a timeout from a
refusal. The message is for a person. A client cannot tell a
thrown refusal from a read-only field change without the
message; both refuse the create and neither is a retry. A later
additive detail can split those two.

`agent.create.after` runs only after the session exists. It is not
awaited by the RPC. Its error is logged with the plugin id and the
hook, and the session stays. The log line does not include the
spec. The test waits on a notification the handler completes,
not on a sleep. A handler still running at shutdown is cancelled through
the same token. Slice 1 starts no process, so shutdown does not
send a signal.

Handlers in this slice exist only under `cfg(test)`. A release
build's registry is empty and answers nothing. There is no
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

## Outside the node

Slice 1 reaches nothing outside the node. It starts no child, opens
no socket, and runs no script.

The helper a later slice may start is design A. What it can reach,
once that slice exists: the filesystem and the network of the
account the node runs as. A process boundary is not a sandbox.
Whose credentials: that same account. The node does not put a
token, a pairing link, a harness home, or any other secret on the
child's command line or into an environment the node adds.
Paseo's preparation copies `process.env` (`preparation.ts` lines
4 to 16). Vessel does not. That is a stated difference, and the
reason is that a secret must not ride along because the daemon
had it. What refuses the child: slice 1, by not having a spawn.
A later slice that does spawn refuses before the spawn when the
id or the path fails validation, and when the one settings key
that slice's own ADR names is absent. Absent means off. This
record adds no `node.settings` key and lists no other track's
key.

The helper, when it exists, is a child through `vessel-proc`: a
process group on Unix, a kill-on-close job object on Windows
(`docs/ARCHITECTURE.md`, the `vessel-proc` row). No signal, no
systemd unit, and no launchd job. Slice 1 has neither, on either
platform.

A fake child in a later test is a test-only feature a release
build cannot enable. It does not answer in a product build.

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
session file except for the two calls.

Costs. A Paseo plugin does not run. A screen, a theme, a slash
command, a provider and a usage source do not exist yet. Create
hooks do not see model, environment or MCP servers, so the
examples that set a Codex sandbox option or inject a server have
nothing to set. `agent.open.before` is not called, so a launch-only
environment cannot be added. An import and a rewind do not run
these hooks. A schedule, a fork and a new side question do. A
client tells a timeout (`Unavailable`) from a refusal
(`InvalidRequest`) by the code. It cannot tell a thrown refusal
from a read-only field change without the message. No
`ErrorDetail` is added for that split.

Effects on other work. The code commit inserts the two calls in
`create_starting` and adds a crate. It must not reorder the
checks that already run there. It must not log the spec. Reviews
of that commit gate the tests listed in the plan, not this record.

What this record does not decide: the helper's framing, the
install switch, whether the helper may be Node, the sandbox
question, and any contribution the app would draw. Each of those
is a later slice with its own review. No dependency paragraph is
owed until one of them wants a crate.
