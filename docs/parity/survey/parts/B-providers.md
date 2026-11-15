# B-providers: providers, catalogs, profiles, skills, commands, subagents, Paseo MCP (Paseo 0.15.4)

Source root: `~/refs/paseo-latest/packages`. Abbreviations: `S` = `server/src/server`, `A` = `app/src`, `P` = `protocol/src`, `C` = `client/src`.
Slice B covers the provider layer (what an agent runs on), the catalogs the UI selects from, and the "agent-side" surfaces
(profiles, skills, slash commands, provider subagents, the Paseo MCP server agents call). Agent lifecycle, timeline, permissions
are other slices; links in `## Cross-area connections`.

---

## 1. Provider model: AgentProvider, AgentClient, AgentSession (the contract every provider implements)

### 1.1 What it is
`AgentProvider = string` (`P/agent-types.ts:3`, `S/agent/agent-sdk-types.ts:23`): an open string id (not an enum). Built-in ids come
from `P/provider-manifest.ts`; custom/plugin ids match `/^[a-z][a-z0-9-]*$/` (`P/provider-config.ts` `PROVIDER_ID_PATTERN`).

### 1.2 `AgentClient` (one per provider id, long lived, `S/agent/agent-sdk-types.ts:764-830`)
Fields: `provider`, `capabilities: AgentCapabilityFlags`.
Required: `createSession(config, launchContext?, options?)`, `resumeSession(handle, overrides?, launchContext?, options?)`,
`fetchCatalog(options: FetchCatalogOptions, context?: ProviderRefreshContext): Promise<ProviderCatalog>`, `isAvailable(signal?, options?)`.
Optional: `getCatalogCacheKey`, `resolveConfiguredModel`, `resolveDefaultModeId`, `resolveCreateConfig`, `isCreateConfigUnattended`,
`listCommands(config)`, `listFeatures(config)`, `listImportableSessions`, `importSession`, `getDiagnostic(): {diagnostic: string}`,
`archiveNativeSession`, `unarchiveNativeSession`, `shutdown`.
- `FetchCatalogOptions = {providerOptions?} & ({scope:"global", force} | {scope:"workspace", cwd, force})` (`:734`).
- `ProviderCatalog = {models: AgentModelDefinition[], modes: AgentMode[], defaultModeId?: string|null}` (`:752`). Models and modes are
  discovered together in ONE call ("at most one provider runtime process" rule, doc comment on `ProviderDefinition.fetchCatalog`).
- `ProviderRefreshContext = {signal: AbortSignal, runActivity(name, op)}` (`:746`): lets a timeout report which upstream activity (e.g.
  `availability`, `version`, `models`, `settings`, `app-server.start`, `model/list`, `session/new`) was pending.
- `AgentCreateSessionOptions {persistSession?: boolean (default true), configuredModelIds?: readonly string[]}` (`:638`),
  `AgentResumeSessionOptions {purpose?: "interactive"|"history", configuredModelIds?}` (`:655`) - `history` purpose may be read-only for archived native sessions.
- `AgentLaunchContext {mcpServers?, mcpServerOwners?, agentId?, env?, paseoTools?: PaseoToolCatalog}` (`:625`): launch-only, NEVER persisted.
  `paseoTools` is the runtime-only internal Paseo tool catalog that providers adapt to their native tool surface (see section 22).

### 1.3 `AgentSession` (one per running agent, `:677-732`)
Readonly: `provider`, `id`, `capabilities`, `features?: AgentFeature[]`, `initialTimeline?`.
Methods: `run`, `startTurn(prompt, options) -> {turnId}`, `steerActiveTurn?(prompt, {expectedTurnId, clearPendingPermissions?}) -> SteerResult {status:"accepted", providerMessageId?}|{status:"unavailable"}`,
`subscribe(cb)`, `streamHistory()`, `getRuntimeInfo()`, `getAvailableModes()`, `getCurrentMode()`, `setMode(modeId) -> void|AgentProviderNotice`,
`getPendingPermissions()`, `respondToPermission(id, response) -> {followUpPrompt?}|void`, `describePersistence()`, `interrupt()`, `canStopRuntime?()`, `close()`,
`listCommands?()`, `setModel?(id|null)`, `setThinkingOption?(id|null) -> void|AgentProviderNotice`, `setFeature?(featureId, value)`,
`revertConversation?/revertFiles?/revertBoth?({messageId})`, `tryHandleOutOfBand?(prompt) -> {run({emit})}|null` (side-effect slash commands such as `/compact`, `/goal` that must not cancel the running turn).
- `usageSession?()` returns `{provider, model?, env, sessionKey}` used by the usage plugin.

### 1.4 `AgentCapabilityFlags` (`P/agent-types.ts` / `agent-sdk-types.ts:169`)
`supportsStreaming, supportsSessionPersistence, supportsSessionListing?, supportsDynamicModes, supportsMcpServers, supportsReasoningStream, supportsToolInvocations, supportsRewindConversation?, supportsRewindFiles?, supportsRewindBoth?` plus open index signature (OpenCode adds `supportsNativePaseoTools: true` when the bridge exists, `opencode-agent.ts:1421`).
Capability table (from source):
| provider | streaming | persistence | listing | dynamicModes | mcp | rewindConv/Files/Both |
|---|---|---|---|---|---|---|
| claude | y | y | y | y | y | y/y/y (`claude/agent.ts:372`) |
| codex | y | y | y | **n** | y | y/n/n (`codex-app-server-agent.ts:~270`) |
| opencode | y | y | y | y | y | n/n/y (`opencode-agent.ts:130`) |
| pi | y | y | y | y | **n** | y/n/n (`pi/agent.ts:163`) |
| omp | y | y | y | y | y | y/n/n (`omp/agent.ts:116`) |
| copilot / generic ACP (`DEFAULT_ACP_CAPABILITIES`) | y | y | y | y | y | n/n/n |
| mock-slow | all false (`providers/mock-slow-provider.ts:14`) |
`S/agent/agent-sdk-capabilities.md` is a design note about the underlying SDKs (codex-sdk 0.58, claude-agent-sdk 0.1.37), not runtime behaviour.

### 1.5 Wire-level shapes
`AgentModelDefinition {provider, id, aliases?, isSelectable?, label, description?, isDefault?, metadata?, contextWindowMaxTokens?, thinkingOptions?: AgentSelectOption[], defaultThinkingOptionId?}`;
`AgentSelectOption {id,label,description?,isDefault?,metadata?}`; `AgentMode {id,label,description?,icon?,colorTier?}`; `ProviderStatus = "ready"|"loading"|"error"|"unavailable"`;
`AgentProviderNotice = {type:"info"|"warning"|"error", message}`. Helpers: `normalizeAgentModelDefinition` (fills `defaultThinkingOptionId` from `isDefault` option), `normalizeAgentModelCatalog` (dedupe by model id, first wins), `filterSelectableAgentModels` (drops `isSelectable === false`).
Zod mirrors in `P/messages.ts:300-420` (`AgentModeSchema`, `AgentSelectOptionSchema`, `AgentModelDefinitionSchema`, `ProviderSnapshotEntrySchema`).

### 1.6 Connections
Agent creation (slice A) calls `ProviderSnapshotManager.resolveCreateConfig/validateAgentConfiguration`; AgentManager owns `clients` map built from the registry (section 4); timeline/permission slices consume `AgentStreamEvent`s the sessions emit.

---

## 2. Built-in provider list and mode tables (the provider manifest)

