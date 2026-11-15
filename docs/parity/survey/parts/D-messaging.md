# D - Messaging, turns, approvals, history search, session store, session layer (Paseo 0.15.4)

Source root: `~/refs/paseo-latest/packages`. Paths are relative to it. Short names used below:
`S` = `server/src/server`, `A` = `app/src`, `P` = `protocol/src`, `C` = `client/src`, `CLI` = `cli/src`.
`S/session.ts` is the websocket "Session" class (8915 lines). Line numbers refer to this checkout.

Wire naming: inbound = client -> daemon, outbound = daemon -> client. All session messages are zod schemas in
`P/messages.ts` (`SessionInboundMessageSchema` union starts ~3290, `SessionOutboundMessageSchema` union ~7040).
Requests that expect an answer carry `requestId` (string); the client correlates on it.

---

## 1. Composer: send path and state machine (app)

### 1.1 What the user sees
- One `Composer` component serves agent chat (`inputMode: "chat"`) and terminal prompts (`"terminal"`): `A/composer/input-mode.ts:36-45`.
  Chat shows attachments, voice/dictation, slash and `@` autocomplete, agent-control chips; terminal hides all of them and uses monospace.
- Mounted in the agent panel: `ActiveAgentComposer` `A/panels/agent-panel.tsx:1520` -> `<Composer ...>` `:1620`; in the draft (new agent) tab: `A/composer/draft/workspace-tab.tsx`. Dock layout: `A/composer/dock/index.tsx` (`ComposerDock`, fullscreen variant).
- Send button, Stop (square) button, queue list above input, send-error line (`sendError`, role alert), attachment tray, context-window meter, voice-mode button (only when idle and no sendable content on compact layouts: `ComposerRightControlsSlot` `A/composer/index.tsx:1152`).
- When the agent has an active turn the right button becomes the Stop button (`ComposerCancelButton` `index.tsx:1092`, tooltip `composer.cancel.interrupt`, spinner while cancelling, a `Shortcut` chip for `agent-interrupt`).
- Placeholders: `composer.placeholders.desktop|mobile|terminal` (`resolveMessagePlaceholder` `index.tsx:227`).
- Recall: ArrowUp on an empty composer walks previously sent user messages (newest first, consecutive repeats collapsed), ArrowDown walks back; any edit ends recall; an open autocomplete owns the arrows. `A/composer/recall.ts` (`ComposerRecall`), source list `selectAgentSentMessages` `A/stores/session-store.ts:336`.

### 1.2 Submit pipeline (end to end)
1. `MessageInput` (`A/composer/input/input.tsx`) raises `onSubmit(MessagePayload{text,attachments,cwd,forceSend?})` -> `handleSubmit` `A/composer/index.tsx:1715`.
2. Client slash commands run first: `resolveClientSlashCommand` / plugin client slash commands (`A/client-slash-commands`, `A/plugins/client-slash-commands`). A handled command returns without touching the agent. The agent panel supplies `onClientSlashCommand` (`agent-panel.tsx` ~`handleClientSlashCommand`): kinds `replace-agent-with-draft` (archive agent, retarget tab to a new draft seeded by `buildDraftAgentSetup`) and a close-tab variant.
3. `sendMessageWithContent` `index.tsx:1656` -> `submitAgentInput` `A/composer/submit.ts:27` returns `"noop" | "queued" | "submitted" | "failed"`:
   - empty text + no attachments + no `hasExternalContent` + no `allowEmptySubmit` -> `noop`;
   - `canSubmit` false (no `sendAgentMessageRef` and no `onSubmitMessage`) -> `noop`;
   - agent running and not `forceSend` -> `queueMessage` (see section 7) and clear input -> `queued`;
   - else clear input + attachments immediately (unless `submitBehavior === "preserve-and-lock"`), `setSendError(null)`, `setIsProcessing(true)`, await `submitMessage`, then `clearDraft("sent")`.
   - On throw: `onSubmitError`, restore text/attachments (when cleared), `setSendError(error.message || i18n composer.errors.failedToSend)` -> `failed`.
4. `submitMessage` `index.tsx:1536`: calls `onMessageSent`, then either the parent's `onSubmitMessage` (draft tab: creates an agent) or `sendAgentMessageRef` which calls `dispatchComposerAgentMessage` (`A/composer/actions.ts:200`).
5. `dispatchComposerAgentMessage`: `splitComposerAttachmentsForSubmit` -> `{images, attachments}`; `clientMessageId = generateMessageId()` (`A/types/stream.ts:30`, format `msg_<Date.now()>_<9 base36 chars>`); build optimistic `UserMessageItem` via `createUserMessage` (with `turnId` only when behavior is `steer` and an active turn id is known); `submission.begin`; encode images to base64 (`encodeImages`); `client.sendAgentMessage(agentId, text, {messageId: clientMessageId, activeTurnBehavior?, images, attachments})`; `submission.accept` or on error `submission.reject` + rethrow.
6. After a successful send `onAttentionPromptSend` fires (clears attention, section 31).

