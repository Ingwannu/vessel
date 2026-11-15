# Area 6, Part B: Notifications, usage and limits, storage, performance

Source: Paseo 0.15.4, ~/refs/paseo-latest/packages. Paths below are relative to `packages/` unless absolute.
`app/src` = app, `server/src/server` = daemon. Line numbers refer to that file as of 0.15.4.

---

## Notification routing (the server-side "who gets notified" plan)

### What the user sees
Nothing directly. It decides whether an attention event (agent finished / needs permission / errored, terminal finished / needs input) becomes (a) an in-app OS notification on exactly one connected client, (b) a mobile push, or (c) nothing.

### How it works end to end
- Trigger (agent): `AgentManager.checkAndSetAttention` (server/src/server/agent/agent-manager.ts:5214) runs inside `emitState` (:5178). Edge-triggered only:
  - `previousStatus === "running" && lifecycle === "idle"` -> `attention = {requiresAttention:true, attentionReason:"finished", attentionTimestamp:new Date()}` and `broadcastAgentAttention(agent,"finished")` (:5231-5238).
  - `previousStatus !== "error" && lifecycle === "error"` -> reason `"error"` (:5242-5249).
  - Skipped when `agent.internal`, or when `attention.requiresAttention` is already true (:5221-5229).
  - Permission: `onStreamPermissionRequested` (:4960) calls `broadcastAgentAttention(agent,"permission")` only when `!hadPendingPermissions && !agent.internal` (:4966). Does NOT set `agent.attention`; permission state is carried by `pendingPermissions`.
  - `broadcastAgentAttention` (:5351) returns early for `isDelegatedAgent(agent)` (sub-agents never notify), else calls `onAgentAttention` callback installed by `AgentManager.setAgentAttentionCallback` (:883) from the `VoiceAssistantWebSocketServer` constructor (class at websocket-server.ts:551; callback installed at :811): `broadcastAgentAttention` (websocket-server.ts:2857).
- Trigger (terminal): `broadcastTerminalAttention` (websocket-server.ts:2955, invoked from the terminal activity subscription at :842-850). Reason derived by `resolveTerminalAttentionReason` (websocket-server.ts:201-211) (`attentionReason finished|needs_input`, `state==="attention"` -> needs_input, `working -> idle` -> finished). Title helper `terminalAttentionTitle` (websocket-server.ts:213): `needs_input` -> "Terminal needs input", else "Terminal finished". Body = terminal name. Push `data` = `{serverId, terminalId, cwd, workspaceId?}`.
- Plan: `computeNotificationPlan` (server/src/server/agent-attention-policy.ts:42). Inputs: `allStates: ClientPresenceState[]` (only connections whose subscription has `notifications:true` for that event, see wantsSourceNotification below), `focusTarget` (`{kind:"agent"|"terminal", id}`), `pushEligible`, `nowMs`.
  - Constant `PRESENCE_THRESHOLD_MS = 180_000` (policy.ts:3). A client is "present" if `lastActivityAtMs` (clamped to now) is within 180 s.
  - If any present client has `appVisible` AND is focused on the target -> `{inAppRecipientIndex:null, shouldPush:false}` (suppress everything).
  - Else the most recently active present client is the single in-app recipient (`shouldPush:false`).
  - Else (no present client) `{inAppRecipientIndex:null, shouldPush: pushEligible}`.
  - `isPushEligibleAttentionReason(reason)` = `reason !== "error"` (policy.ts:78). Terminal: `pushEligible:true` always (websocket-server.ts:2997).
- Client presence state: `getClientActivityState` (websocket-server.ts:2838) reads `ClientMetadata.activity` (session.ts:679-680 fields `focusedAgentId`, `focusedTerminalId`) fed by `client_heartbeat` (see "Client presence heartbeat"). No heartbeat yet -> `{appVisible:false, focusedAgentId:null, lastActivityAtMs:null}` = not present.
- Payload: `buildAgentAttentionNotificationPayload` (protocol/src/agent-attention-notification.ts:196). Titles: permission "Agent needs permission", error "Agent needs attention", finished "Agent finished" (:172). Body = preview or fallback ("Permission requested." / "Encountered an error." / "Finished working.", :190). Preview for `finished` = last assistant message (chunks of consecutive trailing `assistant_message` items joined, `findLatestAssistantMessageFromTimeline` :139), markdown stripped (`stripMarkdownToText` :53), whitespace collapsed, truncated to `NOTIFICATION_PREVIEW_LIMIT = 220` with "..." (:1, :45). Preview for `permission` = `title - description`, else JSON of `input`, else `metadata`, else `name`/`kind` (`buildPermissionDetails` :105). `error` has no preview. `data = {serverId, workspaceId, agentId, reason}` (`AgentAttentionNotificationData` :5).
- Delivery to clients: for each subscribed connection a message is sent with `shouldNotify` true only for the chosen recipient. Modern clients: `agent_attention_required` (schema protocol/src/messages.ts:4972, payload `{subscriptionId?, agentId, reason, timestamp, shouldNotify, notification?{title,body,data}}`) or `terminal_attention_required` (messages.ts:6687, payload `{subscriptionId?, serverId?, terminalId, cwd, workspaceId?, reason: finished|needs_input, title, body, shouldNotify}`). Legacy clients (no `selectiveAgentTimeline` cap) get an `agent_stream` event `{type:"attention_required", provider, reason, timestamp, shouldNotify, notification}` (messages.ts:820; websocket-server.ts:2908-2935).
- Per-subscription gating: `Session.wantsSourceNotification` (session.ts:2336) true for legacy sources, else only subscriptions created with `notifications:true` (`session.events.set_subscription.request` field `notifications`, messages.ts:3255, handler session.ts:2830). `emitEventToOwner` (session.ts:8597) additionally ensures at most one subscription per connection source keeps `shouldNotify` (the `notified` set).
- Push send: `this.pushNotificationSender.send(notification)` (websocket-server.ts:2917 agent, :3010 terminal). `pushNotificationSender` is injectable (`bootstrap.ts:483 pushNotificationSender?`) else the real `PushNotifications`.

### States, errors, limits, defaults
- `PRESENCE_THRESHOLD_MS` 180 000. Preview limit 220 chars. Error attention never pushes.
- Push failures are logged only ("Failed to send push notification").

### Platforms
Server-side only; identical for all clients.

### Connections
Reads: agent lifecycle (agent manager), pending permissions, timeline (last assistant message), terminal activity tracker, client heartbeat. Writes: push service, `agent_attention_required`/`terminal_attention_required` messages, `agent.attention` state (agent persisted record fields `requiresAttention`, `attentionReason`, `attentionTimestamp`).

---

## Client presence heartbeat (`client_heartbeat`)

### What the user sees / does
Invisible. Tells the daemon which agent/terminal the user is looking at and when they last interacted, so notifications go to the right device.

### How it works end to end
- App: `useClientActivity` (app/src/hooks/use-client-activity.ts:142) mounted from `SessionProvider` (contexts/session-context.tsx:250 `useClientActivity({client, focusedAgentId, focusedTerminalId})`). Pure logic `createClientActivityTracker` (app/src/hooks/client-activity-tracker.ts:39).
- Constants (client-activity-tracker.ts:1-3): `HEARTBEAT_INTERVAL_MS = 15_000` (periodic), `ACTIVITY_HEARTBEAT_THROTTLE_MS = 5_000` (immediate heartbeat on activity at most every 5 s), `DESKTOP_IDLE_POLL_INTERVAL_MS = 5_000` (Electron OS idle poll).
- Inputs: AppState change (native + web) -> `notifyAppVisibility`; web `visibilitychange`, `focus`, `pointerdown`, `keydown`, `wheel`, `touchstart` -> `recordUserActivity` + `maybeSendImmediateHeartbeat` (use-client-activity.ts:174-207). Electron: polls `readDesktopSystemIdleTimeMs(invokeDesktopCommand)` every 5 s and advances `lastActivityAt` using OS idle time (`notifySystemIdleMs`, tracker :106) so a backgrounded desktop window still counts as present. Focus changes (`setFocusedAgentId`/`setFocusedTerminalId`) record activity and heartbeat immediately.
- `deviceType`: `isWeb ? "web" : "mobile"` (use-client-activity.ts:155) (desktop Electron reports "web").
- Client method `DaemonClient.sendHeartbeat` (client/src/daemon-client.ts:2473) -> message `client_heartbeat` (protocol messages.ts:2960): `{deviceType: "web"|"mobile", focusedAgentId: string|null, focusedTerminalId?: string|null (default null), lastActivityAt: ISO string, appVisible: boolean, appVisibilityChangedAt?: ISO string}`.
- Server: `Session.handleClientHeartbeat` (session.ts:5043) stores `metadata.activity` per connection; if `appVisible && focusedTerminalId` it clears that terminal's attention (`clearFocusedTerminalAttention` session.ts:5072 -> `terminalManager.clearTerminalAttention`, terminal-manager.ts:453 -> `tracker.clearAttention()` terminal-activity-tracker.ts:42); if the connection has a push token it renews its lease (`pushNotifications.renew`, session.ts:5067-5069).
- Heartbeat is sent only when `client.isConnected`; interval restarts on connection status.

