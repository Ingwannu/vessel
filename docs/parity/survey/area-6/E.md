# Plugins (area 6, part E)

Sources: docs/plugins.md (docs), packages/plugin/src (SDK), packages/server/src/server/plugins (daemon), packages/app/src/plugins (app), packages/app/src/screens/settings/plugins-page.tsx, packages/cli/src/commands/plugin/index.ts, plugins/* (built-ins). Paths below are relative to ~/refs/paseo-latest/packages unless stated.

## Plugin model and trust
- A plugin = a directory with `paseo-plugin.json` + optional `index.server.ts(x)` (runs in a daemon subprocess) + optional `index.client.ts(x)` (compiled to a bundle and evaluated in every connected app). At least one entry. Dirs `client/`, `server/`, `shared/` own code; other code files in the root are rejected by the compiler (docs/plugins.md "Contribute behavior and UI").
- Trusted and unsandboxed: server code and preparation (`build`) commands run with the daemon user's access; client code runs inside the app. `paseo plugin add/install` means "I trust this code".
- Global switch: daemon config `pluginsEnabled` (default `false`, server/src/server/config.ts:683). Installed plugins do not start unless true; built-ins ignore it. Runtime-safe toggle via `configStore.onFieldChange("pluginsEnabled")` (server/src/server/plugins/index.ts:239). Changing it in config.json needs `paseo reload`.
- Config shape: `plugins: Record<PluginId, {source:"directory", path, enabled?}>` (protocol/src/plugin-config.ts; `PluginIdSchema = /^[a-z][a-z0-9-]*$/`); `pluginRegistries: {host: {authorization}}` (persisted-config.ts:344); env `PASEO_PLUGIN_REGISTRY` (config-environment.ts:32, config.ts:632) sets default registry base.
- Manifest (server/src/server/plugins/manifest.ts): `id`, `name?`, `description?`, `icon?` (relative PNG path), `media?` (relative paths or https URLs), `requirements?: {paseo: semver range}` (strict), `build?: string[][]` (argv arrays, no shell). Manifest filename const `MANIFEST_FILENAME = "paseo-plugin.json"`. The config key is the runtime plugin ID; same directory may be installed under several IDs (`--id`).
- Compatibility: `assertPluginCompatibility` on the app side before evaluating a bundle (app/src/plugins/registry.ts:6); `requirements` is sent in the catalog.

## Install sources and managed storage
### Directory source
- CLI `paseo plugin install <abs-dir> [--id]` -> `plugin.directory.install.request {path,id?}`; `plugin.directory.inspect.request {path}` returns the manifest id (PluginService.installDirectory/inspectDirectory, plugins/index.ts:296,321). Removing never deletes the directory.
### Git source
- `git:owner/repo[:subpath]`, `github:owner/repo`, full Git URL, `--ref` (initial branch/tag/commit once). Managed by `ManagedPluginSources` (plugins/managed-source.ts): root `$PASEO_HOME/plugins/<id>/<uuid>/checkout`, metadata `$PASEO_HOME/plugins/sources.json`, staging `.staging/<uuid>`. `GIT_TIMEOUT_MS = 120_000`, `GIT_TERMINAL_PROMPT=0`. Manifest `build` argv commands run in the staged checkout (output limit `OUTPUT_LIMIT = 64*1024` in preparation-build.ts) before validate/compile/activate; failure discards the candidate and the running version stays.
### npm source
- `npm:` identifiers (managed-source/npm.ts, `NPM_SOURCE_PATTERN`); installed under `<versionRoot>/node_modules/<package>`, lifecycle scripts disabled; lockfile kept.
### Registry source
- `owner/slug` or `host/owner/slug`: fetches `<base>/plugins/<id>.json` (managed-source/registry.ts), 30 s timeout, `redirect:"error"`, credentials only for that host and only over HTTPS (or localhost), header `X-Paseo-Install: 1` on install; resolves to an npm or git artifact with pin (npm: version+resolved+integrity; git: commit). Registry installs keep registry identity; updates follow the approved pin.
### Messages
- `plugin.source.install.request {source, id?, ref?, pluginPath?(compat)}` -> `.response {plugin: PluginListItem}` (session.ts:2641).
- `plugin.source.status.request {pluginId?}` -> items with `installation {identity, currentRevision}` (identity kinds directory | git | npm).
- Reviewed updates: `plugin.source.update.preview.request {pluginId?, target?: {git ref | npm version}}` -> previews with `proposal {id, expected{identity, installationRoot, revision}, target}`; `plugin.source.update.apply.request {proposals[min 1]}` -> per-plugin results. Legacy `plugin.source.update.request` returns update-client guidance. Server: previewUpdates/applyUpdates (plugins/index.ts:410,438). Apply rechecks state; no review session stored.
- Feature flags in server_info: `pluginManagement`, `pluginLogs`, `pluginSourceInstallation`, `pluginSourceUpdates` (protocol/src/messages.ts:3736-3745); app gates UI with `useHostFeature` (plugins-page.tsx:271-275).

## Plugin lifecycle management (daemon)
- Handlers in server/src/server/session.ts:2552-2714: `plugin.list.request` -> `plugin.list.response {plugins: PluginListItem[]}`; `plugin.logs.get.request {pluginId}`; `plugin.reload|enable|disable|remove.request {pluginId}`; `plugin.catalog.get.request {cachedClientBundles?}`; `plugin.rpc.invoke.request {pluginId, method, input}`.
- `PluginListItem` fields: id, name?, icon?, media?, description?, path, enabled, `status: "running"|"disabled"|"failed"`, source?: "directory"|"git", npm?, installation?, remote?, ref?, commit?, error? (protocol/src/messages.ts:6855).
- PluginService methods (plugins/index.ts): start:218, listPlugins:244, getLogs:285, reloadPlugin:456 (stops/tears down old, compiles from disk; a failed reload stays failed), enablePlugin:470, disablePlugin:485, removePlugin:506 (deletes `$PASEO_HOME/plugin-settings/<id>`), invokePluginRpc:531, stopAllPlugins:535.
- Startup: plugin sessions may connect while app WebSockets are paused; daemon accepts clients after configured plugins settle and the initial catalog is built.
- Status pushes: `status` message with `status:"plugin_catalog_changed" {pluginId}` and `"plugin_settings_changed" {pluginId, settingsId}` (messages.ts:3975-3985; session.ts:2728-2733).
- Logs: in-memory tail per plugin, `MAX_LOG_ENTRIES=500`, `MAX_LOG_BYTES=256*1024`, `MAX_LOG_LINE_BYTES=16*1024` (plugins/runtime.ts:43-45); `[paseo]` entries for loading/ready/stopping/stopped; compile/load failures as stderr; retained across reload/disable/failure, cleared on remove, lost on daemon restart (structured copies in `$PASEO_HOME/daemon.log`).

## Plugin subprocess runtime
- `plugins/runtime.ts` (PluginRuntime) + `plugin-process.ts` + `plugin-process-protocol.ts`: IPC messages host->plugin: `initialize`, `invoke{method,input}`, `hook{name,input}`, `hook.cancel`, `tool.call`, `tool.cancel`, `tools.detach`, `usage.fetch`, `usage.discover`, `provider.status|catalog_key|connect|send|close`, `shutdown`, `paseo_frame/paseo_close`. Plugin->host: `ready`, `result`, `error`, `fatal`, `settings.changed`, `hooks.changed`, provider events (`provider.connected|connect_failed|accepted|rejected|event|closed`), `paseo_frame/close`.
- Timeouts: `REQUEST_TIMEOUT_MS=30_000`, `TOOL_CALL_TIMEOUT_MS=10*60_000`, `SOFT_SHUTDOWN_TIMEOUT_MS=2_000` (runtime.ts:41-46).
- Each subprocess has an exclusive `plugin:<id>` daemon session (plugin-session-identity.ts, session-socket.ts) used by `context.paseo`; reserved from normal clients, cleaned immediately on exit.
- Compile: esbuild worker (compiler.ts, compiler-esbuild.ts, compiler-imports.ts; worker idle stop `IDLE_TIMEOUT_MS=1000`); import-boundary rules enforced (client vs server vs shared, no Node in client, no React in server); SDK specifiers list in plugin-sdk-specifiers.ts; `ESBUILD_BINARY_PATH` env for packaged apps.

## Server contribution API (index.server.ts)
`PluginServerContext` (packages/plugin/src/server/contracts.ts): `registerSettings(def)`, `handle(rpcContract, handler)`, `registerProvider(reg)`, `registerUsageSource(src)`, plus lifecycle `addHook(name, handler)` (and deprecated `on`/`before`). Handler context `{paseo: PaseoApi}`.
### Plugin RPCs
- `defineRpc({name,input,output})`, name regex `/^[a-z][a-z0-9._-]*$/` (plugin/src/rpc.ts); zod validation of input and output in both the app and plugin process (`callPluginRpc`). App calls via `rpc()` / `useRpc`; transport = `plugin.rpc.invoke.request`.
### Settings
- `defineSettings({id /^[a-z][a-z0-9_-]*$/, scope:"host", version>=1, schema, migrate?})` (plugin/src/settings.ts); auto RPCs `settings.<id>.read|write|reset`; results `ready|invalid` and `saved|conflict|invalid` with optimistic `revision`. Stored at `$PASEO_HOME/plugin-settings/<pluginId>/<settingsId>.json` as `{version, values}` (plugins/settings/index.ts:94,174; bootstrap.ts:684). Change emits `plugin_settings_changed`; app invalidates react-query key (catalog-sync-model.ts:67).
- App side: `useSettings` hooks (app/src/plugins/settings/use-settings.ts), `SettingsState` status loading|error|invalid|ready + `save(values, revision)`, `reset`, `reload`; `addSettingsScreen` adds a screen under host settings.
### Lifecycle hooks (plugin/src/server/lifecycle.ts)
Registry `pluginHookRegistry` (waits = host awaits handler):
- Request hooks (return modified request): `workspace.create.before` (waits), `worktree.create.before` (waits; PluginWorktreeCreateRequest kinds branch-off | checkout-branch | checkout-change-request | checkout-github-pr), `agent.create.before` {config, env?} (waits), `agent.open.before` {PluginSessionOpenRequest: agentId, workspaceId, provider, cwd, reason create|resume|refresh|import, purpose interactive|history, title, labels, tools, env, mcpServers} (waits).
- Event hooks: `workspace.create.after`, `workspace.archive.after`, `worktree.create.after` (waits), `agent.create.after`, `agent.close.after`, `agent.archive.after`, `agent.turn.before`, `agent.turn.after {outcome completed|failed|canceled; timeline deprecated empty}`, `agent.permission.before {request}`, `agent.permission.after {requestId, resolution}` (all non-waiting).
- Context `{paseo, signal: AbortSignal}`. Legacy names (workspace.created, agent.turn_started, agent.session_open ...) map to the new ones (COMPAT(pluginHookNames), remove after 2027-04-30). Hook subscription changes are announced via `hooks.changed`.
- Server impl: plugins/lifecycle/index.ts.
### Agent tools
- `defineAgentTool({name /^[a-z][a-z0-9_]*$/ <=64, title?, description, input zod, execute(input,{agent,paseo,signal})})` -> `{text?, structuredContent?, isError?}` (plugin/src/server/agent-tools.ts). Added to `PluginSessionOpenRequest.tools` inside `agent.open.before`; exposed to the agent as MCP tools (plugins/agent-tools.ts, `tool.call` IPC with 10 min timeout; `tools.detach` on agent end).
### Provider plugins
- `registerProvider(ProviderRegistration {id,label,description?,icon?,requiresConfiguration?,command?,...})` (plugin/src/server/provider.ts); `PROVIDER_PROTOCOL_VERSION=1`, `PROVIDER_CAPABILITIES` list; status/connect/send/close IPC; model/mode/thinking/setting catalog types; ACP adapter helpers in plugin/src/server/acp.ts, acp-internal/connection.ts. `requiresConfiguration:true` providers participate only when `agents.providers.<id>` exists in config (Hoplite). Provider IDs differ from installation IDs.
### Usage sources
- `registerUsageSource({id,label,icon?,input zod, discover(scope), fetch(input)})`; `UsageReport` = available{planLabel, windows[], balances[], details[]} | unavailable{problem: expired|rejected|no_quota} | error (plugin/src/server/usage.ts). Window fields id,label,shortLabel,summary,usedPct,remainingPct,resetsAt,runsOutAt,shortfallPct,tone; balance units usd|credits|requests|tokens. Tone helpers: `toneFromUsedPct` (>90 danger, >=70 warning, else ok). Daemon-side manager (plugins/usage-sources/index.ts) owns account identity and a 5-minute fetch cache (`ttlMs = 300_000`), forced refresh bypasses it. See part B for the UI.

## Client contribution API (index.client.tsx)
`PluginClientContext` (plugin/src/client/contracts.ts:~150-190) gives: `i18n`, `playAudio({base64,mimeType})`, `paseo` (PaseoApi), `rpc`, `openScreen`, `openSettings`, `openPanel`, and:
- `addScreen {id, title|fn(params), Component}`: route `/h/<serverId>/plugin/<pluginId>/<kind>/<id>` (app/src/app/h/[serverId]/plugin/[pluginId]/[contributionKind]/[contributionId].tsx; plugins/routes.ts:79 regex for legacy `sidebar|surface`). Params ride in the query with prefix `param.` (`SCREEN_PARAM_PREFIX`), all values strings (routes.ts:11-41). Surface host: plugins/surface-screen.tsx, error boundary surface-error-boundary.tsx.
- `addSidebarHeaderItem` / `addSidebarFooterItem {id,title,Component(props: currentScreen, openScreen)}`; same item across hosts shown once with a host picker in the screen header (plugins/sidebar-items/index.tsx, sidebar-groups.ts, hosts/index.ts). Legacy `addSidebarItem`/`addSurface` aliases (COMPAT pluginSidebarAliases).
- `addSettingsScreen {id,title,icon,Component}` (plugins/settings/index.tsx).
- `addWorkspacePanel` / `addAgentPanel {id,title,icon,locations?: ("workspace"|"explorer")[],Component}`; props workspaceId, params (agentId for agent panel), revision, `tab` (setModified, state, setState); legacy `context` prop (workspace-panels/panel.tsx, locations.ts).
- `addFilePanel {id,title,icon,files:{extensions,filenames?},Component(file: PluginFile, params path/lineStart/lineEnd)}`.
- `addCommandCenterItem {id,title,icon,keywords?,context: global|workspace|agent,onSelect(ctx)}` (plugins/command-center/*).
- `addSlashCommand {name,description,argumentHint,context: workspace|agent,onSubmit({args,...ctx})}` (plugins/client-slash-commands).
- `addHeaderButton {id,workspaceId,button}` and `addComposerPill {+agentId}`; `PluginButton {title,icon,label?,visible?,disabled?,behavior: action|menu|popover}`; registration `update(patch)` / `remove()` (plugin/src/client/buttons.ts; app/src/plugins/buttons/*).
- `addAttachmentSource {id,title,icon,pickerTitle,searchPlaceholder,search: RpcContract}` -> picker in composer (plugins/attachments/picker.tsx; `PluginAttachmentItemSchema` in plugin/src/attachments.ts).
- `addTheme {id,name,appearance: light|dark,colors {background,foreground,raised,control,border,accent?,mutedForeground,ring}}` (plugins/themes/index.ts); joins the theme picker (part C).
- `addTimelineTransformer {id, query{itemType}, transform({item,phase: streaming|complete}) -> {items: PluginTimelineItem[]}|undefined}` and `addTimelineRenderer {kind,version,schema,Component}` (plugins/timeline/*); plugin items `{type:"plugin", id?, kind, version, data}`; server can append via `agent.timeline.append.request` (messages.ts:1496+).
- Contribution function returns cleanup; theme DTO and layout facts (`layout.compact`) from host; UI kit `@getpaseo/plugin/client/ui` (Icon, Modal, SidebarRow, ...). Icons: Lucide names or safe SVG (plugins/icons.ts).
- Translations: plugin supplies resources, host-owned engine follows the app language (plugin/src/client/translations.tsx, docs/plugins.md "Client translations"); never send locale over RPC.

## App-side registry, catalog sync, bundle cache
- `PluginRegistry` (app/src/plugins/registry.ts:17): `installCatalog(serverId, catalog, {client, audio, replacePluginId?})`, one `InstalledPlugin` per host+plugin with its own `QueryClient`, `PluginTranslationRuntime`, Paseo API (`createPaseoApi` bound to the host client). Evaluation errors kept per `serverId/pluginId` (`getEvaluationError`). Bundle execution: plugins/evaluate.ts (`runPluginClientBundle`).
- `syncPluginCatalog` (plugins/catalog-sync-model.ts): requires feature support + audio engine, else removes host; on connected: `client.getPluginCatalog()` then `installCatalog`; subscribes `observeEvents(["status.plugin_catalog_changed","status.plugin_settings_changed"])`; refresh queue serialized; on snapshot invalidates plugin-settings queries (refetchType all).
- Client bundle cache per host: IndexedDB on web (client-bundles/index.web.ts, store.ts); keyed by plugin id + daemon SHA-256; cold client verifies contents before advertising hashes; `cachedClientBundles` in the request lets the daemon omit unchanged bundles; only newest catalog kept; one write at a time, newest pending catalog wins.
- Offline hosts keep rendering installed plugins (reconnect on same client); an offline selected host never falls back to another host's installation.

## Settings page: Plugins (host-scoped)
- `plugins-page.tsx` (+ plugins-page-state.ts, plugin-install-form-model.ts): section "settings.plugins.title". Elements: trust notice, global enable switch (patches daemon config `pluginsEnabled` via `useDaemonConfig(serverId).patchConfig`, plugins-page.tsx:276,386), install form (single source string input, `canSubmit` when non-empty; model `openPluginInstallForm`), docs link, plugin rows (`testID plugin-row-<id>`: status badge running/disabled/failed, enable toggle, error hint, overflow menu: Logs / Reload / Remove with confirm "removeConfirmTitle"), logs sheet (`plugin-logs-sheet`, Refresh button, loading/error/empty states), states: offline, daemon-too-old update alert, loading, error + Retry, empty. Queries via `pluginQueryKey(serverId)`; operations: `client.listPlugins`, reload/enable/disable/remove, install source (`pluginSourceInstallation` gate), logs (`pluginLogs` gate). Update preview/apply UI in app: not found in this page (CLI `paseo plugin update` only; looked in plugins-page.tsx).

## CLI `paseo plugin` (cli/src/commands/plugin/index.ts:211+)
Subcommands: `init <dir>` (scaffold.ts, writes files only), `ls [id]`, `status` (hidden), `logs <id>`, `install <source>` (`--id`, `--ref`), `update` (update.ts: review then apply), `enable|disable|reload <id>`, `remove <id>`; all take `--json` and daemon host options.

## Built-in plugins (plugins/<id>/, list in server/src/server/plugins/builtin/index.ts:5)
Loaded in process via `BuiltinPluginLoader` regardless of `pluginsEnabled`; hidden from `plugin ls`; IDs reserved; desktop ships them as `builtin-plugins` resource outside app.asar (resolveBuiltinPluginsRoot). Entries:
- Providers: `antigravity-provider`, `hoplite-provider` (requiresConfiguration opt-in), `muse-provider` (Muse Code provider + usage source).
- Usage sources: `claude-usage-source`, `codex-usage-source`, `copilot-usage-source`, `cursor-usage-source`, `grok-usage-source`, `kimi-usage-source`, `minimax-usage-source`, `opencode-go-usage-source`, `zai-usage-source`.
- `worktree-include`: copies gitignored files listed in `.worktreeinclude` after worktree creation (`worktree.create.after`).
Examples (not shipped): ~/refs/paseo-latest/plugin-examples/* (buttons, catppuccin theme, hosts, inline-thinking, lifecycle-actions, linear, markdown-panel, modal-ui, pdf-panel, provider-acp-transformer, provider-direct, settings, timeline-items, translations, worktree-placement ...).

## Cross-area connections
- Usage UI (part B) consumes usage sources; themes (part C) join the appearance picker; command center (part C) shows plugin items; sidebar (part C) renders plugin header/footer items; settings (part D) hosts plugin settings screens and the plugins page; MCP tools (part A) receive plugin agent tools; providers/models (other area) get plugin providers; workspace creation, agent create/open/turn/permission flows trigger hooks; timeline area renders plugin timeline items.

## Inventory
- Plugin model, trust and `pluginsEnabled`; manifest; directory/git/npm/registry install; reviewed updates; reload/enable/disable/remove; logs; subprocess runtime; plugin RPC; plugin settings; lifecycle hooks; agent tools; provider plugins; usage-source plugins; client contributions (screen, sidebar header/footer, settings screen, workspace/agent/file panels, command center, slash commands, header buttons, composer pills, attachment sources, themes, timeline transformer/renderer, translations, playAudio); registry + catalog sync + bundle cache; Plugins settings page; CLI plugin commands; built-in plugins.
