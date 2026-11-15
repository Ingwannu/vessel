# What can host a Paseo plugin

The inventory is `docs/plans/plugins-inventory.md`. This note compares
three ways a Rust node and a GPUI app could host that behaviour. The
decision is ADR 0068. Nothing here adds a crate.

Paseo already splits the problem in two, and the split is the
constraint:

- The server entry is a function. It registers hooks, RPCs, a
  provider and a usage source, and it may use Node: files, processes,
  the network, and the whole daemon SDK.
- The client entry is a React function. It returns components. The
  app evaluates the bundle and draws those components with the app's
  own React Native kit.

Vessel's app draws with GPUI and has no web view (ADR 0035). A React
component tree is not something that app can mount. That is true for
every design below, so "run Paseo's plugins" cannot mean "draw
Paseo's screens" without abandoning the native app.

## What already exists, and why it is not the plugin layer

| Asset | What it is | Why it is not this |
| --- | --- | --- |
| Automation hooks (`vessel-proto` `HookId`, protocol 11) | an HTTP callback the node signs and delivers | a different product: one URL, one secret, no request rewrite, no UI. Extending it into a plugin host would overload a wire type that already has peers |
| `session.create` (`vessel-node` `service/sessions.rs`) | the place a create hook would stand | a call site, not a host. The spec it takes is a harness, a directory, a profile, a title, a place and a mode, not Paseo's provider options |
| The lesson in `docs/LESSONS.md` | "out-of-process plugins over versioned JSON-RPC; `subject.operation.before/after` hook names" | the direction already chosen, with no contract yet |
| GPUI kit / view / link / app | screens drawn from typed models | can draw a contribution that is data (a title, a command, a color token). Cannot evaluate a component |

No workspace crate embeds a JavaScript engine or a WebAssembly
runtime. Checked by searching the workspace manifests for those
names; none are dependencies. Adopting one is a new dependency and
would need its own paragraph (licence, size, why the standard
library does not do it). None of the three designs below is allowed
to add that dependency inside the first slice.

## Design A. A helper process, and a versioned protocol

The node speaks typed messages to a child: the hook request, the
hook result, later a tool call, an RPC, a provider event. The
child is a program the node starts. Paseo is already this for
installed plugins (`runtime.ts` forks a Node worker; the message
union is `PluginProcessRequest`). Built-ins are the same messages
without the fork (`InternalPluginChild`).

**Security.** A process boundary is not a sandbox. What the child
can reach: the filesystem and the network of the account the node
runs as. Whose credentials: that account's, and no others. Paseo's
install warning says the same trust model. Vessel refuses to start
the child while the enable key is absent. That key is one
top-level `node.settings` field, named in the ADR of the slice
that adds it, not in this note. Absent means off. The node does
not put a secret on the child's command line or into an
environment it adds. Paseo's preparation copies `process.env`
(`preparation.ts` lines 4 to 16). Vessel does not, because a
secret the daemon already holds must not be handed over for that
reason alone. A bad id or a bad path is a typed refusal before
the spawn. The child is started through `vessel-proc` (a process
group on Unix, a kill-on-close job object on Windows,
`docs/ARCHITECTURE.md`), not by a signal, systemd, or launchd.
Slice 1 starts no child. A fake child answers only in a test
build. The child can still do anything that account can. Calling
that a sandbox would be false.

**Compatibility with existing Paseo plugins.** None, until a child
exists that evaluates Paseo's server bundle. That child is Node,
because the bundle is CommonJS, `require`s `node:` modules, and is
evaluated with `globalThis.eval`. Writing that child is a
compatibility adapter on top of the protocol, not the protocol
itself. It runs the server entry only. The client entry still does
not draw. Built-ins that import `@getpaseo/plugin/server` would
need the same adapter. A plugin that patches daemon config through
`PaseoApi.config.patch` would need a narrowed API or a deliberate
decision to expose it; handing the child the node's full client
would let it call whatever the node lets that credential call.

**Cost.** The protocol is types and a framed stdio codec. A fake
child in tests is test-only and does not answer in a release
build. No real Node and no network in those tests. No new crate
from outside in the first slice. The Node adapter, if it is ever
built, is a maintained JavaScript program and a Node install on
the machine, which the node does not have today. A wire call
takes the number given at merge. It is not chosen here.

**What the app can draw.** Only a contribution that arrived as
data: a screen id and a title, a command name, a slash command, a
theme's color strings mapped onto kit tokens, a timeline item whose
kind the app has a renderer for. It cannot draw `addScreen`'s
`Component`, a popover, a file panel's body, or a timeline
renderer. Those stay impossible until someone designs a data UI
the kit can render, which is a different and later feature, not a
matter of which process the JavaScript ran in.