### 1.3 Send settings and shortcuts
- Setting `sendBehavior: "interrupt" | "steer" | "queue"`, default `"steer"` (`A/hooks/use-settings/storage.ts:71,166`; legacy `"interrupt"` migrated to `"steer"` in `use-settings/migrations.ts:42`; zod `.catch("steer")` `storage.ts:225`). UI: Settings > General > Sending (`A/screens/settings/general/sending-section.tsx`).
- Setting `sendShortcut: "enter" | "mod-enter"`, default `"enter"` (`storage.ts:166,271`). `resolveMessageInputEnterAction` (`A/composer/input/state.ts`): Shift+Enter never submits; with `enter`: plain Enter = default send, Mod+Enter while agent running and `onQueue` present = alternate send; with `mod-enter`: only Mod+Enter submits.
- `resolveAlternateSendAction`: while running, alternate of `queue` is `send`, alternate of `steer`/`interrupt` is `queue`; null while idle. Long-press on Send = alternate (`runAlternateSendAction`).
- `resolveActiveSendBehavior(sendBehavior, hasPendingPermission)` (`state.ts:9`): `"queue"` while a permission is pending degrades to `"interrupt"` (queueing behind a permission prompt would strand the message because the turn is parked).
- In `queue` mode the default send queues while running (`runDefaultSendAction`); `"steer"`/`"interrupt"` send directly with that `activeTurnBehavior` (`index.tsx:1553`: anything not `steer` sends `interrupt`).
- Keyboard: `agent-interrupt` = Escape (`A/keyboard/keyboard-shortcuts.ts:1140`, scopes `message-input`,`other`, not in command center, no preventDefault) -> action `agent.interrupt` -> `ComposerKeyboardRegistration` (`index.tsx:582`,`dispatchComposerKeyboardAction :548`) calls `handleCancelAgent` only if running, not cancelling, connected. `message-input.mode-cycle` = Shift+Tab (`keyboard-shortcuts.ts:1077`, help id `cycle-agent-mode`). Dictation/voice actions: `message-input.dictation-toggle|cancel|confirm`, `voice-toggle`, `voice-mute-toggle` (`A/keyboard/actions.ts:5-17`, `runMessageInputKeyboardAction` in `composer/input/state.ts`).

### 1.4 Dictation transcript into composer
`applyDictationTranscript` (`composer/input/state.ts`): pads with a space if the text does not end in whitespace; with `autoSend` and `queue` behavior while running -> queue; else submit with `forceSend: isAgentRunning || undefined`.

### 1.5 States / limits
- Composer locked only for `submitBehavior="preserve-and-lock"` + `isSubmitLoading` (`resolveIsComposerLocked` `index.tsx:200`) - used by the draft tab while creating.
- Input surface vs overlay presentation `resolveComposerSurfacePresentation`; fullscreen composer (`ComposerFullscreenBody/Footer`).
- Auto-focus only on desktop web with a fine pointer (`messageInputAutoFocus` `index.tsx:2405`); native `blurOnSubmit`.
- Platforms: native paste of images `handleNativePasteImages` (`composer/native-pasted-image.ts`), iOS hardware keyboard submit hook `hooks/use-ios-hardware-keyboard-submit.ts`, web drag-drop (`useFileDrop`), desktop clipboard `readClipboardImage` (`composer/clipboard-image.ts`).
- Connections: draft store (2), attachments (9), queue (7), cancel (8), attention (31), voice (36), agent controls (mode, model, thinking, features) in `A/composer/agent-controls/*` (owned by another slice).

---

## 2. Composer drafts and draft store (app)

- Store: `useDraftStore` zustand + persist, storage name `"paseo-drafts"`, `DRAFT_STORE_VERSION = 5`, `A/stores/draft-store/{index,state,persistence,migration}.ts`.
- Shapes (`state.ts`): `DraftInput{text, attachments: UserComposerAttachment[]}`, `DraftRecord{input, lifecycle: "active"|"abandoned"|"sent", updatedAt, version}`; state `{drafts: Record<key, DraftRecord>, createModalDraft}` (+ in-memory `attachmentFocusRequestByDraftKey`).
- Keys (`A/stores/draft-keys.ts`): agent chat `agent:<serverId>:<agentId>`; draft tab `draft:<serverId>:<draftId>`; new-workspace `new-workspace` / `new-workspace:draft:<id>`; `generateDraftId()` = `draft_<msgid>`.
- `editDraftRecordText`: bumps `version`, lifecycle `active` while any text/attachment else `abandoned`. `clear(lifecycle)` from composer marks `sent` or `abandoned`.
- Persistence debounce `DRAFT_PERSIST_INTERVAL_MS = 200` (`persistence.ts:3`); `flushDraftPersistStorage()`.
- Finalised (non-active) records pruned after `FINALIZED_DRAFT_TTL_MS = 5 min` (`state.ts:10,244`).
- Image attachments are persisted separately (`A/attachments/store.ts`, kinds `web-indexeddb` | `desktop-file` | `native-file`); `scheduleAttachmentGc` (`draft-store/index.ts:220`) garbage-collects ids not referenced by active drafts, queued messages (`collectQueuedMessageAttachmentIds :191`), or stream user images (`collectStreamUserImageIds`). Legacy draft images migrated by `migration.ts` (`migrateDraftInput`).
- Hook: `useAgentInputDraft({draftKey, composer?})` `A/composer/draft/input-draft.ts` returns `{textSource, editText, replaceText, textReplacement, attachments, setAttachments, clear, isHydrated, attachmentFocusRequestId, composerState}`. `composerState` (draft tab only) wraps `useAgentFormState` (provider/model/mode/thinking selection) and builds `DraftAgentControlsProps` (`input-draft-core.ts buildDraftAgentControls`). `TextReplacement.kind: "initial" | "hydrated" | "replace"` (`A/composer/types.ts`).
- Queued (not yet sent) messages are NOT in the draft store: they live in the session store (section 25) and are lost on reload.

