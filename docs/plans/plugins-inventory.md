# Paseo plugins, as of 0.15.4

Read of the Paseo 0.15.4 tree (`package.json` version `0.15.4`,
`@getpaseo/plugin` the same). Paths below are relative to that tree.
This is the inventory the plugin decision is made from. It is not a
port, and it does not copy Paseo's code.

What was read: the plugin documents (`docs/plugins.md`,
`public-docs/plugins/hooks.md`, `public-docs/plugins/index.md`,
`public-docs/plugins/providers.md`), the SDK contracts under
`packages/plugin/src`, the daemon host under
`packages/server/src/server/plugins/`, the protocol plugin modules, the
CLI command, and the call sites named below. What was not read line by
line is listed at the end. A line number is the line in that file in
this tree; a later Paseo revision can move it.

## What a plugin is

A plugin is trusted, unsandboxed code installed into one daemon
(`docs/plugins.md` lines 1 to 6, `public-docs/plugins/index.md` lines
155 to 157). Server code and preparation commands run as the daemon
user on the daemon machine. Client code runs inside every connected
app, including phones. The trust decision covers the codebase, its
dependencies and its later updates.

Two processes, one protocol between them:

- An installed plugin's server bundle is a forked Node child.
  `packages/server/src/server/plugins/runtime.ts` lines 237 to 240
  fork the worker with `stdio: ["ignore", "pipe", "pipe", "ipc"]`.
  The parent speaks `PluginProcessRequest` /
  `PluginProcessMessage` (`plugin-process-protocol.ts` lines 36 to
  105).
- A built-in uses the same messages inside the daemon process.
  `InternalPluginChild` (`internal-child.ts` lines 7 to 39) copies
  each message with `structuredClone` and delivers it on a
  microtask. `docs/plugins.md` line 115: built-ins run in process,
  stay active when `pluginsEnabled` is not true, are absent from the
  installed list, and still publish client bundles. Editing one in
  development needs a daemon restart. An install cannot take a
  built-in id.

The bundle is CommonJS (`compiler-build.ts` lines 353 to 358:
`format: "cjs"`, `platform: "node"` for the server target and
`"neutral"` for the client target). The daemon evaluates the server
bundle with `globalThis.eval` and a `require` that hands back the
host SDK and `zod`, and Node's own `require` for anything else
(`bundle-evaluator.ts` lines 13 to 41). Preparation commands from
the manifest's `build` array run with the daemon's environment
copied in (`preparation.ts` lines 4 to 16).

`pluginsEnabled` defaults to false (`packages/server/src/server/config.ts`
line 683, `pluginsEnabled: persisted.pluginsEnabled ?? false`). While
it is false, configured plugins are not started
(`packages/server/src/server/plugins/index.ts` lines 222 to 232).
Turning it on is a person's explicit act (`docs/plugins.md` lines 81
to 87).

## Manifest

`paseo-plugin.json`, read by `readPluginManifest`
(`packages/server/src/server/plugins/manifest.ts` lines 7 to 51).

| Field | Rule | Lines |
| --- | --- | --- |
| `id` | `PluginIdSchema`: `^[a-z][a-z0-9-]*$` | `packages/protocol/src/plugin-config.ts` line 3; manifest line 29 |
| `name`, `description` | optional trimmed non-empty strings | manifest lines 30 to 31 |
| `icon` | optional relative path ending in `.png` | lines 32 to 35 |
| `media` | optional relative paths or `https` URLs | line 36 |
| `requirements` | optional `{ paseo?: string }`, strict, so a misspelled key fails | lines 37 to 38; schema `plugin-config.ts` lines 4 to 6 |
| `build` | optional list of argv arrays, each argument non-empty | lines 8 to 10 and 39 |

Asset paths stay inside the package: no absolute path, no `..`, no
trailing slash (`manifest.ts` lines 12 to 27). `requirements.paseo`
is an npm semver range (`plugin-requirements.ts` lines 6 to 14). A
manifest with no range is treated as `<0.8.0`
(`plugin-requirements.ts` lines 25 to 26). The runtime id is the
config key; `--id` overrides the manifest id at install; the daemon
does not compare the two (`docs/plugins.md`, the paragraph after the
manifest example).

Entries: `index.server.ts` and `index.client.tsx`, each
default-exporting one function that registers and returns a cleanup
(`public-docs/plugins/index.md`, the scaffold). Either entry may be
omitted. Code under `client/` compiles only into the app bundle,
`server/` only into the daemon bundle, `shared/` into both. A
`node:` import from client code is a compile error.

