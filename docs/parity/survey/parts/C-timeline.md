# C — Agent stream / timeline and its rendering (Paseo 0.15.4)

Source root: `~/refs/paseo-latest/packages`. Abbreviations: `P` = protocol/src, `S` = server/src/server, `A` = app/src, `C` = client/src.
Everything below was read from code; "not found" means I grepped the named place.

---------------------------------------------------------------------------------------------------
## 1. Timeline item model (the wire/canonical vocabulary)

### 1.1 AgentTimelineItem (canonical item, `P/agent-types.ts:~395`; zod mirror `AgentTimelineItemPayloadSchema` `P/messages.ts:717`)
Discriminated by `type` (note: zod uses `z.union`, not discriminatedUnion, because of a zod-aot bug, `messages.ts:715`):
| type | fields | notes |
|---|---|---|
| `user_message` | `text`, `messageId?`, `clientMessageId?` | `clientMessageId` = id minted by the app composer; `messageId` = provider id (server sets `messageId=clientMessageId` for the optimistic row, provider id goes to row.providerMessageId) |
| `assistant_message` | `text`, `messageId?` | chunks; merged by projection |
| `reasoning` | `text` | chunks; merged by projection |
| `tool_call` | see 1.2 | identity = `callId` |
| `todo` | `items: AgentTaskItem[]` = `{text, completed, id?, status?: "pending"|"in_progress"|"completed", activeForm?}` | |
| `error` | `message` | |
| `notification` | `level: "info"|"warning"|"error"`, `message` | needs client cap `timeline_notifications` (older clients get it filtered, `S/session.ts:1366`) |
| `compaction` | `status: "loading"|"completed"`, `trigger?: "auto"|"manual"`, `preTokens?` | |
| `plugin` | `id`, `pluginId`, `kind`, `version`, `data: JsonValue` | identity = `pluginId/id`; needs cap `plugin_timeline_items`; data ≤ `PLUGIN_TIMELINE_DATA_MAX_BYTES = 64*1024` (`S/agent/agent-timeline-content.ts:4`) |
Server and protocol define the same shapes (`S/agent/agent-sdk-types.ts` re-declares them; `ToolCallDetail` there at :244).

### 1.2 ToolCallTimelineItem
Common (`ToolCallBase`): `type:"tool_call"`, `callId`, `name`, `detail: ToolCallDetail`, `metadata?: Record<string,unknown>`, `agentMessage?: AgentMessage` (+ index signature in the TS type).
Status variants (zod discriminatedUnion on `status`, `messages.ts:707`): `running` (`error:null`), `completed` (`error:null`), `failed` (`error: non-null unknown`), `canceled` (`error:null`).
`AgentMessage` (`P/agent-message.ts`): `{event: "message"|"finished"|"errored"|"permission-required"|"closed", sender:{id, serverId?, title?}, text}`.
Icon names for `plain_text.icon` (`TOOL_CALL_ICON_NAMES`): `wrench, square_terminal, eye, pencil, search, bot, sparkles, brain, mic_vocal`.

### 1.3 ToolCallDetail — complete list (`P/agent-types.ts:215`, zod `ToolCallDetailPayloadSchema` `P/messages.ts:577`) — 11 kinds
| `type` | fields |
|---|---|
| `shell` | `command`, `cwd?`, `output?`, `exitCode?: number\|null` |
| `read` | `filePath`, `content?`, `offset?`, `limit?` |
| `edit` | `filePath`, `oldString?`, `newString?`, `unifiedDiff?` |
| `write` | `filePath`, `content?` |
| `search` | `query`, `toolName?: "search"\|"grep"\|"glob"\|"web_search"`, `content?`, `filePaths?: string[]`, `webResults?: {title,url}[]`, `annotations?: string[]`, `numFiles?`, `numMatches?`, `durationMs?`, `durationSeconds?`, `truncated?`, `mode?: "content"\|"files_with_matches"\|"count"` |
| `fetch` | `url`, `prompt?`, `result?`, `code?`, `codeText?`, `bytes?`, `durationMs?` |
| `worktree_setup` | `worktreePath`, `branchName`, `log`, `commands: {index>0, command, cwd, log(default ""), status:"running"\|"completed"\|"failed", exitCode:number\|null, durationMs?}[]`, `truncated?`. Producer `S/worktree-bootstrap.ts:236 buildWorktreeSetupDetail` (always sends `log:""`, COMPAT workspaceSetupCombinedLog) |
| `sub_agent` | `subAgentType?`, `description?`, `childSessionId?`, `log?` (COMPAT: sent empty), `actions?: {index>0,toolName,summary?}[]` (COMPAT: always `[]`, nothing reads it). A "launch marker": child activity lives in the sub-agent panel (see other slice, `A/subagents`) |
| `plain_text` | `label?`, `text?`, `icon?` |
| `plan` | `text` |
| `unknown` | `input`, `output` (any JSON; used by every provider tool without a canonical mapping, MCP tools, Paseo tools, `speak`, `thinking`) |
Producers (server, grep `type: "<kind>"`): worktree_setup `S/worktree-bootstrap.ts`; sub_agent in `S/agent/providers/{codex/tool-call-mapper,claude/sidechain-tracker,claude/agent,omp/tool-call-mapper,opencode/tool-call-detail-parser,pi/extensions/*-pi-subagents}`; plan in `claude/tool-call-mapper.ts:130`, `codex-app-server-agent.ts:1161/3933/4894`; plain_text in `agent-messages/envelope.ts:188,213`, `omp/advisor-message.ts`, `codex/async-questions.ts`, `acp-agent.ts`.

### 1.4 AgentStreamEvent (server-internal, `P/agent-types.ts` AgentStreamEvent) vs WIRE `AgentStreamEventPayloadSchema` (`P/messages.ts:771`)
Server-internal events: `thread_started{sessionId,provider}`, `turn_started{provider,turnId?}`, `turn_completed{provider,usage?,turnId?}`, `usage_updated{provider,usage,turnId?}`, `mode_changed{currentModeId,availableModes}`, `model_changed{runtimeInfo}`, `thinking_option_changed{thinkingOptionId}`, `turn_failed{error,code?,diagnostic?,turnId?}`, `turn_canceled{reason,turnId?}`, `timeline{item,turnId?,timestamp?}`, `permission_requested{request,turnId?}`, `permission_resolved{requestId,resolution,turnId?}`, `attention_required{reason:"finished"|"error"|"permission",timestamp}`.
The WIRE union only has: `thread_started`, `turn_started`, `turn_completed{usage?: AgentUsageSchema}`, `turn_failed`, `turn_canceled`, `timeline{provider,item,turnId?}`, `permission_requested`, `permission_resolved`, `attention_required{+shouldNotify, notification?{title,body,data{serverId,workspaceId?,agentId,reason}}}`.
`usage_updated`, `mode_changed`, `model_changed`, `thinking_option_changed` are NOT in the wire union: `serializeAgentStreamEvent` (`S/messages.ts:24`) validates with the zod schema and returns `null` for them → `session.ts:2064` drops them. Their effect reaches clients through `agent_status`/`agent_update` snapshots (`lastUsage`, `runtimeInfo`, `currentModeId`) instead (`agent-manager.ts:4668-4699` mutates the managed agent and calls `emitState`). Attention is routed separately: `session.ts:1366-1380` also emits `agent_attention_required`.
`AgentUsage` (`agent-types.ts`): `inputTokens?, cachedInputTokens?, outputTokens?, totalCostUsd?, contextWindowMaxTokens?, contextWindowUsedTokens?` (sanitized to finite numbers by `sanitizeUsage`, `S/agent/agent-projections.ts:532`).

### 1.5 Wire envelopes carrying items
- `agent_stream` (`AgentStreamMessageSchema`, `messages.ts:~4250`): `payload{subscriptionId?, agentId, event: AgentStreamEventPayload, timestamp, seq?: int>=0, epoch?: string}`. `seq`/`epoch` present only for timeline events ("maps 1:1 to canonical in-memory timeline rows").
- `fetch_agent_timeline_response` entries (`AgentTimelineEntryPayloadSchema` `messages.ts:4776`): `{provider, item, turnId?, timestamp, seqStart, seqEnd, sourceSeqRanges:{startSeq,endSeq}[], collapsed: ("assistant_merge"|"reasoning_merge"|"tool_lifecycle"|"identity")[]}`.
- `agent.provider_subagents.update` / `...timeline.get.response` carry the same item schema for provider sub-agents (rows have `seq`, optional `seqStart/seqEnd/sourceSeqRanges`; `projection:"projected"` only). Details belong to the sub-agent slice.

---------------------------------------------------------------------------------------------------
## 2. Server timeline store (sequence numbers, epochs, rows)