### 2.1 Where
`P/provider-manifest.ts` (`AGENT_PROVIDER_DEFINITIONS`, `DEV_AGENT_PROVIDER_DEFINITIONS`, `BUILTIN_PROVIDER_IDS`, `getAgentProviderDefinition`, `isValidAgentProvider`, `getUnattendedModeId`, `getModeVisuals`).
`AgentProviderDefinition {id,label,description,enabledByDefault?,defaultModeId: string|null,modes: AgentProviderModeDefinition[],voice?: {enabled,defaultModeId,defaultModel?}}`.
`AgentProviderModeDefinition = AgentMode & {icon, colorTier: "safe"|"moderate"|"dangerous"|"planning"|"#rrggbb", isUnattended?}`. `isUnattended` marks the provider's most-permissioned no-prompt mode.

### 2.2 Built-in providers (exact ids, defaults)
| id | label | defaultModeId | enabledByDefault | voice default |
|---|---|---|---|---|
| `claude` | Claude | `auto` | true | enabled, mode `default`, model `haiku` |
| `codex` | Codex | `auto-review` | true | enabled, mode `auto`, model `gpt-5.4-mini` |
| `copilot` | Copilot | `https://agentclientprotocol.com/protocol/session-modes#agent` | true | - |
| `opencode` | OpenCode | `null` (comment: users may rename/delete any agent incl. `build`) | true | enabled, mode `build` |
| `pi` | Pi | `null`, modes `[]` | true | - |
| `omp` | Oh My Pi | `full` | **false** (opt-in) | - |
Dev only (only when `isDev`): `mock` ("Mock Load Test", modes `load-test`,`approval-test`), `mock-slow` ("Mock Slow Provider", mode `default`; hangs in model discovery to exercise loading/timeout UI).
Not in the manifest but constructed by the registry: `cursor` (factory `PROVIDER_CLIENT_FACTORIES.cursor`, command default `["cursor-agent","acp"]`, `provider-registry.ts:189,231`), generic `acp`, `kimi`, `kiro`, `traecli`, `hoplite`/other catalog entries (all are `extends:"acp"` configured providers - section 10). Gemini is NOT a built-in client: it only exists as an entry in the app's ACP catalog (`A/data/acp-provider-catalog.ts:183`, id `gemini`, "Gemini CLI", https://geminicli.com) installed as `extends:"acp"`.
`hub-e2e` is a synthetic ACP provider id with exact-MCP-preapproval contract used by the Hub harness (`provider-registry.ts:163-187`).

### 2.3 Mode tables (id: label - colorTier [icon], `*` = isUnattended)
- Claude (`CLAUDE_MODES`): `plan` Plan Mode planning [ShieldEllipsis]; `default` Always Ask safe [Shield]; `acceptEdits` Accept File Edits moderate [ShieldPlus]; `auto` Auto mode moderate [ShieldCheck]; `bypassPermissions`* Bypass dangerous [ShieldOff]. Server may drop `auto` (section 5.2).
- Codex (`CODEX_MODES`): `auto` Default Permissions moderate [Shield]; `auto-review` Auto-review moderate [ShieldCheck]; `full-access`* Full Access dangerous [ShieldOff].
- Copilot: `...session-modes#agent` Agent moderate; `...#plan` Plan planning; `allow-all`* Allow All dangerous.
- OpenCode: `build` Build moderate [Shield]; `plan` Plan planning [ShieldEllipsis] (static fallback only; runtime modes come from OpenCode agents).
- OMP (`OMP_MODES`): `full`* Full Access dangerous (yolo); `write` Write Approval moderate [ShieldAlert]; `ask` Always Ask safe [ShieldCheck].
- Pi: none.
Icon names are an open string; clients look icons up in a registry and fall back to a default (`AgentModeIcon`).

### 2.4 Protocol / CLIENT_CAPS gating for older clients (port only if wire compat with old apps matters)
`CLIENT_CAPS.allProviders = "all_providers"`; `S/session.ts:290-316,2162` `isProviderVisibleToClient`: a client sees every provider if it advertises `all_providers` or its app version >= `MIN_VERSION_ALL_PROVIDERS = "0.1.45"`, else only `LEGACY_PROVIDER_IDS = {claude, codex, opencode}`.
`CLIENT_CAPS.customModeIcons = "custom_mode_icons"`: without it, mode icons outside `{ShieldCheck, ShieldAlert, ShieldOff, ShieldQuestionMark}` are rewritten to `ShieldCheck` (`provider-catalog-session.ts:29,106`).
Other caps in this slice: `compactProviderSnapshots="compact_provider_snapshots"`, `providerSnapshotReferences="provider_snapshot_references"`, `providerSubagents="provider_subagents"`, `projectedSubagentTimeline="projected_subagent_timeline"`, `pluginTimelineItems`. Server feature flags (`server_info.features`, `P/messages.ts:3673-3877`): `providersSnapshot`, `providersSnapshotCwd`, `providerUsageList`, `providerSubagents`, `projectedSubagentTimeline`, `providerSubagentNesting`, `providerRemoval`, `hopliteProvider`, `forgeProviders`, `agentProfiles`.

---

## 3. Provider launch config: command, env, disallowedTools (provider-launch-config)

### 3.1 Schemas (`P/provider-config.ts`, re-exported by `S/agent/provider-launch-config.ts`)
- `ProviderCommandSchema = {mode:"default"} | {mode:"append", args?: string[]} | {mode:"replace", argv: string[] (min 1, each min 1)}`.
- `ProviderRuntimeSettings = {command?: ProviderCommand, env?: Record<string,string>, disallowedTools?: string[]}`.
- `ProviderOverrideSchema` (this is the persisted per-provider config, `agents.providers.<id>`):
  `extends?: string` (a built-in id or the literal `"acp"`), `label?`, `description?`, `command?: string[]` (min 1; = replace argv), `env?`, `options?: Record<string,unknown>` (typed by each provider, becomes `providerOptions`), `params?` (COMPAT(providerParams) v0.10.0, remove after 2027-03-30; `options` wins when both set), `models?: ProviderProfileModel[]` (REPLACES the discovered list), `additionalModels?: ProviderProfileModel[]` (appended/merged), `disallowedTools?: string[]`, `paseoTools?: {enabled?: boolean, disabledTools?: string[]}`, `enabled?: boolean`, `order?: number`.
  `ProviderProfileModelSchema {id(min1), label(min1), description?, isDefault?, thinkingOptions?: [{id,label,description?,isDefault?}]}`.
- `ProviderOverridesSchema` validation (superRefine): id must match `/^[a-z][a-z0-9-]*$/`; a non-built-in id with `extends` must declare `label` ("Custom provider X must declare label"); `extends` must be a built-in (`claude,codex,copilot,opencode,pi,omp`) or `acp`; `extends:"acp"` requires `command`.
- Legacy shape `{command:{mode,...}, env}` (`ProviderRuntimeSettingsSchema`) is migrated by `migrateProviderSettings` and `normalizeAgentProviders` (`persisted-config.ts:201`): replace argv becomes `command: argv`; `append` entries are dropped.

### 3.2 Resolution helpers (`S/agent/provider-launch-config.ts`)
- `resolveProviderLaunch({commandConfig, defaultBinary}) -> {command,args,source: "default"|"append"|"override", env?}`: `replace` wins verbatim (`source:"override"`); else default binary (+ appended args). Throws "defaultBinary is required when provider command is not replaced".
- `checkProviderLaunchAvailable(launch, defaultBinary?) -> {available, resolvedPath}` via `findExecutable`/`executableExists` (absolute commands are checked on disk).
- `resolveProviderCommandPrefix`, `isProviderCommandAvailable`.
- Env: `createProviderEnv/createProviderEnvSpec({baseEnv, runtimeSettings, overlays})` merges `runtimeSettings.env` then overlays and UNSETS `CLAUDECODE, CLAUDE_CODE_ENTRYPOINT, CLAUDE_CODE_SSE_PORT, CLAUDE_AGENT_SDK_VERSION` (`PARENT_SESSION_ENV_VARS`) so a daemon started inside Claude Code does not make children refuse to launch.
- `resolveShellEnv()` caches `process.env` copy.