### Platforms
Native: AppState only. Web/desktop: DOM events; desktop adds OS idle polling.

### Connections
Feeds Notification routing, Push token lease renewal, terminal attention auto-clear. Focus ids come from the active workspace/pane selection (workspace/pane focus feature, other area).

---

## In-app OS notification display (desktop Electron, web Notification API)

### What the user sees and does
A system notification (title, body, app icon) when the single chosen client is away from the agent. Clicking focuses the app and navigates to the agent/terminal (see "Notification open routing"). Native (iOS/Android) apps never show local notifications: they rely on remote push.

### How it works end to end
- Event handlers in `SessionProvider` (app/src/contexts/session-context.tsx):
  - Subscribes `client.observeEvents(["agent_attention_required","terminal_attention_required","agent_permission_request","agent_permission_resolved","agent.provider_subagents.update","checkout_status_update","status.server_info"], {notifications:true})` (:482-492).
  - `agent_attention_required` with `payload.shouldNotify` -> `notifyAgentAttention` (:257): returns if `reason==="error"` (:268); skips if the app is actively visible AND `session.focusedAgentId === agentId` (`getIsAppActivelyVisible`, app/src/utils/app-visibility.ts); dedups with `attentionNotifiedRef` map agentId -> last timestamp ms (skips if `lastNotified >= timestampMs`); builds payload via `resolveAgentAttentionNotification` (:148) (uses server `notification` if it carries `data.workspaceId`, else rebuilds locally from stream head/tail assistant message and pending permission; returns null with no workspaceId); then `sendOsNotification({title, body, data})`.
  - `terminal_attention_required` with `shouldNotify` -> `sendOsNotification({title, body, data:{serverId, terminalId, cwd, workspaceId?}})` (:658-678).
- `sendOsNotification` (app/src/utils/os-notifications.ts:144): native -> returns false. Desktop (`getDesktopHost().notification.sendNotification`) -> IPC; else web `Notification` constructor after `ensureNotificationPermission` (:67; `permission==="granted"` ok, `"denied"` false, otherwise one shared `requestPermission()` promise). Web notification gets `{body, data, icon}` where icon is `assets/images/notification-icon.png` resolved via expo-asset (:102); click handler dispatches `window` CustomEvent `WEB_NOTIFICATION_CLICK_EVENT = "paseo:web-notification-click"` (cancelable) with `{data}` only when `parseNotificationOpenTarget(data)` is non-null (:98, :135).
- Permission bootstrap: `AppShell` calls `ensureOsNotificationPermission()` once at start when `getDesktopHost()` is present (app/src/app/_layout.tsx:865-867). On plain web (no desktop host) permission is requested lazily on the first `sendOsNotification`. Native: `ensureOsNotificationPermission` returns false (os-notifications.ts:91).
- Electron main: `registerNotificationHandlers` (desktop/src/features/notifications.ts:81, registered at desktop/src/main.ts:971). IPC `paseo:notification:isSupported` -> `Notification.isSupported()`. IPC `paseo:notification:send` (:86): requires non-empty trimmed `title`; optional `body`, `data` (plain object); icon from `../assets/icon.png`, `64x64.png`, `128x128.png` (first non-empty, :32); `silent: !settings.notifications.playSound` (:104). On click: show/restore/focus the sender window and send `paseo:event:notification-click` with `{data}` only if `data` has keys (:109-116). `activeNotifications` Set holds references until click/close. Preload bridge: `notification.isSupported`, `notification.sendNotification` (desktop/src/preload.ts:89-92).
- macOS registration: `ensureNotificationCenterRegistration` (desktop/src/features/notifications.ts:70, called main.ts:967) shows a silent probe notification (closed on `show` or after 2 000 ms) so the app appears in System Settings > Notifications.

### States, errors, limits, defaults
- Return value false = not delivered (no permission / unsupported / native).
- Duplicate suppression: per-agent monotonically increasing timestamp (client side) + one-recipient rule (server side).

### Platforms
- Desktop (Electron): Electron `Notification`, sound controlled by desktop setting.
- Web: `Notification` API with permission prompt.
- Mobile: no local notifications; push only (see "Mobile push notifications").

### Connections
Triggered by Notification routing; routes into Notification open routing; reads Desktop notification settings; uses session store (`focusedAgentId`, stream head/tail, pending permissions).

---

## Desktop notification settings (Settings > Notifications)

### What the user sees and does
Desktop-only settings section "Notifications" (`desktopOnly: true`, icon `Bell`; app/src/screens/settings-screen.tsx:216-221, content `DesktopNotificationsSection`, app/src/desktop/components/desktop-notifications-section.tsx:20). Rows: Permission (`DesktopPermissionRow` with request button), "Play sound" Switch (testID `desktop-notifications-play-sound-switch`), "Send test notification" Button (enabled only when notifications permission is `granted`, not busy), success Alert (testID `desktop-notifications-test-success`) or error Alert (`desktop-notifications-test-error`), Refresh button in the header (re-reads permission snapshot).

### How it works end to end
- Settings: `useDesktopSettings` (app/src/desktop/settings/desktop-settings.ts); shape `DesktopSettings.notifications.playSound: boolean`, default `true` (app/src/desktop/settings/desktop-settings.ts:33-35). Update via `updateSettings({notifications:{playSound}})` -> IPC to Electron main; stored in the desktop settings store (desktop/src/settings/desktop-settings.ts:34-35 default, schema `NotificationsSchema` :47 with `.catch(default)`, patch coercion :147-150). Read at each notification send (desktop/src/features/notifications.ts:99).
- Permission: `useDesktopPermissions` (app/src/desktop/permissions/use-desktop-permissions.ts) -> `createDesktopPermissions` (app/src/desktop/permissions/desktop-permissions.ts:123). Notifications status: on desktop host, `desktopHost.notification.isSupported()` -> `granted`/`unavailable` (:134-146); else web `Notification.permission` mapped (`granted`, `denied`, `default`->`prompt`, other->`unknown`, :97); `requestPermission` calls `Notification.requestPermission()` (:250). Non-web (native) -> `unavailable`.
- Test: `sendTestNotification` (use-desktop-permissions.ts:121) -> `sendOsNotification` with i18n title/body (`desktop.permissions.testNotification.*`); states `idle|sending|success|error` (`TestNotificationState` :23); not delivered -> error "notDelivered".
- i18n keys: `settings.notifications.{title,permission,playSound,playSoundHint,test,testHint,permissionRequired,send,sending,sentTitle,sentDescription,sendFailedTitle,refreshAccessibility}`, `settings.permissions.{refresh,refreshing,actions.*}`.

### States, errors, defaults
`playSound` default true. Settings write error is surfaced by `useDesktopSettings` (comment in section file :45). No other notification settings exist anywhere (no per-event, per-host or quiet-hours settings: grepped app/server/desktop for `notif`/`sound`).

### Platforms
Desktop only (section hidden if `!isDesktopApp`, :81). Web and mobile have no notification settings UI.

### Connections
Settings (desktop settings store), permissions section (microphone), OS notification display.

---

## Notification open routing (tap/click -> navigate)

### What the user sees and does
Tapping a mobile push, clicking a desktop/web notification opens the relevant agent, terminal tab, or host root.

