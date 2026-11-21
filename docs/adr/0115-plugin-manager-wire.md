# ADR 0115: Plugin manager wire and directory installs

Status: proposed (impl P, 2026-11-19; CATALOGUE F-M1..F-M39, TRACKER 696;
coordinator ruling 2026-11-19 approving the plan
`/tmp/work/m12-claude-wave/plugins/inventory-plan.md`). The coordinator
recorded 0115 as this ADR's number. The protocol, drafted 35,
was assigned 36 (`PLUGINS_PROTOCOL` in `vessel_proto::plugins`) at the
node-line merge, after ADR 0088's `forge.repos` took 35. Extends ADR 0079 (architecture, trust) and ADR 0068 (hook
names); neither is changed except where this record says so.

## Context

Paseo 0.15.4 lets a person install a plugin on a host, switch plugins on for
that host, enable, disable, reload and remove each one, read its logs, and see
it running or failed (`packages/server/src/server/plugins/index.ts:218-548`,
`packages/server/src/server/session.ts:2552-2714`,
`packages/app/src/screens/settings/plugins-page.tsx:266-588`,
`packages/cli/src/commands/plugin/index.ts:211-268`).

ADR 0079 fixed how Vessel hosts a plugin: a trusted, unsandboxed helper
process supervised by the node and speaking private versioned frames, and
declarative client data drawn by the app's own controls; never a React bundle,
never code evaluated in an app. The node line has those values and the
supervisor (`vessel_proto::plugins`, crate `vessel-plugin`, both reviewed), but
nothing uses them: no manifest, no install, no storage, no method, no CLI, no
page. This record fixes those, so the first packets make Paseo's plugin
management reachable.

## Decision

### Manifest: `vessel-plugin.json`

A plugin is a directory holding `vessel-plugin.json`:

