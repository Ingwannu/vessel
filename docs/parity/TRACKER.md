# Parity tracker (Paseo 0.15.4 → Vessel)
State: missing / skeleton / built / passed / merged. A worker updates its rows when its package passes; the native lead marks merged.
Generated 2026-11-15 from FEATURES.md; the state column starts at 'unknown' until the package's worker audits it against Vessel.

| # | Pkg | Owner | Feature | Source | State |
|---|---|---|---|---|---|
| 1 | A1 | W3 | Agent storage (JSON files per agent, atomic queued writes, delete fence) | A-lifecycle.md | unknown |
| 2 | A1 | W3 | Stored record schema and wire payload projections (snapshot, list item, stored payload) | A-lifecycle.md | unknown |
| 3 | A1 | W3 | Agent owner (daemon execution ownership) | A-lifecycle.md | unknown |
| 4 | A1 | W3 | Legacy workspaceId backfill migration | A-lifecycle.md | unknown |
| 5 | A1 | W3 | Agent state buckets and status priority | A-lifecycle.md | unknown |
| 6 | A1 | W3 | Lifecycle statuses and transitions | A-lifecycle.md | unknown |
| 7 | A1 | W3 | Create agent: wire protocol (agent.create.request/response/update, legacy create_agent_request) | A-lifecycle.md | skeleton (staged; checks/runtime integration pending) |
| 8 | A1 | W3 | Create agent: session request layer (cwd checks, worktree targets, intent resolution) | A-lifecycle.md | skeleton (staged; checks/runtime integration pending) |
| 9 | A1 | W3 | Create agent: createAgentCommand and AgentManager.createAgent | A-lifecycle.md | skeleton (staged; checks/runtime integration pending) |
| 10 | A1 | W3 | Create agent mode resolution and configuration validator | A-lifecycle.md | unknown |
| 11 | A1 | W3 | Create agent titles (explicit/provisional) | A-lifecycle.md | unknown |
| 12 | A1 | W3 | CreationService idempotency receipts and creation.subscribe | A-lifecycle.md | skeleton (staged; checks/runtime integration pending) |
| 13 | A1 | W3 | Auto-archive after first turn (autoArchive flag) | A-lifecycle.md | unknown |
| 14 | A1 | W3 | New workspace screen (project/host/isolation/ref pickers, launch target) | A-lifecycle.md | skeleton (staged; checks/runtime integration pending) |
| 15 | A1 | W3 | New workspace submit flows (chat, empty workspace, terminal, background handoff) | A-lifecycle.md | skeleton (staged; checks/runtime integration pending) |
| 16 | A1 | W3 | Workspace draft tab and draft create flow state machine | A-lifecycle.md | skeleton (staged; checks/runtime integration pending) |
| 17 | A1 | W3 | create-agent-preferences (form preferences, optimistic layer) | A-lifecycle.md | skeleton (staged; checks/runtime integration pending) |
| 18 | A1 | W3 | agent-controls policy/labels and composer agent controls | A-lifecycle.md | skeleton (staged; checks/runtime integration pending) |
| 19 | A1 | W3 | CLI paseo run (+ options, workspace policy, output schema) | A-lifecycle.md | unknown |
| 20 | A1 | W3 | CLI agent ls/stop/delete/archive/reload/detach/update/import/open/wait/send/inspect/logs/attach/mode | A-lifecycle.md | unknown |
| 21 | A2 | W21 | Archive / unarchive / close_items (cascade to children, detach rules) | A-lifecycle.md | skeleton (atomic archive pending API + conditional current-parent child API; full cascade/workspace and resident hooks not ready) |
| 22 | A2 | W21 | App archive UX (optimistic caches, archived callout, tab close policy) | A-lifecycle.md | skeleton |
| 23 | A2 | W21 | Auto-archive on merge | A-lifecycle.md | unknown |
| 24 | A2 | W21 | Delete agent | A-lifecycle.md | skeleton |
| 25 | A1 | W3 | Resume (resume_agent_request, ensureAgentLoaded) | A-lifecycle.md | unknown |
| 26 | A1 | W3 | Refresh/reload agent (refresh_agent_request, stale session reload) | A-lifecycle.md | unknown |
| 27 | A1 | W3 | Idle runtime cleanup ("park"), no explicit wake | A-lifecycle.md | unknown |
| 28 | A1 | W3 (app) / N5 (node) | Restart recovery (resumeOnStartup, resumePrompt) and shutdown capture | A-lifecycle.md | node: built; app: unknown |
| 29 | A4 | W2 | Import from provider history: list (fetch_recent_provider_sessions) | A-lifecycle.md | built (import base strict passed; native Codex history + admission upgrade checks pending) |
| 30 | A4 | W2 | Import from provider history: import_agent_request and archived re-activation | A-lifecycle.md | built (resume/import + account map; final tests/review pending) |
| 31 | A4 | W2 | Plugin session history | A-lifecycle.md | missing (canonical intake reserved; plugin provider registration pending) |
| 32 | A4 | W2 | Import session sheet (app) | A-lifecycle.md | built (native sheet strict Clippy passed; final tests/captures pending) |
| 33 | A4 | W2 | Fork (fork_context request, curated chat history attachment) | A-lifecycle.md | skeleton (bounded canonical ChatHistory producer; draft action pending) |
| 34 | A4 | W2 | Fork app flow (useForkAgent, draft seeding, cwd remap) | A-lifecycle.md | skeleton (W3 seed/W12 existing-workspace target pending) |
| 35 | A4 | W2 | Rewind (agent.rewind.request, capabilities, app menu) | A-lifecycle.md | built (parity audit: native conversation rewind exists; files/both remain W9 checkpoint gap) |
| 36 | A4 | W2 | Export (not found) and handoff (skill/background handoff only) | A-lifecycle.md | built (audit: no standalone Paseo export; move B9 remains beyond; skill handoff external) |
| 37 | A4 | W2 | Detach subagent | A-lifecycle.md | built (store/auth scoped V3 passed; native/final gate pending) |
| 38 | A1 | W3 | Titles (provisional, lazy first-message, manual rename) | A-lifecycle.md | unknown |
| 39 | A6 | W10b | Labels (reserved labels, update, filter) | A-lifecycle.md | skeleton |
| 40 | A5 | W7 | Attention state, clear, mark unread | A-lifecycle.md | skeleton |
| 41 | A5 | W7 | Attention notification policy and fan-out (presence, push, in-app) | A-lifecycle.md | skeleton |
| 42 | A2 | W21 | Agent deep links (build/parse, route, desktop, CLI open) | A-lifecycle.md | skeleton |
| 43 | A1 | W3 | Agent directory fetch/subscribe (fetch_agents, agent_update) | A-lifecycle.md | unknown |
| 44 | A1 | W3 | Agent history fetch and search | A-lifecycle.md | unknown |
| 45 | A2 | W21 | Sessions screen | A-lifecycle.md | skeleton |
| 46 | A1 | W3 | Sidebar agent list entries and state buckets (app) | A-lifecycle.md | unknown |
| 47 | A9 | G3 | Provider model: AgentProvider, AgentClient, AgentSession (the contract every provider implements) | B-providers.md | unknown |
| 48 | A9 | G3 | `AgentClient` (one per provider id, long lived, `S/agent/agent-sdk-types.ts:764-830`) | B-providers.md | unknown |
| 49 | A9 | G3 | `AgentSession` (one per running agent, `:677-732`) | B-providers.md | unknown |
| 50 | A9 | G3 | `AgentCapabilityFlags` (`P/agent-types.ts` / `agent-sdk-types.ts:169`) | B-providers.md | unknown |
| 51 | A9 | G3 | Built-in provider list and mode tables (the provider manifest) | B-providers.md | unknown |
| 52 | A9 | G3 | Built-in providers (exact ids, defaults) | B-providers.md | unknown |
| 53 | A9 | G3 | Mode tables (id: label - colorTier [icon], `*` = isUnattended) | B-providers.md | unknown |
| 54 | A9 | G3 | Protocol / CLIENT_CAPS gating for older clients (port only if wire compat with old apps matters) | B-providers.md | unknown |
| 55 | A9 | G3 | Provider launch config: command, env, disallowedTools (provider-launch-config) | B-providers.md | unknown |
| 56 | A9 | G3 | Schemas (`P/provider-config.ts`, re-exported by `S/agent/provider-launch-config.ts`) | B-providers.md | unknown |
| 57 | A9 | G3 | Resolution helpers (`S/agent/provider-launch-config.ts`) | B-providers.md | unknown |
| 58 | A9 | G3 | `provider-options` (`S/agent/provider-options.ts`) | B-providers.md | unknown |
| 59 | A9 | G3 | Provider registry (`S/agent/provider-registry.ts`) | B-providers.md | unknown |
| 60 | A9 | G3 | Build algorithm (`buildProviderRegistry`, `:908`) | B-providers.md | unknown |
| 61 | A9 | G3 | Model merge rules (`mergeModels`, `mergeModelAdditions`, `:371-436`) | B-providers.md | unknown |
| 62 | A9 | G3 | Client wrapper (`wrapClientProvider`, `wrapSessionProvider`) | B-providers.md | unknown |
| 63 | A9 | G3 | Provider runtime (`S/agent/provider-runtime.ts`) | B-providers.md | unknown |
| 64 | A9 | G3 | Provider snapshot manager (`S/agent/provider-snapshot-manager.ts`) - catalog cache + change feed | B-providers.md | unknown |
| 65 | A9 | G3 | What it does | B-providers.md | unknown |
| 66 | A9 | G3 | Refresh pipeline (`loadProvider -> resolveCatalog -> refreshProvider`) | B-providers.md | unknown |
| 67 | A9 | G3 | Diagnostics (`getProviderDiagnostic`) | B-providers.md | unknown |
| 68 | A9 | G3 | Extra clients | B-providers.md | unknown |
| 69 | A9 | G3 | Create-config resolution: modes, unattended, inheritance (`S/agent/create-agent-mode.ts`) | B-providers.md | unknown |
| 70 | A9 | G3 | Provider: Claude (`providers/claude/`) | B-providers.md | unknown |
| 71 | A9 | G3 | Provider: Codex (`providers/codex-app-server-agent.ts`, `providers/codex/`) | B-providers.md | unknown |
| 72 | A9 | G3 | Provider: OpenCode (`providers/opencode-agent.ts`, `providers/opencode/`, `opencode/v2/`) | B-providers.md | unknown |
| 73 | A9 | G3 | Providers: ACP agents (`providers/acp-agent.ts`, `generic-acp-agent.ts`, cursor/copilot/kimi/kiro/trae) | B-providers.md | unknown |
| 74 | A9 | G3 | Base `ACPAgentClient` (provider id passed in; speaks Agent Client Protocol via `@agentclientprotocol/sdk`) | B-providers.md | unknown |
| 75 | A9 | G3 | Generic ACP (`GenericACPAgentClient`, provider `acp`; config `extends:"acp"` + `command`) | B-providers.md | unknown |
| 76 | A9 | G3 | Specialised ACP clients | B-providers.md | unknown |
| 77 | A9 | G3 | ACP provider catalog in the app (`A/data/acp-provider-catalog.ts`, `A/hooks/use-acp-provider-catalog.ts`) | B-providers.md | skeleton |
| 78 | A9 | G3 | Providers: Pi, Oh My Pi (OMP), mock | B-providers.md | unknown |
| 79 | A9 | G3 | Pi (`providers/pi/`) - minimal terminal coding agent | B-providers.md | unknown |
| 80 | A9 | G3 | OMP (`providers/omp/`, id `omp`, label "Oh My Pi", `enabledByDefault:false`) | B-providers.md | unknown |
| 81 | A9 | G3 | Mock providers (dev only) | B-providers.md | unknown |
| 82 | A9 | G3 | Plugin providers (`S/agent/plugin-provider.ts`, `packages/plugin/src/server/provider.ts`) | B-providers.md | unknown |
| 83 | A9 | G3 | Wire protocol between daemon and plugin (`PROVIDER_PROTOCOL_VERSION = 1`) | B-providers.md | unknown |
| 84 | A9 | G3 | Provider command/env overlay and parent-session environment removal | B-providers.md | unknown |
| 85 | A9 | G3 | Exact MCP tool preapproval translation and unsupported-contract rejection | B-providers.md | unknown |
| 86 | A9 | G3 | Replacement vs additive model catalog merge | B-providers.md | unknown |
| 87 | A9 | G3 | Catalog scope cache key, timeout/loading/error and diagnostic refresh | B-providers.md | unknown |
| 88 | A9 | G3 | Claude supportedCommands skills, question mapping and native import titles | B-providers.md | unknown |
| 89 | A9 | G3 | Codex app-server discovery, version-gated auto-review and goal commands | B-providers.md | unknown |
| 90 | A9 | G3 | Codex plan approval follow-up implementation prompt | B-providers.md | unknown |
| 91 | A9 | G3 | OpenCode runtime version selection and auto_accept feature | B-providers.md | unknown |
| 92 | A9 | G3 | ACP auto_accept and configuration-derived select features | B-providers.md | unknown |
| 93 | A9 | G3 | Copilot custom agent/allow-all modes | B-providers.md | unknown |
| 94 | A9 | G3 | Cursor fast mode and parameterized model discovery | B-providers.md | unknown |
| 95 | A9 | G3 | Kimi per-model thinking discovery | B-providers.md | unknown |
| 96 | A9 | G3 | Kiro command/skill notification discovery | B-providers.md | unknown |
| 97 | A9 | G3 | Trae available_commands_update discovery | B-providers.md | unknown |
| 98 | A9 | G3 | ACP install catalogue config patches and bundled Hoplite gate | B-providers.md | unknown |
| 99 | A9 | G3 | Pi model/thinking/commands and OMP rpc-ui questions | B-providers.md | unknown |
| 100 | A9 | G3 | Plugin capability negotiation and provider subsession adoption | B-providers.md | unknown |
| 101 | A7 | C2 | Timeline item types (9): user_message, assistant_message, reasoning, tool_call, todo, error, notification, compaction, plugin | C-timeline.md | skeleton |
| 102 | A7 | C2 | Tool-call statuses and common fields (running/completed/failed/canceled; callId, name, detail, metadata, agentMessage) | C-timeline.md | skeleton |
| 103 | A7 | C2 | ToolCallDetail kinds (11): shell, read, edit, write, search, fetch, worktree_setup, sub_agent, plain_text, plan, unknown | C-timeline.md | skeleton |
| 104 | A7 | C2 | AgentStreamEvent internal vs wire union (which events never reach the wire) | C-timeline.md | skeleton |
| 105 | A7 | C2 | Wire envelopes: agent_stream, fetch_agent_timeline_response entries, collapsed kinds | C-timeline.md | skeleton |
| 106 | A7 | C2 | Server in-memory timeline store: epoch, seq, window, limits | C-timeline.md | skeleton |
| 107 | A7 | C2 | Durable timeline store interface (unwired) | C-timeline.md | skeleton |
| 108 | A7 | C2 | Item content size limiting (budgets, truncation marker) | C-timeline.md | skeleton |
| 109 | A7 | C2 | Turn id assignment on rows | C-timeline.md | skeleton |
| 110 | A7 | C2 | Timeline projection merges (assistant, reasoning, tool lifecycle, plugin identity) | C-timeline.md | skeleton |
| 111 | A7 | C2 | Projection paging (tail/before/after, wholeTurns, spans, sourceSeqRanges) | C-timeline.md | skeleton |
| 112 | A7 | C2 | Stream coalescer (60 ms leading+trailing) | C-timeline.md | skeleton |
| 113 | A7 | C2 | AgentManager stream dispatch and error-as-assistant-message ("[System Error]") | C-timeline.md | skeleton |
| 114 | A7 | C2 | Session timeline subscriptions (set_subscription request/response, owned subscriptions, capability filtering) | C-timeline.md | skeleton |
| 115 | A7 | C2 | Timeline replacement (rewind) and legacy reconstructed replay | C-timeline.md | skeleton |
| 116 | A7 | C2 | fetch_agent_timeline request/response (cursor, direction, limit, reset/stale/gap, mergeWindow, wholeTurns) | C-timeline.md | skeleton |
| 117 | A3 | W1 | Timeline search (agent.timeline.search.*) | C-timeline.md | skeleton |
| 118 | A3 | W1 | Prompt index (agent.timeline.list_prompts.*) | C-timeline.md | skeleton |
| 119 | A7 | C2 | Plugin timeline append (agent.timeline.append.*) | C-timeline.md | skeleton |
| 120 | A7 | C2 | Fork context from timeline (agent.fork_context.*, curator) | C-timeline.md | skeleton |
| 121 | A7 | C2 | Agent message envelopes <paseo-system> and projection to tool_call rows (agent_message, heartbeat) | C-timeline.md | skeleton |
| 122 | A7 | C2 | Tool-name normalization and Paseo tool name detection | C-timeline.md | skeleton |
| 123 | A7 | C2 | Activity curator text rendering | C-timeline.md | skeleton |
| 124 | A7 | C2 | Tool-call display model (names, summaries, error text) | C-timeline.md | skeleton |
| 125 | A7 | C2 | Paseo tool presentation catalog (agents/workspaces/terminals/schedules/providers/browser) and detail sections | C-timeline.md | skeleton |
| 126 | A7 | C2 | Tool call presentation in app (icons, meaningful detail, open-file path extraction, plan outcome) | C-timeline.md | skeleton |
| 127 | A7 | C2 | App StreamItem model (8 kinds) and id rules | C-timeline.md | skeleton |
| 128 | A7 | C2 | Reducer rules (text chunks, tool merge, todo extraction, compaction, plugin) | C-timeline.md | skeleton |
| 129 | A7 | C2 | Head/tail streaming model and user message reconciliation | C-timeline.md | skeleton |
| 130 | A7 | C2 | Session stream reducer queue (48 ms/rAF), sequencing gate (accept/stale/epoch/gap), response processing policies | C-timeline.md | skeleton |
| 131 | A7 | C2 | Viewed-timeline sync (subscribe demand, catch-up state machine, resume, retry, status) | C-timeline.md | skeleton |
| 132 | A7 | C2 | Timeline sync plans and TIMELINE_FETCH_PAGE_SIZE=40, prompt-jump window | C-timeline.md | skeleton |
| 133 | A7 | C2 | Replica cache and painted/synced states | C-timeline.md | skeleton |
| 134 | A7 | C2 | Older-history loading | C-timeline.md | skeleton |
| 135 | A7 | C2 | Turn liveness state machine and turn timing | C-timeline.md | skeleton |
| 136 | A7 | C2 | Sync status toast | C-timeline.md | skeleton |
| 137 | A7 | C2 | Stream presentation pipeline (plugin transform, agent-message rows, spoken-input, detail level, hidden plan rows) | C-timeline.md | skeleton |
| 138 | A7 | C2 | Tool-call grouping and overview summaries | C-timeline.md | built |
| 139 | A7 | C2 | Turn collapse (collapseTurns) and expanded rail | C-timeline.md | built |
| 140 | A7 | C2 | AgentStreamView (props, handle, row switch, empty state, scroll-to-bottom) | C-timeline.md | skeleton |
| 141 | A7 | C2 | Stream layout (gaps, tool sequences, footers) | C-timeline.md | skeleton |
| 142 | A7 | C2 | Render model, web partial virtualization, mounted window | C-timeline.md | skeleton |
| 143 | A7 | C2 | Stream history window (local reveal then remote) | C-timeline.md | skeleton |
| 144 | A7 | C2 | Platform strategies (web vs native inverted list) and constants | C-timeline.md | skeleton |
| 145 | A7 | C2 | Bottom-anchor controller | C-timeline.md | skeleton |
| 146 | A7 | C2 | History-start pagination state machine | C-timeline.md | skeleton |
| 147 | A7 | C2 | Prompt-jump settle and scroll-to-message | C-timeline.md | skeleton |
| 148 | A3 | W1 | Chat find | C-timeline.md | skeleton |
| 149 | A3 | W1 | Chat outline rail | C-timeline.md | skeleton |
| 150 | A7 | C2 | User message renderer (images, attachments, rewind, preview, sender) | C-timeline.md | skeleton |
| 151 | A7 | C2 | Assistant message renderer (markdown, 32 000-char cap, byte notice) | C-timeline.md | skeleton |
| 152 | A7 | C2 | Text reveal pacing | C-timeline.md | skeleton |
| 153 | A7 | C2 | Reasoning (thought) row | C-timeline.md | built |
| 154 | A7 | C2 | Tool call row and details sheet/inline, tool group view | C-timeline.md | built |
| 155 | A7 | C2 | Detail bodies per kind (shell, edit diff, write, read, search, fetch, plain_text, unknown/Paseo, none for sub_agent/worktree_setup) | C-timeline.md | skeleton |
| 156 | A7 | C2 | Plan card | C-timeline.md | built |
| 157 | A7 | C2 | Todo card | C-timeline.md | skeleton |
| 158 | A7 | C2 | Notification/error row | C-timeline.md | skeleton |
| 159 | A7 | C2 | Compaction marker | C-timeline.md | skeleton |
| 160 | A7 | C2 | Speak message | C-timeline.md | skeleton |
| 161 | A7 | C2 | Turn footer (working, copy, fork, duration/timestamp) | C-timeline.md | skeleton |
| 162 | A7 | C2 | Usage/context window display (pointer only) | C-timeline.md | skeleton |
| 163 | A7 | C2 | Agent presentation descriptor | C-timeline.md | skeleton |
| 164 | A7 | C2 | Capability and feature gates | C-timeline.md | skeleton |
| 165 | B2 | W12 | Composer: send path and state machine (app) | D-messaging.md | skeleton |
| 166 | B2 | W12 | What the user sees | D-messaging.md | skeleton |
| 167 | B2 | W12 | Submit pipeline (end to end) | D-messaging.md | skeleton |
| 168 | B2 | W12 | Send settings and shortcuts | D-messaging.md | skeleton |
| 169 | B2 | W12 | Dictation transcript into composer | D-messaging.md | unknown |
| 170 | B2 | W12 | States / limits | D-messaging.md | skeleton |
| 171 | B2 | W12 | Composer drafts and draft store (app) | D-messaging.md | skeleton |
| 172 | B2 | W12 | Send prompt: `send_agent_message_request` (server) | D-messaging.md | unknown |
| 173 | B2 | W12 | Wire | D-messaging.md | skeleton |
| 174 | B2 | W12 | Handler `handleSendAgentMessageRequest` (`S/session.ts:8304`) | D-messaging.md | unknown |
| 175 | B2 | W12 | Client | D-messaging.md | skeleton |
| 176 | B2 | W12 | Other entry points that reuse the same pipeline (`sendPromptToAgent`) | D-messaging.md | unknown |
| 177 | B2 | W12 | Client message id and optimistic submission lifecycle | D-messaging.md | unknown |
| 178 | B2 | W12 | Identity | D-messaging.md | unknown |
| 179 | B2 | W12 | When the server records the prompt row | D-messaging.md | unknown |
| 180 | B2 | W12 | App-side state machine (`A/composer/submission/model.ts`) | D-messaging.md | skeleton |
| 181 | B2 | W12 | Message receipts (server idempotency) | D-messaging.md | unknown |
| 182 | B2 | W12 | Active-turn behavior (steer / interrupt) | D-messaging.md | skeleton |
| 183 | B2 | W12 | Queued messages (two different queues) | D-messaging.md | unknown |
| 184 | B2 | W12 | App-side composer queue ("send after the agent finishes") | D-messaging.md | unknown |
| 185 | B2 | W12 | Daemon-side agent message delivery queue (agent-to-agent) | D-messaging.md | unknown |
| 186 | B2 | W12 | Interrupt / cancel | D-messaging.md | skeleton |
| 187 | B2 | W12 | Prompt attachments | D-messaging.md | unknown |
| 188 | B2 | W12 | Wire types (`P/messages.ts:1108-1230`) | D-messaging.md | unknown |
| 189 | B2 | W12 | App-side composer attachments (`A/attachments/types.ts`, `ComposerAttachment`) | D-messaging.md | unknown |
| 190 | B2 | W12 | agent-prompt module (`S/agent/agent-prompt.ts`) | D-messaging.md | unknown |
| 191 | B2 | W12 | agent-response-loop (`S/agent/agent-response-loop.ts`) - structured responses, not a chat loop | D-messaging.md | unknown |
| 192 | B2 | W12 | Composer optimistic preparing/sending/accepted/rejected message transitions | D-messaging.md | skeleton |
| 193 | B2 | W12 | Creation handoff preserves client message identity | D-messaging.md | unknown |
| 194 | B2 | W12 | Default send shortcut and active-run send behavior preferences | D-messaging.md | skeleton |
| 195 | B2 | W12 | Workspace-scoped draft attachments and local image re-encoding | D-messaging.md | unknown |
| 196 | B2 | W12 | Server accepted/rejected submission rows and provider acknowledgement | D-messaging.md | unknown |
| 197 | B2 | W12 | Per-recipient inter-agent quiet-window delivery | D-messaging.md | unknown |
| 198 | B2 | W12 | Prompt source envelopes and sender-title resolution | D-messaging.md | unknown |
| 199 | B2 | W12 | Uploaded files, workspace ranges and browser selection screenshot prompts | D-messaging.md | unknown |
| 200 | B2 | W12 | Forge auto-attachment and legacy forge wire fallback | D-messaging.md | unknown |
| 201 | B2 | W12 | Review and fork-history text attachments | D-messaging.md | unknown |
| 202 | B2 | W12 | Structured JSON extraction, schema correction retries and provider fallback | D-messaging.md | unknown |
| 203 | B2 | W12 | Internal generation-agent cleanup after success/failure | D-messaging.md | unknown |
| 204 | B2 | W12 | Composer shell: modes (chat/terminal), readOnly, preserve-and-lock, placeholder, auto-height, fullscreen editor, dock, editing session, focus rules | area-2-composer-input.md | skeleton |
| 205 | B2 | W12 | Draft persistence: draft store, keys, lifecycle, throttled persist, after-paint staging, hydration, TTL, draft-agent create flow | area-2-composer-input.md | skeleton |
| 206 | B2 | W12 | Send path: Enter / Mod+Enter / long-press, default vs alternate action, sendBehavior + sendShortcut settings, interrupt vs steer, optimistic submissio | area-2-composer-input.md | skeleton |
| 207 | B2 | W12 | Stop/interrupt button, Escape, cancel RPC and cancelling state | area-2-composer-input.md | skeleton |
| 208 | B2 | W12 | Message queue: queue, edit queued, send-now, auto-drain as steer, pending-permission override | area-2-composer-input.md | unknown |
| 209 | B2 | W12 | History recall (Up/Down) | area-2-composer-input.md | skeleton |
| 210 | B2 | W12 | Client slash commands (/exit, /clear + aliases) | area-2-composer-input.md | skeleton |
| 211 | B2 | W12 | Plugin client slash commands | area-2-composer-input.md | skeleton |
| 212 | B2 | W12 | Provider (agent) slash commands and skills, inline skill commands, draft command listing | area-2-composer-input.md | skeleton |
| 213 | B1 | W6 | `@` file/dir mentions (directory suggestions) | area-2-composer-input.md | skeleton |
| 214 | B1 | W6 | Autocomplete popover mechanics and states | area-2-composer-input.md | skeleton |
| 215 | B3 | dictation+W12 | Dictation: mic button, overlay, shortcuts, capture (web/native), streaming sender with replay, server stream manager, auto-commit/silence, timeouts, r | area-2-composer-input.md | unknown |
| 216 | B3 | dictation+W12 | Speech providers/settings/models: local (sherpa) and OpenAI, readiness reasons, env vars, model library download/delete, settings page and provider sh | area-2-composer-input.md | unknown |
| 217 | B4 | W19 | Realtime voice mode: button, overlay, mute/stop, runtime phases, capture upload, grouped TTS playback and acks, barge-in, thinking cue, server VoiceSe | area-2-composer-input.md | skeleton (staged node/audio/overlay; activation and final evidence pending) |
| 218 | B4 | W19 | Audio engines (web/native), playback queue, `expo-two-way-audio` module (iOS/Android), plugin audio playback helper | area-2-composer-input.md | skeleton (shared capture source; duplex/platform/plugin audio unfinished) |
| 219 | B1 | W6 | Attachment types, wire conversion, attach menu | area-2-composer-input.md | skeleton |
| 220 | B1 | W6 | Image attach: picker (desktop/web/native), paste (web/native), clipboard "Paste image", drop | area-2-composer-input.md | skeleton |
| 221 | B1 | W6 | Generic file upload: picker, drop, 50 MB limit, pending pills, chunked binary upload, server FileUploadStore (sanitization, stale timeout) | area-2-composer-input.md | skeleton |
| 222 | B1 | W6 | Local attachment byte stores (IndexedDB / desktop files / native files), encoding for send, GC and retention | area-2-composer-input.md | skeleton |
| 223 | B1 | W6 | Attachment pills, tray, lightbox, previews, user-message attachment rendering | area-2-composer-input.md | skeleton |
| 224 | B1 | W6 | Workspace attachments (browser element, review, chat history, PR context) and suppression rules | area-2-composer-input.md | skeleton |
| 225 | B1 | W6 | Workspace-file attachments: add to chat action, drag source/drop, line_range type (no creator found) | area-2-composer-input.md | skeleton |
| 226 | B1 | W6 | Forge issue/PR picker and URL auto-attach | area-2-composer-input.md | unknown |
| 227 | B1 | W6 | Plugin attachment sources | area-2-composer-input.md | unknown |
| 228 | B2 | W12 | Assistant selection copy (markdown + code-aware, web only) | area-2-composer-input.md | skeleton |
| 229 | B2 | W12 | Assistant file links (parse, resolve, lookup, open) | area-2-composer-input.md | skeleton |
| 230 | B2 | W12 | Assistant images (file RPC acquisition, cache, retention, lightbox) | area-2-composer-input.md | skeleton |
| 231 | B2 | W12 | File download token endpoint (used by explorer; documented as a connection) | area-2-composer-input.md | unknown |
| 232 | B2 | W12 | Find in pane widget and Cmd/Ctrl+F policy | area-2-composer-input.md | unknown |
| 233 | B2 | W12 | Keyboard shortcuts for composer actions | area-2-composer-input.md | skeleton |
| 234 | C1 | W11 | 01 Add Project flow: open/close modal request. | p1-projects-workspaces.md | skeleton |
| 235 | C1 | W11 | 01 Host selection and preferred/single-host skip. | p1-projects-workspaces.md | unknown |
| 236 | C1 | W11 | 01 Method selection and per-host capability/update gates. | p1-projects-workspaces.md | unknown |
| 237 | C1 | W11 | 01 Back stack, remembered in-flow input drafts, keyboard selection and loading/error/empty states. | p1-projects-workspaces.md | unknown |
| 238 | C1 | W11 | 01 Electron local Browse. | p1-projects-workspaces.md | unknown |
| 239 | C1 | W11 | 02 Register existing directory without workspace creation. | p1-projects-workspaces.md | skeleton |
| 240 | C1 | W11 | 02 Exact-root project deduplication and snapshot/navigation update. | p1-projects-workspaces.md | skeleton |
| 241 | C1 | W11 | 02 Compatibility Open Project: find/create workspace by directory. | p1-projects-workspaces.md | unknown |
| 242 | C1 | W11 | 03 Create empty directory under selected existing parent. | p1-projects-workspaces.md | skeleton |
| 243 | C1 | W11 | 03 Name/path validation and registration rollback. | p1-projects-workspaces.md | skeleton |
| 244 | C1 | W11 | 03 Git-init action in Add Project: not found. | p1-projects-workspaces.md | unknown |
| 245 | C1 | W11 | 04 GitHub account repository search and authentication/unavailable states. | p1-projects-workspaces.md | unknown |
| 246 | C1 | W11 | 04 Manual full remote / owner-repo HTTPS or SSH choices. | p1-projects-workspaces.md | skeleton |
| 247 | C1 | W11 | 04 Clone parent suggestions, remembered parent, existing target disable. | p1-projects-workspaces.md | unknown |
| 248 | C1 | W11 | 04 Staged clone, timeout, cleanup and register-project transition. | p1-projects-workspaces.md | skeleton |
| 249 | C1 | W11 | 05 Onboarding history-derived suggested projects per enabled provider. | p1-projects-workspaces.md | unknown |
| 250 | C1 | W11 | 05 Root grouping, exclusions, ranking and limits. | p1-projects-workspaces.md | unknown |
| 251 | C1 | W11 | 05 Default preselection and user override. | p1-projects-workspaces.md | unknown |
| 252 | C1 | W11 | 05 Sequential add, partial failure and late-provider arrivals. | p1-projects-workspaces.md | unknown |
| 253 | C1 | W11 | 06 Project custom name/reset. | p1-projects-workspaces.md | unknown |
| 254 | C1 | W11 | 06 Choose image, URL/website icon import, preview and automatic icon reset. | p1-projects-workspaces.md | unknown |
| 255 | C1 | W11 | 06 Server icon validation and host-local icon persistence. | p1-projects-workspaces.md | unknown |
| 256 | C1 | W11 | 06 Icon discovery/revision/cache and placeholder display. | p1-projects-workspaces.md | unknown |
| 257 | C1 | W11 | 07 Project removal capability/client preflight. | p1-projects-workspaces.md | skeleton |
| 258 | C1 | W11 | 07 Archive project contents while preserving directories. | p1-projects-workspaces.md | unknown |
| 259 | C1 | W11 | 07 Multi-host partial removal result and empty-project row removal. | p1-projects-workspaces.md | unknown |
| 260 | C1 | W11 | 08 Host-targeted project settings load/empty/invalid/missing/offline states. | p1-projects-workspaces.md | skeleton |
| 261 | C1 | W11 | 08 Revision-aware config save and stale-write reload. | p1-projects-workspaces.md | skeleton |
| 262 | C1 | W11 | 08 Unknown-field preservation and uncommitted setup warning. | p1-projects-workspaces.md | skeleton |
| 263 | C1 | W11 | 09 Worktree setup and teardown command editing. | p1-projects-workspaces.md | unknown |
| 264 | C1 | W11 | 09 Named script add/edit/remove/type/port fields. | p1-projects-workspaces.md | unknown |
| 265 | C1 | W11 | 09 Metadata instruction overrides. | p1-projects-workspaces.md | unknown |
| 266 | C1 | W11 | 09 Missing-setup callout and per-project dismissal. | p1-projects-workspaces.md | unknown |
| 267 | A1 | W3 | 10 New Workspace project and host selection/remembered context. | p1-projects-workspaces.md | unknown |
| 268 | A1 | W3 | 10 Local versus worktree isolation preference/capability. | p1-projects-workspaces.md | unknown |
| 269 | A1 | W3 | 10 Branch/change-request picker and source mapping. | p1-projects-workspaces.md | unknown |
| 270 | A1 | W3 | 10 Pending/error/retry state. | p1-projects-workspaces.md | unknown |
| 271 | A1 | W3 | 11 Import-session entry and fork-context handoff. | p1-projects-workspaces.md | unknown |
| 272 | A1 | W3 | 11 Empty workspace launch. | p1-projects-workspaces.md | unknown |
| 273 | A1 | W3 | 11 Combined workspace plus chat-agent launch and early handoff. | p1-projects-workspaces.md | unknown |
| 274 | A1 | W3 | 11 Blank shell launch and typed command input. | p1-projects-workspaces.md | unknown |
| 275 | A1 | W3 | 11 Terminal profile prompt substitution/read-only command launch. | p1-projects-workspaces.md | unknown |
| 276 | A1 | W3 | 11 Navigation-safe completion after leaving create screen. | p1-projects-workspaces.md | unknown |
| 277 | A1 | W3 | 12 Workspace create schema, durable receipts/id claims, idempotent operation phases and reconnect. | p1-projects-workspaces.md | unknown |
| 278 | A1 | W3 | 12 Fresh record creation for shared cwd. | p1-projects-workspaces.md | unknown |
| 279 | A1 | W3 | 12 Initial-agent relative cwd mapping. | p1-projects-workspaces.md | unknown |
| 280 | A1 | W3 | 12 Background visibility/inheritance and removed internal-workspace rejection. | p1-projects-workspaces.md | unknown |
| 281 | A1 | W3 | 12 Plugin create hook and durable placement fields. | p1-projects-workspaces.md | unknown |
| 282 | A1 | W3 | 13 Branch-off worktree mode. | p1-projects-workspaces.md | unknown |
| 283 | A1 | W3 | 13 Checkout existing branch mode. | p1-projects-workspaces.md | unknown |
| 284 | A1 | W3 | 13 Checkout change request mode, forge mismatch/cross-repo source. | p1-projects-workspaces.md | unknown |
| 285 | A1 | W3 | 13 Managed root/hash/slug and exact subdirectory mapping. | p1-projects-workspaces.md | unknown |
| 286 | A1 | W3 | 13 Creation hooks/config seeding, ownership metadata and rollback. | p1-projects-workspaces.md | unknown |
| 287 | A1 | W3 | 14 Asynchronous sequential setup and auto-terminals. | p1-projects-workspaces.md | unknown |
| 288 | A1 | W3 | 14 Runtime environment/port injection. | p1-projects-workspaces.md | unknown |
| 289 | A1 | W3 | 14 Setup progress snapshots, command logs and truncation. | p1-projects-workspaces.md | unknown |
| 290 | A1 | W3 | 14 Setup subscription, waiting/no-command states, expandable logs/durations and failed/blocked setup-tab surfacing. | p1-projects-workspaces.md | unknown |
| 291 | A1 | W3 | 14 Explicit automation trust release; generic failed-setup rerun not supplied by this RPC. | p1-projects-workspaces.md | unknown |
| 292 | A1 | W3 | 14 Archive cancellation of setup. | p1-projects-workspaces.md | unknown |
| 293 | A2 | W21 | 15 Optimistic workspace archive, rollback and route cleanup. | p1-projects-workspaces.md | unknown |
| 294 | A2 | W21 | 15 Bulk-workspace archive helpers and partial errors (production UI caller not found). | p1-projects-workspaces.md | unknown |
| 295 | A2 | W21 | 15 Workspace ownership archive versus worktree-scope archive. | p1-projects-workspaces.md | unknown |
| 296 | A2 | W21 | 15 Teardown and last-reference owned directory deletion. | p1-projects-workspaces.md | unknown |
| 297 | A2 | W21 | 15 Unowned/failed-removal directory preservation and empty-project persistence. | p1-projects-workspaces.md | unknown |
| 298 | A2 | W21 | 16 Host automatic-archive-after-merge toggle. | p1-projects-workspaces.md | unknown |
| 299 | A2 | W21 | 16 Open-to-merged same-URL latch, fresh snapshot and per-cwd dedupe. | p1-projects-workspaces.md | unknown |
| 300 | A2 | W21 | 16 Dirty/ahead/ownership checks, consumed merged URL and optional-hook boundary versus manual archive. | p1-projects-workspaces.md | unknown |
| 301 | A2 | W21 | 17 Inspect archived workspace recovery. | p1-projects-workspaces.md | unknown |
| 302 | A2 | W21 | 17 Unarchive existing directory. | p1-projects-workspaces.md | unknown |
| 303 | A2 | W21 | 17 Restore deleted worktree and original subdirectory. | p1-projects-workspaces.md | unknown |
| 304 | A2 | W21 | 17 Missing-source/unavailable/unsupported/inspection-error states and retry. | p1-projects-workspaces.md | unknown |
| 305 | A6 | W10b | 18 Workspace label picker/search/check/uncheck. | p1-projects-workspaces.md | skeleton |
| 306 | A6 | W10b | 18 Create-and-assign label with host catalog color. | p1-projects-workspaces.md | skeleton |
| 307 | A6 | W10b | 18 Case-insensitive name normalization and assignment persistence. | p1-projects-workspaces.md | skeleton |
| 308 | A6 | W10b | 19 Label manager search and host selection. | p1-projects-workspaces.md | skeleton |
| 309 | A6 | W10b | 19 Atomic label rename/recolor and collision rejection. | p1-projects-workspaces.md | skeleton |
| 310 | A6 | W10b | 19 Delete affected-count inspection, confirmation and assignment removal. | p1-projects-workspaces.md | skeleton |
| 311 | A6 | W10b | 19 Catalog snapshot/delta synchronization and offline host projection. | p1-projects-workspaces.md | skeleton |
| 312 | F4 | S2+W22 | 20 JSON project/workspace registry load and serialized mutation. | p1-projects-workspaces.md | unknown |
| 313 | F4 | S2+W22 | 20 Workspace/project list filters, sort, pagination and empty-project rows. | p1-projects-workspaces.md | unknown |
| 314 | F4 | S2+W22 | 20 Owned subscriptions, bootstrap delta queue, generation/sequence resume. | p1-projects-workspaces.md | unknown |
| 315 | F4 | S2+W22 | 20 App cached snapshot/hydration and stale-response/archive suppression. | p1-projects-workspaces.md | unknown |
| 316 | F4 | S2+W22 | 21 Project root observation and bounded periodic reconciliation. | p1-projects-workspaces.md | unknown |
| 317 | F4 | S2+W22 | 21 Missing-directory safe archive and unreachable-root preservation. | p1-projects-workspaces.md | unknown |
| 318 | F4 | S2+W22 | 21 Git metadata changes with immutable identity/membership. | p1-projects-workspaces.md | unknown |
| 319 | F4 | S2+W22 | 22 Aggregated workspace status and transition/activity timestamps. | p1-projects-workspaces.md | unknown |
| 320 | F4 | S2+W22 | 22 Pending initial-agent running overlay. | p1-projects-workspaces.md | unknown |
| 321 | F4 | S2+W22 | 22 Mark read except permission attention. | p1-projects-workspaces.md | unknown |
| 322 | F4 | S2+W22 | 22 Mark unread via eligible finished root agent and pane unfocus. | p1-projects-workspaces.md | unknown |
| 323 | F4 | S2+W22 | 23 Workspace title override; SDK/CLI reset, UI empty-name restriction. | p1-projects-workspaces.md | unknown |
| 324 | F4 | S2+W22 | 23 Shared workspace pin/unpin and duplicate-toggle guard. | p1-projects-workspaces.md | unknown |
| 325 | F4 | S2+W22 | 24 Cross-host project grouping and placement rows. | p1-projects-workspaces.md | unknown |
| 326 | F4 | S2+W22 | 24 Collapse/expand and empty-project New Workspace row. | p1-projects-workspaces.md | unknown |
| 327 | F4 | S2+W22 | 24 Project settings/window/folder/remove actions. | p1-projects-workspaces.md | unknown |
| 328 | F4 | S2+W22 | 24 Workspace copy/rename/read/pin/label/folder/archive actions. | p1-projects-workspaces.md | unknown |
| 329 | F4 | S2+W22 | 24 Branch/host/PR/check/service/label metadata row. | p1-projects-workspaces.md | unknown |
| 330 | F4 | S2+W22 | 24 Manual/created/last-prompt order, filters and loading states. | p1-projects-workspaces.md | unknown |
| 331 | D2 | W8 | 25 Workspace script/service list and health tones. | p1-projects-workspaces.md | skeleton |
| 332 | D2 | W8 | 25 Start and script terminal reveal. | p1-projects-workspaces.md | skeleton |
| 333 | D2 | W8 | 25 Stop and state-observed restart. | p1-projects-workspaces.md | skeleton |
| 334 | D2 | W8 | 25 SDK/CLI named script list/start/stop status contract. | p1-projects-workspaces.md | unknown |
| 335 | D2 | W8 | 25 Service URL route preference/copy and sidebar health. | p1-projects-workspaces.md | skeleton |
| 336 | C2 | W15 | 26 New-tab launcher built-ins, plugin panels and terminal profiles. | p1-projects-workspaces.md | unknown |
| 337 | C2 | W15 | 26 Tab target identity and workspace-local persistence key. | p1-projects-workspaces.md | skeleton |
| 338 | C2 | W15 | 26 Agent/terminal hydration-safe reconciliation and subagent ownership. | p1-projects-workspaces.md | skeleton |
| 339 | C2 | W15 | 27 Context/tab menu copy/resume/id/path/open-with actions. | p1-projects-workspaces.md | unknown |
| 340 | C2 | W15 | 27 Agent/terminal tab rename and agent reload. | p1-projects-workspaces.md | unknown |
| 341 | C2 | W15 | 27 Root-agent archive-on-close versus child-agent layout-only close. | p1-projects-workspaces.md | skeleton |
| 342 | C2 | W15 | 27 Terminal kill confirmation and passive/dirty-panel close. | p1-projects-workspaces.md | skeleton |
| 343 | C2 | W15 | 27 Close left/right/above/below/others and batched lifecycle handling. | p1-projects-workspaces.md | skeleton |
| 344 | C2 | W15 | 28 CLI project create/list/rename/reset/delete. | p1-projects-workspaces.md | unknown |
| 345 | C2 | W15 | 28 CLI workspace create modes/title/visibility, list, rename/reset, setup and archive. | p1-projects-workspaces.md | unknown |
| 346 | C2 | W15 | 28 CLI worktree list/create and whole-worktree archive. | p1-projects-workspaces.md | unknown |
| 347 | C2 | W15 | 28 CLI validation/host targeting and missing restore/icon CLI surfaces. | p1-projects-workspaces.md | unknown |
| 348 | C2 | W15 | 29 Pane split/focus/move/reorder/close and depth/size limits. | p1-projects-workspaces.md | skeleton |
| 349 | C2 | W15 | 29 Preferred main/side opens and modified-preview protection. | p1-projects-workspaces.md | skeleton |
| 350 | C2 | W15 | 29 Explorer Changes/PR placement and one-time background PR tab. | p1-projects-workspaces.md | unknown |
| 351 | C2 | W15 | 29 Compact sheets, promotion, parent relationship and dirty-state retention. | p1-projects-workspaces.md | skeleton |
| 352 | C2 | W15 | 29 Validated layout persistence/migrations. | p1-projects-workspaces.md | skeleton |
| 353 | C2 | W15 | 30 Route/active workspace selection and warm-deck retention. | p1-projects-workspaces.md | unknown |
| 354 | C2 | W15 | 30 Workspace/agent/terminal/browser/files/changes keyboard entry points. | p1-projects-workspaces.md | skeleton |
| 355 | C2 | W15 | 30 Platform-specific workspace/tab jump/cycle/close shortcuts. | p1-projects-workspaces.md | skeleton |
| 356 | C2 | W15 | 30 Pane keyboard operations, focus gates and user overrides. | p1-projects-workspaces.md | skeleton |
| 357 | C1 | W11 | 31 Desktop editor/file-manager bridge and local-placement gate. | p1-projects-workspaces.md | unknown |
| 358 | C1 | W11 | 31 Active file/line or selected-directory open target. | p1-projects-workspaces.md | unknown |
| 359 | C1 | W11 | 31 Forge web links and project Open in new window. | p1-projects-workspaces.md | unknown |
| 360 | N(titles) | titles | 32 First-context provisional/generated workspace title. | p1-projects-workspaces.md | skeleton (session-title implementation built; workspace target pending) |
| 361 | N(titles) | titles | 32 One-shot branch-off placeholder rename and collision suffixes. | p1-projects-workspaces.md | missing (branch slice not started) |
| 362 | N(titles) | titles | 32 Intervening user rename protection and metadata failure behavior. | p1-projects-workspaces.md | built (session actor guard; workspace target pending) |
| 363 | C1 | W11 | 33 Host Settings Projects list, host-specific rows and empty/loading/error behavior. | p1-projects-workspaces.md | skeleton |
| 364 | C1 | W11 | 33 No-editable-target settings route and Back to Projects. | p1-projects-workspaces.md | unknown |
| 365 | C1 | W11 | 33 Open Project home Add/Import/Providers/Pair device links and host errors. | p1-projects-workspaces.md | unknown |
| 366 | C3 | G1 | Protocol: checkout_status_request/response/checkout_status_update | p2-git.md | skeleton |
| 367 | C3 | G1 | Protocol: checkout.diff.get.request/response (one-shot diff) | p2-git.md | skeleton |
| 368 | C3 | G1 | Protocol: subscribe_checkout_diff_request/response, checkout_diff_update, unsubscribe_checkout_diff_request | p2-git.md | unknown |
| 369 | C3 | G1 | Protocol: checkout.refresh.request/response | p2-git.md | unknown |
| 370 | C3 | G1 | Protocol: checkout_commit_request/response | p2-git.md | unknown |
| 371 | C3 | G1 | Protocol: checkout_merge_request/response (merge into base, merge/squash) | p2-git.md | unknown |
| 372 | C3 | G1 | Protocol: checkout_merge_from_base_request/response | p2-git.md | unknown |
| 373 | C3 | G1 | Protocol: checkout_pull_request/checkout_pull_response | p2-git.md | unknown |
| 374 | C3 | G1 | Protocol: checkout_push_request/response | p2-git.md | unknown |
| 375 | C3 | G1 | Protocol: checkout.discard_changes.request/response | p2-git.md | unknown |
| 376 | C3 | G1 | Protocol: checkout.commits.list.request/response, checkout.commits.file_diff.request/response | p2-git.md | skeleton |
| 377 | C3 | G1 | Protocol: checkout_switch_branch_request/response, checkout.rename_branch.request/response | p2-git.md | unknown |
| 378 | C3 | G1 | Protocol: validate_branch_request/response, branch_suggestions_request/response | p2-git.md | unknown |
| 379 | C3 | G1 | Protocol: stash_save/pop/list request/response | p2-git.md | unknown |
| 380 | C3 | G1 | Protocol: authorization scopes per message; CheckoutError codes; ParsedDiffFile/DiffHunk/DiffLine schemas; CheckoutDiffCompare; ProjectCheckoutLite; W | p2-git.md | skeleton |
| 381 | C3 | G1 | Server: CheckoutSession read side (status, branch validate, suggestions, diff read/subscribe, refresh) | p2-git.md | unknown |
| 382 | C3 | G1 | Server: CheckoutSession write side (commit, discard, switch/rename branch, merge, merge from base, pull, push, stash) | p2-git.md | unknown |
| 383 | C3 | G1 | Server: commit message generation (git-metadata-generator.ts) | p2-git.md | unknown |
| 384 | C3 | G1 | Server: GitMutationService (notifyGitMutation, checkoutExistingBranch, createBranchFromBase) | p2-git.md | unknown |
| 385 | C3 | G1 | Server: status-projection.ts (snapshot -> checkout status payload) | p2-git.md | unknown |
| 386 | C3 | G1 | Server: CheckoutDiffManager (targets, debounce, fingerprints, working-tree watch) | p2-git.md | unknown |
| 387 | C3 | G1 | Server: CheckoutDiffCache (LRU 64, 15 s / 2 s windows, invalidation) | p2-git.md | skeleton |
| 388 | C3 | G1 | Server: getCheckoutDiff computation and size limits (checkout-git.ts) | p2-git.md | skeleton |
| 389 | C3 | G1 | Server: commits list and per-commit file diff (checkout-git.ts) | p2-git.md | skeleton |
| 390 | C3 | G1 | Server: branch suggestions / resolveBranchCheckout / checkoutResolvedBranch | p2-git.md | unknown |
| 391 | C3 | G1 | Server: WorkspaceGitService snapshots, refresh pipeline, auxiliary caches | p2-git.md | unknown |
| 392 | C3 | G1 | Server: working tree + repo metadata watchers, degraded polling, recovery ladder | p2-git.md | unknown |
| 393 | C3 | G1 | Server: background git fetch (workspace-git-fetch.ts) | p2-git.md | unknown |
| 394 | C3 | G1 | Server: git-metadata-event-rules.ts, git-observation-paths.ts, workspace-git-metadata.ts | p2-git.md | unknown |
| 395 | C3 | G1 | Server: WorkspaceGitObserverService (checkout_status_update + workspace_update fan-out) | p2-git.md | unknown |
| 396 | C3 | G1 | Client: DaemonClient git methods and capability flags | p2-git.md | skeleton |
| 397 | C3 | G1 | App: checkout status query + push cache (use-status-query, checkout-status-cache, query-keys) | p2-git.md | unknown |
| 398 | C3 | G1 | App: checkout diff subscription (use-diff-query, push-router, diff-sharing, diff-order) | p2-git.md | unknown |
| 399 | C3 | G1 | App: working diff comparison state (uncommitted/base/all) and useWorkingDiff | p2-git.md | skeleton |
| 400 | C3 | G1 | App: commits query, commit file diff queries, useCommitDiffFiles | p2-git.md | skeleton |
| 401 | C3 | G1 | App: ChangesSurface / diff pane (header, mode menu, options, refresh, empty/loading/error/too-large states) | p2-git.md | skeleton |
| 402 | C3 | G1 | App: working_diff, changes_tree, commit_diff panels and persisted panel state / preferences | p2-git.md | skeleton |
| 403 | C3 | G1 | App: changed-files tree, folder rows, file header, file context menu | p2-git.md | skeleton |
| 404 | C3 | G1 | App: jump-to-file sheet (compact) | p2-git.md | unknown |
| 405 | C3 | G1 | App: commits section (graph rows, collapse persistence) | p2-git.md | unknown |
| 406 | C3 | G1 | App: discard/revert file changes (uncommitted mode only) | p2-git.md | unknown |
| 407 | C3 | G1 | App: branch switcher with stash-and-switch and stash restore | p2-git.md | unknown |
| 408 | C3 | G1 | App: git actions policy (primary/secondary selection, availability messages) | p2-git.md | skeleton |
| 409 | C3 | G1 | App: git actions store (pending/success states) and useGitActions (toasts, ship default, archive promotion) | p2-git.md | skeleton |
| 410 | C3 | G1 | App: GitActionsSplitButton / WorkspaceActions / menu-only variant | p2-git.md | skeleton |
| 411 | C3 | G1 | App: worktree archive warning | p2-git.md | unknown |
| 412 | C3 | G1 | App: diff document model (virtualized rows, split/unified, wrap, collapse, anchors) | p2-git.md | skeleton |
| 413 | C3 | G1 | App: diff document web canvas surface (selection, copy, context menu, sticky headers) | p2-git.md | skeleton |
| 414 | C3 | G1 | App: diff document native Skia slab surface | p2-git.md | unknown |
| 415 | C3 | G1 | App: diff too-large / binary states | p2-git.md | skeleton |
| 416 | C3 | G1 | App: keyboard shortcut for Changes tab | p2-git.md | unknown |
| 417 | C4 | G1+W16 | Forge manifest (5 forges, neutral fallback, sign-in recipes, check traits) | p3-forge-review.md | unknown |
| 418 | C4 | G1+W16 | Forge registry / resolver / host probing (cloud match, CLI auth probe, SSH alias resolution, caches) | p3-forge-review.md | unknown |
| 419 | C4 | G1+W16 | Forge CLI runner and error classes (gh/glab/tea, timeouts, auth/missing/command errors) | p3-forge-review.md | unknown |
| 420 | C4 | G1+W16 | ForgeService adapter contract (reads, writes, polling, invalidation, ForgeReadOptions force/reason) | p3-forge-review.md | unknown |
| 421 | C4 | G1+W16 | GitHub adapter (gh): list/view/search/timeline/check details/create/merge/auto-merge/repo search, batched GraphQL poller + rate-limit guard | p3-forge-review.md | unknown |
| 422 | C4 | G1+W16 | GitLab adapter (glab): MR lookup, approvals, discussions, pipeline stages/jobs, create/merge/auto-merge, guards | p3-forge-review.md | unknown |
| 423 | C4 | G1+W16 | Gitea/Forgejo/Codeberg adapter (tea): PR list/view, timeline, commit status/Actions, merge, no auto-merge, family detection | p3-forge-review.md | unknown |
| 424 | C4 | G1+W16 | Generic non-GitHub PR-status poller (20 s / 120 s / 300 s backoff) | p3-forge-review.md | unknown |
| 425 | C4 | G1+W16 | Current-branch PR status (snapshot, `checkout_pr_status_*`, `checkout_status_update.prStatus`, caching, hooks, authState) | p3-forge-review.md | unknown |
| 426 | C4 | G1+W16 | Workspace descriptor `githubRuntime` + `forge`; sidebar PR badge, CI indicator, hover card, menu entry | p3-forge-review.md | unknown |
| 427 | C4 | G1+W16 | PR actions: Create PR (with LLM-generated title/body), merge (3 methods), enable/disable auto-merge, View PR, ship default, primary-action policy | p3-forge-review.md | unknown |
| 428 | C4 | G1+W16 | Merge capability per forge (github/gitlab/gitea) and server-side merge guards | p3-forge-review.md | unknown |
| 429 | C4 | G1+W16 | Pull/Push RPCs (named `checkout_pull_request`, not PR checkout) | p3-forge-review.md | unknown |
| 430 | C4 | G1+W16 | PR tab (panel kind, auto-open, open-location setting, content states, discovery/demand) | p3-forge-review.md | unknown |
| 431 | C4 | G1+W16 | PR pane: header/state, toolbar, checks section (ring, groups, presentation classes), GitLab pipeline + approvals, activity timeline (threads, review n | p3-forge-review.md | unknown |
| 432 | C4 | G1+W16 | PR timeline RPC (`pull_request_timeline_*`) and unsupported-daemon fallback | p3-forge-review.md | unknown |
| 433 | C4 | G1+W16 | Check details RPC (`checkout.forge.get_check_details.*`, legacy github twin), failed jobs/log tails, annotations | p3-forge-review.md | unknown |
| 434 | C4 | G1+W16 | Add-to-chat context attachments for comments/reviews/threads/checks | p3-forge-review.md | unknown |
| 435 | C4 | G1+W16 | Forge search RPC (`forge.search.*`, legacy `github_search_*`) and query hook | p3-forge-review.md | unknown |
| 436 | C4 | G1+W16 | Composer forge auto-attach from pasted URLs; forge URL/ref parsing; forge URL builders ("Open on <forge>") | p3-forge-review.md | unknown |
| 437 | C4 | G1+W16 | New-workspace PR picker and picker-owned PR attachment | p3-forge-review.md | unknown |
| 438 | C4 | G1+W16 | Create worktree/workspace from PR (checkoutSource, intent resolution, fetch refs, fork branch naming, push remotes, metadata hint) | p3-forge-review.md | unknown |
| 439 | C4 | G1+W16 | MCP workspace-create `checkout-pr` mode; plugin worktree hook intent kinds | p3-forge-review.md | unknown |
| 440 | C4 | G1+W16 | GitHub repository search + clone RPCs and add-project UI; CLI `clone` | p3-forge-review.md | unknown |
| 441 | C4 | G1+W16 | CLI: `worktree create --mode checkout-pr`, `workspace create --mode checkout-pr --forge`, `run --worktree-mode checkout-pr` | p3-forge-review.md | unknown |
| 442 | C4 | G1+W16 | Review: draft store (persisted), draft keys per mode/base/whitespace, inline editor + threads + geometry, compact comment sheet, keyboard shortcuts | p3-forge-review.md | unknown |
| 443 | C4 | G1+W16 | Review attachment snapshot (context lines, hunk header) and workspace attachment publishing | p3-forge-review.md | unknown |
| 444 | C4 | G1+W16 | Review feedback send workflow (recipients, disabled reasons, steer/interrupt, partial clear), success toast | p3-forge-review.md | unknown |
| 445 | C4 | G1+W16 | Server rendering of review/forge attachments as agent prompt text | p3-forge-review.md | unknown |
| 446 | C4 | G1+W16 | Auto-archive on merge (connection only) | p3-forge-review.md | unknown |
| 447 | C5 | W13 | Path safety, binary detection, limits (shared rules, server `file-explorer/service.ts`) | p4-files.md | skeleton |
| 448 | C5 | W13 | Protocol: file explorer read RPC (`file_explorer_request/response`) | p4-files.md | skeleton |
| 449 | C5 | W13 | Binary file-transfer frames (`FileBegin/FileChunk/FileEnd`) | p4-files.md | skeleton |
| 450 | C5 | W13 | File explorer panel (tree UI, sort, hidden files, refresh, context menu) | p4-files.md | skeleton |
| 451 | C5 | W13 | Entry mutations: create / rename (git mv) / duplicate / delete (`fs.entry.*`) | p4-files.md | skeleton |
| 452 | C5 | W13 | File download (token + `/api/files/download`, or session socket, download store) | p4-files.md | skeleton |
| 453 | C5 | W13 | File upload (`file.upload.request`, chunked frames, `FileUploadStore`) | p4-files.md | missing |
| 454 | C5 | W13 | Live file subscription (`fs.file.subscribe/update/unsubscribe`, per-file watcher with poll fallback) | p4-files.md | skeleton |
| 455 | C5 | W13 | Files panel registration and open-file routing (`file-open`, preview target resolution, open-in-side setting) | p4-files.md | skeleton |
| 456 | C5 | W13 | File tab pane (stat, panel choice, modified/close-discard flow) | p4-files.md | missing |
| 457 | C5 | W13 | FileHandle / file session (`PluginFile` API, versioned writes, deletion confirmation) | p4-files.md | skeleton |
| 458 | C5 | W13 | File panel resolution, associations, picker, open-with menu, Settings > Files, plugin hosting | p4-files.md | skeleton |
| 459 | C5 | W13 | Source panel: modes/budgets, CodeMirror editor, Vim, read-only view, toolbar | p4-files.md | skeleton |
| 460 | C5 | W13 | Editor save flow and conflict handling (`FileEditorModel`, autosave 800 ms, conflict alert) | p4-files.md | built |
| 461 | C5 | W13 | Server write path (`fs.file.write`, 1 MiB cap, atomic rename, revision check) | p4-files.md | built |
| 462 | C5 | W13 | Find in file (web) | p4-files.md | built |
| 463 | C5 | W13 | Markdown preview (front matter table) | p4-files.md | skeleton |
| 464 | C5 | W13 | HTML preview (sandboxed iframe + CSP) | p4-files.md | missing |
| 465 | C5 | W13 | Image viewer | p4-files.md | missing |
| 466 | C5 | W13 | DocumentFrame (PDF/sandbox rule, plugin-only) | p4-files.md | missing |
| 467 | C5 | W13 | Open in editor / reveal in file manager (desktop bridge, editor-target registry, planner, preferred editor, forge link) | p4-files.md | missing |
| 468 | C5 | W13 | Legacy daemon editor RPCs (stubbed) | p4-files.md | missing |
| 469 | C5 | W13 | Server directory observer (`file-observer`, feeds git) | p4-files.md | skeleton |
| 470 | C5 | W13 | Directory sync (sequenced agent/workspace/project directory replica protocol) | p4-files.md | missing |
| 471 | C5 | W13 | Directory suggestions and project icon RPCs | p4-files.md | skeleton |
| 472 | C5 | W13 | Private files and atomic file write helpers | p4-files.md | built |
| 473 | C5 | W13 | LSP: not found | p4-files.md | skeleton |
| 474 | D1 | C2 | Terminal PTY session (spawn, env, shell integration, protocol replies, title, exit info, kill) | area-4-terminals-scripts-browser.md | unknown |
| 475 | D1 | C2 | Terminal manager + detached terminal worker (pool, records, reaper, incompatibility handling) | area-4-terminals-scripts-browser.md | unknown |
| 476 | D1 | C2 | Terminal WS protocol: list/subscribe directories/create/rename/subscribe/input/kill/capture | area-4-terminals-scripts-browser.md | unknown |
| 477 | D1 | C2 | Binary terminal stream (slots, opcodes, snapshot/replay/restore, backpressure catch-up, coalescer) | area-4-terminals-scripts-browser.md | unknown |
| 478 | D1 | C2 | Terminal size ownership (claim/update) | area-4-terminals-scripts-browser.md | unknown |
| 479 | D1 | C2 | Terminal activity tracker, activity HTTP endpoint, agent hooks, `enableTerminalAgentHooks` | area-4-terminals-scripts-browser.md | unknown |
| 480 | D1 | C2 | Terminal attention notifications (daemon fan-out, push, OS notification, attention clear) | area-4-terminals-scripts-browser.md | unknown |
| 481 | D1 | C2 | Terminal tab lifecycle in the app (create/queue/list sync/rename/close/confirm/reconcile) | area-4-terminals-scripts-browser.md | unknown |
| 482 | D1 | C2 | TerminalPane (stream controller, loading/error states, input queue, modifiers) | area-4-terminals-scripts-browser.md | skeleton (T1 read-only loading/error; D1 input/stream cutover pending) |
| 483 | D1 | C2 | Terminal resize and focus claim logic | area-4-terminals-scripts-browser.md | unknown |
| 484 | D1 | C2 | Terminal paste (bracketed paste, image paste), file paste upload queue, file drop routing | area-4-terminals-scripts-browser.md | skeleton (pure bracketed-paste codec staged; clipboard/upload/drop pending) |
| 485 | D1 | C2 | Terminal local file links and external links | area-4-terminals-scripts-browser.md | unknown |
| 486 | D1 | C2 | xterm.js web emulator (addons, key handling, fonts, scrollback setting) | area-4-terminals-scripts-browser.md | unknown |
| 487 | D1 | C2 | Native grid terminal renderer (selection handles, scroll, hidden input) | area-4-terminals-scripts-browser.md | skeleton (T1 grid/scroll passed; D1 selection/input pending) |
| 488 | D1 | C2 | Terminal find | area-4-terminals-scripts-browser.md | missing |
| 489 | D1 | C2 | Virtual key bar / soft keyboard controls | area-4-terminals-scripts-browser.md | unknown |
| 490 | D1 | C2 | Terminal profiles (defaults, prompt sentinel, adoption, icons, settings UI, launch) | area-4-terminals-scripts-browser.md | unknown |
| 491 | D1 | C2 | Agent terminal tools and `paseo terminal` CLI | area-4-terminals-scripts-browser.md | unknown |
| 492 | D1 | C2 | Worktree auto terminals (`worktree.terminals`) | area-4-terminals-scripts-browser.md | unknown |
| 493 | D1 | C2 | New-workspace terminal launch target | area-4-terminals-scripts-browser.md | unknown |
| 494 | C2 | W15 | Layout model (split tree, tabs, persistence, placements) | area-4-terminals-scripts-browser.md | skeleton |
| 495 | C2 | W15 | Split container rendering, resize handles, pane focus, maximize, focus mode | area-4-terminals-scripts-browser.md | skeleton |
| 496 | C2 | W15 | Panel registry/manifest/descriptor contract and panel kinds | area-4-terminals-scripts-browser.md | skeleton |
| 497 | C2 | W15 | Tab drag and drop, drop zones, tab reorder, move to pane, keyboard pane commands | area-4-terminals-scripts-browser.md | skeleton |
| 498 | C2 | W15 | Mobile three-panel shell (motion model, gestures, thresholds, gesture blocking) | area-4-terminals-scripts-browser.md | unknown |
| 499 | C2 | W15 | Compact workspace sheets and layout modes | area-4-terminals-scripts-browser.md | skeleton |
| 500 | D2 | W8 | `paseo.json` scripts/services/servicePorts configuration and project settings editor | area-4-terminals-scripts-browser.md | missing |
| 501 | D2 | W8 | Script/service launch, stop, restart, exit handling, runtime store | area-4-terminals-scripts-browser.md | skeleton |
| 502 | D2 | W8 | Service env and port plan/allocation (range, portScript, free port) | area-4-terminals-scripts-browser.md | missing |
| 503 | D2 | W8 | Port discovery (not found) | area-4-terminals-scripts-browser.md | skeleton |
| 504 | D2 | W8 | Service proxy: hostnames, routes, HTTP/WebSocket proxying, 404/502 behaviour, standalone listener, public base URL, branch rename re-route | area-4-terminals-scripts-browser.md | missing |
| 505 | D2 | W8 | Script health monitor | area-4-terminals-scripts-browser.md | missing |
| 506 | D2 | W8 | Script status projection and `script_status_update` delivery | area-4-terminals-scripts-browser.md | missing |
| 507 | D2 | W8 | Scripts menu UI (rows, link selector, route preference store, open service URL behaviour) | area-4-terminals-scripts-browser.md | skeleton |
| 508 | A9 | W22 | Managed-process ledger | area-4-terminals-scripts-browser.md | missing |
| 509 | D3 | W18 | In-app browser pane (toolbar, device presets, store, resident webviews, profile, shortcuts, new-tab requests, browser data clear) | area-4-terminals-scripts-browser.md | skeleton |
| 510 | D3 | W18 | Browser element annotate/screenshot attachments | area-4-terminals-scripts-browser.md | unknown |
| 511 | D3 | W18 | Agent browser tools (22 tools, daemon broker, host registration, error codes, config switch) | area-4-terminals-scripts-browser.md | skeleton |
| 512 | D3 | W18 | Desktop browser automation host and Electron/CDP executor | area-4-terminals-scripts-browser.md | skeleton |
| 513 | D3 | W18 | Preview of services in the app (via "View service" -> browser tab; no separate preview feature found) | area-4-terminals-scripts-browser.md | skeleton |
| 514 | E1 | E2 | Daemon supervisor + worker process model (fork, heartbeat, crash restart, lifecycle intents) | area-5-hosts-devices-desktop.md | unknown |
| 515 | E1 | E2 | Daemon CLI lifecycle commands (start/run/status/stop/restart/reload/pair/config/set-password) | area-5-hosts-devices-desktop.md | unknown |
| 516 | E1 | E2 | PID/instance lock `paseo.pid` (boot id, heartbeat, stale detection) | area-5-hosts-devices-desktop.md | unknown |
| 517 | E1 | E2 | Daemon identity: `server-id`, `daemon-keypair.json`, hostname, version | area-5-hosts-devices-desktop.md | unknown |
| 518 | E1 | E2 | Persisted config `config.json` (schema, defaults, env/CLI precedence, JSON schema generation) | area-5-hosts-devices-desktop.md | unknown |
| 519 | E1 | E2 | DaemonConfigStore (live patch, reload, reloadable paths, override pinning) and `get/set_daemon_config`, `daemon.config.reload`, `daemon_config_changed | area-5-hosts-devices-desktop.md | unknown |
| 520 | E1 | E2 | Daemon authentication: password (bcrypt), HTTP bearer, WS subprotocol/hello auth, `hello.rejected`, close codes | area-5-hosts-devices-desktop.md | unknown |
| 521 | E1 | E2 | Local credential (`local-credential`, desktop-managed auto-auth) | area-5-hosts-devices-desktop.md | unknown |
| 522 | E1 | E2 | Permissions model and session authorization (9 daemon permissions) | area-5-hosts-devices-desktop.md | unknown |
| 523 | E1 | E2 | Network gates: Host header allowlist, CORS, WS origin check, trusted proxies | area-5-hosts-devices-desktop.md | unknown |
| 524 | E1 | E2 | WebSocket transport: hello/server_info, session grace, socket lease, client keepalive/reconnect, ping/pong latency, chunked transport | area-5-hosts-devices-desktop.md | unknown |
| 525 | E1 | E2 | Outbound proxy support (env proxies for fetch and WS) | area-5-hosts-devices-desktop.md | unknown |
| 526 | E1 | E2 | Relay transport (daemon outbound control/data sockets, relay v1/v2, Cloudflare Durable Object relay server) | area-5-hosts-devices-desktop.md | unknown |
| 527 | E1 | E2 | Relay end-to-end encryption (Noise `protected` vs `legacy`, `requireProtectedSecurity`, app-side security pinning) | area-5-hosts-devices-desktop.md | unknown |
| 528 | E2 | G4 | Pairing offers (ConnectionOffer v2, `#offer=` URL, `relay://`/`#connect=`, `daemon.get_pairing_offer`) | area-5-hosts-devices-desktop.md | skeleton |
| 529 | E2 | G4 | Pair device UI (enable relay consent, QR, copy link) and CLI `paseo daemon pair` QR | area-5-hosts-devices-desktop.md | skeleton |
| 530 | E2 | G4 | Add connection method chooser and welcome-screen actions | area-5-hosts-devices-desktop.md | skeleton |
| 531 | E2 | G4 | Paste pairing link modal and password prompt | area-5-hosts-devices-desktop.md | skeleton |
| 532 | E2 | G4 | QR scan screen (`/pair-scan`, scan model) and F-Droid stubs | area-5-hosts-devices-desktop.md | skeleton |
| 533 | E2 | G4 | Host confirmation sheet (new host / changed connection, key fingerprint) | area-5-hosts-devices-desktop.md | skeleton |
| 534 | E1 | E2 | App host registry (`@paseo:daemon-registry`, HostProfile, connection kinds, merge rules, routed connections) | area-5-hosts-devices-desktop.md | skeleton (ADR0097 contract passed; typed ID/private-file staging; upsert/format/runtime unbuilt) |
| 535 | E1 | E2 | Host runtime controller (status machine, probing, best-connection selection, adaptive switch, auth failure state) | area-5-hosts-devices-desktop.md | unknown |
| 536 | E1 | E2 | Host boot/bootstrap (localhost probe, env override, initial hint, desktop daemon registration) | area-5-hosts-devices-desktop.md | unknown |
| 537 | E1 | E2 | Host switching/pickers (sidebar HostPicker, Settings host switcher, HostChooserModal, new-workspace host picker) | area-5-hosts-devices-desktop.md | unknown |
| 538 | E1 | E2 | Host settings page (status badges, connection error, appearance: rename/color/badge, remove host, restart daemon, update daemon) | area-5-hosts-devices-desktop.md | unknown |
| 539 | E1 | E2 | Host connections page (latency rows, remove connection) | area-5-hosts-devices-desktop.md | unknown |
| 540 | E1 | E2 | Host appearance (name, color, badge display) and sidebar host badges | area-5-hosts-devices-desktop.md | unknown |
| 541 | E1 | E2 | Add host: Direct connection modal (host/port/SSL/password/advanced URI) | area-5-hosts-devices-desktop.md | unknown |
| 542 | E1 | E2 | Host target grammar (`parseHostTarget`: tcp, ws, unix, pipe, ssh, relay, offer URL) | area-5-hosts-devices-desktop.md | unknown |
| 543 | E1 | E2 | Add host: Remote SSH (desktop; ssh `-W` tunnel to existing daemon, installs nothing) | area-5-hosts-devices-desktop.md | unknown |
| 544 | E1 | E2 | Daemon restart (RPC `restart_server_request`, keep/kill terminals, wait-for-replacement) | area-5-hosts-devices-desktop.md | unknown |
| 545 | E1 | E2 | Daemon shutdown RPC (`shutdown_server_request`) | area-5-hosts-devices-desktop.md | unknown |
| 546 | E1 | E2 | Daemon self-update (`daemon.update.request`, progress, npm global install, origin validation, restart) | area-5-hosts-devices-desktop.md | unknown |
| 547 | E1 | E2 | Daemon status RPC (`daemon.get_status`) and `/api/health`, `/api/status` | area-5-hosts-devices-desktop.md | unknown |
| 548 | E1 | E2 | Connected hosts (daemon-to-daemon `hosts.json`, hosts.* RPCs, routing, identity pinning, limits) | area-5-hosts-devices-desktop.md | unknown |
| 549 | E1 | E2 | Connected hosts CLI (`paseo host ...`) and app discovery of routed hosts | area-5-hosts-devices-desktop.md | unknown |
| 550 | E3 | W19b | Desktop app shell (Electron packaging, targets, `paseo://` scheme, custom `paseo://app` renderer protocol) | area-5-hosts-devices-desktop.md | skeleton |
| 551 | E3 | W19b | Desktop windows (state persistence, new window, chrome modes, single instance) | area-5-hosts-devices-desktop.md | skeleton |
| 552 | E3 | W19b | Desktop application menu and context menus | area-5-hosts-devices-desktop.md | skeleton |
| 553 | E3 | W19b | Deep links (`paseo://h/<serverId>/agent/<agentId>`) and open-project launches | area-5-hosts-devices-desktop.md | skeleton |
| 554 | E3 | W19b | Desktop CLI passthrough and CLI install (`~/.local/bin/paseo`, shell rc PATH) | area-5-hosts-devices-desktop.md | skeleton |
| 555 | E3 | W19b | Desktop login-shell environment inheritance | area-5-hosts-devices-desktop.md | skeleton |
| 556 | E3 | W19b | Desktop preload bridge (`window.paseoDesktop`, IPC `paseo:invoke`, events) | area-5-hosts-devices-desktop.md | skeleton |
| 557 | E3 | W15 | Desktop built-in daemon management (start/stop/restart, ownership, version mismatch, keep-running-after-quit, pause management) | area-5-hosts-devices-desktop.md | skeleton |
| 558 | E3 | W19 | Desktop local daemon transport (unix socket/pipe via main process) and Local Daemon settings card (logs, CLI status, stop) | area-5-hosts-devices-desktop.md | unknown |
| 559 | E3 | W19 | Startup splash/blocker states for the managed daemon | area-5-hosts-devices-desktop.md | unknown |
| 560 | E3 | W15 | Desktop quit lifecycle (stop daemon, install update on quit, signals) | area-5-hosts-devices-desktop.md | skeleton |
| 561 | E3 | W19b | Desktop settings file (`desktop-settings.json`) and migrations | area-5-hosts-devices-desktop.md | skeleton |
| 562 | E3 | W19 | Desktop notifications and permissions settings sections | area-5-hosts-devices-desktop.md | unknown |
| 563 | E3 | W19b | Desktop app updates (electron-updater, channels stable/beta, staged rollout, quit-time install, callouts, Rosetta callout) | area-5-hosts-devices-desktop.md | skeleton |
| 564 | E3 | W19b | Changelog sheet (What's new) | area-5-hosts-devices-desktop.md | skeleton |
| 565 | E4 | W15 | Onboarding (steps, decision rules, orchestration choices, re-run) | area-5-hosts-devices-desktop.md | skeleton |
| 566 | E4 | N1+W20 | App diagnostic sheet (client + desktop + per-host report, redaction) | area-5-hosts-devices-desktop.md | passed |
| 567 | E4 | N1+W20 | Daemon diagnostics RPC (`diagnostics.request`) and report sections | area-5-hosts-devices-desktop.md | missing |
| 568 | A9 | G3+W22 | Provider diagnostics RPC (`provider_diagnostic_request`) | area-5-hosts-devices-desktop.md | missing |
| 569 | E4 | N1+W20 | Process diagnostics and daemon log (`daemon.log`) viewing | area-5-hosts-devices-desktop.md | skeleton |
| 570 | E4 | N1+W20 | App native watchdog / render trace (`PaseoNativeTrace`) and macOS compositor watchdog | area-5-hosts-devices-desktop.md | unknown |
| 571 | E4 | N1+W20 | Native shims (iOS hardware keyboard submit, F-Droid camera/notification stubs) | area-5-hosts-devices-desktop.md | unknown |
| 572 | E4 | N1+W20 | Bundled web UI hosting by the daemon (`features.webUi`) | area-5-hosts-devices-desktop.md | unknown |
| 573 | E4 | N1+W20 | Not found: system tray, device list/revocation UI, cross-host settings/file sync | area-5-hosts-devices-desktop.md | unknown |
| 574 | F1 | G2 | Schedule data model + status/cadence/target/run schema (`proto/schedule/types.ts`) | A.md | unknown |
| 575 | F1 | G2 | Schedule on-disk store (`<paseoHome>/schedules/<id>.json`, atomic writes, per-schedule serialisation, invalid-file skipping) | A.md | unknown |
| 576 | F1 | G2 | Cron parser/validator (5 fields, 0-6 dow) and next-run computation (timezone, 366-day lookahead); `every` legacy cadence; `everyMsToFiveFieldCron` | A.md | unknown |
| 577 | F1 | G2 | ScheduleService tick loop (1 s), claim-and-advance, overlap skip, maxRuns/expiry completion, run recovery after daemon restart, orphan sweep, complete | A.md | unknown |
| 578 | F1 | G2 | Run execution: agent-target (steer) and new-agent (workspace per run, archive on finish, isolation local/worktree, output capture) | A.md | unknown |
| 579 | F1 | G2 | Schedule ops: create (runOnCreate), createOrReplace (idempotent by name+target), update (+ newAgentConfig patch), pause, resume, delete, run-once, ins | A.md | unknown |
| 580 | F1 | G2 | Schedule RPC messages (`schedule/*` + `/response`, `rpc_error` code `schedule_request_failed`) and `DaemonClient` schedule methods | A.md | unknown |
| 581 | F1 | G2 | Schedules screen (`/schedules`, sidebar row, command-center entry, Active/Ended filter, host filter, empty/loading/error/host-error banner states) | A.md | skeleton |
| 582 | F1 | G2 | Aggregated schedules across hosts (react-query, per-host errors, reconnect invalidation) | A.md | skeleton |
| 583 | F1 | G2 | Schedule row (derived state badge, title/target/meta line, kebab menu: Resume/Pause/Run now/Edit/Delete, delete confirm, optimistic pause/resume/delet | A.md | skeleton |
| 584 | F1 | G2 | Schedule form sheet (create/edit new-agent schedule, heartbeat cadence-only edit; host/project/model/thinking/mode/isolation/archive-on-finish/max run | A.md | skeleton |
| 585 | F1 | G2 | Cadence editor (presets, cron text, validation, preview, device timezone) | A.md | skeleton |
| 586 | F1 | G2 | Schedule CLI (`paseo schedule create/ls/inspect/logs/pause/resume/delete/run-once/update`, duration parsing, legacy `--target`) | A.md | unknown |
| 587 | F2 | W21 | Heartbeats (agent-target schedules; `paseo heartbeat create/update/delete`; MCP `create_heartbeat`/`delete_heartbeat`; UI wording and restrictions) | A.md | unknown |
| 588 | F1 | G2 | Saved prompts / prompt library: not found | A.md | unknown |
| 589 | F2 | W21 | Agent MCP endpoint `/mcp/agents` (streamable HTTP, stateless, capability-token auth, `callerAgentId` scoping) | A.md | missing (deliberate stdio/per-session-credential transport; HTTP proposal deferred in ADR 0065) |
| 590 | F2 | W21 | MCP injection into agent configs (`mcpServers.paseo`, env vars, strip on persist) and native delivery for OpenCode/omp | A.md | skeleton (node injection/snapshot/migration built; provider transports and checks pending) |
| 591 | F2 | W21 | Paseo tools visibility: settings card (`mcp.injectIntoAgents`), `--no-mcp`/`--no-inject-mcp`, per-provider `paseoTools {enabled, disabledTools}` | A.md | skeleton (typed node policy/inject built; native card/CLI integration and gates pending) |
| 592 | F2 | W21 | Tool catalog scoping (agent vs top-level), `serverId` host fan-out, plugin tools, result serialisation | A.md | skeleton (policy/scoping/machine/structured-result edge built; plugin producer integration pending) |
| 593 | F2 | W21 | MCP tools: agents (12), workspaces/scripts (7), terminals (5), schedules/heartbeats (11), providers/hosts/voice (6), browser (22) | A.md | skeleton (existing-call facade built; creation/provider/launch-profile/host/browser/voice dependencies pending) |
| 594 | F2 | W18 | Skills settings section (install switch, Update, uninstall confirm, picker sheet, destructive-removal confirm) | A.md | built (native typed owner/cards/picker/confirmation/recovery source; view/link9 tests passed; UI/union checks, whole gate, captures/review owed; MCP_PARITY=None) |
| 595 | F2 | W15 | Onboarding "orchestration" step (tools, skills, CLI) | A.md | skeleton |
| 596 | F2 | W21 | Skills protocol (`agent.skills.*` requests/responses) + client methods | A.md | built (typed pending rows/client methods; MCP_PARITY=None; whole gate/review owed) |
| 597 | F2 | W21 | Skills controller/queue, status diff, sync with managed-file manifest, crash-safe transactions, startup auto-update | A.md | built (node writer/filesystem transaction machinery; scoped tests only; whole gate/review owed) |
| 598 | F2 | W21 | Selection store (`agents.skills.selection`, default all) and legacy desktop selection migration | A.md | built (private all/custom store and explicit import-only-if-unset/AlreadySet; no automatic legacy file: Vessel never wrote one, per coordinator; gates/review owed) |
| 599 | F2 | W21 | Bundled skills: paseo, paseo-advisor, paseo-committee, paseo-handoff, paseo-help, paseo-plugin; install targets `~/.agents/skills`, `~/.claude/skills` | A.md | built (six independent Vessel guides and shared/Claude targets; whole gate/review owed) |
| 600 | A5 | W7 | Notification routing (the server-side "who gets notified" plan) | B.md | built |
| 601 | A5 | W7 | What the user sees | B.md | built |
| 602 | A5 | W7 | Client presence heartbeat (`client_heartbeat`) | B.md | built |
| 603 | A5 | W7 | What the user sees / does | B.md | built |
| 604 | A5 | W7 | In-app OS notification display (desktop Electron, web Notification API) | B.md | built |
| 605 | A5 | W7 | Desktop notification settings (Settings > Notifications) | B.md | built |
| 606 | A5 | W7 | Notification open routing (tap/click -> navigate) | B.md | skeleton |
| 607 | A5 | W7 | States, errors | B.md | built |
| 608 | A5 | W7 | Mobile push notifications (Expo push) | B.md | missing |
| 609 | A5 | W7 | Agent attention / unread state ("requires attention") | B.md | built |
| 610 | A5 | W7 | Terminal attention / activity | B.md | built |
| 611 | A5 | W7 | What the user sees | B.md | built |
| 612 | A5 | W7 | Sidebar / tab attention badges, favicon and dock badge | B.md | skeleton |
| 613 | A5 | W7 | What the user sees | B.md | skeleton |
| 614 | A5 | W7 | In-app toasts | B.md | built |
| 615 | A5 | W7 | What the user sees | B.md | built |
| 616 | A5 | W7 | Notification sounds | B.md | built |
| 617 | A5 | W7 | What the user sees | B.md | built |
| 618 | A5 | W7 | Timeline "notification" items and agent finish subscriptions (adjacent, not OS notifications) | B.md | missing |
| 619 | F3 | W5 | Account usage reports: protocol and daemon pipeline | B.md | skeleton |
| 620 | F3 | W5 | Usage source plugins (per provider fetchers) | B.md | skeleton |
| 621 | F3 | W5 | Usage query layer and caching (app) | B.md | skeleton |
| 622 | A5 | W7 | States | B.md | unknown |
| 623 | A5 | W7 | Platforms / Connections | B.md | unknown |
| 624 | F3 | W5 | Usage cards, bars, formatting and tone | B.md | skeleton |
| 625 | A5 | W7 | What the user sees | B.md | unknown |
| 626 | F3 | W5 | Sidebar Usage summary item (pinned windows) | B.md | skeleton |
| 627 | F3 | W5 | Usage modal (and host filter, options menu) | B.md | skeleton |
| 628 | F3 | W5 | Settings keys (see storage table): `AppSettings.usage = {displayAs: "used"/"remaining" (default "used"), pins: UsagePin[]/null (default null), serverI | B.md | skeleton |
| 629 | F3 | W5 | Per-agent usage (context window meter + account usage) | B.md | skeleton |
| 630 | F3 | W5 | CLI: `paseo usage` | B.md | unknown |
| 631 | A5 | W7 | Presence age threshold and focused visible client suppression | B.md | built |
| 632 | A5 | W7 | Most-recent active client selection vs mobile push fallback | B.md | built |
| 633 | A5 | W7 | Client heartbeat activity/focus/visibility coalescing | B.md | built |
| 634 | A5 | W7 | Expo push token registration, revocation and lease renewal | B.md | missing |
| 635 | A5 | W7 | F-Droid push stubs and permission-denied behavior | B.md | missing |
| 636 | A5 | W7 | Notification target navigation and queued host route bootstrap | B.md | skeleton |
| 637 | A5 | W7 | Deferred focus-entry unread clearing vs input/send/blur | B.md | built |
| 638 | A5 | W7 | Workspace Mark as read and Mark as unread | B.md | missing |
| 639 | A5 | W7 | Permission/error/finished sidebar bucket aggregation | B.md | built |
| 640 | A5 | W7 | Terminal activity attention and focus clearing | B.md | skeleton |
| 641 | A5 | W7 | Desktop sound preference and notification test | B.md | built |
| 642 | A5 | W7 | Toast lifetime, dismissal and error/info presentation | B.md | built |
| 643 | F3 | W5 | Account report discovery and per-agent account association | B.md | skeleton |
| 644 | F3 | W5 | Usage source plugin refresh/caching and structured failures | B.md | skeleton |
| 645 | F3 | W5 | Usage used/remaining display, pinned windows and host filter | B.md | skeleton |
| 646 | F3 | W5 | Per-agent context-token meter vs account quota reports | B.md | skeleton |
| 647 | F3 | W5 | Usage CLI list/inspect and unsupported-host errors | B.md | unknown |
| 648 | F4 | S2+W22 | Sidebar shell (LeftSidebar, layout, resize, collapse) | C.md | skeleton |
| 649 | F4 | S2+W22 | What the user sees | C.md | unknown |
| 650 | F4 | S2+W22 | How it works | C.md | skeleton |
| 651 | F4 | S2+W22 | States, limits | C.md | skeleton |
| 652 | F4 | S2+W22 | Sidebar header nav rows (sidebar-nav) | C.md | unknown |
| 653 | F4 | S2+W22 | What the user sees | C.md | unknown |
| 654 | F4 | S2+W22 | How it works | C.md | unknown |
| 655 | F4 | S2+W22 | Settings section "Sidebar" | C.md | unknown |
| 656 | F4 | S2+W22 | Workspace list: grouping, filtering, sorting, pinning | C.md | unknown |
| 657 | F4 | S2+W22 | What the user sees | C.md | unknown |
| 658 | F4 | S2+W22 | How it works | C.md | unknown |
| 659 | F4 | S2+W22 | Sidebar display-preferences menu | C.md | unknown |
| 660 | F4 | S2+W22 | What the user sees | C.md | unknown |
| 661 | F4 | S2+W22 | How it works | C.md | unknown |
| 662 | F4 | S2+W22 | Project row and workspace row | C.md | unknown |
| 663 | F4 | S2+W22 | What the user sees (project row, project grouping) | C.md | unknown |
| 664 | F4 | S2+W22 | What the user sees (workspace row) | C.md | unknown |
| 665 | F4 | S2+W22 | Sidebar callouts | C.md | skeleton |
| 666 | F4 | S2+W22 | What the user sees | C.md | skeleton |
| 667 | F4 | S2+W22 | How it works | C.md | skeleton |
| 668 | F4 | S2+W22 | Routes that matter for navigation | C.md | unknown |
| 669 | F4 | S2+W22 | What the user sees / how it works | C.md | unknown |
| 670 | F4 | S2+W22 | Keyboard shortcuts: engine, bindings, dispatch | C.md | skeleton |
| 671 | F4 | S2+W22 | What the user sees | C.md | skeleton |
| 672 | F4 | S2+W22 | How it works | C.md | skeleton |
| 673 | F4 | S2+W22 | Binding to action routing (`routeKeyboardShortcut`) | C.md | skeleton |
| 674 | F4 | S2+W22 | Default shortcut table (binding id, action, combo; `when` summary) | C.md | built (a675 scoped Clippy + App/Kit; current union, gate/captures/V4 pending) |
| 675 | F4 | S2+W22 | Shortcut help dialog | C.md | built (a675 scoped Clippy + App/Kit; current union, gate/captures/V4 pending) |
| 676 | F4 | S2+W22 | Settings > Shortcuts (rebinding) | C.md | skeleton |
| 677 | F4 | S2+W22 | Keyboard shift (mobile on-screen keyboard) | C.md | unknown |
| 678 | F4 | S2+W22 | Command center | C.md | skeleton |
| 679 | F4 | S2+W22 | How it works | C.md | skeleton |
| 680 | F4 | S2+W22 | Registered contribution sources (what items exist) | C.md | skeleton |
| 681 | F4 | S2+W22 | Sidebar persistent width and collapsed state | C.md | skeleton |
| 682 | F4 | S2+W22 | Sidebar project/status grouping and custom project ordering | C.md | unknown |
| 683 | F4 | S2+W22 | Workspace title/branch source and trailing diff/timestamp choice | C.md | unknown |
| 684 | F4 | S2+W22 | Host/project/label filters including Unlabelled | C.md | unknown |
| 685 | F4 | S2+W22 | Workspace pin, copy path/branch, rename/read/unread/archive menus | C.md | unknown |
| 686 | F4 | S2+W22 | Project action menus and metadata status rows | C.md | unknown |
| 687 | F4 | S2+W22 | Sidebar setup/changelog/host-permission callouts | C.md | unknown |
| 688 | F4 | S2+W22 | Keyboard binding scopes, contexts and override persistence | C.md | skeleton |
| 689 | F4 | S2+W22 | Reserved/native editing and IME shortcut exclusions | C.md | built (a675 scoped Clippy + App/Kit; current union, gate/captures/V4 pending) |
| 690 | F4 | S2+W22 | Shortcut help and rebinding conflict/reset states | C.md | skeleton |
| 691 | F4 | S2+W22 | Mobile keyboard shift behavior | C.md | unknown |
| 692 | F4 | S2+W22 | Command center search ranking, group navigation and nested pages | C.md | skeleton |
| 693 | F4 | S2+W22 | Command center root/workspace/host/project action registrations | C.md | skeleton |
| 694 | F4 | S2+W22 | Plugin command-center contribution integration | C.md | skeleton |
| 695 | F5 | E2+K+L | Settings shell and navigation; app settings store; General; Sending; Chat; Terminal; Editor; Files; Open location; Browser data; Integrations; Notific | D.md | unknown |
| 696 | F6 | W16 | Plugin model, trust and `pluginsEnabled`; manifest; directory/git/npm/registry install; reviewed updates; reload/enable/disable/remove; logs; subproce | E.md | skeleton |
| 697 | A8 | W4 | Permission cards and plan/mode approval | gaps.md | skeleton (simultaneous native cards/shared Plan source built; rich actions/plan runtime pending) |
| 698 | A8 | W4 | Agent question forms and answer encoding | gaps.md | skeleton (retained multi-request fields/navigation/retry source built; metadata/preview/empty rules pending) |
| 699 | A8 | W4 | Nonblocking Codex questions and durable pending requests | gaps.md | skeleton |
| 700 | A8 | W4 | Permission event fan-out, cancellation and mode policy | gaps.md | skeleton |
| 701 | A8 | W11 | Agent attention policy — corrections and missing boundary details | gaps.md | unknown |
| 702 | A8 | W4 | CLI permission listing, allow and deny | gaps.md | skeleton (parser/preflight/confirmation staged; checked output/respond wiring pending) |
| 703 | A10 | N3+G3 | Host agent profiles: create, edit, remove and reorder | gaps.md | missing |
| 704 | A10 | N3+G3 | Applying profiles to drafts and existing agents | gaps.md | missing |
| 705 | A10 | N3+G3 | Legacy favorites migration and account boundary | gaps.md | missing |
| 706 | F5 | E2+K+L | Agent skills: coverage boundary and provider-native skills | gaps.md | unknown |
| 707 | N(titles)/F5 | titles+E2 | Agent titles versus workspace generated titles | gaps.md | skeleton (node provenance built; app/workspace pending) |
| 708 | N(titles)/F5 | titles+E2 | Metadata generation selection and project prompt overrides | gaps.md | built (node selector/styles; settings UI pending) |
| 709 | F5 | E2+K+L | Built-in appearance themes and system theme | gaps.md | built |
| 710 | F5 | E2+K+L | Plugin-contributed themes | gaps.md | missing |
| 711 | F5 | E2+K+L | Fonts, text sizes, content width, syntax palette and preview | gaps.md | skeleton |
| 712 | F5 | E2+K+L | Localization mechanism and language selection | gaps.md | passed |
| 713 | E4 | W15 | Onboarding state machine, completion and reopening | gaps.md | skeleton |
| 714 | E4 | W15 | Onboarding step effects and project-history suggestion integration | gaps.md | skeleton |
| 715 | F5 | E2+K+L | Validated local storage, settings migrations and hydration safety | gaps.md | unknown |
| 716 | F5 | E2+K+L | Replica cache: offline paint, row storage, budgets and invalidation | gaps.md | unknown |
| 717 | F5 | E2+K+L | Performance controls and local render tracing | gaps.md | unknown |
| 718 | F5 | E2+K+L | Workspace label journal limits and uncertain-storage recovery | gaps.md | unknown |
| 719 | A4 | W2 | Subagent track, child navigation and archive/detach actions | gaps.md | skeleton (native track/navigation/detach wired; A2 bulk/keyed union pending) |
| 720 | Q | operator | Hub CLI login, credentials and guided onboarding | gaps.md | unknown |
| 721 | Q | operator | Hub daemon relationship, grants, reconnect and revocation | gaps.md | unknown |
| 722 | Q | operator | Hub-owned agent execution and control | gaps.md | unknown |
| 723 | Q | operator | Hub triggers as code: export, deploy and project bundles | gaps.md | unknown |
| 724 | F5 | E2+K+L | Web overlay layering, focus trap and shortcut isolation | gaps.md | unknown |
| 725 | A8 | W4 | Permission tool/plan/mode cards and provider-defined actions | gaps.md | skeleton |
| 726 | A8 | W4 | Plan markdown and implementation follow-up turn | gaps.md | skeleton |
| 727 | A8 | W4 | Question navigation, radio/checkbox, custom answer, preview and dismiss semantics | gaps.md | skeleton |
| 728 | A8 | W4 | Question response failure/retry and header-keyed encoding | gaps.md | skeleton |
| 729 | A8 | W4 | Codex asynchronous questions: active-turn steer and idle follow-up | gaps.md | skeleton |
| 730 | A8 | W4 | Runtime permission maps, persistence metadata and offline stripping | gaps.md | skeleton |
| 731 | A8 | W4 | Cross-client request/resolution events, direct acknowledgement and cancellation | gaps.md | skeleton (selected-connection response fence source built; durable receipts/live resident cutover pending) |
| 732 | A8 | W11 | Presence selection, delegated callback suppression and unread corrections | gaps.md | unknown |
| 733 | A8 | W4 | CLI permit listing, batch allow default, deny flags and confirmation errors | gaps.md | skeleton |
| 734 | A10 | N3+G3 | Profile CRUD, order, notes, glyph/color and catalog-backed form | gaps.md | missing |
| 735 | A10 | N3+G3 | Profile draft apply, live bundle apply, invalid-mode omission and partial failure | gaps.md | missing |
| 736 | A10 | N3+G3 | Legacy favorite migration markers, deduplication and catalog retry | gaps.md | missing |
| 737 | F5 | E2+K+L | Provider account/usage boundary and absent core account switcher | gaps.md | unknown |
| 738 | F5 | E2+K+L | Bundled skill install versus native provider skill autocomplete | gaps.md | unknown |
| 739 | N(titles)/F5 | titles+E2 | Agent provisional/manual titles versus generated workspace names | gaps.md | skeleton (node provenance built; app/workspace pending) |
| 740 | N(titles)/F5 | titles+E2 | Metadata Automatic/Preferred candidate order and fallback | gaps.md | built (node selection; UI pending) |
| 741 | N(titles)/F5 | titles+E2 | Project title/branch/commit/PR instruction overrides | gaps.md | skeleton (title style reader built; branch/commit/PR callers pending) |
| 742 | F5 | E2+K+L | Built-in theme picker, system adaptation and Android system bars | gaps.md | built |
| 743 | F5 | E2+K+L | Plugin theme validation, multi-host deduplication and missing-theme fallback | gaps.md | missing |
| 744 | F5 | E2+K+L | Font-family validation, per-axis text sizes, width reset, syntax theme and preview | gaps.md | skeleton |
| 745 | F5 | E2+K+L | Offline locale backend, system tag resolution and plugin text boundary | gaps.md | built |
| 746 | E4 | W15 | Onboarding gate/completion/reopen and step visit tokens | gaps.md | skeleton |
| 747 | E4 | W15 | Onboarding step effects, early project suggestions and late-result guarding | gaps.md | skeleton |
| 748 | F5 | E2+K+L | Validated JSON/Zustand storage, hydration protection and settings migration markers | gaps.md | unknown |
| 749 | F5 | E2+K+L | SQLite/IndexedDB replica rows, checkpoint repair, timeline/byte budgets and eviction | gaps.md | unknown |
| 750 | F5 | E2+K+L | Browser render profiling, native trace gates and explicit query freshness | gaps.md | unknown |
| 751 | F5 | E2+K+L | Workspace label journal catch-up, compound commit and uncertain-storage block | gaps.md | unknown |
| 752 | A4 | W2 | Managed/provider subagent track, parent pill/callout and nested timelines | gaps.md | built (native track/nested pane source; final checks/keyed union pending) |
| 753 | A4 | W2 | Archive finished progress/retry, local provider hide, detach and tab-close policy | gaps.md | skeleton (local hide/detach wired; A2 conditional archive integration pending) |
| 754 | Q | operator | Hub browser login, per-origin credentials and explicit automation grant | gaps.md | unknown |
| 755 | A8 | W11 | Hub relationship state, permission update, liveness/retry and revocation | gaps.md | unknown |
| 756 | Q | operator | Hub execution idempotency, authority generation, owned events and control | gaps.md | unknown |
| 757 | Q | operator | Hub trigger export, validate-only deploy, sequential install and legacy bundles | gaps.md | unknown |
| 758 | F5 | E2+K+L | Web overlay relative layers, focus trap/restore, IME and DOM popover Escape | gaps.md | unknown |

## Checkpoints (W9, ADR 0077)

| # | Pkg | Owner | Feature | Source | State |
|---|---|---|---|---|---|
| 759 | checkpoints | W9 | Per-turn file snapshot (files mode; before/after boundaries) | parts/A-lifecycle.md#Rewind | skeleton |
| 760 | checkpoints | W9 | Preview changed files before confirmation (files mode) | parts/A-lifecycle.md#Rewind | skeleton |
| 761 | checkpoints | W9 | Restore files to a turn while preserving conversation (files mode) | parts/A-lifecycle.md#Rewind | skeleton |
| 762 | checkpoints | W9 | Typed checkpoint refusals and durable restore outcomes (files mode) | parts/A-lifecycle.md#Rewind | skeleton |
| 763 | checkpoints | W9 | Checkpoint capture, storage and response limits (files mode) | parts/A-lifecycle.md#Rewind | skeleton |