**What it is**: `InMemoryAgentTimelineStore` (`S/agent/agent-timeline-store.ts`), one per daemon, keyed by agentId. Row = `AgentTimelineRow {seq, timestamp, item, readonly turnId?, readonly providerMessageId?}` (`agent-timeline-store-types.ts:4`).
### 2.1 State, epoch and seq
- Per agent: `{epoch: randomUUID(), projection: TimelineProjection, minSeq, nextSeq}`. `nextSeq` starts at 1; `append()` assigns `seq = nextSeq++` (`:119`). `minSeq` = first row's `seqStart`, 0 if empty (`hasRows = minSeq !== 0`).
- Epoch is minted on `initialize()` (unless seeded) and changes whenever the in-memory timeline is rebuilt: `timelineStore.delete(agentId)` then re-`register` (reload-from-disk `agent-manager.ts:1716`; rewind re-hydration `:3499-3510`). A new epoch invalidates all client cursors (`staleCursor`).
- `window = {minSeq, maxSeq: nextSeq-1, nextSeq}`.
- `DEFAULT_TIMELINE_FETCH_LIMIT = 200` (items) when `limit` undefined; `limit: 0` = everything (`:25`, `:94`).
- Durable store interface `AgentTimelineStore` (`appendCommitted, fetchCommitted, getLatestCommittedSeq, getCommittedRows, getLastItem, getLastAssistantMessage, deleteAgent, bulkInsert, updateCommittedRow`) exists and is called (fire-and-forget `enqueueDurableTimelineAppend/BulkInsert/Update`, `agent-manager.ts:5261-5300`), but **no implementation is wired in 0.15.4** (grep `durableTimelineStore|implements AgentTimelineStore` in server/src: only agent-manager and the type file). So timelines live in memory; after daemon restart history is re-read from the provider (`hydrateTimelineFromProvider` / `primeTimelineFromLegacyProviderHistory`, `agent-manager.ts:3466,4345`) and gets a new epoch.
### 2.2 Fetch semantics (`fetch()` `agent-timeline-store.ts:73`)
Input `AgentTimelineFetchOptions {wholeTurns?, direction?: "tail"|"before"|"after" (default tail), cursor?{epoch,seq}, limit?}`.
- `staleCursor = cursor && cursor.epoch !== state.epoch`.
- `gap = !staleCursor && direction==="after" && cursor && hasRows && cursor.seq < minSeq-1`.
- `reset = staleCursor || gap`; when reset the page is served as `tail`.
- Result `{epoch, direction (as requested), reset, staleCursor, gap, window, hasOlder, hasNewer, startSeq, endSeq, rows}`; each row has `seq = seqEnd`, item passed through `limitAgentTimelineItemContent` again.
### 2.3 Content limiting (`agent-timeline-content.ts`)
Applied at record time (`recordTimeline`, `agent-manager.ts:5164`), at dispatch (`dispatchStream :5381`) and on fetch. Constants: `TOOL_CALL_BUDGET = 64*1024`, `ITEM_BUDGET = 256*1024` (UTF-16 units + `NODE_COST=8` per node + key lengths), `MARKER_COST=32`, `SHORT_STRING = 1024` (ids/names/paths stay whole), `MAX_DEPTH = 64`. Over-budget: strings cut to a binary-searched cap (>= SHORT_STRING, ≈15/16 of budget minus structure), marker `…[truncated <n> B|KB|MB]`, `data:` URLs dropped whole, arrays drop trailing elements that do not fit. One fetch = one WS message, hence the bound.
### 2.4 Turn ids on rows
`withTurnId` (`agent-manager.ts:~520-550`): live events without `turnId` get `activeForegroundTurnId ?? activeTurnId ?? undefined`; `turn_started` without id gets `autonomous-<uuid>` when no foreground turn. History events (`fromHistory`) keep absent ids deliberately ("provider history has no daemon turn identity").
### 2.5 Related
- `getSubmittedUserMessage(agentId, clientMessageId)` / `enrichSubmittedUserMessage(...)` (sets `providerMessageId` on the optimistic user row; used by rewind, which refuses with "Cannot rewind before the provider acknowledges the submitted prompt", `agent-manager.ts:3478`).
- `getLastAssistantMessage`, `getLastItem` used by wait-for-finish / MCP `get_agent_status`.

---------------------------------------------------------------------------------------------------
## 3. Server projection (canonical rows → projected entries)