## Where an install comes from

`parsePluginSourceReference`
(`packages/protocol/src/plugin-source-reference.ts` lines 8 to 20):

| Spelling | Kind |
| --- | --- |
| An absolute or relative directory, or `~` | `directory` |
| `owner/slug`, or `host/owner/slug` | registry (`plugin-registry.ts` `parsePluginRegistryReference`, lines 88 to 102; default registry host is `plugins.paseo.sh`) |
| `git:`, `github:`, a Git URL, `npm:` | `managed` |

A `:plugin/path` suffix selects a subdirectory
(`plugin-source-reference.ts` lines 22 to 55). The path must be
relative and portable: no `..`, no drive letter, no second colon.

The registry artifact is either an npm package pinned by version,
resolved URL and `sha512` integrity, or a Git remote pinned by a
40-to-64 hex commit (`plugin-registry.ts` lines 14 to 32). Listing
text may carry translations; English `name` and `description` stay
required (lines 39 to 47).

Config stores only directory sources on the wire schema
(`plugin-config.ts` lines 8 to 18). Managed installs are a later
shape in the installation record the CLI prints
(`formatPluginIdentity`, `plugin-source-reference.ts` lines 74 to
83). This inventory does not restate every field of that record.

## Hooks

Registered with `server.addHook(name, handler)`
(`packages/plugin/src/server/lifecycle.ts` lines 153 to 172). The
name is `subject.operation.before` or `subject.operation.after`.
Unknown names throw at registration (`createLifecycleRegistration`,
lines 216 to 236). Every handler is `(payload, context) => Promise`.
`context` is `{ paseo, signal }` (lines 11 to 14): the daemon SDK,
and an `AbortSignal` aborted on the 30 second deadline or on plugin
stop.

The wait bit is data, not a guess (`pluginHookRegistry`, lines 174
to 190):

| Hook | Waits | Payload | What a handler may change |
| --- | --- | --- | --- |
| `workspace.create.before` | yes | `{ request }` | the whole explicit create request |
| `workspace.create.after` | no | `{ workspace }` | nothing |
| `workspace.archive.after` | no | `{ workspace }` | nothing |
| `worktree.create.before` | yes | `{ request }` | every field except `repoRoot` and `sourceCheckout` |
| `worktree.create.after` | yes | `{ worktree }` | files in the new worktree; a throw aborts and removes it |
| `agent.create.before` | yes | `{ request: { config, env? } }` | saved config except `config.cwd`; `env` for the first launch only, not saved |
| `agent.create.after` | no | `{ agent }` | nothing |
| `agent.open.before` | yes | `{ request }` | `env`, `tools`, `mcpServers` only |
| `agent.close.after` | no | `{ agent }` | nothing |
| `agent.archive.after` | no | `{ agent, archivedAt }` | nothing |
| `agent.turn.before` | no | `{ agent, turnId }` | nothing |
| `agent.turn.after` | no | `{ agent, turnId, outcome, timeline }` | nothing; `timeline` is always `[]` and deprecated (lines 99 to 107) |
| `agent.permission.before` | no | `{ agent, request }` | nothing; the handler answers through the SDK |
| `agent.permission.after` | no | `{ agent, requestId, resolution }` | nothing |

Waiting hooks: the daemon awaits them. A throw, a value that fails
the schema, or 30 seconds fails the operation with
`Plugin <id> <hook> failed: …`. Later handlers of that hook do not
run. `undefined` keeps the request. There is no deep merge
(`public-docs/plugins/hooks.md`, "Ordering and returned values").

Notifications: the daemon does not await them. A throw is logged
against the plugin and the operation continues. Different events may
overlap. Delivery is live and best effort: no replay, no
persistence, no retry (same document, "Context and cleanup").

Order across plugins is `String.prototype.localeCompare` on the
plugin id (`runtime.ts` lines 549 to 550 and 569 to 571). Order
inside one plugin is registration order. `env` on
`agent.open.before` accumulates across plugins. `tools` and
`mcpServers` start empty for each plugin, accumulate only inside
that plugin, and are merged afterwards in plugin-id order. A name
that collides with a saved server, the internal `paseo` server, or
another plugin fails the open and names both owners. A tool name
must match `/^[a-z][a-z0-9_]*$/` and be at most 64 characters
(`agent-tools.ts` lines 25 to 32). Opens with `purpose: "history"`
drop tools and MCP servers (`lifecycle/index.ts` lines 350 to 353).