---

## 3. Send prompt: `send_agent_message_request` (server)

### 3.1 Wire
Inbound `send_agent_message_request` (`P/messages.ts:1428` `SendAgentMessageRequestSchema`): `requestId`, `agentId` (full id, unique prefix, or exact full title), `text`, `sourceAgentId?`, `sourceAgentServerId?`, `sessionMode?`, `messageId?`, `activeTurnBehavior?: "interrupt"|"steer"`, `images?: [{data(base64), mimeType}]`, `attachments?: AgentAttachment[]` (invalid attachments are silently dropped by `normalizeAgentAttachments` `P/messages.ts:1226`).
Outbound `send_agent_message_response` (`:5123`): `{requestId, agentId, accepted, deliveryStatus?: "queued", error}`.
Legacy `SendAgentMessageSchema` (`type:"send_agent_message"`, `:1257`) is defined but not in the inbound union and not dispatched (searched `S/session.ts`, `C/daemon-client.ts`: not found).
Permission: `["workspace.write","hub.execute"]` (`S/authorization/operation-permissions.ts:180`).

### 3.2 Handler `handleSendAgentMessageRequest` (`S/session.ts:8304`)
1. `resolveAgentIdentifier` (`:5349`): exact id (live or retired-internal) -> exact id in storage/live -> unique id prefix -> unique exact title among non-internal stored records; ambiguous -> error text `Agent identifier "x" is ambiguous (...)` / `Agent title ... is ambiguous`; not found -> `Agent not found: x`. Failure -> response `accepted:false, error`.
2. `buildAgentPrompt(text, images, attachments)` (`S/agent/prompt-attachments.ts:6`): returns trimmed string when no images/attachments; else `AgentPromptContentBlock[]` ordered: `chat_history` text attachments, then trimmed text block (if non-empty), then image blocks, then remaining attachments.
3. Two delivery paths:
   - **Agent-sourced** (`sourceAgentId` set): `agentMessages.enqueue({recipientId, messageId, prompt, source:{kind:"agent-message", agentId, serverId?, title?}, activeTurnBehavior ?? "steer", sessionMode, clearPendingPermissions:true})` (section 15). Response returns as soon as queued (`deliveryStatus:"queued"`).
   - **Human**: `sendPromptToAgent({..., activeTurnBehavior: msg.activeTurnBehavior ?? "interrupt", sessionMode, clearPendingPermissions:true})`; if disposition is `turn_started` also `waitForAgentRunStartWithTimeout` (section 10). Response after the provider accepted the turn.
4. If `messageId` present, the whole send is wrapped in `messageReceipts.send({agentId, messageId, request:{prompt, sourceAgentId, sourceAgentServerId, sessionMode, activeTurnBehavior}, prepare, send})` (section 5). `prepare` (human only) = `prepareAgentMessage` `:8291`: `ensureAgentLoaded`, and if the stored agent has no title and no `lastUserMessageAt` sets a provisional title from the text (`resolveCreateAgentTitles({initialPrompt})`). Without `messageId` there is no receipt and no provisional title.
5. Errors: `handleAgentRunError` (emits `activity_log` error `"<context>: <friendly message>"` `S/session.ts:1692`) and response `accepted:false, error: errorToFriendlyMessage(error)`. Aborted request signal -> no response. Queued agent-message dispatch failures after the early response are reported only via `activity_log` ("Failed to dispatch queued agent message").
6. Any unhandled handler exception becomes `rpc_error{requestId, requestType, error:"Request failed: ...", code:"handler_error"}` plus an `activity_log` error (`S/session.ts:2246-2282`).

### 3.3 Client
`DaemonClient.sendAgentMessage(agentId, text, options)` `C/daemon-client.ts:3887`: checks server features (`agentMessageProvenance` required for `sourceAgentId`, `agentMessageBatching` for `sourceAgentServerId`; error `Update the Paseo host to send messages with agent provenance.`), `messageId` defaults to `crypto.randomUUID()`, `skipQueue: true` (sent even while the transport is reconnecting? - `skipQueue` bypasses the offline outbound queue), throws `Error(payload.error ?? "sendAgentMessage rejected")` when `!accepted`, returns `{deliveryStatus?}`. `sendMessage` is an alias.