File `S/agent/timeline-projection.ts`. "Projected" is the only mode the app/wire serves (`session.ts:7868` hard-codes `projection = "projected"`; the request field `projection: "projected"|"canonical"` in `FetchAgentTimelineRequestMessageSchema` is accepted and ignored there). `canonical` mode is used by internals (`projectTimelineRows({mode})`).
### 3.1 Merge rules (all keyed to adjacent seq and same `turnId`)
- **Assistant merge** (`mergeAssistantChunks`): adjacent `assistant_message` rows with `prev.seqEnd+1 === next.seqStart`, same `turnId`, and (`next.messageId` undefined or equal to prev's) → one entry, text concatenated, `collapsed += "assistant_merge"`.
- **Reasoning merge** (`mergeReasoningChunks`): adjacent `reasoning` with contiguous seq, same turnId → `collapsed += "reasoning_merge"` (wire strips `reasoning_merge` for clients without cap `reasoning_merge_enum`, `session.ts:7929`).
- **Tool lifecycle collapse** (`mergeIdentityEntries`, `tool_call` branch): same `callId` and same `turnId` → one entry; `mergeToolCallItems` = `{...existing,...incoming}`, detail via `mergeToolCallDetail` (unknown never overwrites a known kind; incoming wins otherwise), metadata shallow-merged, `error` = incoming.error if failed; null if completed/canceled; kept if incoming.error defined while running. `seqStart` stays at first, `seqEnd` = max, `timestamp` = latest, `collapsed += "tool_lifecycle"`.
- **Plugin identity** (`plugin` branch): later row with same `pluginId/id` replaces earlier (keeps first `seqStart`), `collapsed += "identity"`.
- Identity function: `timelineItemIdentity` (`P/timeline-identity.ts`): tool_call → `callId`, plugin → `${pluginId}/${id}`, else null.
- Every entry keeps `sourceSeqRanges` (merged contiguous ranges of source seqs) and `collapsed` kinds. A row's `seq` = `seqEnd` ("latest source event, not display anchor").
### 3.2 Incremental class `TimelineProjection`
Maintains `rows`, `identities: Map<identity,rowIndex>`, `lastAssistantRowIndex`, `spans` (disjoint source-coverage spans in display order; a tail starting inside a span is pulled back to the span's first anchor so interleaved lifecycle updates are covered) and `sourceRanges` (sorted by startSeq, each → rowIndex). `append(row)` = identity merge if known else merge-with-previous (assistant, reasoning) else push.
### 3.3 Paging (`selectPage`)
- **tail**: last `limit` projected rows (limit 0 = all), start moved back to span start; with `wholeTurns` start moved back until `!continuesProjectedTurn` (and iterated again through span coverage). `hasOlder = start>0`, `hasNewer=false`, `endSeq = bounds.maxSeq`.
- **before** cursor C: rows with `seqStart <= C-1`; `limit` rows back; `wholeTurns` extends start back AND extends the right edge forward over the following same-turn rows (`selectedEnd = nextRow.seqStart-1`) so a prompt-jump target never gets lost; if `endSeq < minSeq` → empty page, `hasNewer:true`.
- **after** cursor C: binary-search `sourceRanges` for ranges with `endSeq >= C+1`, collect distinct rows (≤ limit), order by first source seq ≥ start (`selectProjectedEntriesAfter`); returned `endSeq` advances only through contiguous source coverage (so a wide tool entry with a future discontiguous range cannot skip seqs). Never returns lifecycle deltas, only the projected entry; clients map back through `sourceSeqRanges`. `wholeTurns` is NOT applied to `after` ("after catch-up stays incremental", `messages.ts:1943`).
- Turn continuity: `continuesTimelineTurn(prevTurnId, nextTurnId, nextIsTrigger)` (`P/timeline-turn.ts`): both ids present → equality; else a visible trigger starts a turn (`!nextIsTrigger`). `isTimelineTurnTrigger(item)` = `user_message`, or `tool_call` with `agentMessage !== undefined` or `name === "heartbeat"`.
### 3.4 Other helpers
`selectTimelineWindowByProjectedLimit` / `selectItemsByProjectedLimit` (flat item arrays for MCP/activity curator; expand window until overlapping entries stable), `projectTimelineRows`.

---------------------------------------------------------------------------------------------------
## 4. Stream coalescer (60 ms live batching)

`S/agent/agent-stream-coalescer.ts`. `AGENT_STREAM_COALESCE_DEFAULT_WINDOW_MS = 60` (option `agentStreamCoalesceWindowMs`).
- Only `timeline` events with `assistant_message`, `reasoning`, `tool_call` are coalescible; empty-text text items are swallowed (`handle()` returns true).
- Buffer per agent: text entries appended; tool-call entries merged by `callId` via `mergeToolCallItems` (so a less-informative update cannot erase detail).
- Flush triggers: terminal tool call (completed/failed/canceled) flushes immediately; **leading edge**: first event after ≥ window idle flushes synchronously; otherwise a trailing `setTimeout(windowMs)` (timer unref'd); `flushFor(agent)` called before any non-coalescible event (`agent-manager.ts:4551-4555`), turn boundaries (`:3118`), `flushAndDiscard` on close/reload (`:4056,4386`), `flushAll` on shutdown (`:5338`).
- `collapseEntries`: consecutive text entries concatenated only if same item type, same `messageId` (assistant), same provider and same turnId.
- onFlush → `recordAndDispatchTimelineItem` (store append + `dispatchStream` with `{seq, epoch, timestamp}`), then foreground-turn waiters notified.
Comment in app `text-reveal.ts`: the daemon sends "one message per 60 ms window", which is why the app paces rendering (§13).

---------------------------------------------------------------------------------------------------
## 5. Server stream dispatch and subscription delivery

### 5.1 AgentManager → session (`S/agent/agent-manager.ts`)
- `handleStreamEvent` switch `dispatchStreamEventByType` (`:4660`): `thread_started` (refresh persistence + runtime info), `usage_updated` (sets `lastUsage`, emitState, not forwarded), `mode_changed`/`model_changed`/`thinking_option_changed` (`flags.shouldDispatchEvent=false`, state only), `timeline` (`onStreamTimelineEvent :4770`), `turn_completed` (merges `usage` into `lastUsage`), `turn_failed`, `turn_canceled`, `turn_started`, `permission_*`.
- `onStreamTimelineEvent`: runs `projectAgentMessages(item)` (§6) over each item; if `user_message` with `clientMessageId` → `reconcileSubmittedPromptEcho` (dedups against the optimistic row, enriches `providerMessageId`); `fromHistory` items are only recorded (no broadcast); live ones `recordAndDispatchTimelineItem` and, for user messages, bump `lastUserMessageAt` and `emitState`.
- **Errors in the timeline**: `turn_failed` appends an `assistant_message` whose text is `"[System Error] " + error[+ "\n\ncode: X"][+ "\n\n" + diagnostic]` (`SYSTEM_ERROR_PREFIX` `:567`, `formatTurnFailedMessage :4989`, `appendSystemErrorTimelineMessage :5143`), de-duplicated against the last item; skipped for history. Pending permissions are auto-resolved (deny "Turn failed" / "Interrupted"). `error` items proper come from providers (`opencode-agent.ts:3184`, `pi/agent.ts:2307`); `notification` items from `plugin-provider.ts:1638`, `omp/event-mapper.ts:76`, `omp/agent.ts:1578`, `pi/agent.ts:2082`, `opencode/runtime-notice.ts:26`, `omp/system-notice.ts`.
- `recordAndDispatchTimelineItem` (`:5008`) also: after a completed `shell` tool whose command `commandMayHaveChangedExternalState` → `onWorkspaceStateMayHaveChanged({cwd})` (git status refresh).
- `appendTimelineItem(agentId, item)` (`:2684`, plugin RPC) and `emitLiveTimelineItem` (`:2709`, not stored).
- `timeline_replacement{agentId, epoch}` manager event (`:241`) emitted after rewind re-hydration.
- `dispatch()` filters by subscriber `agentId`, internal agents hidden from global subscribers unless `includeInternal`.
### 5.2 Session (`S/session.ts`)
- `subscribeToAgentEvents :1989`, `forwardAgentStream :1383`: builds `agent_stream` payload (`buildAgentStreamPayload :2131`, timestamp default now, `seq`/`epoch` if numbers) and delivers to each **timeline subscription** (`timelineSubscriptions: Map<id,{owner, agentIds:Set}>`) whose `agentIds` contains the agent; filters: attention events skipped for modern/selective clients (they use `agent_attention_required`), `supportsTimelineItem` (notification/plugin caps). COMPAT paths: non-modern clients without `selective_agent_timeline` receive every agent's stream implicitly; with no clientSources at all the session emits globally.
- Request `agent.timeline.set_subscription.request {agentIds: string[], requestId}` (`messages.ts:1987`) → `session.ts:2846`: dedupe+sort, new owned subscription (replaces nothing; each call is a new owner released by the client), responds `agent.timeline.set_subscription.response {subscriptionId?, agentIds, requestId}`. The client `observeTimeline(agentIds)` (`C/daemon-client.ts:3827`) wraps this in an `OwnedSubscription` (re-established after reconnect).
- `deliverTimelineReplacement :4932`: for each subscription with the agent: `agent.timeline.replacement {subscriptionId?, agentId, epoch}` to modern clients, or to non-initiating legacy clients with cap `timeline_replacement_invalidation`; older legacy clients get the whole reconstructed timeline replayed as `agent_stream` events (`emitReconstructedTimelineRows :4965`). The rewind initiator is excluded for legacy clients (it refetches itself).
### 5.3 Fetch handler `handleFetchAgentTimelineRequest` (`session.ts:7862`)
Request `fetch_agent_timeline_request` (`messages.ts:1941`): `{agentId, requestId, direction?: "tail"|"before"|"after", cursor?: {epoch,seq}, limit?: int>=0 (0=all), projection?, wholeTurns?, mergeWindow?}`.
- Default direction = `cursor ? "after" : "tail"`. `ensureAgentLoaded` (loads archived/stored agent into manager, which hydrates provider history), builds agent snapshot (`agent` field), `agentManager.fetchTimeline`, filters entries by client caps, returns `fetch_agent_timeline_response.payload {requestId, agentId, agent: AgentSnapshotPayload|null, direction, projection, epoch, reset, staleCursor, gap, window{minSeq,maxSeq,nextSeq}, startCursor|null, endCursor|null (each {epoch,seq}), hasOlder, hasNewer, mergeWindow?, entries[], error}`.
- On exception: same message with `agent:null, epoch:"", window{0,0,0}, entries:[], error: message`.
- Client method `DaemonClient.fetchAgentTimeline(agentId, opts)` (`C/daemon-client.ts:3664`): `wholeTurns` is only sent when the server advertises `features.wholeTurnTimeline === true` (COMPAT; server sets it at `websocket-server.ts:2129`), `mergeWindow` only when true; `skipQueue:true`; throws `Error(payload.error)`.
### 5.4 Search, prompt index, append, fork context (timeline-adjacent)
- `agent.timeline.search.request {agentId, requestId, query, cursor?: seq}` → `S/agent/chat-search/index.ts`: iterate projected rows with `seqEnd > cursor`, only `user_message`/`assistant_message`; assistant text parsed with markdown-it (`html:false`) into searchable blocks (text, inline code, fences); query split on whitespace, regex-escaped, joined with `\s+`, flags `giu`; per-row `count` = matches summed over blocks; `PAGE_SIZE = 200` locations per page, yields to event loop every 8 ms. Response `agent.timeline.search.response {requestId, agentId, epoch, locations:{seq, role:"user"|"assistant", count?}[], nextCursor: seq|null, error}`; if the epoch changed during the scan the response errors "History changed; search again" (`session.ts:8021`).
- `agent.timeline.list_prompts.request {agentId, requestId}` → `S/agent/timeline-prompt-index.ts`: all `user_message` rows → `{seq, timestamp, preview}`; preview = whitespace-collapsed, `PROMPT_PREVIEW_MAX_LENGTH = 120` (119 chars + "…"). Response `{requestId, agentId, epoch, prompts[], error}`. Gated by server feature `agentTimelinePromptIndex`.
- `agent.timeline.append.request {requestId, agentId, item:{type:"plugin", id, kind, version>0, data}}` — only plugin sessions (`parsePluginClientId(clientId)`); response `agent.timeline.append.response {requestId, seq, epoch}` (`session.ts:7986`; client `appendAgentTimelineItem`).
- `agent.fork_context.request {agentId, boundaryCursor?, boundaryMessageId?, requestId}` → `buildAgentForkContextAttachment` (`activity-curator.ts`, see §6.3).

---------------------------------------------------------------------------------------------------
## 6. Message envelopes, tool-name normalization, curator

### 6.1 Agent-to-agent messages as timeline rows (`S/agent/agent-messages/envelope.ts`)
- Wire text format: `<paseo-system version="1" kind="user-message|agent-message|agent-notification" source-agent-id=... message-id=... [source-server-id] [source-agent-title] [event]>\nXML-escaped body\n</paseo-system>`; `formatAgentMessage`, `parseAgentMessage(s)` (strict regex; a sequence of envelopes separated by newlines is accepted, any prose around it → plain user text). Schedule/heartbeat bodies use the unversioned `<paseo-system>\n…\n</paseo-system>` (`formatSystemNotificationPrompt`, `isSystemInjectedEnvelope`).
- `prepareAgentMessage(prompt, source?, id?)`: when a HUMAN send contains a reserved envelope it is re-wrapped as `kind="user-message"` so it is quoted, never trusted as system provenance.
- `projectAgentMessages(item)` (applied to provider echoes, acceptance, import, replay): `user_message` containing envelopes → per-envelope rows: sourceless → `user_message{text, clientMessageId: message-id}`; from another agent → `tool_call{callId:"paseo-agent-message:<id>", name:"agent_message", status:"completed", agentMessage{event: kind==="agent-message" ? "message" : source.event, sender, text}, detail: plain_text{text, icon:"bot"}}` (COMPAT: `name:"agent_message"` kept for old clients); schedule envelope → `tool_call{callId:"paseo-system:<sha256>", name:"heartbeat", detail: plain_text{text, icon:"sparkles"}}`.
- `delivery.ts` `createAgentMessageDelivery` (notification delivery to waiting parents) — belongs to the agent-messaging slice.
### 6.2 Tool-name normalization (`P/tool-name-normalization.ts`; there is no file in server/agent, only a test)
`normalizeToolName` (trim+lowercase), `tokenizeToolName` (`[a-z0-9]+`), `getToolLeafName`, `isSpeakToolName` (leaf == "speak"), `isLikelyNamespacedToolName` (contains `. : /`, or `__` with ≥3 segments or 2 segments where the 2nd has `_`), `isPaseoToolName` (`mcp__paseo[_x]__<leaf>` with ≥3 segments, or `paseo[_x].<leaf>`; "speak" excluded), `getPaseoToolLeafName`, `isLikelyExternalToolName` (speak or namespaced).
### 6.3 Activity curator / fork context (`S/agent/activity-curator.ts`)
Text rendering of a projected timeline for MCP `get_agent_activity` and fork: `[User]`, `[Assistant]` (only when `labelAssistantMessages`), `[Thought]`, `[<Display name>] <summary>` for tool calls (via `buildToolCallDisplayModel`; for external/namespaced tools raw input JSON ≤ `MAX_TOOL_INPUT_CHARS=400`), `[Tasks]` + `- [x]/[ ] text`, `[Error]`, `[Compacted]`; summaries `MAX_TOOL_SUMMARY_CHARS=200`; agent messages as `[<event label>] <sender>` + body. `summarizeAgentActivity` header "Showing N of M activities (limited to L)". Fork: `buildAgentForkContextAttachment` → text attachment `{type:"text", mimeType:"text/plain", contextKind:"chat_history", title:"Chat history"}`, only user/assistant/tool_call kinds, `MAX_FORK_CONTEXT_CHARS = 100_000` (keeps the newest entries, prefix `"[Earlier chat history omitted]\n"`, single oversize entry middle-truncated with `\n[Message truncated]\n`), metadata ≤ `MAX_FORK_METADATA_CHARS=1000`, wrapped in `<chat-history-summary>`. Boundary selection: by cursor (`row.seqEnd === cursor.seq`; epoch mismatch → "Selected timeline position is no longer available.") or by last assistant row with `messageId`; refuses when the boundary lies inside a merged entry ("This checkpoint changed after it was created. Fork from a later completed response instead.").

---------------------------------------------------------------------------------------------------
## 7. Tool-call display model (shared protocol → app and server)

`P/tool-call-display.ts` `buildToolCallDisplayModel({name,status,error,metadata,detail,agentMessage,cwd?,targetAgentTitle?}) → {displayName, summary?, errorText?}`.
Order: (1) `agentMessage` → `buildAgentMessageDisplay` (labels: message "Message received", finished "Agent finished", errored "Agent errored", permission-required "Agent needs permission", closed "Agent closed"; summary = sender title or id; detail plain_text `"From agent: <id>\n\n<text>"`); (2) Paseo tool (only when `detail.type === "unknown"`): catalog label + context summary; (3) canonical per detail type:
shell → "Shell"/command; read/edit/write → "Read"/"Edit"/"Write" + path with cwd prefix stripped (`stripCwdPrefix`); search → "Search"/query; fetch → "Fetch"/url; worktree_setup → "Worktree setup"/branchName; sub_agent → `subAgentType ?? "Task"`/description; plain_text → (name humanized)/label; plan → "Plan"; unknown → humanized tool name. (4) overrides for unknown: tool `task` → "Task"/`metadata.subAgentActivity`; `thinking` → "Thinking"; `terminal` → "Terminal"/plain_text label.
`humanizeToolName`: names containing `: . /` or `__` are shown verbatim; otherwise `_-.` → spaces, lowercased, first letter capitalised. `errorText` only when `status==="failed"`: string, or `error.content`, else pretty JSON.
App wrapper `A/utils/tool-call-display.ts` re-exports; `A/tool-calls/presentation.ts` (`buildToolCallPresentation`) adds: `icon` (resolver below), `isLoadingDetails = running && no error && !hasMeaningfulToolCallDetail(detail)` (never for worktree_setup/sub_agent), `hasDetails = error || meaningful detail`, `canOpenDetails = hasDetails || isLoadingDetails`, `openFilePath = extractToolCallFilePath` (read/edit/write filePath; for shell only simple `cat|bat|less|more|head|tail|wc|nl|tac|od|xxd|file|stat|column|md5|md5sum|sha1sum|sha256sum|shasum <file>` without shell operators `|><&;\`$()`), `isPlan`, `planOutcome` (`canceled` if status canceled; `metadata.approved===false` → rejected; `true` → approved; running → pending).
`hasMeaningfulToolCallDetail` (`A/utils/tool-call-detail-state.ts`): shell always; read/write need filePath or content; edit any of filePath/unifiedDiff/old/new; search query/content/filePaths/webResults/annotations; fetch url/result/codeText; plain_text label or text; plan non-blank text; unknown any non-empty input/output; worktree_setup and sub_agent never.
Icon resolver `A/utils/tool-call-icon-name.ts`: agentMessage → `paseo`; plain_text.icon wins; `thinking`+unknown → brain; `speak` → mic_vocal; Paseo tool → `paseo`; `task` → bot; else by detail: shell→square_terminal, read→eye, edit/write→pencil, search/fetch→search, worktree_setup→square_terminal, sub_agent→bot, plain_text/unknown→wrench, plan→brain.

### 7.1 Paseo tool presentation catalog (`P/paseo-tool-presentation/catalog.ts`, `index.ts`, `detail-sections.ts`, `summary-text.ts`)
- `MAX_SUMMARY_CHARS = 120` (summary whitespace-collapsed from a 480-char scan window, ellipsis, no dangling surrogate). Context value formats: string → summary text, number → string, array → first + ` +N`.
- Leaf-name recognition: `mcp__paseo__<leaf>`, `paseo.<leaf>`, or `paseo_<leaf>` (not `paseo__`). Unknown leaf → humanized leaf name, no summary. `getPaseoToolActionLabel(toolName)` = `completedLabel` (used by group summaries). `getPaseoToolTargetAgentId` reads `input.agentId` (app hook `use-target-agent-title.ts` resolves the title).
- Catalog (label / completedLabel / summary context), exact names (`PASEO_TOOL_CATALOG`):
  agents: `create_agent` Create agent/Created agent (title|initialPrompt), `send_agent_prompt` Send message/Sent message (prompt), `get_agent_status` Check agent/Checked agent (target), `get_agent_activity` Read agent activity (target), `list_agents` List agents/Listed agents, `update_agent` Update agent/Updated agent (name|target), `set_agent_mode` Set agent mode (modeId), `cancel_agent` Cancel agent/Canceled agent, `archive_agent` Archive agent/Archived agent, `kill_agent` Kill agent/Killed agent, `list_pending_permissions` List permissions/Listed permissions, `respond_to_permission` Respond to permission/Responded to permission.
  workspaces: `create_workspace` Create workspace (title|branchName|branch|basename(path)), `list_hosts`, `list_workspaces`, `rename_workspace` (title), `archive_workspace` (workspaceId), `list_workspace_scripts`, `start_workspace_script`/`stop_workspace_script` Start/Stop script (scriptName).
  terminals: `create_terminal` (name|command), `list_terminals`, `capture_terminal` "Read terminal" (terminalId), `send_terminal_keys` "Send keys" (keys), `kill_terminal`.
  schedules: `create_schedule`, `create_heartbeat` (name|cron), `update_schedule` (name|id), `list_schedules`, `inspect_schedule`, `schedule_logs` "Read schedule logs", `run_schedule_once` "Run schedule"/"Ran schedule", `pause_schedule`, `resume_schedule`, `delete_schedule`, `delete_heartbeat` (id).
  providers: `list_providers`, `list_models` (provider), `list_profiles`, `inspect_provider` (provider).
  browser (22): `browser_list_tabs`, `browser_new_tab` Open browser tab (url), `browser_close_tab` (browserId), `browser_navigate` Navigate (url), `browser_back` Go back, `browser_forward` Go forward, `browser_reload` Reload page, `browser_snapshot` Snapshot page, `browser_screenshot` Take screenshot, `browser_click` Click element (ref), `browser_hover` Hover element, `browser_drag` Drag element (sourceRef), `browser_fill` Fill field, `browser_select` Select option (value|ref), `browser_type` Type text (text), `browser_keypress` Press key (key), `browser_upload` Upload files (filePaths), `browser_wait` Wait for page (text|url), `browser_scroll` Scroll page, `browser_resize` Resize browser (`WxH`), `browser_logs` Read browser logs, `browser_evaluate` Run JavaScript (function).
- Detail sections (`buildPaseoToolDetailSections(toolName, input, output)` → `PaseoToolDetailSection[] | null`): `{kind:"prose", title:"Prompt", text}` for the entry's `promptField` (`initialPrompt` for create_agent, `prompt` for send_agent_prompt/create_schedule/create_heartbeat/update_schedule), then `{kind:"fields", title:"Details", fields:[{label,value}]}` from input ordered by `inputOrder` (WORKSPACE_FIELDS, AGENT_FIELDS, AUTOMATION_FIELDS, BROWSER_FIELDS, or small lists), then `{kind:"fields", title:"Result"}` from output restricted/ordered by `outputFields` (MCP result unwrapped: `structuredContent`, else single `content[text]` parsed as JSON). Labels via `FIELD_LABELS` map else humanised camelCase; values: null→"None", bool→Yes/No, arrays bulleted. `P/paseo-tool-call-detail.ts` is a COMPAT re-export (remove after 2027-04-19).
- Server side use: `S/agent/paseo-tool-policy.ts`/`mcp-server.ts` define the tools themselves (other slice).

---------------------------------------------------------------------------------------------------
## 8. App: stream state model (StreamItem, reducers)

`A/types/stream.ts`. Server timeline items are converted to client `StreamItem`s (`kind`):
| kind | fields |
|---|---|
| `user_message` | `id, clientMessageId?, messageId?, turnId?, timelineCursor?, text, timestamp: Date, images?: AttachmentMetadata[], attachments?: AgentAttachment[], sender?` (sender is added only by presentation for received agent messages) |
| `assistant_message` | `id, messageId?, turnId?, timelineCursor?, text, timestamp` |
| `thought` | `id, timelineCursor?, turnId?, text, timestamp, status: "loading"|"ready"` |
| `tool_call` | `id, timelineCursor?, turnId?, timestamp, payload: {source:"agent", data:{agentMessage?, provider, callId, name, status:"running"|"completed"|"failed"|"canceled", error, detail, metadata?}} | {source:"orchestrator", data:{toolCallId, toolName, arguments, result?, error?, status:"executing"|"completed"|"failed"}}` |
| `todo_list` | `id, timelineCursor?, turnId?, timestamp, provider, items: TodoEntry[], activity: TaskActivity` (`{type:"created",count}` or `{type:"added"|"started"|"completed", task}`) |
| `notification` | `id, sourceType:"error"|"notification", timelineCursor?, turnId?, timestamp, level, message` (an `error` item becomes level "error", message default "Unknown error") |
| `compaction` | `id, timelineCursor?, turnId?, timestamp, status, trigger?, preTokens?` |
| `plugin` | `id, sourceId?, timelineCursor?, turnId?, timestamp, pluginId, pluginItemId, itemKind, version, data` |
`TimelinePosition = {epoch, seq}` (seq = entry.seqEnd). `getStreamItemMessageId(item)` = plugin `sourceId ?? id`, else `id` (find/outline/scroll address messages, not rows).
### 8.1 Reducer rules (`reduceStreamUpdate`, `reduceTimelineEvent`)
- IDs: `generateMessageId()` = `msg_<ms>_<rand9>`; timeline ids `<prefix>_<ms>_<hash36>_<stateLength36>`; assistant id = provider `messageId` if free, else `<messageId>:segment:<ms36>[:<n>]`; tool call id = `agent_tool_<identity>`, identity = `turn:<enc turnId>/<enc callId>` when turn known else `callId` (app identity includes turn, unlike protocol identity).
- Text chunks: `\r` stripped; blank-only chunks ignored when starting a new item. assistant chunk appends to last assistant if `messageId` undefined or equal; during live interrupt it can extend the assistant row one back when a user row follows. Reasoning appends to last thought of the same `turnId`, status "loading". Thoughts finalize to "ready" on any non-reasoning timeline event and on `thread_started/turn_started/turn_completed/turn_failed/turn_canceled/permission_*/attention_required`.
- Tool calls: merge by identity: status merge `failed` sticky, `canceled` sticky unless completed, else `completed` beats running; `error` kept only for failed (default `{message:"Tool call failed"}`); detail merge (`mergeToolCallDetail`: unknown never replaces known; same type shallow merge; different type incoming wins; unknown+unknown merges non-empty values); metadata shallow-merged.
- Special tools → todo list instead of tool row (`extractTaskEntriesFromToolCall`, `A/utils/tool-call-parsers.ts:298`): `TodoWrite`/`todo_write` (Claude: items text = `activeForm || content`), `update_plan` (`plan[].step/status`); for provider "claude" `TaskCreate/TaskUpdate/TaskList` rows are dropped; `ExitPlanMode` is not a task list. `todo` timeline items replace the last todo list of same provider; `deriveTaskActivities` creates `created` (first list), `added`, `started` (→in_progress), `completed` activity rows (an `added` right after a pure-pending `created` is folded into it).
- Compaction: a `completed` item upgrades the earliest `loading` compaction row (fills trigger/preTokens), else new row; `loading` always adds a row.
- Plugin items: upsert by `pluginId/id`.
- Turn membership: `applyTimelineTurnId` stamps `turnId` onto the rows the event wrote; canonical user rows overwrite an optimistic row's turn (`reconcileCanonicalUserTurnMembership`).
### 8.2 Head/tail model (`applyStreamEvent`, `flushHeadToTail`)
`tail` = committed rows, `head` = actively streaming rows. Streamable kinds (`assistant_message`, `thought`) go to head; a non-streamable event, a different streamable lane, a different assistant `messageId`, or any turn completion event (`turn_completed|turn_failed|turn_canceled`) flushes head→tail (dedup by id; thoughts forced ready). A tail-final assistant is pulled back into head if the same message continues. User messages: `applyCanonicalUserMessageEvent` → `upsertUserMessageAcrossStream` (matching by `clientMessageId`/`messageId`, policies `canonical-incoming` | `handoff`). Source modes `live` | `canonical`.
### 8.3 Session reducers (`A/timeline/session-stream-reducers.ts`)
- Live path `processAgentStreamEvent(s)` with `AgentStreamReducerQueue`: events are queued per agent and committed on `requestAnimationFrame` OR a `AGENT_STREAM_REDUCER_FLUSH_DELAY_MS = 16*3 = 48 ms` timer, whichever first (`:20,1859`).
- **Sequencing gate** (`processTimelineSequencingGate :1496`, `classifySessionTimelineSeq :282`): only for timeline events carrying both `seq` and `epoch`, and only when the client holds an authoritative baseline (`hasAuthoritativeBaseline`). Against cursor `{epoch,endSeq}`: no cursor → `init` (cursor=seq); epoch differs → `drop_epoch` (except `seq===1`: reset live timeline, new cursor); `seq <= endSeq` → `drop_stale`; `seq === endSeq+1` → `accept` (advance cursor); else `gap` → event NOT applied and side effect `catch_up {epoch, endSeq}` (→ `recoverGap`, fetches `direction:"after"`).
- Cursor state `TimelineCursor {epoch, startSeq, endSeq, retainedRanges?: {startSeq,endSeq,hasOlder?}[]}` (session-store `agentTimelineCursor`); `AgentTimelineState` = `cold | painted{items} | synced{items, range, older:"available"|"none", newer:"available"|"none"}` (`A/stores/session-store.ts:306`).
- **Fetch-response path** `processTimelineResponse :1232`: error → leave state, reject init. Entries → units `{seq: seqStart, seqEnd, sourceSeqRanges, event: timeline, timestamp}` and hydrated with `timelineCursor {epoch, seq: seqEnd}` via `hydrateStreamState(source:"canonical")`. Policies: bootstrap tail while initializing → replace; `reset` → replace; resume tail (`deriveResumeTailPolicy :345`): different epoch → replace(no continuity); `window.maxSeq === cursor.endSeq` → discard; `maxSeq < endSeq` → replace; page start ≤ endSeq+1 → append; else replace with continuity preserved; `mergeWindow` → `mergeTimelineWindow` (prompt-jump pages merged outside the contiguous range, rows sorted by cursor seq, `retainedRanges` tracked, `older` recomputed); `after` → incremental append; `before` → prepend (`acceptOlderTimelineUnits`, boundary merge for identity-split tool rows, overlapping projected assistant/reasoning reconciliation). Init resolves when response direction equals init direction (and `after` has no more pages). `isTimelineCatchUpComplete`, `isTimelineResumeSnapshotAuthoritative` in `timeline-sync-plan.ts`.
- `deriveAgentStreamTurnLiveness(events)` maps `turn_started` → `stream_open{turnId, startedAt}` and `turn_completed|failed|canceled` → `stream_close{turnId}`.

---------------------------------------------------------------------------------------------------
## 9. App: viewed-timeline sync (subscriptions, catch-up, resume)

Files `A/timeline/{viewed-timeline-sync,timeline-sync-plan,timeline-fetch-policy,timeline-replacement,fetch-agent-timeline-once,turn-liveness,turn-time,sync-status}`, `A/contexts/session-context.tsx:380-470`, `A/runtime/{host-runtime,directory-sync,replica-cache}`.
- **Constants**: `TIMELINE_FETCH_PAGE_SIZE = 40` projected items (`timeline-fetch-policy.ts`). Every plan uses `projection:"projected"`, `wholeTurns:true`.
- **Plans** (`timeline-sync-plan.ts`): `planTimelineTailFetch` = `{direction:"tail", limit:40}`; `planTimelineCatchUpAfter(cursor)` = `{after, cursor, limit 40}`; `planTimelineResumeFetch(range)` = after-cursor if a loaded range exists else tail; `planTimelineOlderFetch(cursor)` = `{before, cursor, limit 40}`; `planTimelinePromptJump(target)` = `{before, cursor:{epoch, seq: target.seq + floor(40/2) + 1}, limit 40, mergeWindow:true}` (puts the target in the middle of a bounded page).
- **Subscription**: `createViewedTimelineOwner` (per host) → `createViewedTimelineSync(ports)`: `replaceVisibleAgentIds(sourceId, ids)` (chats on screen), `replaceOpenTabAgentIds` (only releases subscriptions for closed tabs, never adds), `setActive` (app foreground), `setConnected`, `recoverGap`, `retryVisibleAgentTimeline`. It calls `client.observeTimeline(agentIds)` (one owned subscription for the demanded set) and `directory.setAgentRouteDemand`. `C/timeline-subscription` `subscribeTimeline` is the SDK/CLI flavour: relays `agent_stream` and `agent.timeline.replacement`, synthesises local `agent.timeline.subscription_restored{agentId,subscriptionId}` (after reconnect re-establishment) and `agent.timeline.error{agentId,error}`; "the consumer owns history reads and gap recovery".
- **Catch-up state machine** per agent: `CatchUpStatus = running|complete|error`; `decideCatchUp` → `keep | keep-and-park | replace` (a same-request running catch-up is kept, a running tail fetch is kept and the new one parked, otherwise replaced); status exposed `ViewedTimelineStatus = ready|pending|error|retrying`, retry only from `error` (manual retry shows `retrying`). Resume after disconnect/background: `planTimelineResumeFetch(readCursor)` (cache range via replica takes priority over store cursor). Forced tail replacement (`fetchLatestTail`) marks the next response "forced" (`consumeForcedTimelineTailReplacement`).
- **Replacement**: `agent.timeline.replacement{agentId, epoch}` (rewind) → `requestTimelineReplacement` = `fetchAgentTimeline(planTimelineTailFetch())` (`session-context.tsx:390`), the new epoch causes `replace`.
- **Fetch plumbing**: `HostRuntimeStore.fetchAgentTimeline` (`host-runtime.ts:2802`) → `directory.fetchTimeline` (`runtime/directory-sync/index.ts:471`, dedups identical in-flight requests via `fetchAgentTimelineOnce` keyed `agentId:JSON(request)`, submits `page.agent` snapshot to the agent directory) → `owner.flushStreamAgent(agentId)` then `owner.applyTimelineResponse(page)` (flush queued live events first so ordering holds).
- **Init state**: `fetchPage` creates an init deferred (`getInitKey`) and `setAgentInitializing(agentId,true)` for non-synced chats; timeouts via `refreshAgentInitializationTimeout`.
- **Replica cache** (`runtime/replica-cache/index.ts`): paints cached timeline before network: `CachedTimeline {agentId, items, range, hasOlder}`, `MAX_TIMELINE_ITEMS = 50`, `PERSIST_DELAY_MS = 1000`, `MAX_CACHE_BYTES = 32 MiB`; cached state is `painted` (not `synced`), replaced canonically when the first authoritative page arrives (`replaceWithCanonicalStream` with `preserveContinuity`).
- **Older history**: `useLoadOlderAgentHistory` (`A/hooks/use-load-older-agent-history.ts`) → `planTimelineOlderFetch({epoch, seq: range.startSeq})`; in-flight flag `agentTimelineOlderFetchInFlight`; failure toast `loadOlderHistory.failed` (2200 ms). `hasOlder` = synced && older==="available".
- **Returning to tail while detached** (`isTimelineDetached = agentTimelineHasNewer`): scroll-to-bottom button calls tail fetch then scrolls (`view.tsx:~650`, `returnToTimelineTail`).
- **Turn liveness** (`timeline/turn-liveness.ts`): `TurnLiveness = {phase:"idle", cancellationRequestId} | {phase:"open", turnId, startedAt, cancellationRequestId}`; transitions `snapshot{activeTurn} | stream_open | stream_close | cancellation_started{requestId} | cancellation_settled | destructive_close`; a close with a different identified turn id is ignored; `resolveTurnPresentation(liveness, hasActiveSubmission)` → `{isActive, isCancelling, startedAt, turnId}` (an active composer submission counts as active before the turn opens). Snapshot source: `AgentSnapshotPayload.activeTurn {turnId, startedAt}`.
- **Sync status toast** (`timeline/sync-status.tsx`): `sync.status==="reconnecting"` → toast `agentPanel.states.reconnecting`; `"catching_up"` with `ui==="status"` → `agentPanel.states.updating`; spinner, no auto-dismiss.

---------------------------------------------------------------------------------------------------
## 10. App: stream presentation pipeline (view model)

`A/agent-stream/presentation.ts` `createStreamPresentation()` (stateful memoised function) called in `view.tsx:~475`:
1. Plugin transforms: `projectPluginTimelineItems(tail/head, transform)` (`A/plugins/timeline/projection.ts`): each source StreamItem is converted back to an `AgentTimelineItem` (`sourceTimelineItem`), passed to the installed transformer with `phase: "streaming"|"complete"` (streaming for loading thoughts, running tool calls, head assistants), and replaced by `plugin` rows with `sourceId`; frozen deep copies. Plugin rows rendered by `PluginTimelineItemView`.
2. `presentRows`: agent-message tool calls with `agentMessage.event==="message"` are shown as `user_message` rows with `sender` (so another agent's message looks like a user bubble with a "Sent by <name>" button); user messages whose text is a complete `<spoken-input>…</spoken-input>` voice envelope (optional trailing `<instruction>This message was spoken by the user.…`) show only the spoken text.
3. Tool-call detail level: setting `chatDisplay: "full"|"collapseToolCalls"|"collapseTurns"` (`A/hooks/use-settings/storage.ts:46`; UI `screens/settings/chat/chat-section.tsx`). `full` → level `detailed` (no grouping); others → `overview` grouping. `autoExpandReasoning` setting ("Expand thinking"): when on, reasoning rows are not groupable.
4. Visibility filter: tool rows named `ExitPlanMode`, and `plan_approval` while `running` are hidden (pending plan shown by the approval UI; stays in the model so resolution reveals a plan card at its position).
5. Grouping (`tool-calls/detail-level/grouping.ts`): consecutive groupable rows (tool calls that are not turn triggers, not `plan` detail, not named `speak`; thoughts when not auto-expanded) in the same turn (`!isTurnBoundary`) form a `ToolCallRun {id: first.id, calls, latest, isSealed}`; ≥2 calls → one host row (id = first id) + `OverviewToolCallGroup {mode:"overview", run, summary, isLoading}`. History is grouped once (`prepareGroupedHistory`, cached by array identity); the live head merges into the trailing run (`groupLiveToolCalls`; trailing run is "active" while `isTurnActive` or any call running).
6. `collapseTurns` (`display === "collapseTurns"`, `agent-stream/turn-collapse/model.ts`): for each closed turn, rows between the trigger (user message or agent-message tool call) and the last assistant message of that run fold into one `TurnSummaryLine` under the trigger. Conditions: host exists in loaded rows, run not the open latest turn (`isLatestTurnOpen`: head present and no boundary, else active turn matches), reply index > run start, folded part has work or an assistant message. Folded rows that stay visible: turn triggers and `compaction` rows. Reasoning only revealed when opened. State `expandedTurnHostIds` (view state, reset per agent); expanded rows render nested in an indent rail (`TurnIndent = none|openedHost|nested|lastNested`). Outputs `turnsByHostId`, `nestedItemIds`, `hostIdByFoldedMessageId` (chat find reveals match by expanding), `hostIdByHiddenRowId` (reading anchor replacement).
7. `readingRowReplacements` maps hidden/grouped row ids to the visible host for scroll anchoring.
Summary labels (`tool-calls/detail-level/overview/model.ts` `summarizeToolCalls`, `label.ts`): counts distinct edited files (edit/write detail), commands (shell), distinct read files, searches (`search` detail or tool name ending `web_search`/`llm_context`), other tools, Paseo actions (`completedLabel` counts, e.g. "created agent ×2"), thoughts (only label if nothing else: `toolCallGroup.thought`). i18n keys `toolCallGroup.{editedFiles,commands,readFiles,searches,otherTools}.{one,other}`, `toolCallGroup.and`, `turnSummary.details`.
Turn timing (`A/timeline/turn-time.ts` `deriveStreamTurnTiming`): walks tail (cached by array identity) + head; per turn `completedAt = last item timestamp`, `durationMs = lastItem - trigger timestamp` (null if no visible trigger); attached to the assistant message(s) of the turn, or the last tool call when no assistant message; `runningStartedAt` = active turn start.

---------------------------------------------------------------------------------------------------
## 11. App: stream view, layout, history window, strategies

### 11.1 AgentStreamView (`A/agent-stream/view.tsx`, props `AgentStreamViewProps`)
Props: `agentId, serverId?, context: AgentScreenAgent, streamItems(tail), streamHead?, pendingPermissions, pendingMessageSubmissions?, turnPresentation, routeBottomAnchorRequest?, isAuthoritativeHistoryReady?, bottomOverlayTailClearance?, bottomOverlayControlClearance?, toast?, onOpenWorkspaceFile?, readOnly?, historyPagination?`. Imperative handle `{scrollToBottom(reason), prepareForViewportChange()}`. Frozen while its panel is hidden (`useRetainedValue(…, isActive)`).
Row renderer `renderStreamItemContent` by `item.kind`: `user_message` → `UserMessage` (+ `TurnSummaryLine` if the turn is collapsed), `assistant_message` → `AssistantMessage` (wrapped in `ChatFindExpansion` and `AssistantFileLinkResolverProvider`), `thought` → `ThoughtSlot` (ToolCall with toolName "thinking"), `tool_call` → group view or single `ToolCallSlot` (`speak` + string input → `SpeakMessage`; plan detail → `PlanCard`), `notification` → `Notification`, `todo_list` → `TodoListCard`, `compaction` → `CompactionMarker`, `plugin` → `PluginTimelineItemView`. Orchestrator-source tool calls (`payload.source==="orchestrator"`) render as unknown-detail ToolCall (args/result).
Other elements: pending permission cards at the stream foot (`PermissionRequestCard`; permissions slice), `TurnFooter`, empty state `agentStream.empty`, scroll-to-bottom floating button (fade 200 ms, disabled on Android because of reanimated#8422), bottom overlay inset, toast `agentStream.historyLoadFailed`. Tool-call inline-expansion tracking: `expandedInlineToolCallIds` disables parent scroll on native.
### 11.2 Layout (`layout.ts` `layoutStream`)
Per row `StreamLayoutItem {item, aboveItem, belowItem, gapBelow, assistantSpacing: "default"|"compactTop"|"compactBottom"|"compactBoth", completedFooter: TurnFooterHost|null, toolSequence: "single|first|middle|last|none", isFirstInUserGroup, isLastInUserGroup, isLastInToolSequence, frameOrder, phase: "streaming"|"complete"}`. Gaps (`spacing.ts`): user→user `SPACING[1]`; user→assistant 0; tool-seq→tool-seq 0 (tool_call/thought/todo_list are "tool sequence" items); user→tool `SPACING[4]`; assistant↔tool `SPACING[1]`; default `SPACING[4]`. Completed footer: attached to the last assistant message of a response (falls back to last tool call when the response has no assistant message); bottom footer row for the latest turn (`auxiliaryTurnFooter`) when no turn active.
### 11.3 Render model & virtualization (`model.ts`, `history-window.ts`, `web-virtualization.ts`)
`buildAgentStreamRenderModel` → `{history, segments{historyVirtualized, historyMounted, liveHead}, turnTiming, boundary, auxiliary}`. Web desktop only: when tail > `DEFAULT_WEB_PARTIAL_VIRTUALIZATION_THRESHOLD = 100` rows the oldest rows are virtualized (`@tanstack/react-virtual`, `overscan: 8`, scrollMargin 48 px, measured heights cached per width, estimates `estimateStreamItemHeight`: user 96 (220 with images), assistant cached estimate or 220, tool_call/thought 40, todo 144, notification 88, compaction 72, other 120), the newest `DEFAULT_MOUNTED_RECENT_STREAM_ITEMS = 20` stay mounted, window start pulled back to a turn boundary (`findMountedWindowStart`, `continuesTurn`). E2E overrides: `globalThis.__PASEO_E2E_WEB_PARTIAL_VIRTUALIZATION_THRESHOLD`, `__PASEO_E2E_WEB_MOUNTED_RECENT_STREAM_ITEMS`.
`useStreamHistoryWindow`: local window start (boundary item id) over already-loaded rows; `loadOlder` first reveals more loaded local rows (`revealLoadedHistory`), then asks the server (`loadRemoteOlder`); `hasOlder = hasLocalHistory || remoteHasOlder`; progress key `${remoteProgressKey}:${historyStart}`.
### 11.4 Strategies (`strategy.ts`, `strategy-web.tsx`, `strategy-native.tsx`, `strategy-resolver.ts`)
`resolveStreamRenderStrategy({platform, isMobileBreakpoint})` → web strategy for web (incl. mobile breakpoint) else native strategy. Differences:
- Web: normal-order DOM scroll container; edge slot = footer; `flatListInverted:false`; `anchorBottomOnContentSizeChange:true`; transport `verificationDelayFrames:0, retryMode:"rescroll"`; near-bottom = `distanceFromBottom <= threshold`; constants `AUTO_SCROLL_BOTTOM_THRESHOLD_PX = 64`, `AUTO_SCROLL_RESUME_THRESHOLD_PX = 1`, `WEB_BOTTOM_SETTLE_TIMEOUT_MS = 200`, `USER_SCROLL_DELTA_EPSILON = 1`, `BOTTOM_OVERSCROLL_TOLERANCE_PX = 2`, `UPWARD_INPUT_EVIDENCE_TIMEOUT_MS = 100`, `HISTORY_START_SLOT_HEIGHT_PX = 32`, `CONTENT_PADDING_TOP_PX = 16`, `HISTORY_START_SETTLE_FRAMES = 2`. Reading anchor (`reading-anchor.ts`, `READING_POSITION_OFFSET_PX = 8`) keeps the row under the top of viewport pinned across prepends, virtual measurement changes, collapse/expand; reports reading row for the outline.
- Native: inverted `FlatList` (tail/head arrays are reversed: `orderTailReverse/orderHeadReverse: true`, `assistantTurnTraversalStep: 1`, footer-then-content frame order, edge slot = header), `maintainVisibleContentPosition {minIndexForVisible:0, autoscrollToTopThreshold:0}`, `initialNumToRender 2`, `maxToRenderPerBatch 10`, `windowSize = NATIVE_STREAM_WINDOW_SIZE = 10`, `animateManualScrollToBottom:true`, parent scroll disabled while inline details expand; `NATIVE_SETTLING_VERIFICATION_DELAY_FRAMES = 4` with `retryMode:"recheck"` during viewport settling (keyboard); keyboard dismiss on scroll (`scroll-keyboard-dismiss/model.ts`: `DISMISS_VELOCITY_POINTS_PER_MS = 1.5`, `RELEASE_SAMPLE_MIN_MS = 30`) and keyboard inset (`useKeyboardStreamInset`).
### 11.5 Bottom anchoring (`bottom-anchor-controller.ts`)
Modes `sticky-bottom | detached`. Requests: route `{reason: "initial-entry"|"resume", agentId, requestKey}` and local `{reason: "jump-to-bottom"|"message-sent"|"rewind", agentId}`. Blocked reasons `waiting_for_history_readiness | waiting_for_measurable_viewport | waiting_for_measurable_content | waiting_for_post_layout_verification`. `MAX_VERIFICATION_RETRIES = 3`, `WEB_PARTIAL_VIRTUALIZED_CONFIRMATION_DELAY_FRAMES = 1`, `USER_SCROLL_AWAY_DELTA_PX = 24` (a user scroll away detaches; returning near bottom re-sticks). Verifies after layout and retries (`rescroll` / `recheck`).
### 11.6 History-start pagination (`history-start-pagination.ts`)
`HISTORY_START_THRESHOLD_PX = 96`. State machine `dormant → ready → loading{requestedProgressKey, requestObserved} → settling{loadedProgressKey} → ready|latched`; load fires when within 96 px of the history start, viewport measured, `hasOlderHistory`, not already loading, progressKey known; latch until the user scrolls away (rearm) to avoid re-request loops; a settle scheduler waits `HISTORY_START_SETTLE_FRAMES=2` frames after content arrives. A 32 px spinner slot (`historyStartSlot`) shows while loading.
### 11.7 Prompt jump / scroll-to-message (`prompt-jump-settle.ts`, `use-scroll-to-message.web.ts`)
Scroll to a message id landing `PROMPT_JUMP_TOP_INSET_PX = 8` below container top; settles over frames: tolerance `POSITION_TOLERANCE_PX = 1`, `REQUIRED_STABLE_FRAMES = 2`, `FRAME_BUDGET = 24`; cancelled by user intent (scroll keys `ArrowUp/Down, Home, End, PageUp/Down, Space`, wheel, touch). Optional `ScrollToMessageOccurrence {signal, targetTop()}` for chat find (lands on an occurrence inside a long message). `scrollToMessage` is optional on `StreamViewportHandle`; only the web hook `use-scroll-to-message.web.ts` implements it (native: not found).

---------------------------------------------------------------------------------------------------
## 12. Chat find (web only)

`A/agent-stream/chat-find/{index.tsx (native stub: ChatFind passes children, useChatFindSelectedMessageId→null), index.web.tsx, model.ts, ranges.web.ts, viewport.web.ts}`.
- Entry: Ctrl/Cmd+F (`isFindShortcut`) while the panel is active/interactive, no web overlay, no IME composition; capture-phase `keydown` on document (RN TextInput swallows bubbling). Widget `PaneFind` overlay top-right (query, status, prev/next, close). Closing returns focus to the stream viewport; panel deactivation closes it.
- `ChatFindModel` (`model.ts`) phases `idle|searching|loading|ready|error`; failures `connection | historyChanged | reveal` (i18n `paneFind.connectionFailure|historyChangedFailure|revealFailure`, `paneFind.retry`; status texts `paneFind.searching|loading|failed|position{current,total}|noMatches`).
- Flow: `setQuery` debounced 120 ms → loop `client.searchAgentTimeline({agentId, query, cursor})` until `nextCursor===null` (host call §5.4; results must keep the same epoch as the loaded timeline else `historyChanged`), accumulate `locations {seq, role, count (default 1 for hosts < v0.9.0)}`, then navigate. Navigate: resolve a location to a loaded row by `timelineCursor {epoch, seq}` and role (a user prompt is a hard boundary); if not loaded → `fetchAgentTimeline(planTimelinePromptJump({epoch, seq}))` (mergeWindow page) and wait up to 5000 ms (`waitForTarget`) for the row; `revealLoadedMessage` expands folded turns / local history; `ChatFindExpansion` lifts the 32 000-char render cap for the selected assistant message and the web virtualizer mounts all its block rows so the whole message can be counted and highlighted (CSS Custom Highlight `::highlight(paseo-chat-find-N)`); occurrence counts replaced by rendered counts after reveal; position = occurrence across whole chat.
- History epoch change while open → `historyChanged` failure.

---------------------------------------------------------------------------------------------------
## 13. Chat outline (web only)

`A/agent-stream/chat-outline/{use-chat-outline.ts, model.ts, rail.web.tsx, rail.tsx(native stub), hover-intent.ts}`. Enabled when server feature `agentTimelinePromptIndex` and setting `chatOutlineEnabled` (and web, `isWeb`).
- Data: `client.listAgentTimelinePrompts(agentId)` (§5.4) re-fetched when the latest loaded prompt seq changes; accepted only if `payload.epoch === timelineEpoch` and the request is the newest.
- Rail: one tick per prompt (`SLOT_HEIGHT 8`, `RAIL_WIDTH 36`, resting pill 10×2 px, active 18, magnified 26×4), dock-style magnification `promptTickMagnification` (raised cosine, `OUTLINE_MAGNIFY_RADIUS = 3` slots), hover preview card 260×48 with `PREVIEW_GAP 4`, shown only if panel width ≥ `MIN_PANEL_WIDTH = 918`; hover intent `INITIAL_ACTIVATION_DELAY_MS = 150`, `HORIZONTAL_TRANSIT_DISTANCE_PX = 4` (`hover-intent.ts`).
- Active prompt = last indexed prompt with `seq <=` reading position seq (`resolveActivePromptSeq`), published through an external store (`createActivePromptPublisher`) to avoid re-rendering the transcript per scroll frame.
- Jump (`jumpToPrompt(seq)`): loaded → (reveal if outside mounted window) `viewport.scrollToMessage(id)`; not loaded → `fetchAgentTimeline(planTimelinePromptJump({epoch, seq}))` then scroll once the row mounts; failure → toast `agentStream.historyLoadFailed`. `PendingPromptJump {requestId, seq, fetchSettled, hasScrolled}`.

---------------------------------------------------------------------------------------------------
## 14. Message and item renderers (components)

### 14.1 User message (`A/components/message.tsx:392 UserMessage`, `components/user-message/index.tsx`)
Bubble with text (selectable), image pills (`UserMessageImagePill` → lightbox from attachment metadata), attachment cards, pending state (`isPending` while its `clientMessageId` is in `pendingMessageSubmissions`: no trailing row), first/last-in-group spacing, trailing row (copy, rewind, timestamp) visible on hover (web) / always on compact & native. Rewind (`useRewindAgentMutation` in `A/components/rewind/use-rewind-agent-mutation.ts`, which calls `fetchAgentTimeline` at :45 after the rewind; modes `RewindMode`, gated by capability flags `supportsRewindConversation|Files|Both`; wire `agent.rewind.request/response`; server `agent-manager.ts:3472 rewind` → provider rewind → re-hydrate → `timeline_replacement`; exact request fields not read here, belongs to another slice). `MessagePreview` (user-message/index.tsx): max height `PREVIEW_HEIGHT = 240` with bottom fade and "Show more/Show less" (`turnSummary.showMore|showLess`). `MessageSender` shows "Sent by <title>" (`turnSummary.sentBy`) and navigates to the sender agent for agent-to-agent messages. Timestamp formatter `formatMessageTimestamp`.
### 14.2 Assistant message (`message.tsx:1144 AssistantMessage`)
Markdown (`MarkdownRenderer`, agent commands enabled) with paced reveal `useRevealedText(text, phase)`; `phase` = layout `streaming|complete`. **Render cap (web)**: `ASSISTANT_MESSAGE_RENDER_CHARACTER_LIMIT = 32_000` UTF-16 units (cut at grapheme boundary), shows notice `agentStream.messageCapped{bytes}` (UTF-8 length) when complete; lifted when chat find selects the message (`renderFullContent`). Native never caps (`assistant-message-render-limit.native.ts`). Height measured and cached (`setAssistantMessageHeight`) for virtualizer estimates. File-path links in text open in the workspace via `createInlinePathOpener`/`AssistantFileLinkResolverProvider`.
### 14.3 Text reveal (`A/agent-stream/text-reveal.ts`, `A/hooks/use-revealed-text.ts`)
Display-only pacing of streamed text (store keeps full text; copy/outline/scroll use real text): `TEXT_REVEAL_HORIZON_MS = 150` (backlog drained over 150 ms), `TEXT_REVEAL_FRAME_INTERVAL_MS = 1000/60` (60 Hz commit cap), per-frame elapsed clamped to `MAX_ELAPSED_MS = 250`, step = `ceil(backlog*elapsed/horizon)` min 1; cut index clamped to a grapheme boundary (`Intl.Segmenter`; where unsupported text is shown whole); states `beginTextReveal/retargetTextReveal/advanceTextReveal/completeTextReveal/isTextRevealSettled`. Applies to assistant text and reasoning text (ThoughtSlot); settles immediately when `phase !== "streaming"` or no rAF.
### 14.4 Reasoning (thought)
Rendered as a tool-call style badge named "thinking" (`ThoughtSlot`, `view.tsx:1411`): detail = unknown with `input = revealedText`, `output=null` → shown as plain text; status loading → "executing" (spinner/shimmer), ready → completed; icon brain; `defaultExpanded = autoExpandReasoning` (`forceInline` when expanded even on mobile).
### 14.5 Tool call row (`message.tsx:2251 ToolCall`, `ExpandableBadge :1880`)
`ExpandableBadge` props `label, secondaryLabel, icon, isExpanded, onToggle, onOpenFile, onDetailHoverChange, renderDetails, isLoading (shimmer; web keyframes `paseo-toolcall-shimmer`, native SVG gradient), isError, isLastInSequence, disableOuterSpacing, borderlessWhenExpanded, testID`. `ToolCall`: label = `displayName`, secondary = `summary`, loading when status running|executing, error styling when failed, open-file affordance (`openFilePath`), inline expansion on desktop (state in `createToolCallExpansion`, `A/tool-calls/expansion.ts`: per-call open state with default, group open state, a lone call opened standalone also pre-opens the group it will join), on compact form factor (mobile) opens a bottom sheet (`ToolCallSheetProvider/useToolCallSheet().openToolCall`, `components/tool-call-sheet.tsx`, `ToolCallSheetData {toolName, displayName, summary, detail, errorText, icon, showLoadingSkeleton}`); `maxDetailHeight` default 400 (200 inside groups, `GROUPED_TOOL_CALL_DETAIL_MAX_HEIGHT`). Plan detail → `PlanCard` instead.
Group view `OverviewToolCallGroupView` (`tool-calls/detail-level/overview/view.tsx`): one badge (wrench icon, label = aggregate summary sentence, loading if any call running); expanded → scrollable (`TOOL_CALL_GROUP_MAX_HEIGHT = 400`, auto-scroll to latest) indent rail of individual rows; on compact devices opens `OverviewToolCallGroupSheet`.
### 14.6 Detail bodies by kind (`components/tool-call-details.tsx` `ToolCallDetailsContent`)
Props `toolName, detail, errorText, maxHeight (default 300), fillAvailableHeight, showLoadingSkeleton`. `buildDetailSections`:
- `shell`: command (trailing newlines trimmed) + output (leading newlines trimmed) in a terminal-style block; full-bleed.
- `edit`: diff view — `parseUnifiedDiff(unifiedDiff)` when present else `buildLineDiff(oldString,newString)` (`DiffLine {type: add|remove|context|header, content, segments?: {text,changed}[] (intra-line word diff), tokens? (syntax)}`), then `highlightDiffLines(lines, filePath)`; full-bleed. Diff components also `components/diff-viewer.tsx`, `diff-stat.tsx` (the checkout diff UI, other slice).
- `write`: file content, syntax highlighted by `filePath`; full-bleed.
- `read`: content with line numbers starting at `offset ?? 1`, highlighted; nothing when no content.
- `search`: content block, file path list, web results as `title\nurl` blocks, annotations.
- `fetch`: `FetchDetailSection` url + result text.
- `plain_text`: scrollable plain text.
- `unknown`: Paseo tool → `buildPaseoToolDetailSections` rendered as Prompt prose + Details/Result field lists (`PaseoDetailSection`); otherwise a single plain input string when output is null; otherwise "Input"/"Output" JSON sections (`toolCallDetails.input|output`).
- `worktree_setup`, `sub_agent`: no body (empty array). `plan`: handled as PlanCard.
- Error: `ErrorSection` (`toolCallDetails.error`) appended; empty body → loading skeleton (3 lines) while pending else `toolCallDetails.empty`. `shouldFill` only for shell/edit/write/read.
### 14.7 Plan card (`components/plan-card.tsx`)
`PlanOutcome = pending|approved|rejected|canceled`; title keys `agentStream.permission.plan|rejectedPlan|approvedPlan|canceledPlan`; expanded by default except rejected/canceled; collapsible; markdown body; optional `footer` (approval buttons are used by the permission card). Reset per outcome (`key={outcome}`).
### 14.8 Todo card (`message.tsx:1466 TodoListCard`)
`ExpandableBadge` with activity label `message.todo.activity.created{count}` / `.added|started|completed` + task text secondary; icons `CheckSquare` (created), `Plus`, `CircleDot`, `Check`; expanded list of `TaskListRow` (id ?? text keyed) or `message.todo.empty`.
### 14.9 Notification and error row (`message.tsx:1326 Notification`)
Colored banner (`info|warning|error` backgrounds/icons), selectable message. Both `notification` and `error` timeline items use it (error = level error). Note: turn failures arrive as `[System Error] …` assistant messages (see §5.1), not as `error` items.
### 14.10 Compaction marker (`message.tsx:1411`, `message-compaction-label.ts`)
Horizontal rule with label: loading → spinner + `message.compaction.loading`; completed → `message.compaction.auto` | `.manual` | (preTokens) `.withTokens{tokens: round(preTokens/1000)}` | `.completed`; scissors icon. Always visible even inside collapsed turns.
### 14.11 Speak message (`message.tsx:1253`)
`speak` tool with string input renders as plain voice text card (header i18n key not read) instead of a tool badge; icon mic_vocal otherwise.
### 14.12 Turn footer (`agent-stream/turn-footer.tsx`, `message.tsx:594 AssistantTurnFooter`, `:693 LiveElapsed`, `:810 TurnCopyButton`)
- Running: `WorkingIndicator` (synced loader 14 px) + in-flight fork menu (`AssistantForkMenu`, no boundary so the live response is included) + `LiveElapsed` (ticks every 1000 ms, `testID turn-working-elapsed`, stops when panel inactive).
- Completed: copy button (`getContent` = all assistant messages of the response joined `\n\n`, `collectAssistantResponseContent`), fork menu (needs `boundary`: `resolveAssistantTurnForkBoundary` → `{boundaryCursor, boundaryMessageId?}` when server feature `agentForkContextCursor` and the row has a cursor, else `{boundaryMessageId}`), then duration label (`getTurnDurationLabel`) which swaps to the end timestamp on hover (web) or tap for `TIMESTAMP_REVEAL_MS = 3000` (native); a turn without visible start shows the end timestamp. `TURN_FOOTER_BOTTOM_SPACING = SPACING[8]`.
### 14.13 Usage line / context window
There is no per-turn usage row in the timeline. `usage` rides on `turn_completed`/`usage_updated` into `agent.lastUsage` (snapshot). UI: context-window meter in the composer opens `components/context-window-details.tsx` (`contextWindow.title|used{percentage}|tokens{used,max}|noData|sessionCost`, then `AgentUsage` provider-usage cards, refreshable). Owned by the composer/controls slice.
### 14.14 Agent presentation (`A/agent-presentation/index.ts`)
`useAgentDescriptor(target{agentId}, {serverId})` → `PanelDescriptor {label, subtitle "<Provider> agent", tooltip, titleState "ready"|"loading", icon (provider icon), statusBucket}`; label from `resolveAgentLabel(title)` (null for blank or case-insensitive "new agent" → titleState loading); `statusBucket = deriveSidebarStateBucket({status: turnActive ? "running" : status, pendingPermissionCount, requiresAttention, attentionReason})`; provider label humanises `-`/`_` splits.

---------------------------------------------------------------------------------------------------
## 15. Capability / feature gates that affect the timeline
Client caps (`P/client-capabilities.ts`): `selective_agent_timeline`, `reasoning_merge_enum`, `projected_subagent_timeline`, `timeline_replacement_invalidation`, `timeline_notifications`, `plugin_timeline_items`. Server `serverInfo.features` relevant: `agentTimelinePromptIndex`, `agentForkContextCursor`, `projectedSubagentTimeline`, `wholeTurnTimeline`, `pluginTimelineItems`, `selectiveAgentTimeline` (advertisement set in `S/websocket-server.ts:2085-2129`).

---------------------------------------------------------------------------------------------------
## Cross-area connections
- Composer / message submission (`A/composer/submission`, `pendingMessageSubmissions`, `getSendingClientMessageIds`): optimistic user rows carry `clientMessageId`; server `recordSubmittedPrompt`/`reconcileSubmittedPromptEcho` dedupe; `message-sent` bottom-anchor reason.
- Permissions (`PermissionRequestCard`, `agent_permission_request`, `permission_requested/resolved` events): rendered at the foot of the stream; `plan_approval`/`ExitPlanMode` tool rows hidden while pending then shown as `PlanCard`; `resolvePendingPermissionsForAgent` on turn failure/cancel.
- Agent lifecycle/status (`agent_status`, `AgentSnapshotPayload.activeTurn`, `lastUsage`, `runtimeInfo`): snapshot returned inside every `fetch_agent_timeline_response.agent`; drives turn liveness, working footer, header usage meter.
- Rewind/fork (`agent.rewind.request/response`, `agent.fork_context.*`, `useForkAgent`, `useRewindAgentMutation`): consume timeline cursors/messageIds; rewind causes `timeline_replacement` + new epoch.
- Sub-agents (`A/subagents`, `provider-subagents/store.ts` which reuses `InMemoryAgentTimelineStore`, `agent.provider_subagents.*`): `sub_agent` detail is a launch marker; their own projected timeline paging uses the same cursor/epoch protocol.
- Attention/notifications (`attention_required`, `agent_attention_required`): split off the timeline stream per client.
- Plugins (`A/plugins/timeline`, `agent.timeline.append.request`, plugin timeline items, `useInstalledTimelineTransform`): server-stored plugin rows and client-side transforms.
- MCP / CLI (`get_agent_activity`, `wait`/`logs` commands): use `activity-curator` and `selectItemsByProjectedLimit`; CLI uses `subscribeAgentTimeline` / `fetchAgentTimeline`.
- Workspace/git: `commandMayHaveChangedExternalState` on completed shell calls triggers workspace git refresh.
- Settings: `chatDisplay`, `autoExpandReasoning`, `chatOutlineEnabled`, content max width.
- Voice: `speak` tool rows and `<spoken-input>` envelope; voice mode auto-allows speak permission (`session.ts:2043`).
- Schedules/heartbeats: `<paseo-system>` envelope → `heartbeat` tool-call rows (turn triggers).
- Providers: all tool-call mapping per provider (`S/agent/providers/*/tool-call-mapper*`) produce the ToolCallDetail kinds; provider history hydration feeds the timeline with `fromHistory` events.
- Host runtime / directory sync / replica cache (`A/runtime`): fetch dedup, agent snapshot merge, cached painting.

---------------------------------------------------------------------------------------------------
## Inventory
1. Timeline item types (9): user_message, assistant_message, reasoning, tool_call, todo, error, notification, compaction, plugin
2. Tool-call statuses and common fields (running/completed/failed/canceled; callId, name, detail, metadata, agentMessage)
3. ToolCallDetail kinds (11): shell, read, edit, write, search, fetch, worktree_setup, sub_agent, plain_text, plan, unknown
4. AgentStreamEvent internal vs wire union (which events never reach the wire)
5. Wire envelopes: agent_stream, fetch_agent_timeline_response entries, collapsed kinds
6. Server in-memory timeline store: epoch, seq, window, limits
7. Durable timeline store interface (unwired)
8. Item content size limiting (budgets, truncation marker)
9. Turn id assignment on rows
10. Timeline projection merges (assistant, reasoning, tool lifecycle, plugin identity)
11. Projection paging (tail/before/after, wholeTurns, spans, sourceSeqRanges)
12. Stream coalescer (60 ms leading+trailing)
13. AgentManager stream dispatch and error-as-assistant-message ("[System Error]")
14. Session timeline subscriptions (set_subscription request/response, owned subscriptions, capability filtering)
15. Timeline replacement (rewind) and legacy reconstructed replay
16. fetch_agent_timeline request/response (cursor, direction, limit, reset/stale/gap, mergeWindow, wholeTurns)
17. Timeline search (agent.timeline.search.*)
18. Prompt index (agent.timeline.list_prompts.*)
19. Plugin timeline append (agent.timeline.append.*)
20. Fork context from timeline (agent.fork_context.*, curator)
21. Agent message envelopes <paseo-system> and projection to tool_call rows (agent_message, heartbeat)
22. Tool-name normalization and Paseo tool name detection
23. Activity curator text rendering
24. Tool-call display model (names, summaries, error text)
25. Paseo tool presentation catalog (agents/workspaces/terminals/schedules/providers/browser) and detail sections
26. Tool call presentation in app (icons, meaningful detail, open-file path extraction, plan outcome)
27. App StreamItem model (8 kinds) and id rules
28. Reducer rules (text chunks, tool merge, todo extraction, compaction, plugin)
29. Head/tail streaming model and user message reconciliation
30. Session stream reducer queue (48 ms/rAF), sequencing gate (accept/stale/epoch/gap), response processing policies
31. Viewed-timeline sync (subscribe demand, catch-up state machine, resume, retry, status)
32. Timeline sync plans and TIMELINE_FETCH_PAGE_SIZE=40, prompt-jump window
33. Replica cache and painted/synced states
34. Older-history loading
35. Turn liveness state machine and turn timing
36. Sync status toast
37. Stream presentation pipeline (plugin transform, agent-message rows, spoken-input, detail level, hidden plan rows)
38. Tool-call grouping and overview summaries
39. Turn collapse (collapseTurns) and expanded rail
40. AgentStreamView (props, handle, row switch, empty state, scroll-to-bottom)
41. Stream layout (gaps, tool sequences, footers)
42. Render model, web partial virtualization, mounted window
43. Stream history window (local reveal then remote)
44. Platform strategies (web vs native inverted list) and constants
45. Bottom-anchor controller
46. History-start pagination state machine
47. Prompt-jump settle and scroll-to-message
48. Chat find
49. Chat outline rail
50. User message renderer (images, attachments, rewind, preview, sender)
51. Assistant message renderer (markdown, 32 000-char cap, byte notice)
52. Text reveal pacing
53. Reasoning (thought) row
54. Tool call row and details sheet/inline, tool group view
55. Detail bodies per kind (shell, edit diff, write, read, search, fetch, plain_text, unknown/Paseo, none for sub_agent/worktree_setup)
56. Plan card
57. Todo card
58. Notification/error row
59. Compaction marker
60. Speak message
61. Turn footer (working, copy, fork, duration/timestamp)
62. Usage/context window display (pointer only)
63. Agent presentation descriptor
64. Capability and feature gates