`agent.open.before` read-only fields, checked by deep equality
(`lifecycle/index.ts` lines 244 to 254): `agentId`, `workspaceId`,
`provider`, `cwd`, `reason`, `purpose`, `title`, `labels`.
`reason` is `create`, `resume`, `refresh` or `import`. `purpose` is
`interactive` or `history` (`lifecycle.ts` lines 73 to 85).
`agent.create.before` cannot change `config.cwd` (`lifecycle/index.ts`
lines 260 to 261). `worktree.create.before` cannot change `repoRoot`
or `sourceCheckout` (lines 267 to 268). A returned worktree path is
resolved and refused when it is not absolute, already exists, or is
the filesystem root, the home directory, the repository, inside
`.git`, or an ancestor of the repository
(`public-docs/plugins/hooks.md`, the path paragraph; the daemon
calls `validateWorktreePath` at `runtime.ts` lines 596 to 601). The
plugin must not create the directory.

Timeouts (`runtime.ts` lines 41 to 46): a hook request is 30
seconds; a tool call is 10 minutes; the log keeps 500 entries, 256
KiB, 16 KiB a line; soft shutdown is 2 seconds.

`worktree.create.after` is the one notification that waits
(`pluginHookRegistry` line 179, `runtime.ts` `wait` at lines 546 to
564). It runs after `git worktree add` and before workspace
registration. Restoring a missing worktree does not fire either
worktree hook.