### 3.4 Other entry points that reuse the same pipeline (`sendPromptToAgent`)
Session voice path `handleSendAgentMessage` (`S/session.ts:4038`, wraps spoken text with `wrapSpokenInput`), MCP `send_agent_prompt` (`S/agent/tools/paseo-tools.ts:1944`), restart-recovery resume (`S/bootstrap.ts:1056`, `unarchive:false`, default prompt "Resume working." when `resumeOnStartup`), schedules/chat mentions (other slices), batch delivery for agent messages (`S/websocket-server.ts:717` uses `sendPreparedPromptToAgent`).

---

## 4. Client message id and optimistic submission lifecycle

### 4.1 Identity
- `messageId` on the wire == `clientMessageId` in timeline items. App generates it (`generateMessageId`), CLI/MCP use uuid. `normalizeClientMessageId` (trim, empty -> undefined) / `resolveClientMessageId(id, gen=uuidv4)` in `S/client-message-id.ts`.
- Server passes it to the provider run as `AgentRunOptions.clientMessageId` (`S/agent/agent-sdk-types.ts:199`), and records a canonical `user_message` timeline row with `clientMessageId` + `messageId = clientMessageId` (`AgentManager.recordSubmittedPrompt` `S/agent/agent-manager.ts:5043`). Provider echoes of the same prompt are reconciled by `reconcileSubmittedPromptEcho` (`:5079`), which can add `providerMessageId` (needed by rewind).
- Wire schema for timeline `user_message` items: `{type, text, messageId?, clientMessageId?}` (`P/messages.ts` `AgentTimelineItemPayloadSchema`).

### 4.2 When the server records the prompt row
`streamAgent` on turn acceptance (`agent-manager.ts:2862`, after publishing `turn_started`), accepted steer (`recordAcceptedSteer :3150`), accepted out-of-band command (`tryRunOutOfBand :2651`), and **rejected** start (`startPendingForegroundTurn :2750`): human `user_message` rows are kept on rejection, delivery notifications (agent messages) are not (`recordSubmittedPrompt` skips non-user rows when `outcome==="rejected"`). Row is idempotent per `clientMessageId` (`getSubmittedUserMessage`).

### 4.3 App-side state machine (`A/composer/submission/model.ts`)
Per agent `messageSubmissions: Map<agentId, MessageSubmissionRecord[]>` where record = `{clientMessageId, providerAcknowledged, rpcSettled}`:
- `beginMessageSubmission`: throws if id already exists; appends both flags false. Store action `beginAgentMessageSubmission` (`A/stores/session-store.ts:1077`) also appends the optimistic row to the stream via `appendSubmittedUserMessage` (head if head non-empty else tail; throws on duplicate `clientMessageId`).
- `acceptMessageSubmission` (RPC returned): if already provider-acknowledged -> remove record; else mark `rpcSettled`.
- `observeMessageSubmissionCanonical` (canonical timeline row with that `clientMessageId` seen, via `setAgentStreamState({acknowledgedClientMessageIds})` `:1001`): if `rpcSettled` remove record else mark `providerAcknowledged`.
- `rejectMessageSubmission` (RPC error): remove record; outcome `"rejected"` (row removed from stream by `removeSubmittedUserMessage`, only if still an unreconciled local row), `"accepted"` (already acknowledged: row stays), `"unknown"`.
- `hasActiveMessageSubmission` (any record not provider-acknowledged) feeds `resolveTurnPresentation(liveness, hasActiveSubmission)`: with no open turn, `isActive` is true while a submission is in flight, so the composer shows Stop and queues further sends (`selectAgentTurnPresentation` `session-store.ts:355`).
- `getSendingClientMessageIds` marks rows as "sending" in the UI.
- Gate: `serverInfo.features.canonicalSubmittedPrompts` (server sets it true `S/websocket-server.ts:2162`). Older daemons: `createUntrackedMessageSubmissionWriter` (`A/composer/submission/writer.ts`) appends/removes the row without tracking.
- `handoffCreatedAgentMessageSubmission` (`writer.ts:62`, store `handoffCreatedAgentUserMessage :1188`): when a draft tab creates an agent, the first prompt row is handed to the new agent's stream (`handoffCreatedAgentUserMessageToStream`, insert tail, matchPolicy `handoff`).
- Legacy matching of local vs canonical rows from daemons without `clientMessageId` echo: `matchesLegacyCanonicalUserMessage` (`A/types/stream.ts:191`, matches by `messageId` or identical text).

---

## 5. Message receipts (server idempotency)

- `S/message-receipts/index.ts`, class `MessageReceipts`, created once in `WebSocketServer` as `new MessageReceipts(join(paseoHome, "agent-requests"))` (`S/websocket-server.ts:740`) and injected into every Session (`:1658`).
- File per request: `<paseoHome>/agent-requests/<sha256 hex>.json` (written with `writeJsonFileAtomic`), key = `sha256(["send", agentId, messageId])` (canonical JSON with sorted object keys). Content `{fingerprint, agentId, state: "pending"|"completed"}` where `fingerprint = sha256(request)` over `{prompt, sourceAgentId, sourceAgentServerId, sessionMode, activeTurnBehavior}`.
- Algorithm `sendOnce`: read receipt; exists with different fingerprint -> throw `agent_request_key_conflict`; exists `completed` -> resolve without sending (idempotent replay); exists `pending` -> throw `agent_request_outcome_unknown` (the provider may have accepted before the receipt committed). Otherwise `prepare()`, write `pending`, `send()`, write `completed`.
- Concurrent calls for the same key are chained in `pending: Map<key, Promise>` so they run serially.
- Comment in code: "Preserve the existing on-disk identity and shape across daemon upgrades" - keep key derivation byte-identical in a port.
- Only used for `send_agent_message_request` with a `messageId`. Creation uses a separate idempotency store (`CreationService`, `idempotencyKey`; other slice).