| Field | Meaning (Paseo's field where one exists) |
|---|---|
| `id` | default installation id, Paseo's `^[a-z][a-z0-9-]*$` (`PluginId`) |
| `name?`, `description?` | listed on the wire (bounded `Caption` / `ContentText`); the page's rows show the id and the description, as Paseo 0.15.4's settings page (it never shows the name) |
| `icon?` | relative PNG path inside the directory (Paseo `icon`; listed, not drawn by Paseo's page) |
| `requirements?.vessel` | semver range the node's version must meet; unmet = refusal before anything runs (Paseo `requirements.paseo`) |
| `server?.command` | argv of the helper: a relative path that stays inside the directory after resolving links, a bare program name found on the node's `PATH`, or an absolute path. No shell |
| `client?` | relative path of a static `ClientDocument` JSON file (a client-only plugin, such as a theme) |
| `build?` | argv lists; accepted only for managed sources (P4), as Paseo runs `build` only for those |

At least one of `server` and `client` (Paseo: at least one entry). Unknown
fields are refused. A directory with `paseo-plugin.json` and no
`vessel-plugin.json` is refused with: "This is a Paseo plugin. Vessel runs
plugins described by vessel-plugin.json: a helper program and declarative
client data; Paseo's TypeScript entries do not run here." (stated difference).

### Storage

- The global switch is `NodeSettings.plugins_enabled` (Paseo `pluginsEnabled`,
  default false), changed with `node.set_settings` (Admin) and applied at once
  without a restart: on, every enabled installation starts; off, every one
  stops. Hidden from peers below the protocol.
- Installations are a node-owned file `<home>/plugins/installations.json`
  (`{id: {source: {kind: directory, path}, enabled}}`; a directory source is
  one absolute path, a subdirectory already named in it), owner-only
  (0600, directory 0700), written atomically by the plugin service alone. Not
  in `settings.json`: that file has its own writer, and two writers to one
  file is what ADR 0079's single namespace was meant to avoid; the namespace
  stays one service.
- Plugin settings (P3) under `<home>/plugin-settings/<id>/<settings>.json`.
- Logs are in memory per installation with Paseo's bounds (500 entries,
  256 KiB, 16 KiB a line; already `MAX_LOG_*`), kept across reload, disable and
  failure, cleared on remove, lost on node restart; `[vessel]` lines for
  loading, ready, stopping, stopped, as Paseo's `[paseo]` lines.
- Remove never deletes a user's directory; it deletes the installation entry,
  the plugin's settings directory and its logs.

### Runtime

- One helper per running installation, through `vessel-plugin::Supervisor`:
  cwd the plugin directory; the node's UID/GID; environment cleared to the
  platform variables the reviewed supervisor admits (`PATH`, `HOME`, `LANG`,
  `LC_ALL`, `LC_CTYPE`, and on Windows `TEMP`, `TMP`, `SYSTEMROOT`, `WINDIR`,
  `USERPROFILE`); no token, pairing link, harness home or secret. The
  installation's identity comes in `initialize` (its address), not in the
  environment (amended at implementation: the supervisor refuses any other
  name, so `VESSEL_PLUGIN_ID`/`_DIR` of the draft are not set). The names
  are matched without case (Windows' `Path`); `HOME` is the node user's home
  the node was configured with, never the node process's own value. Proven
  by `plugins::host::tests::no_credential_shaped_variable_reaches_a_helper`
  (the launch built from a node environment holding `ANTHROPIC_API_KEY`,
  `OPENAI_API_KEY`, `GH_TOKEN`, `GITHUB_TOKEN`, `VESSEL_TOKEN` and other
  names with TOKEN, KEY, SECRET, PASSWORD or AUTH keeps none of them) and
  `plugins::tests::a_real_helper_sees_no_credential_the_node_has` (a real
  helper prints its environment; none of those names or values is in it).
- Status `running | disabled | failed` (Paseo's three). Running = the helper
  answered `Ready` within the request deadline (30 s) with a valid document, or
  a client-only plugin's document validated. Failed carries a node-made
  sentence (manifest invalid, program not found, exited with code n, no ready
  within 30 s, protocol violation); helper output goes only to logs.
- install, enable and reload answer after the start settles: the installation,
  or, when the start fails, an `unavailable` error with the reason while the row
  stays failed (Paseo's `startExplicit` throws the same way; amended at
  implementation from "answers the failed row"). Reload stops the old helper first
  and may end failed, as Paseo. Disable, global off and remove stop the helper
  (soft shutdown 2 s, then the process group) before answering. At node start
  the configured helpers start in the background; `plugin.list` waits for that
  first settle (bounded by the deadline) so its first answer is authoritative.

### Wire (protocol 36)

| Method | Scope | Request -> reply |
|---|---|---|
| `plugin.list` | Read | `{}` -> `{plugins: [PluginListItem]}` |
| `plugin.logs` | Admin | `{plugin}` -> `{entries: [LogEntry]}` |
| `plugin.inspect` | Admin | `{path}` -> `{id, name?, description?}` |
| `plugin.install` | Admin | `{source, id?, ref?}` -> `PluginListItem` |
| `plugin.enable` / `plugin.disable` / `plugin.reload` / `plugin.remove` | Admin | `{plugin}` -> `PluginListItem` (remove -> `Ack`) |
| `plugin.watch` | Read | `{after: generation}` -> generation (a bare number) |

- `PluginListItem`: `id, name?, description?, icon?, path?, source, enabled,
  status, error?`. `path` and `error` only for an Admin caller (ADR 0079:
  scoped reads omit private paths and diagnostics).
- `source` in install is Paseo's source string. P1 takes a directory: a
  string starting with `/`, `~`, `./`, `../` or a drive, optionally
  `:<portable subpath>`; the node takes absolute or `~` paths (ADR 0113's
  expansion); the CLI makes a relative path absolute before sending, as
  Paseo's CLI resolves it. `git:`, `github:`, `npm:`, URLs and registry names
  answer `invalid_request` "this node installs plugins from directories only"
  until P4. `ref` is refused for directories (Paseo's message).
- Errors as the node's other calls: invalid id, reserved/unknown id, unmet
  requirement, Paseo manifest `invalid_request`; id already installed
  `conflict` ("choose another id with --id", Paseo's); disk failures
  `internal`. An agent principal is refused every Admin call (guard), as
  other node management.
- `plugin.watch` is a long poll like `fleet.watch` (no new frame type): it
  answers as soon as the plugin generation differs from `after` (any
  install, status, enable, catalog or settings change), at once when it
  already does (a client's first ask uses a number the node never has), or
  with the current one after its wait limit. It replaces Paseo's `plugin_catalog_changed` and
  `plugin_settings_changed` status pushes; clients refetch on a new
  generation.
- Older peers: every method is unregistered below the protocol, and the
  client and app say "Update this machine to manage plugins" (Paseo's
  daemon-too-old alert).

### CLI (P1b)

`vessel plugin ls [id] | install <source> [--id] [--ref] | enable | disable |
reload | remove | logs <id> | init <dir>`, each with `--json` and the usual
node selection, and `vessel node settings --plugins on|off` for the switch.
`init` writes a manifest, a client document with one theme and a README that
describes the helper frames; it downloads nothing. (Amended at
implementation: a helper example lands with plugin RPC in P3, when a helper
has calls to answer; a helper that only says ready teaches nothing.)

### Managed sources (P4, decided here, built later): security note

- Fetch only inside an Admin's explicit `plugin.install` or reviewed
  update apply; never when a page, a list or a form is viewed, never at node
  start, never on reconnect.
- git: one bounded child (`GIT_TERMINAL_PROMPT=0`, no credential helper
  added, 120 s), clone into `<home>/plugins/<id>/<uuid>/` via a private
  staging directory; the resolved commit is recorded and shown.
  npm: lifecycle scripts off, the resolved version, tarball and integrity
  recorded; the integrity is checked against the downloaded tarball.
  Registry: HTTPS only (loopback excepted), no redirects, 30 s, the
  configured credential sent only to its own host and never placed in a
  helper's environment; the registry answer is a pin (git commit or npm
  version + integrity) that is verified as above.
- `build` argv runs without a shell, in staging, with the helper's minimal
  environment and Paseo's 64 KiB output bound; any failure discards the
  candidate and the running version stays.
- Reviewed update: preview returns a proposal binding id, installed root,
  revision and target; apply rechecks that tuple and refuses a changed one.
- Tests use local bare git repositories, local tarballs and a loopback
  registry only; no test reaches the network.

### Catalog and invoke (2026-11-20, P3a; coordinator ruling 2026-11-20)

Protocol 41 (`PLUGIN_CATALOG_PROTOCOL`), assigned at its node-line merge, the next after the
workspace-folder terminals' 40.

| Method | Scope | Request -> reply |
|---|---|---|
| `plugin.catalog` | Read | `{}` -> `{generation, catalog: ClientCatalog, may_invoke}` |
| `plugin.invoke` | Admin | `{plugin, generation, method, input, context}` -> `{output}` |

- Purpose: Paseo's clients read a host's catalog (`plugin.catalog.get`) and
  call a plugin's methods (`plugin.rpc.invoke`); the page and the app need
  both so a machine's plugin themes, Command Center items and slash commands
  reach them.
- The catalog holds every running installation, and only those (stopped,
  failed, disabled and plugins-off installations are absent). A client-only
  installation's entry is the `client` document read at its start. A helper's
  entry is the document it declared in `ready` (or a later
  `client_changed`); a `client` file beside a helper is checked but not
  served. The reviewed `ClientCatalog` admits the set (owner, duplicates,
  128 installations, 256 themes, 4 MiB); a set over the budget is refused as
  a whole (`internal`) rather than trimmed. `required_client` is the catalog
  protocol. The first answer after node start waits for the start-up settle,
  as `plugin.list` does. `generation` is `plugin.watch`'s count, so a client
  watches from the catalog it holds and reads it again when the count moves;
  a helper that declares again moves the count.
- The client file is opened only after its real path (links resolved) is
  found inside the plugin directory's real path; a link that leads outside
  fails the start ("it leads outside the plugin directory"). Asset paths
  (`client`, `icon`) are at most 1 KiB.
- Scope (coordinator ruling): `plugin.catalog` needs `read`. This is a
  difference from Paseo's `daemon.manage`: Vessel has scopes Paseo does not,
  and a Read device must still draw a machine's themes and the shape of its
  commands. The catalog is declarative data with bounded strings, no paths
  and no code. `plugin.invoke` needs `admin` and is refused to agents: a
  method runs plugin code with the node's access. Clients show invoke-bound
  items only on devices holding `admin`: the reply's `may_invoke` says so,
  since a client does not otherwise know which scopes its credential
  holds.
- Invoke reaches only a running helper. A client-only installation answers
  `not_found` ("it has no helper program, so it has no methods"). A stopped
  or failed installation answers `unavailable`. As ADR 0079 asks of an
  action, a call carries the run (`generation`, from the catalog entry's
  address) and the context it was taken in (`global`; a `workspace`, which
  in Vessel is a project or one of its worktrees, as a session's
  `workspace` names it; or an `agent`, a session with its project and
  worktree); a call from a run that has since been
  replaced answers `conflict` ("read it again"). The helper receives
  `{input, context}`, the context objects Paseo's `onSelect` and `onSubmit`
  get. The node passes the context as the caller stated it and does not
  check that the project, worktree or session exists: the caller already
  holds `admin`, whose scopes are node-wide, so naming another project
  grants nothing the credential lacks. A plugin treats the context as
  caller input, as it treats `input`. A call is refused
  (`unavailable`) from the moment plugins are switched off or the
  installation is disabled, though its helper stops a moment later. The
  supervisor's actor checks
  that the running generation declared the method (`not_found` otherwise),
  admits the input within the helper payload budget (1 MiB, `invalid_request`
  over it), and keeps its own deadline. A helper's failure reaches the
  caller only as a class: refused `invalid_request`; callback failed
  `internal` ("its log may say why"); busy, restarting, stopping or timed out
  `unavailable`. The helper's diagnostic text is never in a reply (ADR 0079);
  what it writes to stderr is in its log.
- Text a caller sends to the manager is bounded before the plugin host sees
  it: a source or a path 4 KiB, a ref 256 bytes (`invalid_request`).
- Plugin settings are the next section; screens, panels, sidebar items,
  header buttons and composer pills are P3b.

### Settings (2026-11-20, P3s; coordinator ruling 2026-11-20)

Paseo 0.15.4 keeps host-scoped settings documents per installation:
`defineSettings({id, scope: "host", version, schema, migrate?})`
(packages/plugin/src/settings.ts:5-51) and its `PluginSettingsStore`
(packages/server/src/server/plugins/settings/index.ts:35-185), with the
author contract in public-docs/plugins/reference.md:1302-1435 and
docs/plugins.md:739-753.

- **The node owns the store and the helper owns the schema.**
  - The file is `plugin-settings/<installation id>/<settings id>.json` in the
    node's data directory: 0700 folders, a 0600 file, Paseo's envelope
    `{version, values}`, written to a temporary file and renamed.
  - The revision is the sha256 of the stored bytes in lowercase hex, or
    `missing` (Paseo's).
  - One lock per installation orders reads that migrate, writes, resets and
    the notices to the helper.
  - **Stated difference from Paseo:** Paseo's store lives in the plugin
    subprocess; here the helper never writes its own settings files, so its
    file access stays narrow and the guarantees (atomic, owner-only,
    revisioned) do not depend on each helper's language. ADR 0079 records
    it.
- **Helper protocol.**
  - `ready` may declare `settings: [{id, version}]`: at most 16, ids unique,
    an id `^[a-z][a-z0-9_-]{0,63}$`, a version from 1 to 2^53-1. An older
    helper sends none and keeps none.
  - The node sends a `settings` call whose `step` is one of:
    - `parse {values}`: the schema's defaults applied, then validated;
    - `migrate {from, values}`: an older version converted;
    - `changed {state}`: the document's state after `ready`, and after each
      save, reset or migration. This is Paseo's server-side `read()` and
      `subscribe()`; its answer is not read.
  - `parse` and `migrate` are answered with `{verdict: "valid", values}` or
    `{verdict: "invalid", message}`. A refusal is an answer, not a protocol
    error.
- **Read** (`plugin.settings.read`, Paseo's `read`):
  - a missing file parses `{}` at revision `missing`;
  - the declared version parses;
  - an older version migrates, is saved once, and tells the helper and
    `plugin.watch`;
  - a newer version is invalid ("Settings were saved by a newer plugin
    version");
  - unreadable bytes are invalid.
  Invalid data stays on disk until a save or a reset replaces it.
- **Write and reset** (`plugin.settings.write`, `plugin.settings.reset`,
  Paseo's `write`):
  - another revision is a `conflict` ("Settings changed on another client.
    Reload before saving again.");
  - a save over a stored document of another version is invalid ("Reload or
    reset settings before saving a different schema version");
  - a reset parses `{}` whatever is stored;
  - a saved document tells the helper, then moves `plugin.watch`.
- **Scope.** The three methods are Admin and refused to agents (Paseo's
  `daemon.manage`): the values may hold anything, and they are ordinary JSON,
  not a credential vault.
- **Refusal words.** The words reach Admin callers as plain text, at most
  1 KiB, with control characters replaced, and never reach agents. The full
  text goes to the plugin's log. This is ADR 0079's stated exception to safe
  error classes; Paseo shows its schema's messages so a person can fix a
  value.
- **Errors.**
  - no installation, an undeclared document or a client-only plugin:
    `not_found`;
  - not running: `unavailable`, as Paseo's settings RPCs need the plugin
    process;
  - values over the helper payload budget: `invalid_request`;
  - a helper failure or timeout: the safe classes.
- **Change notice.** `plugin.watch`, as decided above. A Vessel client's
  catalog re-read is data only and disposes nothing, so Paseo's reason to
  keep settings changes off the catalog reload path (open drafts lost when
  the plugin is disposed) does not apply. Clients keep a draft keyed by
  machine, plugin and document across re-reads.
- **Lifecycle.** Values survive node restart, reload, disable and update.
  Remove deletes the installation's folder after any write in flight.
  Reinstalling the id starts from defaults.
- **UI.** Settings screens bind a document by id; see "Surfaces" below.
- **Protocol.** 43 (`PLUGIN_SETTINGS_PROTOCOL`), after the origin link's 42; assigned at its node-line merge.

### Surfaces (2026-11-21, P3b-1; coordinator ruling 2026-11-21)

Paseo 0.15.4 lets a plugin add screens, sidebar header and footer items and
settings screens as React components
(public-docs/plugins/reference.md:637-846 and 1302-1352). Paseo owns the
route, header, back and error boundary; the plugin owns the body. Its
settings screens are listed in the plugin row's menu on Settings › Plugins
(packages/app/src/screens/settings/plugins-page.tsx:86,
packages/app/src/plugins/settings/index.tsx:44-153), titled
`<plugin id> · <title>` (packages/app/src/screens/settings-screen.tsx:1314-1321),
and built from `SettingsSection`, `SettingsCard`, `SettingsRow`,
`SettingsSwitch`, `SettingsSelect`, `SettingsInput` and `SettingsAction`
(packages/app/src/components/settings/index.tsx:44-199) bound to
`useSettings` (packages/app/src/plugins/settings/use-settings.ts:16-85).
Vessel draws the reviewed declarative `Surface` (ADR 0079) with its own
controls instead, so the binding is data:

| Control | Fields | Behaviour |
|---|---|---|
| `card` | `surface` | Paseo's `SettingsCard`: one card, a divider between direct children |
| `settings_switch` | `settings`, `key`, `label`, `hint?` | Saves at once: `{...values, key: bool}` at the loaded revision |
| `settings_select` | `settings`, `key`, `label`, `hint?`, `options` | Saves at once with the option's id as the string |
| `settings_input` | `settings`, `key`, `label`, `hint?`, `placeholder?`, `mode` (`plain` or `secret`) | Keeps a draft; Enter saves it at the revision taken when the draft began; Escape discards it |

`Action::OpenSettings {screen}` is Paseo's `openSettings(id)`
(packages/app/src/plugins/actions.ts:36-40). It opens a settings screen of
the same document, as `OpenScreen` opens a screen.

- **Validation.**
  - A bound document must be one the same helper declared in `ready`. The
    actor refuses a `ready` or a `client_changed` that binds another
    document (a protocol violation).
  - A client-only plugin has no settings: a `client` file that binds one
    fails the start ("it binds settings, and a plugin without a helper has
    none").
  - `open_settings` names a declared settings screen.
  - A select's options are nonempty and distinct.
  - **Stated limit:** `key` is one top-level property, an ASCII identifier
    (`^[A-Za-z_][A-Za-z0-9_]{0,63}$`), with no nested paths. A select's
    values are option ids (lowercase identifiers).
- **States** follow `useSettings`:
  - `loading`: no default is drawn as if it were saved;
  - `ready`;
  - `invalid`: the bounded error and "Reset to defaults", since a reset
    recovers invalid stored data;
  - `error`: Retry.
  - A conflict keeps the draft and shows Paseo's words with Reload; an
    invalid answer shows the bounded message on its row.
  - Drafts are keyed by machine, plugin, document and key.
  - `plugin.watch` moving re-reads the documents a screen shows.
- **Read devices (stated difference; Paseo has no such tier).** A device
  holding `read` draws screens: text, Markdown and layout render, and every
  control that invokes a method renders disabled, with one muted line at
  the top of the surface ("Changing this needs admin access to <machine>").
  Settings screens are hidden there, as `plugin.settings.read` is Admin.
  Commands and slash commands stay hidden without `may_invoke`, as decided
  above.
- **No screen params (stated difference).** Paseo's `openScreen({screenId,
  params})` passes strings to a component that fetches its own data. A
  Vessel surface is a static document; a helper changes what a screen shows
  by declaring again (`client_changed`, which moves the catalog).
- **What a control sends** (the authors' contract, as Paseo's `onSubmit`
  gets its context):
  - `text_field` invokes its method with `{value: <text>}`;
  - `choice` with `{value: <option id>}`;
  - `toggle` with `{value: <bool>}`;
  - a slash command with `{arguments: <text>}`.
  - Each call carries the context it was taken in: `global` for screens and
    settings, `workspace` or `agent` for panels.
- **Compatibility.**
  - A native app decodes the catalog strictly, so a kind it does not know
    fails the whole reply.
  - Each entry's `required_client` is the oldest protocol that decodes every
    kind its document uses: the catalog's for the first kinds, and this
    protocol for `card`, the settings controls and `open_settings`.
  - `plugin.catalog` leaves out an entry above the caller's negotiated
    protocol. The rest of the reply still decodes, and the plugin appears
    once the client is updated.
  - Refusing new kinds while an older client is connected was the
    alternative; it cannot be decided when a helper declares.
- **Protocol.** Assigned 44 (`PLUGIN_SURFACES_PROTOCOL`) at its node-line
  merge, after the settings' 43; no method is added.
- **Order.** The node part comes first. Then the page and the app draw
  screens, sidebar items and settings screens. Panels (P3b-2), then header
  buttons and composer pills (P3b-3), follow.
  - The app comes in two steps: first the screen route, the sidebar rows
    and the Command Center's opening items with every non-settings kind
    (2026-11-21); then its settings screens and their binding. Until then
    an app screen draws no settings control and `open_settings` does
    nothing there.
  - **App stated differences.** A `choice` is the kit's pill choices. A
    popover button unfolds its controls in place under it, since the app
    has no anchored popover for arbitrary content. A menu button lists its
    flat items only, as the page does.

## Alternatives

- Install Paseo plugins unchanged by running Node.js and esbuild on the node
  and evaluating React bundles in the page: ADR 0079 refused it (ADR 0035,
  code evaluated in apps); this record keeps that.
- Keep installations inside `settings.json` as Paseo keeps them in
  `config.json`: two writers to one file; refused above.
- Push `plugin_catalog_changed` as a new server frame: the node has no
  node-wide event frame; a long poll is what `fleet.watch` already does.
- Write a semver range parser: the `semver` crate (1.0.28) is already in the
  lock through other crates; adding it as a direct dependency of `vessel-node`
  adds no new code to the build and is the widely used implementation.

## Consequences

- Seven method families and one protocol number; guard, forward policy,
  version tables, samples, TypeScript bindings and the unserved lists carry
  them as every method.
- `vessel-node` depends on `vessel-plugin` and `semver`.
- Stated differences from Paseo: helper executable plus declarative client
  data, `vessel-plugin.json`, Paseo plugins refused with a message, a long
  poll instead of status pushes, path and error shown to Admin callers only,
  built-in plugins are core features (usage fetchers), no `PaseoApi`
  (a narrow broker is a later record).
- The catalog and invoke (2026-11-20) add two methods at their own protocol,
  with the same tables; `plugin.catalog` is a stated scope difference from
  Paseo (Read instead of `daemon.manage`).
- The surfaces (2026-11-21) add declaration kinds at their own protocol and
  no method; an entry using them is left out for an older caller. Read
  devices and the missing screen params are stated differences.
- Not in this record: panels, header buttons and composer pills (P3b-2 and
  P3b-3), lifecycle hooks (P5), agent tools, providers, usage
  sources, timeline and attachments (P6).