### 3.3 `provider-options` (`S/agent/provider-options.ts`)
- `ProviderOptions = Record<string, unknown>`; stored as `AgentSessionConfig.providerOptions`.
- `validateProviderOptions(provider, zodSchema, options)` throws `ProviderOptionsValidationError {code:"provider_options_invalid", provider, issues:[{path,message}]}` with message `Invalid providerOptions for 'X': providerOptions.a.b: msg`. The wire error code is part of create-agent responses (`P/messages.ts:3195`).
- `mergeProviderOptions(base, override)`: plain objects merge recursively, everything else replaces (uses `Object.defineProperty` to be safe against `__proto__`).
- `ToolPolicyUnsupportedError {code:"tool_policy_unsupported"}`: "Provider 'X' cannot preapprove exact MCP tools for unattended execution; select Claude, Codex, or OpenCode".
- Per-provider option schemas (each provider validates in `assertConfig`): Claude `ClaudeProviderOptionsSchema` (strict: `allowedTools, disallowedTools, additionalDirectories, extraArgs, sandbox{...}, settings{permissions{allow,ask,deny}, sandbox{...}}`, `claude/options.ts`); Codex `CodexProviderOptionsSchema` (strict: `approval_policy` (untrusted|on-request|never|granular{sandbox_approval,rules,mcp_elicitations,request_permissions,skill_approval}), `sandbox_mode` (read-only|workspace-write|danger-full-access), `sandbox_workspace_write{writable_roots,network_access,exclude_slash_tmp,exclude_tmpdir_env_var}`, `web_search` (disabled|cached|indexed|live), `features{network_proxy, multi_agent_v2}`; `codex/options.ts`); OpenCode `OpenCodeProviderOptionsSchema` (strict `permission`: ask|allow|deny or per-tool rules for read, edit, glob, grep, list, bash, task, external_directory, todowrite, question, webfetch, websearch, codesearch, repo_clone, repo_overview, lsp, doom_loop, skill; `opencode/options.ts`); Pi `{sessionDir?, rpcTimeoutMs (default DEFAULT_PI_RPC_TIMEOUT_MS), extensionTimeoutMs}`; OMP `{sessionDir?, rpcTimeoutMs?, smolModel?, slowModel?, planModel?}` (`omp/provider-config.ts`; defaults mode `full`, ready timeout 20_000ms, rpc timeout 60_000ms, `MIN_SUPPORTED_OMP_VERSION = "16.3.9"`); ACP `ACPProviderOptionsSchema` `{supportsMcpServers?, clientCapabilities?{fs{readTextFile,writeTextFile}, terminal}}` passthrough (`providers/acp-options.ts`).
- Tool policy (`ToolPolicy {preapproved: McpToolRef{kind:"mcp",server,tool}[]}`) translation: Claude adds `allowedTools += mcp__<server>__<tool>` (`applyClaudeToolPolicy`); Codex writes `mcp_servers.<server>.tools.<tool>.approval_mode="approve"` (`applyCodexToolPolicy`); OpenCode creates permission rules `{permission:"<server>_<tool>", pattern:"*", action:"allow"}` (`buildOpenCodePermissionRules`); others reject (contract `supportsExactMcpPreapproval=false`).

---

## 4. Provider registry (`S/agent/provider-registry.ts`)

### 4.1 Purpose
Turns (built-in manifest + runtime settings + `agents.providers` overrides + plugin registrations + dev flag) into `Record<id, ProviderDefinition>`; each entry owns a client factory, model merge rules, and tool-policy contract.

### 4.2 Build algorithm (`buildProviderRegistry`, `:908`)
1. `buildResolvedBuiltinProviders`: for every `AGENT_PROVIDER_DEFINITIONS` (+ dev mocks if `isDev`): `resolveRegisteredProvider` merges `runtimeSettings[id]` with `toRuntimeSettings(override)` (override.command => `{mode:"replace",argv}`; env merged shallow; `disallowedTools` concatenated), label/description override, `enabled = override.enabled ?? definition.enabledByDefault ?? true`, `providerOptions = override.options ?? override.params`. Contract: `PROVIDER_CONTRACTS` gives `supportsExactMcpPreapproval:true` for claude, codex, opencode only.
2. Plugin providers (`options.pluginProviders`): id conflicting with built-in => throw "Plugin provider 'X' conflicts with a built-in provider"; `requiresConfiguration && !override` => skipped; override with `extends` => warn "Plugin provider shadowed by configured provider" and skipped; else resolved like built-in with `contract.supportsExactMcpPreapproval` from the plugin definition (always true for plugins).
3. `addDerivedProviders`: for each override id not already built-in/resolved:
   - no `extends` => ignored (a plugin override may precede registration);
   - `extends:"acp"` => needs `command` (non-empty); definition `{id,label (default id),description (default "Custom ACP provider"), defaultModeId:null, modes:[]}`; client class by id: `cursor`->`CursorACPAgentClient`, `kimi`->`KimiACPAgentClient`, `kiro`->`KiroACPAgentClient`, `traecli`->`TraeACPAgentClient`, otherwise `GenericACPAgentClient`; `enabled = override.enabled !== false`; contract unsupported except `hub-e2e`.
   - `extends:<builtin>` => `baseProvider = resolvedProviders.get(extends)` else throw "Custom provider X extends unknown provider Y"; definition = base definition with new id/label (label required)/description; runtime settings = base merged with override; `providerOptions = override options ?? base options`; client = base factory called with `customProvider {id,label,extends}` (Codex uses it to build a `model_providers` entry, section 7); `derivedFromProviderId = extends`; contract inherited.
4. `createRegistryEntry` builds the `ProviderDefinition` (`:68`): `configuration` (effective inputs, used to detect config changes by deep equality), `enabled`, `derivedFromProviderId`, `supportsExactMcpPreapproval`, `applyToolPolicy(config, toolPolicy)` (throws `ToolPolicyUnsupportedError` if policy given and contract lacks support), `createClient(logger)`, `resolveCreateConfig` (client's own or `resolveDefaultAgentCreateConfig`, section 6), `isCreateConfigUnattended` (client's or default), `fetchCatalog(options, client?, context?)`.

### 4.3 Model merge rules (`mergeModels`, `mergeModelAdditions`, `:371-436`)
- `models` (profile models) with `profileModelsAreAdditive=false` REPLACE runtime discovery: runtime model discovery is skipped, `additionalModels` merged on top; if provider has static modes the registry only probes `resolveDefaultModeId`, else it still calls `fetchCatalog` to get dynamic modes (`hasReplacementModels` branch in `createRegistryEntry`).
- Otherwise runtime models + `[...models, ...additionalModels]` merged by id: existing entry is spread-overridden (`{...existing,...addition}`), and a model that was `isSelectable:false` (compat model) becomes `isSelectable:true` when the addition does not state `isSelectable`.
- If any addition has `isDefault:true`, every other model gets `isDefault:false`.
- `client.resolveConfiguredModel(model)` lets a provider fill defaults for configured models (Claude: `resolveConfiguredClaudeModel` adds manifest thinking options, or `getClaudeCustomModelThinkingOptions()` for unknown ids).
- `decorateModes`: modes lacking `icon/colorTier` inherit them from the static manifest mode with the same id.