---

## 6. Active-turn behavior (steer / interrupt)

- Schema: `ActiveTurnBehaviorSchema = z.enum(["interrupt","steer"])` (`P/messages.ts:1254`).
- Defaults: app setting `steer` (section 1.3) and the app always sends an explicit value (`interrupt` for anything not steer); server default for human sends = `"interrupt"` (`S/session.ts:8365`), for agent-sourced = `"steer"`; MCP `send_agent_prompt` default `steer` for agent callers, `interrupt` otherwise (`paseo-tools.ts:1944+`); CLI `--behavior` default `steer` when `PASEO_AGENT_ID` env set else `interrupt` (`CLI/commands/agent/send.ts:204`).
- Engine (`S/agent/agent-prompt.ts`): `sendPreparedPromptToAgent` -> optional unarchive -> `ensureAgentLoaded` -> optional `setAgentMode(sessionMode)` -> `startAgentRun(..., {replaceRunning:true, activeTurnBehavior, clearPendingPermissions, runOptions})`.
  - `startAgentRun` first tries `agentManager.tryRunOutOfBand(prompt)`: a provider handler (`session.tryHandleOutOfBand`, e.g. `/goal pause`) runs **without** interrupting any turn; disposition `"out_of_band"`. Agent-envelope text is unwrapped (`parseAgentMessage`) so providers see the raw slash command.
  - Then `reloadStalePluginSession`; on `isStaleProviderSessionError` it reloads the agent session from persistence and retries once (also retried when the stream iterator throws stale).
  - `steerOrReplaceActiveRun` only when behavior is `steer`: `AgentManager.steerOrReplaceActiveTurn` (`agent-manager.ts:3050`): waits for a still-starting foreground turn (`runs.waitForForegroundStart`), no active turn id -> `{status:"inactive"}` (falls through to a normal new turn); provider `session.steerActiveTurn(prompt, {expectedTurnId, clearPendingPermissions, clientMessageId})` returns `{status:"accepted", providerMessageId?}` -> disposition `"steered"` (row recorded with the active turn id); `"unavailable"` -> `replaceAdmittedForegroundTurn` (cancel + new turn) unless the active turn changed in between (throws `Active turn changed before steering could be delivered`). Providers implementing `steerActiveTurn`: claude, codex, opencode, omp, pi (`S/agent/providers/*`).
  - `interrupt` (or steer fallthrough): `replaceAgentRun` (`:2984`): if running, sets `pendingReplacement = true`, lifecycle `running`, `cancelAgentRunBefore("replace")`, then `streamAgent`. Starting a stream while a run is tracked and not replacing throws `Agent <id> already has an active run`.
  - Dispositions: `PromptDispatchDisposition = "out_of_band" | "steered" | "turn_started"` (`agent-prompt.ts:41`).