### How it works end to end
- Mounted: `NotificationOpenRouter` component (app/src/notification-open/react/index.tsx:35) with `startupBlocked` prop; module-level `createNotificationOpenRouter` with `createNotificationOpenSource()`.
- `parseNotificationOpenTarget(data)` (app/src/notification-open/model.ts:16): requires string `serverId`; `workspaceId`+`terminalId` -> `{kind:"terminal"}`; else `agentId` -> `{kind:"agent"}`; else `{kind:"host"}`. Values trimmed.
- Router (`createNotificationOpenRouter`, model.ts:36): dedups by `tap.id` (`handledIds` set), awaits `whenReadyToNavigate()`, then agent -> `navigateToAgent({serverId, agentId, pin:true})` (app/src/utils/navigate-to-agent), terminal -> `navigateToWorkspace({serverId, workspaceId, target:{kind:"terminal", terminalId}})`, host -> `router.navigate(buildHostRootRoute(serverId))`; then `source.acknowledge(tap)`. Launch tap (`takeLaunchTap`) consumed once (`launchTaken`).
- Readiness gate (react/index.tsx:41-58): `root.key && registryStatus==="ready" (useHostRegistryStatus) && selectionHydrated (useIsLastWorkspaceSelectionHydrated) && !startupBlocked && !atStartupIndex` (index routes redirect first).
- Sources:
  - native (`source.native.ts`): `expo-notifications` `setNotificationHandler` returns `{shouldShowAlert:false, shouldShowBanner:false, shouldShowList:true, shouldPlaySound:false, shouldSetBadge:false}` (foreground shows only in notification list); tap = `addNotificationResponseReceivedListener`, id = `request.identifier`, data = `content.data`; launch = `getLastNotificationResponseAsync`; ack = `clearLastNotificationResponse`.
  - web (`source.web.ts`): listens to `WEB_NOTIFICATION_CLICK_EVENT` on `window`, `preventDefault`, id `web:<n>`; no launch tap.
  - electron (`source.electron.ts`): web source + `getDesktopHost().events.on("notification-click")` (Electron IPC `paseo:event:notification-click`), id `electron:<n>`.
  - Selection: `source.ts` re-exports native; platform-specific files `source.web.ts`/`source.electron.ts` chosen by bundler (app/src/notification-open/source.ts:1-12).

### States, errors
Unparseable data -> no navigation (acked anyway). Terminal taps need both `workspaceId` and `terminalId` (terminal push `data` includes `cwd` as fallback but router ignores `cwd`).

### Platforms
Native, web, Electron as above.

### Connections
Navigation (workspace/agent routes), host registry, workspace selection store, OS notification display, push.

---

## Mobile push notifications (Expo push)

### What the user sees and does
On iOS/Android (non-F-Droid), the first time a host session starts the OS permission prompt appears; afterwards agent finished / permission / terminal notifications arrive as push when the user has no present client (see routing). Android channel "default".

### How it works end to end
- App registration: `startPushNotifications({client, serverId})` (app/src/push-notifications/index.native.ts:4; `.web.ts` and generic `.ts` are no-ops) invoked in `SessionProvider` `useEffect` (contexts/session-context.tsx:255).
  - `resolveToken` (app/src/push-notifications/internal/subscriptions.ts:31): reads cached token from AsyncStorage key `@paseo:expo-push-token:<serverId>` (`STORAGE_PREFIX` :9, validated `z.string().trim().min(1)`); `ensurePushPermission` (:23): `getPermissionsAsync`, if not granted and `canAskAgain` -> `requestPermissionsAsync`; denied -> removes cached key, returns null. Android: `setNotificationChannelAsync("default", {name:"default", importance: DEFAULT})`. Expo project id from `Constants.easConfig.projectId` or `expoConfig.extra.eas.projectId` (:16-21); missing -> warns and uses cached token; else `getExpoPushTokenAsync({projectId})`, trims, persists.
  - `register`: when token present and `client.isConnected` -> `client.registerPushToken(token)` (client/src/daemon-client.ts:2492) -> message `register_push_token {token}` (protocol messages.ts:2999); re-sent on every `connected` status via `subscribeConnectionStatus`.
- Revoke: `revokePushNotifications({client, serverId})` (index.native.ts:8; called from host removal `HostRuntimeStore` host-runtime.ts:2255): reads cached token; if connected and `serverInfo.features.pushTokenRevocation === true` -> `client.unregisterPushToken(token)` (daemon-client.ts:2499, message `push.unregister.request {token, requestId}` -> `push.unregister.response {requestId}`, timeout `PUSH_TOKEN_REVOCATION_TIMEOUT_MS = 2_000` daemon-client.ts:1126) then removes the AsyncStorage key regardless. Server handler session.ts:3252-3262 revokes the token and clears the connection's `pushToken`.
- Server token handling: `Session.handleRegisterPushToken` (session.ts:5087) stores `ClientMetadata.pushToken` and calls `PushNotifications.renew(token)`.
- Server token store: `createPushNotifications` (server/src/server/push/index.ts:18). `PUSH_TOKEN_LEASE_MS = 48 * 60 * 60 * 1000` (index.ts:8). `PushTokenStore` (server/src/server/push/token-store.ts:12): Map token -> expiry ms; `renewToken` only rewrites when remaining lease <= half (`currentExpiry - now > leaseMs/2` -> noop, :193); `revokeToken`; `getActiveTokens` prunes expired and persists. File `<PASEO_HOME>/push-tokens.json` (websocket-server.ts:806) written with `writePrivateFileAtomicSync`, shape `{"subscriptions":[{"token": string, "expiresAt": ISO}]}`; legacy shape `{"tokens":[...]}` migrated on load with a fresh lease (:255-267). Tokens are global per daemon (not per client/agent): every active token gets every push.
- Server send: `PushNotifications.send(payload)` (index.ts:38) logs token count, no-op when 0 tokens, then `PushService.sendPush` (push-service.ts:85): messages `{to, title, body, data, sound:"default"}`, batches of `MAX_BATCH_SIZE = 100`, POST `EXPO_PUSH_URL = "https://exp.host/--/api/v2/push/send"` (JSON, parallel batches). Tickets with `details.error` `DeviceNotRegistered` or `InvalidCredentials` revoke that token (:145-150). HTTP errors and exceptions only logged.
- When sent: only when `computeNotificationPlan.shouldPush` (no present client; never for agent `error`). Payload = same title/body/data as in-app notification (agent: `{serverId, workspaceId, agentId, reason}`; terminal: `{serverId, terminalId, cwd, workspaceId?}`).

### States, errors, limits, defaults
Lease 48 h, renewed by heartbeats (`handleClientHeartbeat` renews on every heartbeat, ~15 s, effective rewrite only past half-life) and on register. Permission denied -> no token. Missing EAS projectId -> warn.
Build: `expo-notifications` plugin (icon `./assets/images/notification-icon.png`, color `#20744A`) only in non-F-Droid builds (app/app.config.js:45-52, `notificationPlugins`); F-Droid builds alias `expo-notifications` to a stub (app/src/fdroid/expo-notifications.ts) that always reports permission denied, empty token, no-op handlers.

### Platforms
Native only. Web and desktop use no push (`index.web.ts`/`index.ts` no-ops).

### Connections
Triggered by Notification routing; token tied to host (serverId); shares AsyncStorage key family (see storage table); host removal revokes; handshake feature flag `pushTokenRevocation` in `server_info.features` (protocol messages.ts:3732).

---

## Agent attention / unread state ("requires attention")

### What the user sees and does
Unread/attention indicators on agents and workspaces: status dot colors in the sidebar, project badges, favicon, macOS dock badge, "Ready to review" group. Attention clears automatically when the user views the agent; can be cleared/restored manually ("Mark as read" / "Mark as unread" in workspace context menu).