### 4.4 Client wrapper (`wrapClientProvider`, `wrapSessionProvider`)
Registry wraps every client so that the Paseo provider id (possibly a derived id) replaces the inner provider id on persistence handles, runtime info, stream events (`mapStreamEvent`) and models; `createSession/resumeSession` receive `providerOptions = mergeProviderOptions(configuredProviderOptions, config.providerOptions)` and `configuredModelIds`; `fetchCatalog` passes configured `providerOptions` and merges models. `ForwardedAgentSession` type forces every `AgentSession` member to cross the wrapper.
Helpers: `getProviderIds`, `createAllClients`, `createClientsFromRegistry`, `shutdownAgentClients(clients, logger)` (parallel, errors logged "Provider client shutdown failed"). `PROVIDER_REGISTRY` constant is deprecated null.

### 4.5 Provider runtime (`S/agent/provider-runtime.ts`)
`createAgentProviderRuntime({paseoHome, logger, snapshotManager})` starts the **OpenCodeBridge** first (HTTP server for Paseo tools in OpenCode; port from `runtimeSettings.opencode.env.PASEO_OPENCODE_BRIDGE_PORT`, must be integer 1..65535 else throws) then builds the `ProviderSnapshotManager` with `openCodeBridge`. Returns `{snapshotManager, setPaseoToolCatalog(catalog) (-> bridge.setManifestCatalog), shutdown()}`; shutdown is idempotent: snapshotManager.shutdown then bridge.close.

### 4.6 Connections
Config (section 12) feeds overrides; snapshot manager (section 5) owns the registry generation; AgentManager receives `AgentManagerProviderState {providerDefinitions: {enabled, derivedFromProviderId, applyToolPolicy}, clients, retiredProviders?}` and must refuse disabled providers (`requireEnabledProvider`: "Provider 'X' is disabled") and unavailable ones with the message "Provider 'X' is not available. Reason: ... Available providers: a, b. Use one of those providers, or install/configure 'X'." (`agent-manager.ts:5689`).

---

## 5. Provider snapshot manager (`S/agent/provider-snapshot-manager.ts`) - catalog cache + change feed

### 5.1 What it does
Single owner of provider availability + model/mode catalogs per "target" (global or per workspace cwd), with coalescing, timeouts, per-provider concurrency limit, content hashes and a change event. Exposed to clients through the providers-snapshot protocol (section 14) and to the daemon through methods below.

### 5.2 Constants
`DEFAULT_REFRESH_TIMEOUT_MS = 120_000`; `MAX_REFRESH_TIMEOUT_MS = 2_147_483_647`; `DEFAULT_DIAGNOSTIC_TIMEOUT_MS = 120_000`; env override `PASEO_PROVIDER_REFRESH_TIMEOUT_MS` (valid positive safe integer <= max, else default); config `agents.catalogRefreshTimeoutMs`; `GLOBAL_PROVIDER_SNAPSHOT_KEY = "paseo:global"`; per-provider discovery concurrency `pLimit(4, rejectOnClear)`. Diagnostic timeout = `max(refreshTimeout, 120000)` unless given. Claude model-discovery inner timeout 5_000ms.

### 5.3 Data model
`ProviderSnapshot {cwd, records: ProviderSnapshotRecord[]}`; `ProviderSnapshotRecord {entry: ProviderSnapshotEntry, contentHash}` where `contentHash = sha256(JSON(["paseo.provider-result/1", entry without fetchedAt])) base64url`.
`ProviderSnapshotEntry {provider, status, enabled, source?: "builtin"|"custom", error?, models?, modes?, fetchedAt?, label?, description?, iconSvg?, defaultModeId?}`.
Initial entry per provider (before any discovery): `status = enabled ? "loading" : "unavailable"`, `source = (plugin or non-builtin with extends) ? "custom" : "builtin"`, label/description/iconSvg from definition, `defaultModeId = definition.defaultModeId ?? null`.
Targets: key = `"paseo:global"` when no cwd (catalog scope `global`) or `resolveSnapshotCwd(cwd)` (absolute; `~` expanded; Windows drive normalised; empty => `homedir()`), scope `workspace`. Internal: `catalogs: Map<key, Map<provider, {result?, stale?, load?}>>` shared across targets via the client's `getCatalogCacheKey` (Claude and Codex return `"host"`, so every cwd shares ONE catalogue; providers without a key use `["target", snapshotCwd]`); `targets: Map<cwd, {bindings, snapshot}>`.

### 5.4 Public methods
`getSnapshot(cwd?)` (non-blocking: returns current snapshot and fires background warm-up for providers still `loading`), `listProviders({cwd?, providers?, wait?})` (wait => await warm-up), `getProvider`, `listModels` (selectable only; throws if provider disabled/error/unavailable), `listModes`, `resolveDefaultModel({provider, requestedModel?, cwd?})` (trimmed request, else default model, else first; swallows errors -> undefined), `resolveCreateConfig({cwd, provider, requestedMode, featureValues, parent: ManagedAgent|null, unattended})` (calls `definition.resolveCreateConfig` with `availableModes` from the ready entry and `unattended = unattended || parent.isUnattended`), `validateAgentConfiguration({provider, model?, modeId?, thinkingOptionId?, providerOptions?})` (used by Hub execution: issues `[{path:["provider"|"model"|"modeId"|"thinkingOptionId"], message}]`, messages "Provider 'X' is not configured", "... is disabled", "... is not available", "Model 'M' is not available for provider 'X'", "Mode ...", "Thinking option ... "; model matched by id or alias, default/first when none requested; thinking option must exist on the selected model; see `agent-configuration-validator.ts`), `getProviderDiagnostic(provider)`, `refreshSnapshotForCwd({cwd, providers?})` (force), `refreshSettingsSnapshot({providers?})` (force the global target, marks every cached catalogue stale, then re-warms every known workspace target), `warmUpSnapshotForCwd({cwd?, providers?})` (non-forced), `listRegisteredProviderIds/hasProvider/supportsConfigRequiredProvider/getProviderLabel`, `setRefreshTimeoutMs`, `on/off("change", listener(transition {previous,current}))`, `shutdown()` (destroys, materializes a client per enabled provider then `shutdownAgentClients`), `destroy()`.
Plugin hooks: `replacePluginProviders(registrations) -> AgentManagerProviderState {…, retiredProviders}` (rejects ids that collide with built-ins or `extraClients`, replaces plugin registry, rebuilds registry, only providers whose registration or effective `configuration` changed get new clients, caches/bindings dropped and re-warmed), `settlePluginProviders()` (after plugins have loaded: warns "Provider override matches no registered provider" for override entries with no `extends` and no provider).
Config hooks: `applyMutableProviderConfig`/`prepareMutableProviderConfig(mutableProviders, {removeProviders?, replace?})` returns `{agentManagerState, commit()}` so the caller can swap AgentManager state first and commit atomically (section 12).

### 5.5 Refresh pipeline (`loadProvider -> resolveCatalog -> refreshProvider`)
1. Disabled providers: nothing is loaded (stay `unavailable`).
2. Cache key: `client.getCatalogCacheKey(options)` with timeout (error => binding.failure `status:"error"` with message `Timed out resolving X catalogue key` / error text; published).
3. If a catalogue already has a non-stale result (or a load in flight) and `force=false` the load is reused; otherwise `discoveryLimit` runs `refreshProvider`.
4. `refreshProvider` = `runProviderRefreshWithDeadline({label: definition.label, timeoutMs: refreshTimeoutMs, ...})` running `client.isAvailable(signal, catalogOptions)` as activity `availability`; false => entry `{...base, status:"unavailable", enabled:true}`; true => `definition.fetchCatalog(...)`; result normalised with `normalizeAgentModelCatalog` (warn "Provider catalog contains repeated model IDs; retaining the first definition") and published `status:"ready", models, modes, defaultModeId (catalog value if !== undefined), fetchedAt: now`; any throw => `status:"error", error: toErrorMessage`. Timeout error text: `Timed out refreshing <label> after <ms>ms; pending: a, b` (`provider-refresh-deadline.ts`).
5. `publishTargets` recomputes each bound target's records; emits `change` only if `sameSnapshotRecords` is false (same provider order, equal `contentHash`, equal `fetchedAt`). Subscriber errors are caught and logged.
Superseded loads are discarded via `isCurrent()` checks (generation counters per binding/catalogue).