- Admission is serialised per agent by `runForegroundMutation` (promise tail), and steer admissions use an event barrier (`runSteerAdmission`) so provider events arriving mid-admission are replayed after it.
- The send handler waits for run start: `AGENT_RUN_START_TIMEOUT_MS = 60_000` (`agent-prompt.ts:~219`; message `<provider> run did not start within 60 seconds (phase: run start)`; must stay above OpenCode's 30 s server boot + 30 s session create).
- `sessionMode` on a send sets the mode first (`agentManager.setAgentMode`).
- Pending permissions: all human/agent sends pass `clearPendingPermissions:true` ("A typed or spoken message from the human answers any permission the agent is blocked on"); providers deny the blocking requests (Codex `clearPendingPermissionsForSteer` denies with message "The user answered with a message instead of approving. Their message follows.").

---

## 7. Queued messages (two different queues)

### 7.1 App-side composer queue ("send after the agent finishes")
- State: `SessionState.queuedMessages: Map<agentId, {id, text, attachments: ComposerAttachment[]}[]>` (`A/stores/session-store.ts:447`, setter `setQueuedMessages :1808`); in-memory only.
- Enqueue: `queueComposerMessage` (`A/composer/actions.ts:~235`) trims text, ignores empty, appends `{id: generateMessageId(), ...}`; composer then clears input/attachments (`queueMessage` `index.tsx:1631`). Triggered by send while running (queue behavior, alternate send, or `handleQueue :1968` from the queue action).
- UI: `renderQueueTrack` / `QueuedMessageRow` (`index.tsx:383,673`) with Edit (`handleEditQueuedMessage :1935` -> `editQueuedComposerMessage` removes the item and puts text + user attachments back into the composer) and Send now (`handleSendQueuedNow :1949` -> `sendQueuedComposerMessageNow`: remove from queue, submit via the normal path (server-side send atomically interrupts/steers), on failure re-insert at the front and show `sendError`).
- Auto-drain: `HostRuntimeStore.drainQueuedAgentMessage(serverId, agentId)` (`A/runtime/host-runtime.ts:2563`) runs when the agent directory replica reports the agent stopped running (`onAgentStoppedRunning`, wired at `host-runtime.ts:1916,2410`; fired from `AgentDirectoryReplica.applyDelta` `A/runtime/directory-sync/agent-replica.ts:110`). It sends only the first queued item at a time (guard set `queuedAgentDrainInFlight`), skips when the agent is `initializingAgents`, and always uses `activeTurnBehavior: "steer"` ("the agent can start a turn of its own in that same moment ... steering folds the message into that turn"). The next item drains on the next stop event.
- Queue does not persist across app restarts.

### 7.2 Daemon-side agent message delivery queue (agent-to-agent)
See section 15 (`createAgentMessageDelivery`): messages from agents are held for a 2 second quiet window, batched, then dispatched in order. Receipt `status:"queued"`.

---

## 8. Interrupt / cancel

- Wire: inbound `cancel_agent_request {agentId, requestId?}` (`P/messages.ts:1908`), outbound `cancel_agent_response {requestId, agentId, agent: AgentSnapshotPayload|null, error?}` (`:4978`). Permission `["workspace.write","hub.execute"]`.
- Handler `handleCancelAgentRequest` `S/session.ts:4838` -> `cancelAgentRunCommand` (`S/agent/lifecycle-command.ts`): agent missing -> `Agent <id> not found`; no in-flight run -> `{cancelled:false}` (no error); `AgentManager.cancelAgentRun` result `not_running | settled | refused`; `refused` throws `AgentRunCancellationError(agentId,"stop")`. Without `requestId` errors only go to `activity_log`.
- `AgentManager.cancelAgentRunNow` (`agent-manager.ts:3351`): serialised on `runForegroundMutation`; finds tracked run or creates an autonomous run if lifecycle is running; `interruptSession` bounded by `INTERRUPT_SESSION_TIMEOUT_MS = 2000` (`:105`, overridable through `rescueTimeouts.interruptSessionMs`); waits for `run.settledPromise` with the same timeout; if interrupt acknowledged but run still active after the timeout it force-dispatches `turn_canceled {reason:"interrupted", turnId}` (or clears a pending foreground run and sets lifecycle `idle` when not replacing); if interrupt not acknowledged returns `settled` only if the run settled anyway, else `refused`. After a settled cancel any pending permissions are resolved as deny with message `"Interrupted"` (`resolvePendingPermissionsForAgent`) and state is emitted.
- Client `DaemonClient.cancelAgent` `C/daemon-client.ts:3980` (throws `payload.error`).
- App: `handleCancelAgent` `A/composer/index.tsx:1894` -> `cancelComposerAgent` (`composer/actions.ts`; null when not running / already cancelling / not connected) -> `getHostRuntimeStore().beginAgentCancellation(serverId, agentId)` (increments `nextCancellationRequestId`, applies turn-liveness transition `cancellation_started{requestId}`) -> on settle `settleAgentCancellation` (`cancellation_settled`). Errors -> toast with the message. Input refocused.
- Also invoked by: Escape shortcut, submit-loading press (`submitLoadingPressHandler`), realtime voice stop (`stopRealtimeVoice` in `composer/input/state.ts`), CLI `paseo agent stop` (`CLI/commands/agent/stop.ts:89`, only for agents with `status === "running"`; `--all`, `--cwd`), CLI delete (`delete.ts:93`), MCP `cancel_agent` (`paseo-tools.ts:2191`, `{success}`).

---

## 9. Prompt attachments

### 9.1 Wire types (`P/messages.ts:1108-1230`)
`AgentAttachmentSchema` = discriminated union on `type`:
| type | fields | rendered text (`S/agent/prompt-attachments.ts renderPromptAttachmentAsText`) |
|---|---|---|
| `forge_change_request` | `mimeType:"application/paseo-forge-change-request"`, `forge` (default `"github"`), `number`, `title`, `url`, `body?`, `projectPath?`, `baseRefName?`, `headRefName?` | `"<Forge> <abbrev> <prefix><n>: <title>\n<url>[\nProject: ...][\nBase: ...][\nHead: ...]\n\n<body>"` using `getForgeDefinitionOrNeutral(forge)` (`displayName`, `changeRequestAbbrev`, `changeRequestNumberPrefix`) from `P/forge-manifest.ts` |
| `forge_issue` | `mimeType:"application/paseo-forge-issue"`, `forge`, `number`, `title`, `url`, `body?`, `projectPath?` | `"<Forge> Issue <prefix><n>: <title>\n<url>..."` |
| `github_pr` (legacy, COMPAT until 2027-01-17) | `mimeType:"application/github-pr"`, number,title,url,body?,baseRefName?,headRefName? | same as change request with forge `github` |
| `github_issue` (legacy) | `mimeType:"application/github-issue"` | same as issue |
| `text` | `mimeType:"text/plain"`, `contextKind?` (only `"chat_history"` survives the transform), `title?`, `text`, `externalResource?{provider,providerLabel,resourceType,id,identifier,title,url}` | the `text` verbatim |
| `review` | `mimeType:"application/paseo-review"`, `cwd`, `mode: "uncommitted"|"base"|"all"`, `baseRef?`, `comments[{filePath, side: "old"|"new", lineNumber, body, context{hunkHeader, targetLine, lines[{oldLineNumber,newLineNumber,type: add|remove|context,content}]}}]` | `Paseo review attachment (<mode>)`, `CWD:`, `Base:`, per comment `Comment n: file:side:line`, body, hunk header and diff lines (`> ` marks the target line; markers `+ - space`) |
| `uploaded_file` | `id`, `fileName`, `mimeType`, `size`, `path` | `Uploaded file: <name>\nPath: <path>\nMIME: <mime>\nSize: <n> bytes` |
Images are separate: `images: [{data: base64, mimeType}]` (`ImageAttachmentSchema` `:1249`), turned into `{type:"image", data, mimeType}` content blocks. `AgentPromptContentBlock = text | image | AgentAttachment` (`S/agent/agent-sdk-types.ts:192`). Providers decide how to feed attachments to the model (text rendering above is used for envelopes, branch-name seeds, and by providers).
`buildAgentBranchNameSeed(firstAgentContext)` (`prompt-attachments.ts:~201`) renders `<user-prompt>` and `<attachments>` blocks for branch-name generation from the first prompt.

### 9.2 App-side composer attachments (`A/attachments/types.ts`, `ComposerAttachment`)
Kinds: `image` (AttachmentMetadata persisted locally), `file` (uploaded_file), `workspace_file` (path + selection `whole_file` | `line_range{startLine,endLine}`), `forge_issue`, `forge_change_request` (+ legacy `github_issue`, `github_pr`), `plugin_resource`, and workspace-scoped (not stored in drafts): `review`, `browser_element` (with optional cropped screenshot sent as an image), `chat_history` (fork context), and pull-request context kinds `forge.change_request_comment|review|check` (+ legacy `github.pull_request_*`).
`splitComposerAttachmentsForSubmit` (`A/composer/attachments/submit.ts`):
- `image` -> `images[]`; `file` -> `uploaded_file`; `workspace_file` -> `type:"text"` titled with the file name and body `Workspace file: <path>[\nLines: a-b]` (`A/attachments/workspace-file.ts workspaceFileAttachmentToAgentAttachment`); `plugin_resource` -> text attachment with `externalResource` (`A/plugins/attachments`); browser element -> text `formatted` (+ screenshot image); PR context -> text; `chat_history` -> its text attachment with `contextKind:"chat_history"`; `review` -> `review` attachment; forge search items -> forge attachments (`buildForgeAttachmentFromSearchItem`) or, when the daemon predates forge search (`serverInfo.features.forgeSearch !== true`), legacy `github_*` (`resolveComposerAttachmentSubmitFormat`).
Adding: image picker (`useImageAttachmentPicker`), file picker (`useFilePicker`), clipboard paste, drag-drop (`composer/attachments/drop.ts`: images / generic files / workspace file drag payload), `@`-autocomplete of files, forge picker (GitHub/forge issue/PR search in composer, `GithubPickerOption`), forge auto-attach (`A/composer/forge-auto-attach.ts`: detects a pasted forge change-request URL in the text and auto-attaches; removing an auto-attached item is remembered via `markForgeAttachmentRemoved`), workspace "add file to chat" (`A/panels/use-add-file-to-chat.ts`), review/diff comments, browser element picker.
Limits: file upload max `FILE_UPLOAD_MAX_BYTES = 50 MiB` (`A/attachments/upload-limit.ts`, error `composer.errors.fileTooLarge`); upload uses `client.uploadFile` (`C/daemon-client.ts:5352`, request `file.upload.request {fileName, mimeType, size, modifiedAt, requestId}` + binary frame; other slice) returning an `uploaded_file` with a daemon path. Pending uploads show placeholder pills (`pendingFiles`). Image data is read from the local attachment store and base64-encoded at send time (`encodeImages`).
Attachment pills open: image -> lightbox (`AttachmentLightbox`), file/workspace_file -> nothing, workspace attachments -> open Changes tab (`openWorkspaceChanges`), forge items -> external URL (`openComposerAttachment` `actions.ts`).
CLI: `paseo agent send --image <path>` (repeatable) reads files, mime by extension (png/jpg/jpeg/gif/webp, default jpeg) and base64 (`CLI/commands/agent/send.ts:50-90`). MCP prompt is text only.

---

## 10. agent-prompt module (`S/agent/agent-prompt.ts`)

Single orchestration used by every surface "so behavior can never drift":
- `sendPromptToAgent(params)`: resolve prompt source title (`resolvePromptSource`: `source.title` or live/stored title of source agent) -> `prepareAgentMessage(prompt, source, messageId)` (envelope, section 15) -> `sendPreparedPromptToAgent` with `runOptions.clientMessageId = delivery.messageId`.
- `sendPreparedPromptToAgent`: if record archived: `unarchive` default true -> `unarchiveAgentState` (remembers `archivedAtToRestore`, re-archives if the load fails); `unarchive:false` + archived -> silent no-op returning `turn_started`. Then `ensureAgentLoaded`, `setAgentMode(sessionMode)`, `startAgentRun`.
- `startCreatedAgentInitialPrompt` (`:369`): the first prompt after creation: no-op when prompt is null; `startAgentRun` without replace; waits for run start (60 s).
- `waitForAgentRunStartWithTimeout(agentManager, agentId, signal?)`: wraps `AgentManager.waitForAgentRunStart` (`agent-manager.ts:3195`, errors: `has no pending run`, `run finished before starting`, pending run failure text, `lastError`).
- `unarchiveAgentState(storage, manager, agentId, updates?)` shared by Session/MCP/CLI.
- `AgentRunController` (picked methods of AgentManager) is the interface for tests.
- `respondToAgentPermission` (`S/agent/permission-response.ts`) lives next to it (section 18).

---

## 11. agent-response-loop (`S/agent/agent-response-loop.ts`) - structured responses, not a chat loop

Not user-facing. Helper for daemon-internal generation (titles, branch names, commit/PR text; used by `S/session/checkout/git-metadata-generator.ts` and `agent-structured-text`):
- `getStructuredAgentResponse({caller, prompt, schema, maxRetries = 2, schemaName = "Response"})`: appends "You must respond with JSON only that matches this JSON Schema:" + schema; extracts JSON from a ```json fence or the first balanced `{}`/`[]` snippet; validates with zod (`z.toJSONSchema` draft-07) or Ajv (`allErrors, strict:false`); on invalid JSON/schema re-asks with a bullet list of errors up to `maxRetries`; throws `StructuredAgentResponseError{lastResponse, validationErrors}`.
- `generateStructuredAgentResponse`: creates a throwaway agent (`manager.createAgent(config, agentId, {persistSession, workspaceId: undefined})`), `manager.runAgent` per attempt (takes `finalText` or last assistant message), always `closeAgent` + `deleteAgentState` in `finally`.
- `generateStructuredAgentResponseWithFallback`: iterates `providers[]` (`DEFAULT_STRUCTURED_GENERATION_PROVIDERS` from `structured-generation-providers.ts`), skips unavailable ones (`manager.getProviderAvailability`), collects `attempts`, throws `StructuredAgentFallbackError(attempts)` with a summary.
- Also builds `buildStructuredAgentResponsePrompt` for callers that run the prompt themselves.

## Cross-area connections

- Composer editing and local drafts feed workspace/agent creation; creation handoff transfers optimistic message identity to the new agent.
- Client message ids and server receipts connect optimistic timeline reconciliation, reconnect retries, idempotency and authoritative prompt dispatch.
- Steer/interrupt capability, cancellation and two distinct message queues depend on provider runtime and agent run lifecycle.
- Attachments join image storage, uploaded files, workspace selections, fork history, forge/review context, browser elements and plugin resources through the common prompt builder.
- Prompt-source envelopes preserve human/voice/MCP/inter-agent sender identity; new human prompts can clear pending permissions. Missing approval RPC/card details are in ../gaps.md.
- Structured-response helpers use temporary internal agents and provider fallback for workspace naming and git commit/PR metadata; they are separate from the user chat loop.

## Inventory

- Composer: send path and state machine (app)
- What the user sees
- Submit pipeline (end to end)
- Send settings and shortcuts
- Dictation transcript into composer
- States / limits
- Composer drafts and draft store (app)
- Send prompt: `send_agent_message_request` (server)
- Wire
- Handler `handleSendAgentMessageRequest` (`S/session.ts:8304`)
- Client
- Other entry points that reuse the same pipeline (`sendPromptToAgent`)
- Client message id and optimistic submission lifecycle
- Identity
- When the server records the prompt row
- App-side state machine (`A/composer/submission/model.ts`)
- Message receipts (server idempotency)
- Active-turn behavior (steer / interrupt)
- Queued messages (two different queues)
- App-side composer queue ("send after the agent finishes")
- Daemon-side agent message delivery queue (agent-to-agent)
- Interrupt / cancel
- Prompt attachments
- Wire types (`P/messages.ts:1108-1230`)
- App-side composer attachments (`A/attachments/types.ts`, `ComposerAttachment`)
- agent-prompt module (`S/agent/agent-prompt.ts`)
- agent-response-loop (`S/agent/agent-response-loop.ts`) - structured responses, not a chat loop

- Composer optimistic preparing/sending/accepted/rejected message transitions
- Creation handoff preserves client message identity
- Default send shortcut and active-run send behavior preferences
- Workspace-scoped draft attachments and local image re-encoding
- Server accepted/rejected submission rows and provider acknowledgement
- Per-recipient inter-agent quiet-window delivery
- Prompt source envelopes and sender-title resolution
- Uploaded files, workspace ranges and browser selection screenshot prompts
- Forge auto-attachment and legacy forge wire fallback
- Review and fork-history text attachments
- Structured JSON extraction, schema correction retries and provider fallback
- Internal generation-agent cleanup after success/failure