### How it works end to end
- Data: server `ManagedAgent.attention: AttentionState` (agent-manager.ts:382: `{requiresAttention:false}` or `{requiresAttention:true, attentionReason:"finished"|"error"|"permission", attentionTimestamp: Date}`), persisted in the agent record (`requiresAttention`, `attentionReason`, `attentionTimestamp`; loaded via `extractAttention` persistence-hooks.ts, :2147). Wire: `AgentSnapshotPayload.requiresAttention?`, `attentionReason?: "finished"|"error"|"permission"|null`, `attentionTimestamp?: string|null` (protocol messages.ts:887-889, also :912-914 stored/list shapes). App: `Agent` type in app/src/stores/session-store.ts:96-98 (`attentionTimestamp` becomes `Date`); copied at session-store.ts:1897-1899.
- Clear (agent): `DaemonClient.clearAgentAttention(agentId | agentId[])` (client/src/daemon-client.ts:2410) -> `clear_agent_attention {agentId, requestId?}` (protocol messages.ts:2954) -> `Session.handleClearAgentAttention` (session.ts:4999) -> `AgentManager.clearAgentAttention` (agent-manager.ts:2389: sets `{requiresAttention:false}`, persists, emits state) -> `clear_agent_attention_response {requestId, agentId, agents[]}` (messages.ts:5018). Fire-and-forget in the app (`.catch(()=>{})`).
- When the app auto-clears: `useAgentAttentionClear` (app/src/hooks/use-agent-attention-clear.ts:28), used in the agent panel (app/src/panels/agent-panel.tsx:767). Triggers type `AgentAttentionClearTrigger = "focus-entry" | "input-focus" | "prompt-send" | "agent-blur"` (app/src/utils/agent-attention.ts:12). `shouldClearAgentAttention` (agent-attention.ts:63): needs agentId, `isConnected`, `requiresAttention`, reason !== `"permission"` (permission attention is never cleared by viewing; resolved by answering), and not (`focus-entry` with a deferred clear). Focus-entry clear fires when the pane becomes focused while the app is actively visible, or the app resumes with the pane focused. Deferred rule: if attention arrives while the pane is already actively viewed (false->true transition), the focus-entry clear is skipped (`deferredFocusEntryClearRef`) so the user sees the new item until they type/send/blur. Blur: pane unfocus (agent-panel.tsx:781) and unmount clear. Input focus and prompt send via composer props `onAttentionInputFocus`/`onAttentionPromptSend` (agent-panel.tsx:1038-1039). Visibility: `getIsAppActivelyVisible` (app/src/utils/app-visibility.ts: AppState active, document visible, window focused on web).
- Clear (workspace, "Mark as read"): `useWorkspaceReadState` (app/src/hooks/use-workspace-read-state.ts:15): `hasClearableAttention = status === "attention" || status === "failed"`; `clearAttention` -> `client.clearWorkspaceAttention(workspaceId)` (daemon-client.ts:2433) -> `workspace.clear_attention.request {workspaceId: string|string[], requestId}` (messages.ts:2764) -> `handleWorkspaceClearAttentionRequest` (session.ts:7634). Server clears only agents with `workspaceId === workspace.workspaceId`, `requiresAttention===true`, not archived, no pending permissions, `attentionReason !== "permission"` (:7678-7687); live agents via manager, stored ones by rewriting the record and emitting `agent_update`; then `emitWorkspaceUpdateForWorkspaceId`. Response `workspace.clear_attention.response {requestId, workspaceId, clearedAgentIds, results[{workspaceId, clearedAgentIds, success, error}], success, error}` (messages.ts:5093); client throws on `!success`.
- Mark unread: `canMarkUnread = hostFeature "workspaceMarkUnread" && status === "done"` (use-workspace-read-state.ts:25). `markWorkspaceUnread` (app/src/workspace/mark-unread.ts:5) first unfocuses the pane (`useWorkspaceLayoutStore.unfocusPane`) so the user's own leaving does not immediately clear it, restores focus on error; then `client.markWorkspaceUnread` (daemon-client.ts:2459) -> `workspace.mark_unread.request {workspaceId, requestId}` (messages.ts:2770) -> `handleWorkspaceMarkUnreadRequest` (session.ts:7770): picks the most recently updated root agent (`resolveWorkspaceRootAgent`) in the workspace with status `idle`/`closed`, not requiring attention, no pending permissions, provider visible to the client; none -> error "Workspace has no finished agent to mark unread". `AgentManager.markAgentUnread` (agent-manager.ts:2398) sets `requiresAttention:true, attentionReason:"finished"` (live agent must be `idle`; stored record must have `lastStatus` idle/closed, not internal/archived) or throws "Agent is no longer finished and read". Response `workspace.mark_unread.response {requestId, workspaceId, markedAgentId|null, success, error}` (messages.ts:5112). Feature flag `features.workspaceMarkUnread` (messages.ts:3814).
- Menu entries: `SidebarWorkspaceMenu` items "Mark as read" (testID `sidebar-workspace-menu-mark-as-read-<workspaceKey>`) and "Mark as unread" (`...-mark-as-unread-...`) at app/src/components/sidebar/sidebar-workspace-menu.tsx:190-208 (strings are hardcoded English, not i18n). Wired in `sidebar-workspace-list.tsx:1301`, `sidebar/sidebar-workspace-row.tsx:125`, `sidebar/sidebar-status-list.tsx:686`; errors show `toast.error` ("Failed to mark workspace as read/unread").
- State buckets: `deriveAgentStateBucket` (protocol/src/agent-state-bucket.ts:22): pending permissions or `attentionReason==="permission"` -> `needs_input`; `status==="error"` or reason error -> `failed`; `running` -> `running`; `requiresAttention` -> `attention`; else `done`. Priority `needs_input 0, failed 1, running 2, attention 3, done 4` (:14). `getAgentStatusPriority` (:42) adds `initializing=3`. Workspace status = best bucket of its agents; sidebar labels (`sidebar.statusBucket.*`: needsInput, failed, readyToReview (for `attention`), working, done; app/src/hooks/sidebar-status-view-model.ts:9). Groups sorted by `statusEnteredAt` desc then project/name; Cmd/Ctrl-number shortcuts for first 9 rows (`buildStatusShortcutIndex` :84).
- `pickAttentionAgent` (agent-attention.ts:31) picks the top-level agent with `requiresAttention` by priority permission > error > finished, then oldest `attentionTimestamp`; sub-agents (`parentAgentId`) skipped. Used by "go to next attention agent" style navigation (grep: callers not traced beyond definition in utils/agent-attention.ts).
- Attention timestamps drive workspace `statusEnteredAt` (server workspace-directory.ts:525-547 prefers `attentionTimestamp`).

### States, errors, defaults
`requiresAttention` default false. Attention is edge-triggered: not re-set while already set. Internal and delegated (sub-)agents never raise attention.

### Platforms
Same on all; clear triggers use AppState on native and DOM focus/visibility on web.

### Connections
Notification routing, sidebar, workspace status, composer focus, pane focus, favicon/dock badge, terminal attention.

---

## Terminal attention / activity

### What the user sees
Terminal tabs and workspace rollups show working / needs-input / finished dots; terminals can notify like agents.

### How it works end to end
- Protocol: `TerminalActivitySchema` `{state: idle|working|attention (unknown -> "idle"), attentionReason?: finished|needs_input|null, changedAt:number}` (protocol/src/terminal-activity.ts:9). `deriveTerminalActivityStatusBucket` (:22): `needs_input` reason -> "needs_input", `finished` -> "attention", `working` -> "running", `state==="attention"` -> "needs_input".
- Server tracker `TerminalActivityTracker` (server/src/terminal/activity/terminal-activity-tracker.ts:12): `set("idle")` after `working` -> idle + `finished`; `set("attention")` -> idle + `needs_input`; `clearAttention()` (:42) -> idle, no reason; `interrupt()` clears when working. State null means unknown (plain shell has no dot).
- Attention clear: server clears on heartbeat when the client is visible and focused on that terminal (session.ts:5064); notification via `terminal_attention_required` as above.
- The agent hook that drives terminal state for Claude: server/src/terminal/agent-hooks/claude/claude.ts (not traced in detail).

### Connections
Notification routing, sidebar workspace status (workspace `status` includes terminals), terminal feature area.

---

## Sidebar / tab attention badges, favicon and dock badge

### What the user sees
- Status dot colour per bucket: `needs_input` amber (`statusDotWarning`), `failed` red (`statusDotDanger`), `running` blue (`statusDotRunning`), `attention` green (`statusDotSuccess`), `done` none (or border colour when `showDoneAsInactive`) (app/src/utils/status-dot-color.ts:4-34).
- Project status badge (14 pt): `needs_input` -> alert glyph; `failed`/`attention`/`running` -> coloured dot (running pulses); else none (app/src/utils/project-status-badge-content.ts:19).
- Browser/desktop favicon: `useFaviconStatus` (app/src/hooks/use-favicon-status.ts:96, mounted app/src/app/_layout.tsx:816). Status: any agent running -> "running"; else any `requiresAttention` or pending permission -> "attention"; else "none"; images `assets/images/favicon-{dark,light}[-running|-attention].png`, follows `prefers-color-scheme`. Disabled on native.
- macOS dock badge (Electron mac only): count of workspaces whose status is `attention`, `needs_input` or `failed` (`deriveMacDockBadgeCountFromWorkspaceStatuses`, app/src/utils/desktop-badge-state.ts:9; undefined when 0). Sent via `getDesktopHost().window.getCurrentWindow().setBadgeCount(count)` (use-favicon-status.ts:81) -> IPC `paseo:window:setBadgeCount` (desktop/src/window/window-manager.ts:181; applies on darwin and linux, `readBadgeCount` coerces non-safe-integer/negative to 0, :20). Only when count changes.

### Connections
Workspace status bucket, session store, Electron window bridge, theme.

---

## In-app toasts

### What the user sees
One-at-a-time pill toast near the top of the app shell (below the header), with optional icon and variant colours; hover pauses auto-dismiss. About 131 call sites across 52 files (grep `useToast(` / `toast.show|copied|error`).