Agent hooks skip the daemon's private disposable helpers. The create
path checks `!config.internal` (`agent-manager.ts` lines 1363 to
1364). `agent.create.after` is not fired for import or resume
(`public-docs/plugins/hooks.md`, that hook's paragraph). Turn hooks
are not fired for history replay. `publishAgentStream` maps
`turn_started`, `turn_completed`, `turn_failed`, `turn_canceled`,
`permission_requested` and `permission_resolved`, and skips
`timeline` (`lifecycle/index.ts` lines 190 to 214; the call is
`agent-manager.ts` lines 5394 to 5396).

Call sites read:

| Hook | Where the daemon fires it |
| --- | --- |
| `workspace.create.before` | `packages/server/src/server/session.ts` line 6872 |
| `workspace.create.after` | `session/workspace-provisioning/workspace-provisioning-service.ts` line 310 |
| `workspace.archive.after` | `bootstrap.ts` line 974 |
| `worktree.create.before` | `worktree-core.ts` line 138 |
| `worktree.create.after` | `paseo-worktree-service.ts` line 83 |
| `agent.create.before` / `agent.create.after` | `agent/agent-manager.ts` lines 1364 and 1409 |
| `agent.open.before` | `agent/agent-manager.ts` line 5601 |
| `agent.close.after` | `agent/agent-manager.ts` line 4099 |
| `agent.archive.after` | `agent/agent-manager.ts` line 2125 |
| turn and permission | `publishAgentStream`, called from `agent-manager.ts` line 5396 |

Creation order (`public-docs/plugins/hooks.md`, "Ordering and
returned values"): `agent.create.before`, then defaults and provider
validation, then launch configuration, then `agent.open.before`,
then `PASEO_AGENT_ID` and `PASEO_AGENT_CWD` are set, then the
provider session opens and the configuration is saved.
`worktree.create.before`, then `git worktree add`, then
`worktree.create.after`, then workspace registration, and that
before the first agent's create and open hooks.

The names before 0.14 still register, mapped onto the new names, and
are scheduled for removal after 2027-04-30 (`lifecycle.ts` lines 191
to 206; the table in the hooks document, "Renamed hooks").
`addHook` accepts only the new names. Mixed old and new
registrations in one plugin run in registration order.

## What the server entry can call

`PluginServerContext` (`packages/plugin/src/server/contracts.ts`
lines 30 to 43), besides the hook registration:

| Call | What it does | Lines |
| --- | --- | --- |
| `registerSettings(definition)` | host-scoped document, versioned, with an optional migrate | contracts 31 to 33; `defineSettings` in `settings.ts` lines 14 to 23: id `^[a-z][a-z0-9_-]*$`, scope must be `"host"`, version an integer ≥ 1 |
| `handle(contract, handler)` | one RPC method; input and output are Zod schemas | contracts 34 to 40; `defineRpc` in `rpc.ts` lines 21 to 29: name `^[a-z][a-z0-9._-]*$` |
| `registerProvider(provider)` | one coding agent | contracts line 41; shape below |
| `registerUsageSource(source)` | one account-usage reader | contracts line 42; `usage.ts` lines 78 to 87 |

`spawnProcess`, `execCommand` and `terminateProcess`
(`packages/plugin/src/server/process.ts` lines 48 to 101) are how a
provider starts a CLI. They exist so Windows `.cmd` / `.bat`
launchers work; the document tells plugins not to call Node's
`spawn` for those.

`defineAgentTool` attaches a tool whose `execute` stays in the
plugin process (`agent-tools.ts` lines 5 to 36). The daemon sees
metadata and JSON input and result. Invalid input is returned to the
agent as schema issues. A throw becomes `{ isError: true, text }`
and the plugin keeps running. Cancel aborts `signal` and drops the
result.

The `paseo` value on a hook, a tool and an RPC handler is `PaseoApi`
(`packages/client/src/index.ts` lines 499 to 509):

| Member | What a plugin can do with it | Lines |
| --- | --- | --- |
| `hosts` | host actions | 500 |
| `dispose` | release the API | 501 |
| `observeEvents` | the low-level daemon event stream | 502 |
| `terminals` | list, create, write, capture | 503; actions in `packages/client/src/terminals/index.ts` |
| `workspaces` | list, open, create, archive, subscribe; a handle creates agents and terminals | 504; `PaseoWorkspaceActions` lines 207 to 230, handle lines 179 to 200 |
| `projects` | list, subscribe | 505; lines 202 to 205 |
| `agents` | list, `ref`, create, subscribe; a handle sends, runs, answers a permission, reloads, archives, reads the timeline | 506; actions lines 397 to 412, handle lines 348 to 395 |
| `providers` | models, modes, features, availability, snapshot, usage, diagnostic | 507; lines 453 to 477 |
| `config` | `get` and `patch` of the daemon config document | 508; lines 479 to 497 |

That is the whole daemon SDK, not a narrowed plugin API. A hook that
sends a follow-up or answers a permission does it through
`agents.ref` (`public-docs/plugins/hooks.md`, the turn-after and
permission-before examples). `config.patch` writes daemon
configuration.

## Provider registration

`PROVIDER_PROTOCOL_VERSION` is 1 (`provider.ts` line 4).
Capabilities (lines 6 to 25): `prompt.message`, `prompt.command`,
`prompt.image`, `prompt.output_schema`, `prompt.steer`,
`session.can_stop_runtime`, `session.archive`, `session.configure`,
`session.list`, `session.persistence`, `session.revert.both`,
`session.revert.conversation`, `session.revert.files`,
`session.subsession`, `session.unarchive`, `permission`,
`permission.tool_policy`, `timeline.plugin`.

`ProviderRegistration` (lines 29 to 43): optional
`requiresConfiguration` (the provider runs only while
`agents.providers` contains its id), optional `getCatalogCacheKey`,
optional default `command`, optional `status`, required `id`,
`label`, optional `description`, optional `icon` (a relative SVG),
required `connect`. `connect` returns `{ version, capabilities,
send, onEvent, close }` (lines 78 to 84).

Inputs the daemon sends (`ProviderInputSchema` from line 895):
`catalog`, `sessions`, `session.open`, `session.prompt`,
`session.interrupt`, `session.permission`, `session.configure`,
`session.revert`, `session.archive`, `session.unarchive`,
`session.can_stop_runtime`, `session.close`.

Events the plugin sends (`ProviderEventSchema` from line 1296):
`catalog`, `sessions`, `request.completed`, `request.failed`,
`session.opened`, `session.ready`, `session.can_stop_runtime`,
`session.closed`, `session.runtime_failed`, `session.persistence`,
`session.prompt_result`, `session.turn` (with `turn`, `steer`,
`completed`, `failed`), `session.usage`, `session.config`,
`session.commands`, `session.permission`,
`session.permission_resolved`, `session.notice`, `timeline.item`.

`runAcpProvider` (`packages/plugin/src/server/acp.ts` line 106)
wraps an ACP command as a `ProviderRegistration`. A transformer may
rewrite vendor updates (`AcpTransformer`, lines 93 to 104). The
field list of each input and event was not copied into this note;
the schemas are the contract.

A provider id that collides with a built-in is refused
(`packages/server/src/server/plugins/index.ts` line 675).

## Usage sources

`UsageSourceRegistration` (`usage.ts` lines 78 to 87): `id`,
`label`, optional `icon`, a Zod `input`, `discover(scope)` and
`fetch(input)`. `discover` only reads. `fetch` re-reads the login
store and never writes it. `UsageScope` is a discriminated union
(`usage.ts` lines 67 to 76); the variants were not restated here.
The built-in sources are the `*-usage-source` directories under
`plugins/` (the list is `builtin/index.ts` lines 5 to 19).

## What the client entry can register

`PluginClientContext` (`packages/plugin/src/client/contracts.ts`
lines 176 to 206). Each call returns a cleanup, except the buttons,
which return `{ update, remove }` (`buttons.ts` lines 41 to 45).

| Call | Contribution | Lines |
| --- | --- | --- |
| `addScreen` | a full screen; title fixed or derived from params | 181; shape 242 to 246 |
| `addSurface` | deprecated alias of a screen, removal after 2027-03-29 | 171 |
| `addSidebarHeaderItem` | a sidebar row that opens a screen | 182; shape 86 to 95 |
| `addSidebarFooterItem` | the same, at the foot | 183 |
| `addSidebarItem` | deprecated; header item bound to a surface id | 173; shape 254 to 259 |
| `addSettingsScreen` | a settings page | 180; shape 230 to 236 |
| `addWorkspacePanel` | a tab beside agents, terminals and files | 185; shape 210 to 222 |
| `addAgentPanel` | the same, bound to an agent | 184; shape 224 to 228 |
| `addFilePanel` | a viewer for extensions and exact filenames | 198; shape 154 to 161 |
| `addCommandCenterItem` | global, workspace or agent; `onSelect` | 186 to 188; shape 330 to 348 |
| `addSlashCommand` | workspace or agent; `onSubmit` receives `args` | 189; shape 350 to 358 |
| `addHeaderButton` | a button on one workspace; action, menu or popover | 190 to 192; `buttons.ts` 47 to 51 and 16 to 18 |
| `addComposerPill` | the same on one agent | 193 to 195; `buttons.ts` 52 to 56 |
| `addAttachmentSource` | a picker backed by an RPC search | 196; `contracts.ts` of the shared module, lines 75 to 82 |
| `addTheme` | `light` or `dark`, plus eight color strings | 197; shared `contracts.ts` lines 68 to 73 |
| `addTimelineTransformer` | rewrite one timeline item type while streaming or complete | 199 to 201; shape 261 to 274 |
| `addTimelineRenderer` | draw a `plugin` timeline item of one kind and version | 202 to 204; shape 287 to 292 |
| `openPanel` | open one of this plugin's panels | 205 |
| `playAudio` | play a base64 audio buffer on this client | 178 to 179 |

Command contexts carry `paseo`, `rpc`, `openScreen`, `openSettings`
(lines 294 to 328). Screens and panels receive host props: theme
colors (`surface0` through `statusDanger`, shared `contracts.ts`
lines 6 to 20), layout, and navigation.

Hooks a component reads, not registers:

| Hook | What it reads | Lines |
| --- | --- | --- |
| `usePaseo` | the same `PaseoApi` | `client/index.ts` line 72; `paseo-context.tsx` lines 14 to 18 |
| `useWorkspace`, `useAgent` | a selection of the cached snapshot; unrelated fields do not re-render | `client/index.ts` line 73; `client-state.tsx` lines 34 and 45 |
| `useRpc` | one RPC contract | `client/index.ts` line 74 |
| `useSettings` | the settings document; `save` never throws | `client/index.ts` lines 78 to 80; state shape `contracts.ts` lines 360 to 369 |
| `useHosts` | configured hosts, including disconnected ones | `client/index.ts` lines 83 to 89 |
| `getPaseoClient(serverId)` | another online host's API, for this install's lifetime | line 91 |
| `openExternalUrl` | an absolute `http` or `https` URL | line 93 |
| `useTranslation` | the plugin's own message catalog | line 95 |
| `useTabState` | JSON state kept on a panel tab | line 99 |
| `useFileText`, `useFileBytes`, `useFileUrl` | a file the host handed the panel | `client/file.ts` lines 82 to 100 |

The UI kit the component is allowed to import
(`packages/plugin/src/client/ui.ts`): `Icon`, `Modal`, `useToast`,
`StreamedText`, `ScrollView`, `FlatList`, `copyText`, `TextInput`,
`SettingsGroup`, `SettingsSection`, `SettingsCard`, `SettingsRow`,
`SettingsSwitch`, `SettingsSelect`, `SettingsInput`,
`SettingsAction`, `ExternalLink`, `SidebarRow`, `SidebarSeparator`,
`Loading`, `DocumentFrame`, and a `Markdown` props type (line 182).
These are React components. The host fills them in; the plugin
bundle does not ship React.

An SVG icon is at most 64 KiB (`icons.ts` line 15). Overview
markdown from the registry is a fixed element allow-list, no raw
HTML, and links are `https` only (`plugin-overview.ts` lines 2 to
39).

## Daemon RPC the app uses to manage plugins

From `packages/protocol/src/messages.ts`. Each is a request type
string, not a Vessel protocol number.

| Request | Line |
| --- | --- |
| `plugin.catalog.get` | 1490 |
| `plugin.list` | 1499 |
| `plugin.logs.get` | 1504 |
| `plugin.directory.install` | 1510 |
| `plugin.directory.inspect` | 1517 |
| `plugin.source.install` | 1523 |
| `plugin.source.update.preview` | 1578 |
| `plugin.source.update.apply` | 1584 |
| `plugin.source.status` | 1590 |
| `plugin.source.update` | 1596 |
| `plugin.reload` / `enable` / `disable` / `remove` | 1605 to 1608 |
| `plugin.rpc.invoke` | 1611 |

Status events: `status.plugin_catalog_changed`,
`status.plugin_settings_changed` (lines 3244 to 3245). Hello
capabilities gate each generation (`plugins`, `pluginManagement`,
`pluginLogs`, `pluginGitManagement`, `pluginSourceInstallation`,
`pluginSourceUpdates`, `pluginThemes`, `pluginSettings`,
`pluginTimelineItems`, lines 3734 to 3751). The catalog response
can omit a client bundle the client already holds, keyed by hash
(line 6835).

The child protocol, which is not the app's wire (`plugin-process-protocol.ts`
lines 36 to 105): parent to child `tool.call`, `tool.cancel`,
`tools.detach`, `provider.status`, `initialize`,
`provider.catalog_key`, `hook`, `hook.cancel`, `usage.fetch`,
`usage.discover`, `invoke`, `provider.connect`, `provider.send`,
`provider.close`, `shutdown`, `paseo_frame`, `paseo_close`. Child
to parent: `ready` (methods, providers, usage sources, hook names),
`result`, `error`, `fatal`, `settings.changed`, `hooks.changed`,
and the provider and frame replies.

## CLI

`paseo plugin` (`packages/cli/src/commands/plugin/index.ts` lines
212 to 268): `init`, `ls` (hidden `status`), `logs`, `install`
(alias `add`), `update`, `reload`, `enable`, `disable`, `remove`.
`install` prints the trust warning on stderr before it does
anything (lines 144 to 148). `init` writes a project and does not
run a package manager (`public-docs/plugins/index.md`).

## Built-ins and examples

Built-ins (`builtin/index.ts` lines 5 to 19): `antigravity-provider`,
`claude-usage-source`, `codex-usage-source`, `copilot-usage-source`,
`cursor-usage-source`, `grok-usage-source`, `hoplite-provider`,
`kimi-usage-source`, `minimax-usage-source`, `muse-provider`,
`opencode-go-usage-source`, `worktree-include`, `zai-usage-source`.
`worktree-include` copies files named by `.worktreeinclude` through
`worktree.create.after` (`docs/plugins.md` lines 103 to 105).
`muse-provider` registers provider id `muse` and a usage source
(lines 122 to 128).

Examples under `plugin-examples/`, named so the documents can be
checked, not as code to copy: `lifecycle-logger`,
`lifecycle-actions`, `agent-configuration`, `agent-tools`,
`worktree-placement`, `provider-direct`, `provider-acp-transformer`,
`settings`, `timeline-items`, `buttons`, `catppuccin` (a theme),
`hosts`, `translations`, `markdown-panel`, `pdf-panel`, `modal-ui`,
`inline-thinking`, `linear`, `local-plugin`.

## Not read line by line

The bodies of `session.ts` (the workspace create hook is one call
in a very large file), both provider schemas' fields, the ACP
transformer internals, the compiler's import fence beyond the
platform line, the app's client-bundle loader, and the e2e specs.
Paseo's scheduler implementation was not read, and the provider
`forkSession` body was not read. `createAgent` itself was read:
it always enters `createAgentInternal`, which skips
`config.internal` only (`agent-manager.ts` lines 1348 to 1370).
The generators that set that flag were read
(`worktree-branch-name-generator.ts` lines 126 to 129,
`git-metadata-generator.ts` lines 224 to 227). Those gaps are
named again in the plan.