### 5.6 Diagnostics (`getProviderDiagnostic`)
Runs in parallel `client.getDiagnostic()` (timeout message `Timed out collecting <label> diagnostic after <ms>ms`; missing method => "Diagnostic: No diagnostic available"; error => `formatProviderDiagnosticError`) and a FORCED global refresh of that provider; appends `"\n  Models: <count|—>\n  Status: Ready|Error: ...|Unavailable|Loading"`. Unknown provider => `Error: Provider X is not configured`. Formatting helpers in `providers/diagnostic-utils.ts` (`formatProviderDiagnostic(name, rows)`, `buildCommandResolutionDiagnosticRows`, `buildBinaryDiagnosticRows` incl. Version, `toDiagnosticErrorMessage`).

### 5.7 Extra clients
`extraClients` (tests/embedding) replace the registry entry's client and catalogue.

---

## 6. Create-config resolution: modes, unattended, inheritance (`S/agent/create-agent-mode.ts`)

Used by `resolveCreateConfig` for every new agent, including MCP `create_agent` child agents (section 22) and schedules.
`resolveAndValidateCreateAgentMode({requestedMode, targetProvider, parent, unattended, availableModes, targetUnattendedMode})`:
1. explicit `requestedMode`: validated against `availableModes` when known (error `Invalid mode 'X' for provider 'P'. Available modes: a, b` or `unknown`/`(none)`), else passed through.
2. no parent: `unattended && targetUnattendedMode` => that mode; else undefined (provider default).
3. same provider as parent: inherits parent's `modeId`.
4. parent unattended (or `unattended`) and target has an `isUnattended` mode => that mode (cross-provider bridging: e.g. a Claude bypass parent spawns Codex at `full-access`).
5. target has zero modes => undefined; else throws `cannot inherit mode 'M' from caller (provider 'P') for new agent (provider 'T'). Pass an explicit mode. Available modes for 'T': ...`.
`isDefaultAgentCreateConfigUnattended`: mode is unattended iff the matching mode has `isUnattended`.
Provider overrides: OpenCode (`resolveOpenCodeCreateConfig`: legacy mode id `full-access` => `build` + `auto_accept=true`; unattended parents set feature `auto_accept=true` and leave the mode unset so OpenCode picks its own default agent; `isOpenCodeCreateConfigUnattended` true when `auto_accept === true`), ACP (`resolveACPCreateConfig`: unattended create sets `featureValues.auto_accept=true`; `isACPAutoAcceptEnabled`).

---

## 7. Provider: Claude (`providers/claude/`)

- **Transport**: Claude Agent SDK `query()` driving the `claude` CLI binary (`pathToFileURL` path from `resolveClaudeBinary(runtimeSettings)`; default binary `claude`). Class `ClaudeAgentClient` (`agent.ts:1597`), session `ClaudeAgentSession`.
- **Availability**: `resolveProviderLaunch({defaultBinary:"claude"})` + executable check. Catalog cache key `"host"`.
- **Models** (`model-manifest.ts` `CLAUDE_MODEL_MANIFEST`): curated manifest filtered by Claude Code version (`minimumClaudeCodeVersion`, compared with `parseClaudeCodeVersion`) and discovery (`requiresDiscovery:true` models, e.g. Mythos 5.2/5.1, shown only if `discoverClaudeModels` (SDK `supportedModels()`, 5s timeout, no persisted session, no MCP, no hooks, `settingSources:["user"]`) returns them). Entries include (id: label, ctx tokens): `claude-opus-5-5` Opus 5.5 (1M, fast), `claude-opus-5`, `claude-mythos-5-2`/`5-1`, `claude-fable-5-2` ("Most powerful model")/`5-1`/`5` (alias `claude-fable-5[1m]`), `claude-opus-4-8[1m]`/`claude-opus-4-8`, `claude-sonnet-5-5`, `claude-sonnet-5`/`[1m]`, `claude-opus-4-7[1m]`/`4-7`, Opus 4.6, `claude-sonnet-4-6[1m]`/`claude-sonnet-4-6`, `claude-haiku-5-5`, `claude-haiku-4-5`. The default model is the manifest entry with highest `defaultPriority` among the available ones. Alias rows become `isSelectable:false` duplicates (COMPAT claudeFable5LegacyCatalogEntry, remove after 2027-02-06). Extra models are read from the Claude config dir `settings.json` keys `model` and `env.{ANTHROPIC_MODEL, ANTHROPIC_SMALL_FAST_MODEL, ANTHROPIC_DEFAULT_FABLE_MODEL, ..._OPUS_..., ..._SONNET_..., ..._HAIKU_...}` (description "From Claude settings.json <key>").
- **Thinking options** per model (`buildThinkingOptions`): effort levels `low, medium, high, [xhigh,] max` (labels Low/Medium/High/Extra High/Max); `off` ("Off", id `CLAUDE_DISABLED_THINKING_OPTION_ID="off"`) only for models with `supportsThinkingDisabled`; `ultracode` ("Ultra Code") appended when `xhigh` exists; default `CLAUDE_DEFAULT_THINKING_OPTION_ID="high"` unless the entry sets `defaultThinkingOptionId` (e.g. `medium` for Opus 5.5, Fable 5.2, Sonnet 5.5, Haiku 5.5). `setThinkingOption`: `null`/`"default"` clears; unknown id throws `Unknown thinking option`; applies on next query restart; returns `THINKING_APPLIES_NEXT_TURN_NOTICE` if a turn is active. Changing model reconciles `off` to the model's fallback and pushes a `thinking_option_changed` event.
- **Modes**: the 5 in section 2.3. `claudeModeCatalog(env)` removes `auto` and defaults to `default` when `CLAUDE_CODE_USE_BEDROCK` or `CLAUDE_CODE_USE_VERTEX` is truthy; `setMode("auto")` then throws "Claude Auto mode requires the Anthropic API ...". `setMode` validates against `VALID_CLAUDE_MODES`, applies SDK `setPermissionMode`; entering `plan` remembers `planResumeMode`.
- **Features** (`feature-definitions.ts`): toggle `fast_mode` {label "Fast", description "Lower latency responses at higher token cost", tooltip "Toggle fast mode", icon `zap`}, only for manifest models with `supportsFastMode`; `setFeature("fast_mode")` -> SDK `applyFlagSettings({fastMode})`; unsupported model throws `Claude fast mode is not available for model ...`; switching to a non-fast model turns it off.
- **Slash commands** (`listCommands`, `agent.ts:3076`): SDK `supportedCommands()`; kind = `command` for the root-only set `{clear, compact, context, debug, extra-usage, heapdump, init, loop, schedule, usage}` else `skill`; always adds `/rewind` ("Rewind tracked files to a previous user message", hint `[user_message_uuid]`); sorted by name.
- **Rewind**: `revertConversation/revertFiles/revertBoth` (`claude/rewind.ts`).
- **Import** (`listImportableSessions`, 45s deadline, scanLimit<=500, LRU descriptor cache max 500 / 8 MB; reads `<claudeConfigDir>/projects/...` jsonl; adopts `/rename` titles via `readClaudeRenamedSessionTitle`).
- **Diagnostic**: rows from command resolution, binary/version, and `Auth` (from `resolveClaudeAuth`).
- **Custom providers**: `extends:"claude"` + `env` (e.g. `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN` for Z.AI/other gateways, see docs/custom-providers.md) + `models`. Subagent-related: `claude/subagents/*`, `sidechain-tracker`, `subagent-handback` (section 20).