### How it works end to end
- Provider: `ToastProvider` (app/src/contexts/toast-context.tsx:7) mounted in app/src/app/_layout.tsx:910; `useToastHost` (app/src/components/toast-host.tsx:75) holds a single `ToastState|null` (a new toast replaces the current one); `ToastViewport` (:135) renders it, portal into `getOverlayRoot()` on web for placement `"app-shell"`, or `FloatingHeaderOverlay` for placement `"panel"` (used by the timeline sync status viewport, app/src/timeline/sync-status.tsx:43). z-index `OVERLAY_Z.toast`.
- API (`ToastApi`, toast-host.tsx:53): `show(content, {icon?, variant?, durationMs?: number|null, nativeAndroid?, testID?}) -> DismissToast`, `copied(label?)` (success variant, check icon, i18n `common.states.copied[Label]`), `error(message)` (error variant, 3 200 ms). `useToast()` from `@/contexts/toast-api-context` throws outside the provider.
- Constants: `DEFAULT_DURATION_MS = 2200`, `TOAST_MAX_WIDTH = 480`, width boundary 92%, enter/exit `FadeIn/FadeOut.duration(140)`. `durationMs: null` = sticky. Empty/whitespace string content is ignored. Variants: default, info (blue border), success (primary icon), warning (amber), error (destructive); `accessibilityRole="alert"`; testIDs `app-toast`, `app-toast-message`.
- Android: `nativeAndroid:true` with string content uses `ToastAndroid.showWithGravity(..., TOP)`, SHORT if `durationMs <= 2500` else LONG (:94-99).
- Provider notices: `showProviderNoticeToast` (app/src/utils/provider-notice-toast.ts:4): error notice -> `toast.error`, else `show(message, {variant: notice.type, durationMs: 5000 for warning})`; invoked from `session-context.tsx` after agent/model operations (`.then(notice => showProviderNoticeToast(...))`, :773, :804).
- Plugin API: `useToast()` for plugins (app/src/plugins/react-native/toast.ts) exposes `show(message, options)` and `error(message)` only.

### Platforms
Web: portal overlay. Native: absolute-positioned overlay with safe-area insets; Android optional system toast.

### Connections
Used by nearly every feature for errors/copy confirmation; plugin client UI; provider notices.

---

## Notification sounds

### What the user sees
Only two sound behaviours exist:
1. Desktop OS notifications: sound on/off via the "Play sound" desktop setting (Electron `silent: !playSound`, desktop/src/features/notifications.ts:104). Default on.
2. Mobile push: `sound: "default"` is always set in the Expo message (server push-service.ts:95); foreground native handler suppresses banner/sound (`shouldShowAlert/Banner/PlaySound: false`, `shouldShowList: true`; source.native.ts:12-18).
- No in-app audio chime, no per-event sound, no custom sound files (grepped app/src, server/src, desktop/src for `sound`; remaining hits are voice/dictation audio, which are a separate area).
- Web `Notification` API: no `silent` option passed (browser default).

---

## Timeline "notification" items and agent finish subscriptions (adjacent, not OS notifications)

