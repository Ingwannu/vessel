# Settings screens (area 6, part D)

Paths relative to ~/refs/paseo-latest/packages unless noted. App = app/src.

## Settings screen shell and navigation
- Entry: route `app/src/app/settings/index.tsx`, `settings/[section].tsx`, `settings/hosts/[serverId]/[hostSection].tsx`, `settings/hosts/[serverId]/projects/[projectId].tsx`, `settings/hosts/[serverId]/plugins/[pluginId]/[screenId].tsx`. Screen: `screens/settings-screen.tsx` (1658 lines). `SettingsView` kinds: root | section | host | project | plugin (navigation/settings-navigation.ts:11). `returnFromSettings` goes back to last workspace (`navigateToLastWorkspace`) or open-project screen; plugin views return to host "plugins", project views to the projects list.
- Layout: left list with two groups, "App" (`SIDEBAR_SECTION_ITEMS`, settings-screen.tsx:165) and "Host" (`HOST_SECTION_ITEMS`, :243), a host picker (selected host persisted in the settings host selection store; `resolveActiveHostServerId`: view host -> picker choice -> connected local daemon -> first host), plus "back to workspace". Compact layout (mobile/narrow) uses push navigation per section, wide layout uses `router.replace`. Plugin settings screens appear as extra rows under the host group (`useInstalledPlugins().settingsScreens`).
- App sections (slug -> content): `general` (language, onboarding "setup" action, sending lives in `settings/general/sending-section.tsx`), `appearance` (AppearanceSection, part C), `sidebar` (SidebarNavSection, part C), `chat` (ChatSection), `terminal` (TerminalSection), `browser` (desktop only, BrowserDataSection), `editor` (web only, EditorSection), `files` (FilesSection), `shortcuts` (desktop only, KeyboardShortcutsSection, part C), `integrations` (desktop only, IntegrationsSection), `notifications` (desktop only, DesktopNotificationsSection), `permissions` (desktop only, DesktopPermissionsSection), `diagnostics`, `about`. Availability: `isSectionAvailable` (desktopOnly/webOnly flags).
- Host sections (slug): `host`, `projects`, `connections`, `pair-device`, `agents`, `metadata`, `voice`, `workspaces`, `providers`, `usage`, `terminals`, `plugins` (renderHostSettingsContent, :259). Host pages need the host connected for daemon-backed cards; otherwise "unavailable" empty cards (`settings.host.agents.unavailable`, `settings.host.workspaces.unavailable`).
- Settings search: not found (looked in settings-screen.tsx, components/settings/*).
- Reusable building blocks: components/settings/{index,headings,collapsible-row,detail-row,row-density,row-header}; styles/settings.

## App-wide settings store (`@paseo:app-settings`)
- Hook `useAppSettings`/`useSettings` (hooks/use-settings/index.ts): react-query key `["app-settings"]` (`APP_SETTINGS_QUERY_KEY`), `staleTime: Infinity`; persisted in AsyncStorage under `APP_SETTINGS_KEY="@paseo:app-settings"` (keys.ts); legacy blob `@paseo:settings` migrated; applied-migration list under `@paseo:settings-migrations` (migrations.ts). Unknown fields from newer builds are preserved on save. `Settings = AppSettings + {manageBuiltInDaemon, releaseChannel}` where those two come from the Electron desktop settings bridge (see Desktop settings below). `resetSettings` writes `DEFAULT_CLIENT_SETTINGS`.
- `AppSettings` fields and defaults (hooks/use-settings/storage.ts:79-170): theme "auto" (ThemePreference), pluginThemeId null, language "system", sendBehavior "steer" (SendBehavior = ActiveTurnBehavior | "queue"), serviceUrlBehavior "ask" (ask|in-app|external), terminalScrollbackLines 10000 (0..1,000,000), uiFontFamily "" (max length 200), monoFontFamily "", uiBaseFontSize platform default 14/15 (10..21), contentFontSize 15/16 (10..21), codeFontSize 12 (9..22), contentMaxWidth null (600..4000), syntaxTheme "one", workspaceTitleSource "title" (title|branch), sidebarWorkspaceTrailing "timestamp" (diff|timestamp|none), sidebarRowItems, sidebarChecksDisplay, sidebarNavItems [], sidebarFooterItems [], usage (UsagePreferences), autoExpandReasoning false, chatDisplay "collapseTurns" (full|collapseToolCalls|collapseTurns), chatOutlineEnabled true, vimKeybindings false, sendShortcut "enter" (enter|mod-enter), filePanelAssociations {}, openInSidePane {explorerFiles,diffs,chatFiles,diffFiles,subagents all false}, pullRequestOpenLocation "explorer" (main|side|explorer). Legacy fields read for migration: uiFontSize, compactToolCalls, toolCallDetailLevel.
- Other AsyncStorage keys seen in the app: `@paseo:daemon-registry` (host list), `@paseo:client-id-v1`, `@paseo:keyboard-shortcut-overrides`, `@paseo:preferred-editor`, `@paseo:create-agent-preferences`, `@paseo:changes-preferences`, `@paseo:onboarding`, `@paseo:expo-push-token:`, `@paseo:replica-cache`, `@paseo:sidebar-callout-dismissals`, `@paseo:project-icon-cache`, `@paseo:review-draft-store`, `@paseo:legacy-favorites-to-agent-profiles:v1:` (part B has the full key table).

## General settings sections
### General
- Language picker (settings-screen.tsx:~340-400; `AppLanguage` = "system" | locale, writes `language`), onboarding "setup" row (`useOpenOnboarding`).
### Sending (settings/general/sending-section.tsx)
- Default send behaviour while an agent is running (`sendBehavior`: steer or queue) and send shortcut (`sendShortcut`: Enter vs Cmd/Ctrl+Enter).
### Chat (settings/chat/chat-section.tsx)
- `autoExpandReasoning`, `chatDisplay` (full / collapse tool calls / collapse turns), `chatOutlineEnabled`.
### Terminal (settings/terminal/terminal-section.tsx)
- `terminalScrollbackLines` numeric field with min/max clamp (`parseTerminalScrollbackLines`).
### Editor (web only, settings/editor-section.tsx)
- `vimKeybindings` switch for the file editor.
### Files (settings/files-section.tsx)
- Lists remembered "open with" associations (`filePanelAssociations`: extension/filename -> panel id, incl. plugin file panels) with "ask again" per entry and "ask every time"; empty/unavailable states.
### Open location (settings/open-location/open-location-section.tsx)
- `openInSidePane` per source (explorer files, diffs, chat files, diff files, subagents), `serviceUrlBehavior` (ask / in-app / external), `pullRequestOpenLocation` (main / side / explorer).
### Appearance and Sidebar sections: documented in part C (fonts, sizes, content width, syntax theme, theme picker, nav item ordering `moveUp/moveDown`).
### Browser data (desktop only, desktop/browser/settings/browser-data-section.tsx)
- "Clear site data" for the embedded browser with confirm dialog; success/error messages.
### Integrations (desktop only, desktop/components/integrations-section.tsx)
- Command-line tool install row (CLI shim) plus docs link.
### Notifications (desktop only, desktop/components/desktop-notifications-section.tsx)
- OS notification permission row (state from desktop permissions snapshot; Request button), `notifications.playSound` switch, "Send test notification" with sending/sent/failed states. Persisted in the Electron `desktop-settings.json`. See part B for how notifications are produced.
### Permissions (desktop only, desktop-permissions-section.tsx)
- Microphone (and notifications) permission rows with Refresh and Request actions.
### Diagnostics (settings-screen.tsx:~430-470)
- "Run app diagnostics" (reads `useAppDiagnosticStore`) and "Test audio" (plays `THINKING_TONE_NATIVE_PCM_BASE64` via the voice audio engine; failure message `settings.diagnostics.playbackFailed`).
### About (settings-screen.tsx:~580-760)
- App version (formatVersionWithPrefix(resolveAppVersion())), daemon version mismatch hint, desktop-only release channel (stable | beta segmented control, writes `releaseChannel`), check for updates / install update (useDesktopAppUpdater; confirm dialog), changelog link (`openChangelog`), community links.

## Desktop settings (Electron)
- `desktop/src/settings/desktop-settings.ts`: file `desktop-settings.json` in Electron userData; schema `{releaseChannel: "stable"|"beta" (default stable), notifications:{playSound: true}, daemon:{manageBuiltInDaemon: true, keepRunningAfterQuit: false}, legacyRendererSettingsImported}` with zod `.catch(default)` per field. App side: `desktop/settings/desktop-settings.ts` (`useDesktopSettings`, query key `["desktop-settings"]`, IPC via `invokeDesktopCommand`). Legacy renderer settings (manageBuiltInDaemon, releaseChannel) imported once by `migrateLegacyDesktopSettings`.

## Host settings: daemon config model
- All host-scoped toggles are keys of the daemon's `config.json` (`CONFIG_FILENAME` in server/src/server/persisted-config.ts:381, under `$PASEO_HOME`), surfaced as `MutableDaemonConfig` (protocol/src/messages.ts:~190-255).
- Read/write path: app hook `useDaemonConfig(serverId)` (app/src/hooks/use-daemon-config.ts): `useReplicaQuery` keyed `daemonConfigQueryKey(serverId)` with push event `status:daemon_config_changed`; `patchConfig(patch)` -> `client.patchDaemonConfig(patch)` -> message `set_daemon_config_request {requestId, config: MutableDaemonConfigPatch}` -> server `daemonConfigStore.patch(msg.config)` (session.ts:2972) -> `set_daemon_config_response {config}`; reads: `get_daemon_config_request` -> `get_daemon_config_response` (session.ts:2951). Store: server/src/server/daemon-config-store.ts (persists to config.json via mapping `PERSISTED_TO_MUTABLE_PATH`, line ~215; per-field `onFieldChange` listeners apply live; `DaemonConfigReloadResult {appliedPaths, restartRequiredPaths, overrideControlledPaths}`; fields forced by environment are `envOverrides` and shown read-only). Reload from disk: `daemon.config.reload.request` (session.ts:2961; handleConfigReloadRequest in server/src/server/session/daemon/daemon-session.ts). Change broadcast: `status` payload `daemon_config_changed {config}` (messages.ts:3970).
- `MutableDaemonConfig` keys: relay {enabled, requireProtectedSecurity?}, mcp {enabled?, injectIntoAgents}, hostnames, cors.allowedOrigins, trustedProxies, git {maxProcessesPerSecond, maxProcessConcurrency}, app.baseUrl, catalogRefreshTimeoutMs, usage {enabled} (COMPAT usageSetting), browserTools {enabled default false}, providers record {paseoTools{enabled?, disabledTools?}, enabled?, additionalModels[]}, metadataGeneration {providers[{provider, model?, thinkingOptionId?}]}, autoArchiveAfterMerge (false), enableTerminalAgentHooks (false), idleRuntimeCleanup?, resumeOnStartup?, resumePrompt?, appendSystemPrompt ("" ), terminalProfiles?, agentProfiles?, skills.selection?, pluginsEnabled?, plugins?, features (speech), envOverrides?. Patch schema adds `removeProviders: string[]`. Live-apply handlers in server/src/server/bootstrap.ts: mcp.enabled:1772, mcp.injectIntoAgents:1779, appendSystemPrompt:1784, relay.enabled:1926, idleRuntimeCleanup:2085, trustedProxies:718, hostnames:742, app.baseUrl:745, cors:806, catalogRefreshTimeoutMs:1002, git limits:1005-1009; pluginsEnabled in plugins/index.ts:239.
- Feature gates in `server_info.features` used by cards: `idleRuntimeCleanupSetting`, `resumeOnStartupSetting`, `terminalsSurviveRestart`, `hopliteProvider`, `providerRemoval`, `pluginManagement` etc. (hooks `useHostFeature`, app/src/runtime/host-features).

## Host: Agents page (host-page.tsx:281)
Shown only when connected. Cards:
### Paseo tools (paseo-tools-card.tsx) 
Switch bound to `mcp.injectIntoAgents` (testID host-page-inject-mcp-card). Per-provider `providers.<id>.paseoTools {enabled, disabledTools}` policy exists in config (protocol/src/provider-config.ts:31); UI for the per-tool list: not found in this card (looked in paseo-tools-card.tsx).
### Browser tools (browser-tools-card.tsx, browser-tools-config.ts)
Switch -> `browserTools.enabled` (default false) with the warning "Allow agents to access and control Paseo browser tabs, including logged-in browser state. Only enable this for agents you trust."; states loading/error (`getBrowserToolsMutationViewState`).
### Append system prompt (host-page.tsx:983)
Textarea draft with save -> `appendSystemPrompt` (applied live to new agents, bootstrap.ts:1784).
### Restart recovery (restart-recovery-card.tsx)
Switch `resumeOnStartup` (default false, gated `resumeOnStartupSetting`), edit sheet for `resumePrompt` (placeholder/default const `DEFAULT_RESUME_PROMPT = "Resume working."`), save/error states. Server resumes agents that were running at shutdown at daemon start (bootstrap.ts:~1050).
### Idle runtime cleanup (idle-runtime-cleanup-card.tsx)
Switch `idleRuntimeCleanup`, effective value `!== false` (default on, gated `idleRuntimeCleanupSetting`); server applies via `cleanup.setEnabled` (bootstrap.ts:2085).
### Agent skills and Agent profiles sections (AgentSkillsSection / AgentProfilesSection)
Skills selection (`skills.selection`, orchestration skills; part A) and agent profiles (`agentProfiles` config; other area). Not detailed here.

## Host: Workspaces page (host-page.tsx:311)
- Card "Auto-archive merged workspaces" -> `autoArchiveAfterMerge` (default false; server module server/src/server/auto-archive-on-merge). 

## Host: Terminals page (host-page.tsx:1650)
- Terminal profiles list (`terminalProfiles`: `{id, name, command, args?, icon?}` passthrough, protocol/src/terminal-profile.ts; falls back to `DEFAULT_TERMINAL_PROFILES` when unset): add/edit/remove/reorder via `TerminalProfileRow`, edit modal `terminal-profile-edit-modal.tsx`; saved with `patchConfig({terminalProfiles: next})`.
- Card "Enable terminal agent hooks" -> `enableTerminalAgentHooks` (default false; host-page.tsx:943).

## Host: Metadata generation page (metadata-generation-page.tsx)
- Segmented control `metadata-generation-mode`: automatic vs preferred; preferred shows provider + model picker; writes `metadataGeneration: {providers: [{provider, model?}]}` (empty list = automatic). Docs link `METADATA_GENERATION_DOCS_URL`. Used by server to generate titles/branch names/commit messages.

## Host: Providers page (providers-section.tsx, 618 lines)
- Lists providers with status chip (available / not installed / error / loading / disabled) and model count; per-provider enable switch -> `patchConfig({providers: {[id]: {enabled}}})`; actions menu (`provider-actions-<id>`, Remove -> `removeProviders:[id]` behind `providerRemoval` feature and confirm); "Add provider" card (`host-page-add-provider-card`) that builds ACP provider config patches (`buildAcpProviderConfigPatch`, honouring `hopliteProvider`). Provider details/diagnostics come from provider RPCs (other area).

## Host: Voice page (settings/voice/voice-page.tsx)
- Three speech sites in `features.<site>`: `dictation.stt`, `voiceMode.stt`, `voiceMode.tts`; each has provider `local` | `openai`; local: model, language; openai: hasApiKey (key never returned), baseUrl, model, language, voice. Edited in `speech-provider-sheet.tsx` / `speech-provider-form.ts`, local model downloads via `use-local-speech-models.ts`; saved via `patchConfig({features: SpeechSettingsPatch})`; fields controlled by env shown through `envOverrides` paths (`speechSiteFieldPath`).

## Host: Usage page
- `HostUsageSection` (usage/host-usage-section.tsx) toggles/displays provider usage for that host (config `usage.enabled`); detailed in part B.

## Host: Plugins page: see part E.

## Host: Connections page and host identity
### Connections (host-page.tsx:251, ConnectionsSection:393)
- Lists the host's connections (direct TCP, relay, SSH/remote); remove one with confirm sheet (`remove-connection-confirm`), error alert; connection error banner (`HostConnectionError`); status badges (`HostStatusBadges`). Host store: `useHostMutations()` (runtime/host-runtime) persisted in `@paseo:daemon-registry`.
### Add host (settings-screen.tsx handleAddHost ...)
- `AddHostMethodModal` offers: direct connection (`AddHostModal`), remote SSH (`AddRemoteSshHostModal`), paste pairing link (`PairLinkModal`), scan QR (route `/pair-scan?source=settings`). After add: navigates to `connections` section of the new host. Triggered also by an `openAddHostIntent` param.
### Pair device (host-page.tsx:266 PairDeviceRow:1095)
- Shows pairing QR/link for mobile via `daemon.get_pairing_offer.request` (desktop `pair-device-modal`/`pair-device-section`).
### Host overview (HostSettingsPage:363)
- `HostAppearanceSection` (hosts/appearance.ts): rename host label, `HostAppearance {color: none|identity colors, badgeDisplay: name|icon|hidden|null}` stored per device (null default resolves local-hidden, remote-shown); local daemon section (desktop: manageBuiltInDaemon, keepRunningAfterQuit, daemon status) ; remote hosts get `UpdateDaemonCard` (`daemon.update.request`, states idle|updating{phase}|complete{workerVersion}|failed); `RemoveHostSection` (removing the local owned daemon stops it; confirm sheets).
### Restart daemon card (RestartDaemonCard:558)
- Button -> `restartDaemonFromSettings(serverId, reason "settings_daemon_restart_<serverId>", {restartServer, getStatus}, {killTerminals})` (screens/settings/daemon-restart/...: daemon-lifecycle.ts, daemon-reconnect.ts); optional "restart and close terminals" when `terminalsSurviveRestart`; disabled when offline; inline error `host-page-restart-error`.

## Project settings (project-settings-screen.tsx, 1253 lines)
- Route `settings/hosts/[serverId]/projects/[projectId]`. Reads/writes the repo's `paseo.json` through `client.readProjectConfig(repoRoot)` (`read_project_config_request`) / `write_project_config_request` (session.ts:2974-2976; server session/project-config). Form model `utils/project-config-form.ts` `ProjectConfigDraft {setupText, teardownText (string | array originals preserved), scripts[], metadataPrompts, metadataGenerationBase}`. UI: edit worktree setup/teardown commands, scripts list (name, command, run-as-service), metadata prompts; read-failure states (invalid / missing), Reload action; "back to projects". Projects list: `projects-screen.tsx` (host section `projects`).

## Cross-area connections
- Daemon config keys drive agents (appendSystemPrompt, mcp injection, browser tools, resume/idle cleanup), workspaces (auto-archive, terminal hooks), providers/models (providers, metadata generation), voice (features), plugins (pluginsEnabled), terminals (profiles). App settings feed chat rendering, composer send behaviour, sidebar, appearance, notifications (desktop playSound), usage preferences. Host store links settings to connection/relay/pairing. Project settings link to workspace setup scripts and service proxy.

## Inventory
- Settings shell and navigation; app settings store; General; Sending; Chat; Terminal; Editor; Files; Open location; Browser data; Integrations; Notifications (desktop); Permissions (desktop); Diagnostics; About and updates; Desktop settings file; daemon config read/patch/reload; Agents page cards (Paseo tools, Browser tools, Append system prompt, Restart recovery, Idle runtime cleanup, Skills, Profiles); Workspaces page (auto-archive); Terminals page (profiles, agent hooks); Metadata generation; Providers; Voice; Usage page; Connections; Add host flows; Pair device; Host overview (appearance, local daemon, update daemon, remove host); Restart daemon; Project settings.