---

## 8. Provider: Codex (`providers/codex-app-server-agent.ts`, `providers/codex/`)

- **Transport**: spawns `codex app-server` (JSON-RPC over stdio, `jsonl-rpc-process.ts`, `codex/app-server-transport.ts`); `initialize` with client name `codex_app_server_daemon` (non-originating, keeps Codex's CLI identity in usage logs); request timeouts `TURN_START_TIMEOUT_MS = 90_000`, `INTERRUPT_TIMEOUT_MS = 2_000`. Binary resolution via `resolveCodexLaunch(runtimeSettings)` (default `codex`).
- **Catalog**: `fetchCatalog` spawns a throwaway app-server: `initialize` -> `model/list` -> `config/read` (configured default model + reasoning effort). Models: id = Codex model id, label via `normalizeCodexModelLabel`, thinking options from `supportedReasoningEfforts` (id normalised by `normalizeCodexThinkingOptionId`, label = id, description), default effort = configured effort ?? model default; `isDefault` = configured default model if it exists, else Codex's flag; `metadata {model, defaultReasoningEffort, supportedReasoningEfforts}`. Cache key `"host"`.
- **Modes**: `auto` / `auto-review` / `full-access` mapped to `MODE_PRESETS`: `auto` = approval `on-request` + sandbox `workspace-write` + reviewer `user`; `auto-review` = same but reviewer `auto_review`; `full-access` = approval `never` + `danger-full-access`; (`read-only` preset exists internally). `auto-review` is only offered when Codex >= `CODEX_AUTO_REVIEW_MIN_VERSION [0,115,0]` (`resolveAutoReviewEnabled`); default mode is `auto-review` when offered else `auto`. `supportsDynamicModes:false`. `setMode` returns `MODE_APPLIES_NEXT_TURN_NOTICE` ("Permission mode applies next turn") during a turn.
- **Thinking**: `setThinkingOption` -> next turn (`THINKING_APPLIES_NEXT_TURN_NOTICE` "Thinking level applies next turn").
- **Features** (`codex-feature-definitions.ts`): select `service_tier` {label "Speed", description "Choose processing speed. Faster tiers increase usage.", tooltip "Select speed", icon `zap`, `desktopTrigger:"icon"`, options `default`="Normal" + model `serviceTiers` from `model/list`}; toggle `plan_mode` {label "Plan", description "Switch Codex into planning-only collaboration mode", icon `list-todo`} shown only if `collaborationMode/list` yields a plan-like mode. COMPAT: `fast_mode` boolean maps to the tier named "fast" (remove 2027-03-29).
- **Slash commands** (`listCommands`): builtin `compact` (kind command, "Summarize conversation to prevent hitting the context limit"), `goal` (hint `[<objective>|pause|resume|clear]`, only when Codex >= `[0,128,0]`, enables `--enable goals`), app-server skills (`kind:"skill"`, fallback filesystem scan `listCodexSkills(cwd, codexHome, workspaceGitService)`), custom prompts (`listCodexCustomPrompts(codexHome)`). Handled out-of-band via `tryHandleOutOfBand` (`/compact`, `/goal ...`).
- **Plan approval**: a permission of kind `plan`; approving yields `followUpPrompt` starting "The user approved the plan. Implement it now. ..." (`CODEX_PLAN_IMPLEMENTATION_PROMPT_PREFIX`).
- **Rewind**: conversation only; `thread/rollback` removed in Codex 0.156.0 (`CODEX_THREAD_ROLLBACK_REMOVED_VERSION`) so rewind forks before the target turn there.
- **Native archive**: `thread/archive` / `thread/unarchive` through a throwaway app-server (`archiveNativeSession`).
- **Custom provider (`extends:"codex"`)**: if `env.OPENAI_BASE_URL` is set, `buildCodexCustomProviderConfig` writes `model_provider = "paseo:<id>"` and `model_providers["paseo:<id>"] = {name: label, base_url: <url, "/v1" appended>, wire_api:"responses", env_key:"OPENAI_API_KEY", requires_openai_auth:false (when key present)}`.
- **Subagents**: child threads are tracked as sub-agent tool calls (`MAX_PENDING_SUB_AGENT_THREADS = 32`, `MAX_PENDING_SUB_AGENT_NOTIFICATIONS_PER_THREAD = 128`; section 20).
- **Diagnostic**: command resolution + binary rows.

---

## 9. Provider: OpenCode (`providers/opencode-agent.ts`, `providers/opencode/`, `opencode/v2/`)

- **Runtime selection** (`OpenCodeRuntimeClient`, `opencode/runtime-client.ts`): explicit external server (`resolveExternalOpenCodeServer(env)`) => v2 client; else `opencode --version` (5s) - major 1 => legacy `OpenCodeAgentClient`, otherwise `OpenCodeV2AgentClient`; probe failure falls back to legacy but is not cached. Sessions created by v2 get a timeline notification "This chat uses OpenCode v<major>." stored in persistence metadata `openCodeRuntimeNotices`.
- **Legacy client** (`OpenCodeAgentClient`): owns a shared `OpenCodeServerManager` (spawns `opencode serve`, SSE `OpenCodeEventConsumer`), uses `@opencode-ai/sdk` v1.14 types.
- **Catalog**: models from `provider.list` (`id = "<providerId>/<modelId>"`, label = model name, description `"<providerName> - <family>"`, `metadata {providerId, providerName, modelId, family, releaseDate, supportsAttachments, supportsReasoning, ...}`, context window from `limit.context`); thinking options = the model's `variants` (a synthetic first option id `OPENCODE_DEFAULT_VARIANT_ID` label "Default" which omits `variant`); modes from `app.agents` filtered by `isSelectable` then `mergeOpenCodeModes`; discovery failure returns `[]` modes (never fabricated). `options.force` => `acquireNew` server else `acquireCurrent`; global scope uses an opencode-home directory (`openCodeCatalogDirectory`).
- **Feature**: toggle `auto_accept` {label "Auto Accept", description "Automatically approves OpenCode tool permission prompts.", icon `shield-check`}. Permission actions: Deny / Allow always (`OPENCODE_PERMISSION_ACTION_ALLOW_ALWAYS`) / allow once.
- **Commands**: `listOpenCodeCommandsFromSdk` (OpenCode `command.list` + skills).
- **Import/archive**: `session.list` via `collectOpenCodeImportableSessionsFromSdk`; archive sets `time.archived = Date.now()` (0 = unarchived).
- **Paseo tools**: `OpenCodeBridge` (`opencode/bridge.ts`) exposes the Paseo tool catalog to OpenCode through an internal HTTP API under `/_internal/opencode` (bearer token `randomBytes(32)`, `MAX_REQUEST_BYTES = 1 MiB`) and a plugin file `bridge-plugin.mjs` injected in OpenCode config `plugin`; per-session bindings (`bindSession({sessionId, env, tools})`). This is how OpenCode agents get `create_agent` etc. without an MCP HTTP endpoint.
- **Diagnostic**: `OpenCode server: <url>` when external; else binary rows.

---

## 10. Providers: ACP agents (`providers/acp-agent.ts`, `generic-acp-agent.ts`, cursor/copilot/kimi/kiro/trae)

### 10.1 Base `ACPAgentClient` (provider id passed in; speaks Agent Client Protocol via `@agentclientprotocol/sdk`)
Constants: `ACP_DIAGNOSTIC_PHASE_TIMEOUT_MS = 20_000`, `ACP_PROBE_CLOSE_TIMEOUT_MS = 2_000`, `ACP_CLOSE_REQUEST_TIMEOUT_MS = 2_000`, `ACP_IMPORT_HISTORY_LOAD_TIMEOUT_MS = 30_000`, `ACP_IMPORT_HISTORY_BUDGET_MS = 60_000`, `COMMAND_INVENTORY_QUIET_MS = 600`, `COMMAND_INVENTORY_POLL_MS = 25`.
- **Catalog**: `fetchCatalog` spawns a probe process, `initialize`, `session/new` with `mcpServers: []` at `cwd` (home for global), derives: models from `SessionModelState.availableModels` (`currentModelId` => `isDefault`) or from select config option category `model`; thinking options from select config option category `thought_level`; modes from `SessionModeState` or select option category `mode`, else the provider's static `defaultModes`. Optional `catalogModelResolver` refines models (Cursor/Kimi). Probe session is closed (`closeProbe`) even on abort.
- **Features**: always `auto_accept` toggle ("Auto Accept", "Automatically approves ACP permission prompts."); plus select features derived from `configFeatureOptions` (`ACPConfigFeatureOption {id, configId, category?, label, description, tooltip, icon, emptyOptionLabel}`) read from a probe `session/new` config options.
- **Commands**: from ACP `available_commands_update` notification; `waitForInitialCommands` waits for the first batch (quiet period + per-provider timeout). `extensionCommandsParser(method, params)` for vendor extensions.
- **Import**: `session/list` (only if agent advertises `sessionCapabilities.list`) + optional `loadSession` history for previews, scanLimit<=500.
- **Capabilities override** from provider options (`supportsMcpServers`, `clientCapabilities.fs/terminal`).
- **Mode/thinking writers**: `providerModeWriter`, `beforeModeWriter`, `thinkingOptionWriter`, `modeIdTransformer`, `sessionResponseTransformer`, `configOptionsTransformer`, `toolSnapshotTransformer` hooks let provider subclasses adapt.
### 10.2 Generic ACP (`GenericACPAgentClient`, provider `acp`; config `extends:"acp"` + `command`)
Command fixed from config (`resolveLaunchCommand`); `isAvailable` = launcher binary on PATH. `getDiagnostic` rows: Provider ID, Configured command, Launcher binary (+ version via `buildVersionProbeCommand`: package runners `npx|bunx|pnpm|uvx` keep their package spec prefix then `--version`), Version command, ACP probe rows (`buildACPProbeDiagnosticRows`, phase timeout 20s). Label format `"<label> (ACP)"`.
### 10.3 Specialised ACP clients
- **Copilot** (`copilot-acp-agent.ts`, built-in id `copilot`): default command `["copilot","--acp"]`; synthetic `allow-all` mode implemented by writing config option `allow_all` = `on`/`off` and switching the ACP mode to Agent first (`COPILOT_ALLOW_ALL_*`); the old ACP `autopilot` mode id is mapped to Agent (COMPAT copilotAutopilotMode, removal 2026-11-12); select feature `agent` ("Use a Copilot custom agent profile", category `_agent`, empty option "Default").
- **Cursor** (`cursor-acp-agent.ts`): default command `["cursor-agent","acp"]` (or `command.argv` when replaced); `waitForInitialCommands`, 10s; client capability meta `{parameterizedModelPicker:true}`; select feature `fast` ("Cursor fast mode"); model catalog via ACP extension `cursor/list_available_models` (error -32601 => "Update Cursor CLI: this version does not support cursor/list_available_models."; avoids switching models because that persists CLI prefs).
- **Kimi**: catalog resolver switches the probe session's model one by one (`setSessionConfigOption`) to read per-model thinking options; failures keep/omit options with warnings.
- **Kiro**: reads commands/skills from extension notification `_kiro.dev/commands/available` (`commands` names with leading `/` stripped, `prompts`; `serverName` startsWith `skill:` => kind skill; hint from `meta.hint`), wait 10s.
- **Trae** (`traecli`): wait for `available_commands_update`, 10s.
### 10.4 ACP provider catalog in the app (`A/data/acp-provider-catalog.ts`, `A/hooks/use-acp-provider-catalog.ts`)
Static list (~41 entries, e.g. `agoragentic-acp`, `amp-acp`, `auggie`, `autohand`, `cline`, `gemini`, ... ) `{id,title,description,version,iconSvg,installLink,command: [string,...string[]], bundledProvider?, env?, params?}`. Installing = `patchConfig({providers: {[id]: {extends:"acp", label:title, description, command, env (default {}), params?}}})` (`buildAcpProviderConfigPatch`); entries with `bundledProvider:true` (Hoplite) instead enable a bundled adapter: `{providers:{[id]:{enabled:true}}}` and require server feature `hopliteProvider` ("Update the host to add Hoplite."). See section 16 for the UI.

---

## 11. Providers: Pi, Oh My Pi (OMP), mock

### 11.1 Pi (`providers/pi/`) - minimal terminal coding agent
RPC process (`pi/runtime.ts`, `cli-runtime.ts`, `rpc-types.ts`); default binary `PI_BINARY_COMMAND`. `supportsMcpServers:false`. Models from RPC `get_available_models` + `get_state` (the configured model gets `isDefault`; `transformPiModels`, `mapPiModel`), modes `[]`. Thinking options fixed: `off, minimal, low, medium, high, xhigh, max` (`PI_THINKING_OPTIONS`). Features `[]`. Commands: Pi RPC `getCommands()` mapped by `mapPiSlashCommands`, handled-builtin set includes `compact` ("Manually compact the session context", hint `[instructions]`), `autocompact`, out-of-band. Diagnostic includes `Auth config (~/.pi/agent/auth.json)` found/not found. Session dir option `sessionDir`; usage poller `pi/usage-poller.ts`.
### 11.2 OMP (`providers/omp/`, id `omp`, label "Oh My Pi", `enabledByDefault:false`)
Default binary `omp`, `MIN_SUPPORTED_OMP_VERSION = "16.3.9"`, requires Bun >= 1.3.14 for npm installs. Launch mode flags: `full` => `--approval-mode yolo`, `write` => `--approval-mode write`, `ask` => `--approval-mode always-ask`, plus model role args from options `smolModel/slowModel/planModel`. Protocol mode `rpc-ui` (UI questions mapped to permissions via `question-ui.ts`, `rpc-ui-permission-mapper.ts`), host tools bridge (`host-tools.ts`, `mcp-bridge.ts`: native approvals, host tools and subagents), `subagent-card-tracker`, `subagent-index`. Catalog: `get_available_models` (`mapOmpModel`/`transformOmpModels`) + static `OMP_MODES`. Diagnostic rows: version support, active profile, config root, agent dir, agent DB found, XDG roots, Bun runtime. Commands `mapOmpRuntimeSlashCommands`, builtin out-of-band `compact`, `autocompact`.
### 11.3 Mock providers (dev only)
`MockLoadTestAgentClient` (id `mock`; synthetic markdown/reasoning/tool traffic for app stress tests) and `MockSlowProviderClient` (id `mock-slow`; hangs in catalog to test timeouts). Registered only when `isDev`.

---

## 12. Plugin providers (`S/agent/plugin-provider.ts`, `packages/plugin/src/server/provider.ts`)

### 12.1 What
A trusted local Paseo plugin can register provider(s) via `ProviderRegistration {id, label, description?, icon? (plugin-relative SVG path), requiresConfiguration?, command?: readonly [string,...string[]], getCatalogCacheKey?, status?(req), connect(req) -> ProviderConnection}`. The daemon adapts it with `PluginAgentClientRegistry` (`replace(registrations)`, `definitions()`, `clients()`, `has`, `shutdown`; duplicate ids throw "Duplicate plugin provider ID: X"; replacing stops clients whose registration object changed). Registration is pushed in by `bootstrap.ts:1031` (`providerSnapshotManager.replacePluginProviders(pluginRuntime.getProviderRegistrations())`).
Definition for registry (`createPluginProviderDefinition`): `defaultModeId:null, modes:[], supportsExactMcpPreapproval:true`, description default "Plugin provider <label>", `iconSvg = icon`.
### 12.2 Wire protocol between daemon and plugin (`PROVIDER_PROTOCOL_VERSION = 1`)
`ProviderConnection {version, capabilities, send(ProviderInput), onEvent(listener), close()}`.
Capabilities (`PROVIDER_CAPABILITIES`): `prompt.message, prompt.command, prompt.image, prompt.output_schema, prompt.steer, session.can_stop_runtime, session.archive, session.configure, session.list, session.persistence, session.revert.both, session.revert.conversation, session.revert.files, session.subsession, session.unarchive, permission, permission.tool_policy, timeline.plugin`. `negotiateProviderCapabilities`, `requireProviderCapabilities` reject inputs whose capability was not negotiated. They map to `AgentCapabilityFlags` via `agentCapabilities()`.
Inputs (`ProviderInputSchema`): `catalog{requestId,cwd?}`, `sessions{...}`, `session.open`, `session.prompt{sessionId,prompt}` (prompt `{delivery?:"steer", input: {type:"message",content[]}|{type:"command",name,arguments}, outputSchema?}`; content parts: text, image, forge_change_request, forge_issue, github_pr, github_issue, text attachments, review, uploaded_file), `session.interrupt`, `session.permission`, `session.configure`, `session.revert`, `session.archive`, `session.unarchive`, `session.can_stop_runtime`, `session.close`.
Events (`ProviderEventSchema`): `catalog{requestId, catalog {models, modes, thinkingOptions?, defaultModel?, defaultMode?, defaultThinkingOption?}}`, `sessions`, `request.completed`, `request.failed{error}`, `session.opened`, `session.ready`, `session.can_stop_runtime`, `session.closed`, `session.runtime_failed`, `session.persistence`, `session.prompt_result` (`turn|steer|completed|failed`), `session.turn`, `session.usage`, `session.config{config: {model?, mode?, thinkingOption?, models, modes, thinkingOptions, settings[]}}` (settings = toggle/select = features), `session.commands`, `session.permission`, `session.permission_resolved`, `session.notice`, `timeline.item` (shell/read/edit/write/search/fetch/worktree_setup/sub_agent/plain_text/plan/unknown tool details and user/assistant/reasoning/todo/error/notification/compaction/plugin items).
`ProviderLaunch {command,args,env}` is the daemon-owned resolved launch (config `command` override + env stripped of parent-session vars) given to `status`/`connect`/catalog.
Config: plugin providers appear in `agents.providers` only for overrides (enable/disable, `env`, `command`, `options`, `models`); `requiresConfiguration:true` plugins load only when an `agents.providers.<id>` entry exists. Subsessions (`session.subsession`) are adopted as child agents by `PluginAgentClient.acceptChild` (`pendingChildren`).
`plugin-session-history.ts` replays history for plugin sessions.

---


## Cross-area connections

- Agent lifecycle and creation use provider/model/mode discovery, resolved launch configuration and capability validation; config changes and plugin registrations rebuild provider registry/catalog state.
- Provider sessions emit timeline, usage and permission/question control events; plan responses can launch implementation turns. Provider modes determine unattended behavior and exact MCP preapproval.
- Composer consumes provider commands and skills; importable sessions feed history/import/resume; persistence handles and native archive/rewind capabilities connect to lifecycle and timeline replacement.
- Provider subagent/subsession adoption connects parentage, workspace placement and timeline cards. Host provider settings expose discovery errors, model availability and diagnostics.
- Plugins negotiate provider capabilities and subprocess messages; host config controls enablement/env/options/catalog overrides. Missing profile CRUD/application is in ../gaps.md; bundled orchestration skill management is in ../.area6-parts/A.md.

## Inventory

- Provider model: AgentProvider, AgentClient, AgentSession (the contract every provider implements)
- `AgentClient` (one per provider id, long lived, `S/agent/agent-sdk-types.ts:764-830`)
- `AgentSession` (one per running agent, `:677-732`)
- `AgentCapabilityFlags` (`P/agent-types.ts` / `agent-sdk-types.ts:169`)
- Built-in provider list and mode tables (the provider manifest)
- Built-in providers (exact ids, defaults)
- Mode tables (id: label - colorTier [icon], `*` = isUnattended)
- Protocol / CLIENT_CAPS gating for older clients (port only if wire compat with old apps matters)
- Provider launch config: command, env, disallowedTools (provider-launch-config)
- Schemas (`P/provider-config.ts`, re-exported by `S/agent/provider-launch-config.ts`)
- Resolution helpers (`S/agent/provider-launch-config.ts`)
- `provider-options` (`S/agent/provider-options.ts`)
- Provider registry (`S/agent/provider-registry.ts`)
- Build algorithm (`buildProviderRegistry`, `:908`)
- Model merge rules (`mergeModels`, `mergeModelAdditions`, `:371-436`)
- Client wrapper (`wrapClientProvider`, `wrapSessionProvider`)
- Provider runtime (`S/agent/provider-runtime.ts`)
- Provider snapshot manager (`S/agent/provider-snapshot-manager.ts`) - catalog cache + change feed
- What it does
- Refresh pipeline (`loadProvider -> resolveCatalog -> refreshProvider`)
- Diagnostics (`getProviderDiagnostic`)
- Extra clients
- Create-config resolution: modes, unattended, inheritance (`S/agent/create-agent-mode.ts`)
- Provider: Claude (`providers/claude/`)
- Provider: Codex (`providers/codex-app-server-agent.ts`, `providers/codex/`)
- Provider: OpenCode (`providers/opencode-agent.ts`, `providers/opencode/`, `opencode/v2/`)
- Providers: ACP agents (`providers/acp-agent.ts`, `generic-acp-agent.ts`, cursor/copilot/kimi/kiro/trae)
- Base `ACPAgentClient` (provider id passed in; speaks Agent Client Protocol via `@agentclientprotocol/sdk`)
- Generic ACP (`GenericACPAgentClient`, provider `acp`; config `extends:"acp"` + `command`)
- Specialised ACP clients
- ACP provider catalog in the app (`A/data/acp-provider-catalog.ts`, `A/hooks/use-acp-provider-catalog.ts`)
- Providers: Pi, Oh My Pi (OMP), mock
- Pi (`providers/pi/`) - minimal terminal coding agent
- OMP (`providers/omp/`, id `omp`, label "Oh My Pi", `enabledByDefault:false`)
- Mock providers (dev only)
- Plugin providers (`S/agent/plugin-provider.ts`, `packages/plugin/src/server/provider.ts`)
- Wire protocol between daemon and plugin (`PROVIDER_PROTOCOL_VERSION = 1`)

- Provider command/env overlay and parent-session environment removal
- Exact MCP tool preapproval translation and unsupported-contract rejection
- Replacement vs additive model catalog merge
- Catalog scope cache key, timeout/loading/error and diagnostic refresh
- Claude supportedCommands skills, question mapping and native import titles
- Codex app-server discovery, version-gated auto-review and goal commands
- Codex plan approval follow-up implementation prompt
- OpenCode runtime version selection and auto_accept feature
- ACP auto_accept and configuration-derived select features
- Copilot custom agent/allow-all modes
- Cursor fast mode and parameterized model discovery
- Kimi per-model thinking discovery
- Kiro command/skill notification discovery
- Trae available_commands_update discovery
- ACP install catalogue config patches and bundled Hoplite gate
- Pi model/thinking/commands and OMP rpc-ui questions
- Plugin capability negotiation and provider subsession adoption