## Design B. WebAssembly components

A plugin is a Wasm component. The node loads it with a runtime and
calls a WIT interface: a hook in, a hook result out, later a
provider poll. Capabilities (files, clocks, sockets) are imports
the node grants, not ambient authority.

**Security.** This is the only one of the three that can refuse a
plugin the account's files without trusting the plugin's author.
That is stricter than Paseo, and it is a real advantage. It is also
easy to ruin: granting the component the whole filesystem, or letting
it spawn the account's `node`, is design A with extra steps. The
runtime crate contains `unsafe`; Vessel's own code would not, but
the dependency still has to be named, sized and licensed before it
is added. No such crate is added here, and its size was not
measured.

**Compatibility.** None with a Paseo plugin. Those plugins are
TypeScript compiled to CommonJS. Nobody ships a Wasm build of
`@getpaseo/plugin`, and the SDK's `PaseoApi` is a Node client, not
a WIT interface. Requiring existing authors to rewrite is a new
plugin language that happens to use the same hook names. A
JavaScript-in-Wasm embedding (a JS engine compiled to Wasm, then
loading the bundle) inherits the engine's gaps and the size of
both runtimes, and still cannot draw React.

**Cost.** A WIT document, a component runtime, and a decision about
which capabilities a hook may ask for. The component model is still
moving; pinning a version is part of the cost. Testing needs a
fixture component compiled in the gate, which is a toolchain the
repository does not have. GPUI gains nothing: the component returns
bytes, and the app draws those bytes only if they are the data
contributions of design A.

**What the app can draw.** The same data contributions as design A.
A component that draws its own pixels would be a second UI toolkit
inside the window, which ADR 0035's "no web view" was meant to
avoid, and a canvas import would be that. Not proposed.

## Design C. An embedded JavaScript runtime, in the node

Link a JS engine into `vesseld` and evaluate the server bundle
there, the way Paseo evaluates a built-in in process. The hope is
that Paseo's own plugins would run.

**Security.** Worse than design A if the engine shares the
process: a plugin fault is the node's fault, which Vessel's
session rule forbids (a misbehaving session must not take the node
down). Isolating the engine in a child is design A plus an engine.
Paseo's `eval` of a trusted bundle is the model, including its
lack of a sandbox. An embedded engine does not add a sandbox unless
its capability API is used, and Paseo's bundles do not target that
API. They call Node.

**Compatibility.** This is the design that sounds like it runs
Paseo's plugins, and it does not.

- The server bundle's platform is Node (`compiler-build.ts` line
  358). It `require`s modules the host does not substitute
  (`bundle-evaluator.ts` lines 13 to 27 fall through to Node's
  `require`). An engine that is not Node fails on the first
  `node:fs` or `node:child_process` import. The engines that are
  Node are Node, which is a child process, which is design A.
- Built-ins are the same bundles. `worktree-include` and the usage
  sources would not run either.
- The client bundle is React. Embedding an engine in the node does
  not put a React reconciler in GPUI. Embedding one in the app
  would mean a second UI runtime beside GPUI, or a web view, which
  ADR 0035 rejected.
- `PaseoApi` talks to a Paseo daemon protocol. Pointing it at
  Vessel's protocol is a translation layer, not an import.

So "an embedded runtime that could run Paseo's own plugins" is, for
the server entry, "ship Node". For the client entry, it is "ship
React Native". Both contradict the native app, and the first is
design A with the child fixed as Node.

**Cost.** A large native dependency, its licence and its size, plus
either a Node compatibility shim that never finishes, or an actual
Node binary. The gate would need to run JavaScript, which today's
fakes do not. A crash in the engine is a crash in `vesseld` unless
the engine is out of process.

**What the app can draw.** Still not the component. The engine
would have to return data. If it returns a component, the app has
nowhere to put it.

## How the three sit next to each other

| | A. Helper process | B. Wasm component | C. Embedded JS |
| --- | --- | --- | --- |
| Runs a Paseo server bundle unchanged | only if the helper is Node | no | only if the engine is Node, which is A |
| Runs a Paseo client bundle | no | no | no, not in GPUI |
| Sandbox | no, unless a later design adds one | yes, if capabilities stay narrow | no |
| New dependency in the first slice | none | a runtime, not measured, not added | an engine, not measured, not added |
| Matches `docs/LESSONS.md` | yes | no | no |
| What the app draws | data contributions, later | the same | the same |

The recommendation is A, and the first slice is not yet the helper.
ADR 0068 says why, and what that slice is allowed to contain.