- Timeline item type `"notification"` (protocol/src/agent-types.ts:383, messages.ts:751); capability `timelineNotifications` (`timeline_notifications`, client-capabilities.ts:45; server filters at session.ts:1358-1362: COMPAT v0.7.2).
- Agent-to-agent finish notifications (a parent agent subscribed to a child's finish): `formatFinishNotificationBody` and `FinishNotificationReason` in server/src/server/agent/finish-subscriptions/internal/notification.ts; message limit `FINISH_NOTIFICATION_MESSAGE_LIMIT = 4000`. Belongs to the agents/orchestration area.

---

# Usage and limits

(All app paths below are in `app/src/usage/` unless prefixed. Per-file line numbers are exact for 0.15.4.)

## Account usage reports: protocol and daemon pipeline

### What the user sees and does
Provider account quotas (Claude session/weekly, Codex windows and credits, Cursor plan, Grok credits, etc.) as cards with bars, countdowns and balances. Surfaces: sidebar footer summary, Usage modal, host settings page section, per-agent context-window popover/sheet, and CLI `paseo usage`.

### How it works end to end
- Wire (protocol/src/messages.ts):
  - `usage.list_reports.request` (:1875) `{requestId, agentId?, reportIds?: string[], forceRefresh?: boolean}`; `agentId` and `reportIds` are mutually exclusive (server throws "agentId and reportIds cannot be combined", client too).
  - Streams `usage.list_reports.update` (:6523) `{requestId, report: UsageReportEntry}` per account as each finishes, then `usage.list_reports.response` (:6527) `{requestId, error: string|null}`.
  - `UsageReportEntry` (:6498): `{id: "<sourceId>:<accountKey>", account:{label?}, fetchedAt (attempt time), sourceId, sourceLabel, icon? (SVG string), report, successfulAt?, lastSuccessful?{fetchedAt, report(available)}, loginErrors?[{harness, report(unavailable|error)}]}`.
  - `UsageReport` (:6493) discriminated by `status`: `available {planLabel?, windows[], balances?[], details?[]}` | `unavailable {problem}` | `error {error}`.
  - `UsageProblem` (:6473): `{kind:"expired", expiresAt, refreshedBy?}` | `{kind:"rejected", status:int, refreshedBy?}` | `{kind:"no_quota", detail}`.
  - Window (`ProviderUsageWindowSchema` :6414): `{id, label, shortLabel?, summary?, usedPct?|null, remainingPct?|null, resetsAt?|null, runsOutAt?|null, shortfallPct?|null, tone?}`; Balance (:6432): `{id,label,used?,remaining?,limit?, unit: "usd"|"credits"|"requests"|"tokens", resetsAt?, tone?}`; Detail (:6443): `{id,label,value,tone?}`; tone enum `default|ok|warning|danger` (:6411).
  - Legacy (COMPAT providerUsageList v0.1.98, remove after 2027-03-26): `provider.usage.list.request` (:1870) -> `provider.usage.list.response` `{requestId, fetchedAt, providers: ProviderUsage[]}` (:6464); errors as `rpc_error` code `provider_usage_list_failed`.
  - Feature flags in `server_info.features` (messages.ts:3691-3693, 3789): `usageSetting`, `usageSources`, `providerUsageList`. Daemon advertises `usageSetting: true, usageSources: true` (server/src/server/websocket-server.ts:2007-2008).
- Client: `DaemonClient.listUsageReports(options, onReport)` (client/src/daemon-client.ts:5786): throws "Update the host to see usage." unless `supportsUsageReports(features)` (:1300 = `usageSources === true || providerUsageList === true`); for old hosts without `usageSources` falls back to `listProviderUsage()` (:5777) and maps providers to entries (`agentId` queries return empty). Otherwise subscribes raw messages by `requestId`, collects `update`s, resolves on `response` (throws if `error !== null`).
- Server session handler: `UsageSession` (server/src/server/session/usage/usage-session.ts:16) created in session.ts:1045; `isEnabled: () => daemonConfigStore.get().usage?.enabled !== false`; dispatch session.ts:3195-3198. When disabled: error "Usage is off on this host."; no plugin runtime: "Plugin runtime is unavailable". Each report is emitted as `usage.list_reports.update` as soon as ready; `response` after all.
- Registry: `UsageSourceRegistry` (server/src/server/plugins/usage-sources/index.ts:53), constructed by `PluginService` (server/src/server/plugins/index.ts:115) with `ttlMs = 300_000` (5 minutes) and `deadlineMs = 20_000` default (index.ts:67 of usage-sources). Key behaviours:
  - `listReports` (:90): global scope discovers accounts from every registered source (`discover({kind:"global"})`), session scope (`agentId`) discovers with `{kind:"session", provider, model, env}` from `agent.usageSession()` (`AgentUsageSession {provider, model?, env, sessionKey}`, agent-sdk-types.ts:670), cached per agent while `sessionKey/provider/model` unchanged (`byAgent` map, pruned when agent disappears); `reportIds` selects known ids only.
  - Account identity: `id = "<sourceId>:<account.key>"`, `key` must match `/^[A-Za-z0-9._-]{1,128}$/`; multiple logins for one account are ordered and tried in parallel inside the same 20 s deadline, first `available` wins, else the last failure plus `loginErrors[]` (`fetchWithFallback` :335). Fetch timeout yields `{status:"error", error:"Usage fetch timed out"}` (:324).
  - Cache: `cache` Map keyed by `cacheIdentity` (id + sha256 of label+login set), fresh while `now - at < ttlMs`; `forceRefresh` bypasses it; concurrent identical fetches share one promise (`pending`); `successful` map keeps the last available report so failed refreshes expose `lastSuccessful`; `successfulAt` set only on `available`.
  - Failed discovery keeps previous accounts for that source (:222-229).
  - `listLegacyUsage` (:241) adapts to the legacy shape.
- Config: `daemon.usage.enabled` (daemon-config-store.ts:186, alias `usage.enabled` :214; default enabled unless explicitly `false`, config.ts:526 `resolveUsageEnabled`, exposed as `usageEnabled` :555). Patched through `MutableDaemonConfigPatch.usage {enabled}` (messages.ts:261, COMPAT usageSetting v0.14, remove after 2027-05-01).
- Sources are plugins: `registerUsageSource` in plugin-process.ts:142 (duplicate id -> "Duplicate usage source"); plugin SDK types `UsageSourceRegistration {id,label,icon?,input: ZodType, discover(scope), fetch(input)}` and helpers (`windowFromUsedPct`, `windowFromReportedDuration`, `toneFromUsedPct`, `balanceToneFromRemaining`, `usedPctOf`, `hashAccountKey`, `unavailable`) in packages/plugin/src/server/usage.ts (`UsageScopeSchema` :67). Tone thresholds (`toneFromUsedPct` :169): used > 90 -> danger, >= 70 -> warning, else ok (null -> default). `durationWindowName` (:147) maps 604800 s -> id `weekly`/"Weekly"/"wk", 18000 s -> id `five_hour`, else `<seconds>s`, label `<n>-<unit>`.
- Built-in usage source plugins (listed in `builtinPlugins`, server/src/server/plugins/builtin/index.ts:5-19, source in repo `plugins/<id>/`): `claude-usage-source`, `codex-usage-source`, `copilot-usage-source`, `cursor-usage-source`, `grok-usage-source`, `kimi-usage-source`, `minimax-usage-source`, `opencode-go-usage-source`, `zai-usage-source` (each `index.server.ts` is 15 lines calling `registerUsageSource`; logic in `server/usage.ts`). Details in "Usage source plugins" below.

### States, errors, limits, defaults
TTL 300 000 ms; per-fetch deadline 20 000 ms; each provider HTTP call uses `AbortSignal.timeout(15_000)`. Daemon usage default ON. Unsupported hosts: "Update the host to see usage." / CLI `UNSUPPORTED_HOST`.

### Platforms
Same on all clients.

### Connections
Plugins runtime (usage source plugins), agent sessions (`usageSession()` implemented in `claude/agent.ts:3631`, `codex-app-server-agent.ts:3532`, `opencode-agent.ts:3484`, `pi/agent.ts:1216`, `omp/agent.ts:736`, `acp-agent.ts:1836`), daemon config, host settings.

---

## Usage source plugins (per provider fetchers)

Each is a built-in plugin (repo `plugins/<name>-usage-source/`, manifest `paseo-plugin.json` e.g. `{"id":"claude-usage-source","requirements":{"paseo":">=0.9.2"}}`). Read-only: they re-read the provider's login store, never write/refresh it; an expired token yields `unavailable{kind:"expired", refreshedBy}`, HTTP 401/403 yields `unavailable{kind:"rejected", status}`.
- Claude (`plugins/claude-usage-source/server/usage.ts`): GET `https://api.anthropic.com/api/oauth/usage` with `anthropic-beta: oauth-2025-04-20` (:24-25, :537), also `/api/oauth/profile` for account identity (cache `PROFILE_TTL_MS = 300_000`, `PROFILE_CACHE_LIMIT = 128`, :639-640). Credentials from `<CLAUDE_CONFIG_DIR|~/.claude>/.credentials.json`, macOS Keychain service `"Claude Code-credentials"` (`CLAUDE_KEYCHAIN_TIMEOUT_MS = 2_000`, fallback account `claude-code-user`), plus Pi/OMP stores (`stores.ts`: `~/.pi/agent/auth.json`, OMP sqlite `auth_credentials`). Windows: `five_hour` "Session" shortLabel `5h` (summary), `seven_day` id `weekly` "Weekly" `wk` (summary) (:123-124); scoped weekly windows from `limits[]` and legacy `seven_day_opus`/`seven_day_omelette` with id `weekly_<dimension>_<id|name>`, label `Weekly · <name>`, shortLabel `wk <name>`. Details: `extra_usage` "Enabled"/"Disabled". Plan label from `subscriptionType`/`rateLimitTier`. 429 handling: per-token `rateLimitedUntil` map honoring `Retry-After` (even on forced refresh), error "Rate limited by Claude. Try again in N m/h." Skips custom `ANTHROPIC_BASE_URL` routes (:444).
- Codex (`plugins/codex-usage-source/server/usage.ts`): GET `https://chatgpt.com/backend-api/wham/usage` (:207) with Bearer + `ChatGPT-Account-Id`; auth from `$CODEX_HOME/auth.json` or `~/.codex/auth.json`, OpenCode `auth.json` (`$XDG_DATA_HOME|~/.local/share/opencode/auth.json`), Pi/OMP stores. Windows from `rate_limit.primary_window/secondary_window` (duration -> `windowFromReportedDuration`, first group `summary`), `additional_rate_limits[]` (scoped id `limit:<name>`), `code_review_rate_limit` (scope `code_review`/"Code review"); unknown length -> ids `unknown_primary`/`unknown_secondary`, labels "Primary limit"/"Secondary limit", shortLabel "". Balance `credits` (unit `credits`, `remaining`, tone by `balanceToneFromRemaining`). Plan = `plan_type`. HTML response -> error "Codex usage API returned HTML". Account key from JWT claims, never token/email.
- Copilot (`copilot-usage-source/server/usage.ts:54`): GET `https://api.github.com/copilot_internal/user`, token from `gh` `hosts.yml` (`~/.config/gh/hosts.yml`, Windows `%APPDATA%/GitHub CLI/hosts.yml`) or env; report has no windows/balances: plan label `copilot_plan` + detail `reset` "Quota reset".
- Cursor (`cursor-usage-source/server/usage.ts:166`): POST `https://api2.cursor.sh/aiserver.v1.DashboardService/GetCurrentPeriodUsage`; token from sqlite key `cursorAuth/accessToken` (legacy `cursorAuthStatus`), `~/.config/cursor/auth.json` (macOS `~/.cursor/auth.json`, Windows `%APPDATA%/Cursor/auth.json`) or env. Windows `plan_usage` "Total" (summary), `cursor_usage` "Cursor", `third_party_usage` "Third Party", reset at billing cycle end; fallback balance `plan_usage` in `usd`.
- Grok (`grok-usage-source/server/usage.ts:124`): GET `https://cli-chat-proxy.grok.com/v1/billing?format=credits` (+`X-XAI-Token-Auth: xai-grok-cli`), token from `~/.grok/auth.json` (keys starting `https://auth.x.ai::` preferred). One window: `weekly`/"Weekly"/`wk` or `monthly`/"Monthly"/`mo` (else `period:<type>`) from `config.creditUsagePercent`; one credits balance.
- Kimi (`kimi-usage-source/server/usage.ts:21`): `https://api.kimi.com/coding/v1/usages`, token `~/.kimi/credentials/kimi-code.json`; windows from `limits[]` labelled like "N-hour limit".
- MiniMax: base `https://api.minimax.io` or `https://api.minimaxi.com` (CN), path `/v1/token_plan/remains`, stores `~/.mmx/credentials.json` / `config.json`; windows `interval_<model>` ("<model> · Interval") and `weekly_<model>`; `no_quota` "No active coding plan".
- OpenCode Go (`opencode-go-usage-source/server/usage.ts:59`): GET `https://opencode.ai/zen/go/v1/usage` Bearer api key from OpenCode `auth.json`; windows `rolling` "Rolling" (shortLabel ""), `weekly` "Weekly" `wk`, `monthly` "Monthly" `mo`.
- Z.ai (`zai-usage-source/server/usage.ts:35`): GET `https://api.z.ai/api/biz/subscription/list`; only details (`status`, `valid`, `purchase_time`) and plan = `productName`; no windows.

---

## Usage query layer and caching (app)

### How it works end to end
- `queries.ts`: TanStack Query keys `["usage", serverId]` (host root), `[..., "reports"]` (host reports), `[..., "agent", agentId]` (:21-31).
- `REPORTS_STALE_TIME_MS = 60_000` (queries.ts:38) (daemon cache is 5 min; only an explicit refresh sends `forceRefresh`).
- `streamReports` (:64): calls `client.listUsageReports({agentId, forceRefresh}, onReport)`; each streamed report is merged into the query cache with `upsertReport` (keeps a newer-fetched copy), the finished list is reconciled with `settleReports` (model.ts:84: keep display order, drop vanished, append new).
- `useHostUsage(serverId)` (:135) -> `{view: UsageView, refresh}`; enabled only when connected, host supports usage and the host `usage.enabled` setting is on; `dataShape:"list"`. `refresh` = `refreshReports` (:118) `fetchQuery` with `staleTime: 0` and `forceRefresh=true` (refresh all).
- `useUsageHostReports(serverId|null)` (:171) feeds the sidebar summary. `useAgentUsage(serverId, agentId)` (:188): `dataShape:"value"`, `gcTime: 0`, `retry:false`, abort signal cancels when details close. `useHostReportsUsage` (:214) = connected && supported && setting enabled. `useUsageHosts` (:222) = per host `{serverId,label,isConnected,supportsUsage}`. `useReportRefresh(serverId, reportId, agentId?)` (:245): mutation `getReport(..., forceRefresh=true)` replacing one card in place (`replaceReport`, model.ts:51; `null` removes it).
- `resolveUsageView` (model.ts:107) -> `UsageView` kinds: `unavailable` (host offline: `providerUsage.hostUnavailable`; host lacks support: `providerUsage.hostUpgradeRequired`), `off` (`providerUsage.off`), `ready {reports, isRefreshing}`, `error {message}`, `loading`. `resolveAgentUsageView` (:150): `none | loading | error (providerUsage.agentError) | ready`; empty list -> `none`. A failed request hides partial reports.
- `resolveUsageReport` (model.ts:209): current outcome owns messages; only successful poll owns values/freshness; for unavailable/error entries falls back to `lastSuccessful` snapshot (`retained=true`), messages from `loginErrors` as `"<harness>: <problem text>"` else the single problem; `successfulAt = entry.successfulAt ?? entry.fetchedAt` for available entries.
- `useUsageSetting(serverId)` (setting.ts:6): reads `config.usage.enabled` via `useDaemonConfig`; `supported` = `features.usageSetting`; `enabled = !supported || (config !== null && config.usage?.enabled !== false)`; `setEnabled` patches `{usage:{enabled}}`; `turnOn` convenience.
- Relative-time labels re-render through `useRelativeTimeTick` (use-relative-time-tick.ts) subscribing to shared `"minute"` tier of `app/src/utils/relative-time-ticker.ts` (`INTERVAL_MS`: minute 60_000, hour 30*60_000, day 60*60_000).

### States
Loading text `providerUsage.loading`; error Alert (`providerUsage.errorTitle`) with Retry button (`providerUsage.retry`); empty `providerUsage.empty`; off with "Turn on" button (`providerUsage.turnOn`).

### Platforms / Connections
Shared. Feeds all usage UIs; uses host registry (connection status), session store `serverInfo.features`, daemon config replica.

---

## Usage cards, bars, formatting and tone

### What the user sees
`UsageCard` (card.tsx:37) per report: header = source icon (`UsageSourceIcon`, inline SVG string from the plugin) + source label + plan `StatusBadge` + status (dot + "error"/"unavailable" text) + per-report Refresh button; then optional error lines (`testID usage-login-error`); window bars (`UsageWindowBar`), balance bars (`UsageBalanceBar`), detail rows (label/value); footer = account label and (on native/compact/non-refreshable) freshness text ("Updated 3m ago" / "Last updated ..." when retained / "no successful update"); `providerUsage.refreshFailed` after a failed per-card refresh (`usage-refresh-error`). Tooltip of the card Refresh button shows freshness. testIDs: `usage-report-<id>`, `usage-refresh`, `usage-freshness[-tooltip]`.
- Window row (window-bar.tsx): label (localized via `formatProviderUsageLabel` for ids session/weekly/monthly/code_review/credits/monthly_credits/five_hour), value `formatDisplayPct` ("31% used" / "69% left"), pin glyph when pinnable, meter bar (`UsageMeter` meter.tsx, fill colour by tone ok/warning/danger/default, `quiet` surface fill for summaries), reset line (`UsageResetLine`: countdown then exact time; the exact time truncates first). `isAtRisk` = `runsOutAt` and `shortfallPct` present (and not already passed when retained): countdown becomes red "runs out in ..." and the exact reset time is hidden. Whole row is a checkbox (`accessibilityRole="checkbox"`, testID `usage-pin-<sourceId>-<windowId>`) when pinnable.
- Balance row (balance-bar.tsx): `limit>0` -> "used / limit" and used%; only `remaining` -> "<amount> left"; only `used` -> amount; else "—". Amount formats (`formatAmount`, format.ts:185): usd -> currency USD, tokens -> compact 1 decimal, others up to 2 fraction digits.
- Formatting (format.ts): `formatPct` clamps to 0-100, integer percent via Intl; durations are two units floored ("2d 23h", "4h 59m", "37m"); `formatResetLabel` ("resets in ...", "resetting now", `resetTimePassed` when retained); `formatResetTime` ("3:00 PM" within 24 h, "Fri 3:00 PM" within 7 days, "Oct 31, 3:00 PM" later; nothing once passed); `formatRunsOutLabel`; `formatAgo` ("just now" under 60 s, minutes, hours, days). Keys under `providerUsage.*`.
- Tone: `windowTone` (tone.ts:4) = source tone if provided, else used% > 90 -> danger, >= 70 -> warning, else default.
- Used/remaining: `displayPercent` (model.ts:14): `remainingPct` preferred in "remaining" mode, `usedPct` or `100 - remainingPct` in "used".

---

## Sidebar Usage summary item (pinned windows)

### What the user sees and does
Optional footer row in the left sidebar (hidden by default: `HIDDEN_BY_DEFAULT = new Set(["usage"])`, app/src/sidebar-nav/model.ts:73; footer built-ins `["usage"]` :12; label key `sidebar.footer.usage`). Shows one line: per account the source icon, then per pinned window a meter and "31% 5h" (testID `sidebar-usage`, children `sidebar-usage-source`, `sidebar-usage-pinned-window`). Renders nothing while no pinned window has data (the footer's Usage gauge icon `CircleGauge`, testID `sidebar-usage-icon`, already opens Usage). Pressing it opens the Usage modal. Mounted in `app/src/components/left-sidebar.tsx` (`UsageSidebarRoot` :428, `SidebarUsageIcon` :460, `SidebarFooterRows` :476-497).

### How it works end to end
- `useUsageSummary` (sidebar-item.tsx:37): only fetches when the item is visible (`useUsageInSidebar`), via `useUsageHostId` (hosts.ts:263) -> `useUsageHostReports` -> `resolvePinnedUsage` (pinned.ts:201). `resolvePinnedUsage` groups accounts by source (first-appearance order), drops non-available reports, keeps windows with a percent that are in `effectiveUsagePins`.
- Responsive layout (pinned.ts:161-184, sidebar-item.tsx:109): `choosePinnedUsageLayout` picks `"meters"` (needs `labelsWidth + windowCount*(MIN_METER_WIDTH 16 + METER_GAP 4) <= available`), else `"labels"`, else `"percents"`; widths measured by off-screen copies (10 000 px wide); before measurement it shows labels.
- `UsageSidebarRoot` mounts `UsageModal` lazily on first open and unmounts on dismiss, so host reports load only while it is shown (sidebar-item.tsx:66-82). `useOpenSidebarUsage` throws outside the root.
- Pin model (preferences.ts): `UsagePin {sourceId, windowId}` (matches every account of that source); `pins: null` = follow source defaults (`window.summary === true` windows; if a report marks none, its first window with a percent) (`effectiveUsagePins` :51); array = explicit selection (may be empty). `toggleUsagePin` (:80) first snapshots the effective defaults, then toggles. Pinning only available while the sidebar summary is on (`UsageList` passes `pinnable={inSidebar}`).
- Visibility toggle: `useUsageInSidebar` (in-sidebar.ts) = `useSidebarNavItems("footer")` item `key==="usage"` visible/`setVisible("usage", ...)`; same switch as Settings > Sidebar and "Show in sidebar" in the usage options menu.

### Platforms
Row appears wherever the left sidebar renders (desktop/web, and mobile drawer).

### Connections
Sidebar nav preferences, host selection preference, app settings.

---

## Usage modal (and host filter, options menu)

### What the user sees and does
Opened by the footer icon or summary row. `UsageModal` (usage-modal.tsx:23): `AdaptiveModalSheet` (desktop dialog `desktopMaxWidth={640}`, bottom sheet on compact), title = `sidebar.footer.usage` label, title-row actions: `HostFilter` (only when more than one connected host; `includeAllHost={false}`, testIDs `usage-host-filter-trigger`, `usage-host-filter-item-<serverId>`), Refresh all (`UsageRefreshButton`, spins while loading/refreshing; hidden when the host's usage is off), Options cog (`UsageOptions`, hidden when host unavailable). Body: `UsageBody` (usage-section.tsx:366) -> `UsageList` (list.tsx) = one card per report with refresh; empty state `providerUsage.noHosts` when no connected host. testIDs `usage-modal`, `usage-modal-body`, `usage-host-<serverId>`.
- Host choice (hosts.ts, model.ts:189-222): `resolveUsageHostId` = picked host -> active workspace host -> first, only among connected hosts that support usage; `resolveUsageModalHostId` = picked connected host even if it cannot report (so the modal tells the user to update) -> sidebar host -> first connected. Picking a host (`useUsageHostSelection().select`) persists `settings.usage.serverId` via `updateSettings`. Modal is keyed by host so refresh state never leaks across hosts.
- Options popover (options.tsx:214; `DropdownMenu compactMode="sheet"`, width 340, testIDs `usage-options-menu`, `usage-options-surface`, `usage-options-fields`): "Check usage" switch (`providerUsage.checkUsage`, host setting `usage.enabled`; only when `features.usageSetting`; disabled until config loaded; error shown as Alert), "Show in sidebar" switch (`usage-show-in-sidebar`), "Display as" segmented control `used`/`remaining` (`usage-display-used`/`usage-display-remaining`, wrapper `usage-display-as`).
- Same controls reused at the host settings page: `HostUsageSection` (host-usage-section.tsx; placed in `app/src/screens/settings/host-page.tsx:358`, title `providerUsage.title`, testID `usage-card`) via `UsageSection` (SettingsSection with controls as trailing).

### Settings keys (see storage table): `AppSettings.usage = {displayAs: "used"|"remaining" (default "used"), pins: UsagePin[]|null (default null), serverId: string|null (default null)}` (preferences.ts:23; hooks/use-settings/storage.ts:104,161,258). Legacy field `pinned` read once (COMPAT usagePinSelection v0.11, remove after 2027-04-02): non-empty -> pins, empty -> null. Malformed fields fall back individually (zod `.catch`).

### Platforms
Dialog on wide layouts, sheet on compact; usage everywhere supported.

### Connections
Host registry/filter, sidebar nav items, app settings, daemon config (`usage.enabled`).

---

## Per-agent usage (context window meter + account usage)

### What the user sees and does
In the composer, a small ring meter (`ContextWindowMeter`, app/src/components/context-window-meter.tsx; mounted app/src/composer/index.tsx:2089) shows how full the agent's context window is. Colour: >90% danger, >=70% warning (`getProgressColor` :63). Hover card (desktop) / tooltip (native wide) / bottom sheet (compact) = `ContextWindowDetails` (components/context-window-details.tsx): "N% used", "Xk / Ym tokens" (`formatTokenCount`: >=1M -> "<n>m", >=1k -> "<n>k"), session cost (`formatSessionCost`, from `totalCostUsd`), then `AgentUsage` (usage/agent-usage.tsx:19) divider + one `UsageCard` per account-report of the agent's login (compact, not pinnable; Refresh button only if the surface is pressable). Popover width `USAGE_POPOVER_WIDTH = 300` when usage is available (`useHostReportsUsage`).
- Data: `AgentUsage` wire shape `{inputTokens?, cachedInputTokens?, outputTokens?, totalCostUsd?, contextWindowMaxTokens?, contextWindowUsedTokens?}` (protocol messages.ts:434; agent-types.ts:192) arrives in `turn_completed.usage` (messages.ts:786) and as agent snapshot `lastUsage` (:883); app `agentState.contextWindowMaxTokens/UsedTokens/totalCostUsd`.
- Provider pollers: Pi (`server/src/server/agent/providers/pi/usage-poller.ts`) and OMP (`omp/usage-poller.ts`) poll session stats 3 000 ms after each trigger (`setTimeout(callback, 3_000)`, :18) and emit `AgentUsage` (tokens in/out/cacheRead, cost, `contextUsage.contextWindow/tokens`); OpenCode v2 maps usage in `opencode/v2/usage.ts`. Claude/Codex/ACP fill it from their own events (not traced in detail).
- Account usage per agent: `useAgentUsage` -> `usage.list_reports.request {agentId}`; the daemon resolves the agent's `usageSession()` (provider, model, env overrides, sessionKey) so an agent launched with a different login (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`) shows that account's quota. Unknown agent -> error "Unknown agent: <id>".

---

## CLI: `paseo usage`

- `paseo usage` / `paseo usage ls [--agent <id>] [--refresh]` / `paseo usage inspect <report-id> [--refresh]` (cli/src/commands/usage/index.ts:75-93; `--json` and daemon host options). Requires `features.usageSources === true` else error `UNSUPPORTED_HOST` "Update the host to use usage reports." `--agent` + `inspect` -> `INVALID_OPTIONS`. `inspect` first lists to discover accounts, then optionally refreshes only that id (`reportIds`, `forceRefresh:true`); not found -> `REPORT_NOT_FOUND`. Output schema `usageSchema` (cli/src/commands/usage/presentation.ts).

## Cross-area connections

- Agent lifecycle and pending permissions drive attention, sidebar status and notification routing; stored unread state and notification callbacks have separate boundaries (corrections in ../gaps.md). Parent finish subscriptions are agent delivery rather than OS notifications.
- Client heartbeat visibility/focus/activity connects websocket presence, notification suppression, recipient selection and push-token lease renewal.
- Notification taps route host/workspace/agent/terminal identity through connection bootstrap and pane navigation; Electron/web permissions and Expo/F-Droid capabilities determine available display channels.
- Unread state feeds sidebar/tab/favicons/dock badge; composer focus/send, pane blur and workspace read/unread actions clear or restore eligible attention.
- Usage source plugins map provider credentials/accounts to daemon queries, sidebar pinned windows, host-filtered modal and per-agent context details. Account quota and model-context usage remain distinct.
- Local settings own usage preferences; Electron settings own notification sound. CLI usage and app usage share daemon reports, plugin RPCs, formatting and cache.

## Inventory

- Notification routing (the server-side "who gets notified" plan)
- What the user sees
- Client presence heartbeat (`client_heartbeat`)
- What the user sees / does
- In-app OS notification display (desktop Electron, web Notification API)
- Desktop notification settings (Settings > Notifications)
- Notification open routing (tap/click -> navigate)
- States, errors
- Mobile push notifications (Expo push)
- Agent attention / unread state ("requires attention")
- Terminal attention / activity
- What the user sees
- Sidebar / tab attention badges, favicon and dock badge
- What the user sees
- In-app toasts
- What the user sees
- Notification sounds
- What the user sees
- Timeline "notification" items and agent finish subscriptions (adjacent, not OS notifications)
- Account usage reports: protocol and daemon pipeline
- Usage source plugins (per provider fetchers)
- Usage query layer and caching (app)
- States
- Platforms / Connections
- Usage cards, bars, formatting and tone
- What the user sees
- Sidebar Usage summary item (pinned windows)
- Usage modal (and host filter, options menu)
- Settings keys (see storage table): `AppSettings.usage = {displayAs: "used"|"remaining" (default "used"), pins: UsagePin[]|null (default null), serverId: string|null (default null)}` (preferences.ts:23; hooks/use-settings/storage.ts:104,161,258). Legacy field `pinned` read once (COMPAT usagePinSelection v0.11, remove after 2027-04-02): non-empty -> pins, empty -> null. Malformed fields fall back individually (zod `.catch`).
- Per-agent usage (context window meter + account usage)
- CLI: `paseo usage`

- Presence age threshold and focused visible client suppression
- Most-recent active client selection vs mobile push fallback
- Client heartbeat activity/focus/visibility coalescing
- Expo push token registration, revocation and lease renewal
- F-Droid push stubs and permission-denied behavior
- Notification target navigation and queued host route bootstrap
- Deferred focus-entry unread clearing vs input/send/blur
- Workspace Mark as read and Mark as unread
- Permission/error/finished sidebar bucket aggregation
- Terminal activity attention and focus clearing
- Desktop sound preference and notification test
- Toast lifetime, dismissal and error/info presentation
- Account report discovery and per-agent account association
- Usage source plugin refresh/caching and structured failures
- Usage used/remaining display, pinned windows and host filter
- Per-agent context-token meter vs account quota reports
- Usage CLI list/inspect and unsupported-host errors
