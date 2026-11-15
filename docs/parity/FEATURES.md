# Paseo 0.15.4 feature catalogue (inventory)
Generated 2026-11-15 from the survey files in this folder; each line names one feature or sub-feature. Details: the source file named under each heading.

## 1 Agents and sessions

### from parts/A-lifecycle.md

1. Agent storage (JSON files per agent, atomic queued writes, delete fence)
2. Stored record schema and wire payload projections (snapshot, list item, stored payload)
3. Agent owner (daemon execution ownership)
4. Legacy workspaceId backfill migration
5. Agent state buckets and status priority
6. Lifecycle statuses and transitions
7. Create agent: wire protocol (agent.create.request/response/update, legacy create_agent_request)
8. Create agent: session request layer (cwd checks, worktree targets, intent resolution)
9. Create agent: createAgentCommand and AgentManager.createAgent
10. Create agent mode resolution and configuration validator
11. Create agent titles (explicit/provisional)
12. CreationService idempotency receipts and creation.subscribe
13. Auto-archive after first turn (autoArchive flag)
14. New workspace screen (project/host/isolation/ref pickers, launch target)
15. New workspace submit flows (chat, empty workspace, terminal, background handoff)
16. Workspace draft tab and draft create flow state machine
17. create-agent-preferences (form preferences, optimistic layer)
18. agent-controls policy/labels and composer agent controls
19. CLI paseo run (+ options, workspace policy, output schema)
20. CLI agent ls/stop/delete/archive/reload/detach/update/import/open/wait/send/inspect/logs/attach/mode
21. Archive / unarchive / close_items (cascade to children, detach rules)
22. App archive UX (optimistic caches, archived callout, tab close policy)
23. Auto-archive on merge
24. Delete agent
25. Resume (resume_agent_request, ensureAgentLoaded)
26. Refresh/reload agent (refresh_agent_request, stale session reload)
27. Idle runtime cleanup ("park"), no explicit wake
28. Restart recovery (resumeOnStartup, resumePrompt) and shutdown capture
29. Import from provider history: list (fetch_recent_provider_sessions)
30. Import from provider history: import_agent_request and archived re-activation
31. Plugin session history
32. Import session sheet (app)
33. Fork (fork_context request, curated chat history attachment)
34. Fork app flow (useForkAgent, draft seeding, cwd remap)
35. Rewind (agent.rewind.request, capabilities, app menu)
36. Export (not found) and handoff (skill/background handoff only)
37. Detach subagent
38. Titles (provisional, lazy first-message, manual rename)
39. Labels (reserved labels, update, filter)
40. Attention state, clear, mark unread
41. Attention notification policy and fan-out (presence, push, in-app)
42. Agent deep links (build/parse, route, desktop, CLI open)
43. Agent directory fetch/subscribe (fetch_agents, agent_update)
44. Agent history fetch and search
45. Sessions screen
46. Sidebar agent list entries and state buckets (app)

### from parts/B-providers.md

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

### from parts/C-timeline.md

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

### from parts/D-messaging.md

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

## 2 Composer and input

### from area-2-composer-input.md

1. Composer shell: modes (chat/terminal), readOnly, preserve-and-lock, placeholder, auto-height, fullscreen editor, dock, editing session, focus rules
2. Draft persistence: draft store, keys, lifecycle, throttled persist, after-paint staging, hydration, TTL, draft-agent create flow
3. Send path: Enter / Mod+Enter / long-press, default vs alternate action, sendBehavior + sendShortcut settings, interrupt vs steer, optimistic submission, receipts/idempotency, error restore
4. Stop/interrupt button, Escape, cancel RPC and cancelling state
5. Message queue: queue, edit queued, send-now, auto-drain as steer, pending-permission override
6. History recall (Up/Down)
7. Client slash commands (/exit, /clear + aliases)
8. Plugin client slash commands
9. Provider (agent) slash commands and skills, inline skill commands, draft command listing
10. `@` file/dir mentions (directory suggestions)
11. Autocomplete popover mechanics and states
12. Dictation: mic button, overlay, shortcuts, capture (web/native), streaming sender with replay, server stream manager, auto-commit/silence, timeouts, retry/discard, insert vs insert-and-send
13. Speech providers/settings/models: local (sherpa) and OpenAI, readiness reasons, env vars, model library download/delete, settings page and provider sheet
14. Realtime voice mode: button, overlay, mute/stop, runtime phases, capture upload, grouped TTS playback and acks, barge-in, thinking cue, server VoiceSession/turn controller/TTS manager/speak tool/system prompt/abort
15. Audio engines (web/native), playback queue, `expo-two-way-audio` module (iOS/Android), plugin audio playback helper
16. Attachment types, wire conversion, attach menu
17. Image attach: picker (desktop/web/native), paste (web/native), clipboard "Paste image", drop
18. Generic file upload: picker, drop, 50 MB limit, pending pills, chunked binary upload, server FileUploadStore (sanitization, stale timeout)
19. Local attachment byte stores (IndexedDB / desktop files / native files), encoding for send, GC and retention
20. Attachment pills, tray, lightbox, previews, user-message attachment rendering
21. Workspace attachments (browser element, review, chat history, PR context) and suppression rules
22. Workspace-file attachments: add to chat action, drag source/drop, line_range type (no creator found)
23. Forge issue/PR picker and URL auto-attach
24. Plugin attachment sources
25. Assistant selection copy (markdown + code-aware, web only)
26. Assistant file links (parse, resolve, lookup, open)
27. Assistant images (file RPC acquisition, cache, retention, lightbox)
28. File download token endpoint (used by explorer; documented as a connection)
29. Find in pane widget and Cmd/Ctrl+F policy
30. Keyboard shortcuts for composer actions

## 3 Projects, git and files

### from parts/p1-projects-workspaces.md

Count: **33 feature sections** above; flat feature/sub-feature list follows. Cross-area connections and this inventory are not counted as features. “Not found” entries explicitly delimit unsupported/unlocated requested surfaces.

- 01 Add Project flow: open/close modal request.
- 01 Host selection and preferred/single-host skip.
- 01 Method selection and per-host capability/update gates.
- 01 Back stack, remembered in-flow input drafts, keyboard selection and loading/error/empty states.
- 01 Electron local Browse.
- 02 Register existing directory without workspace creation.
- 02 Exact-root project deduplication and snapshot/navigation update.
- 02 Compatibility Open Project: find/create workspace by directory.
- 03 Create empty directory under selected existing parent.
- 03 Name/path validation and registration rollback.
- 03 Git-init action in Add Project: not found.
- 04 GitHub account repository search and authentication/unavailable states.
- 04 Manual full remote / owner-repo HTTPS or SSH choices.
- 04 Clone parent suggestions, remembered parent, existing target disable.
- 04 Staged clone, timeout, cleanup and register-project transition.
- 05 Onboarding history-derived suggested projects per enabled provider.
- 05 Root grouping, exclusions, ranking and limits.
- 05 Default preselection and user override.
- 05 Sequential add, partial failure and late-provider arrivals.
- 06 Project custom name/reset.
- 06 Choose image, URL/website icon import, preview and automatic icon reset.
- 06 Server icon validation and host-local icon persistence.
- 06 Icon discovery/revision/cache and placeholder display.
- 07 Project removal capability/client preflight.
- 07 Archive project contents while preserving directories.
- 07 Multi-host partial removal result and empty-project row removal.
- 08 Host-targeted project settings load/empty/invalid/missing/offline states.
- 08 Revision-aware config save and stale-write reload.
- 08 Unknown-field preservation and uncommitted setup warning.
- 09 Worktree setup and teardown command editing.
- 09 Named script add/edit/remove/type/port fields.
- 09 Metadata instruction overrides.
- 09 Missing-setup callout and per-project dismissal.
- 10 New Workspace project and host selection/remembered context.
- 10 Local versus worktree isolation preference/capability.
- 10 Branch/change-request picker and source mapping.
- 10 Pending/error/retry state.
- 11 Import-session entry and fork-context handoff.
- 11 Empty workspace launch.
- 11 Combined workspace plus chat-agent launch and early handoff.
- 11 Blank shell launch and typed command input.
- 11 Terminal profile prompt substitution/read-only command launch.
- 11 Navigation-safe completion after leaving create screen.
- 12 Workspace create schema, durable receipts/id claims, idempotent operation phases and reconnect.
- 12 Fresh record creation for shared cwd.
- 12 Initial-agent relative cwd mapping.
- 12 Background visibility/inheritance and removed internal-workspace rejection.
- 12 Plugin create hook and durable placement fields.
- 13 Branch-off worktree mode.
- 13 Checkout existing branch mode.
- 13 Checkout change request mode, forge mismatch/cross-repo source.
- 13 Managed root/hash/slug and exact subdirectory mapping.
- 13 Creation hooks/config seeding, ownership metadata and rollback.
- 14 Asynchronous sequential setup and auto-terminals.
- 14 Runtime environment/port injection.
- 14 Setup progress snapshots, command logs and truncation.
- 14 Setup subscription, waiting/no-command states, expandable logs/durations and failed/blocked setup-tab surfacing.
- 14 Explicit automation trust release; generic failed-setup rerun not supplied by this RPC.
- 14 Archive cancellation of setup.
- 15 Optimistic workspace archive, rollback and route cleanup.
- 15 Bulk-workspace archive helpers and partial errors (production UI caller not found).
- 15 Workspace ownership archive versus worktree-scope archive.
- 15 Teardown and last-reference owned directory deletion.
- 15 Unowned/failed-removal directory preservation and empty-project persistence.
- 16 Host automatic-archive-after-merge toggle.
- 16 Open-to-merged same-URL latch, fresh snapshot and per-cwd dedupe.
- 16 Dirty/ahead/ownership checks, consumed merged URL and optional-hook boundary versus manual archive.
- 17 Inspect archived workspace recovery.
- 17 Unarchive existing directory.
- 17 Restore deleted worktree and original subdirectory.
- 17 Missing-source/unavailable/unsupported/inspection-error states and retry.
- 18 Workspace label picker/search/check/uncheck.
- 18 Create-and-assign label with host catalog color.
- 18 Case-insensitive name normalization and assignment persistence.
- 19 Label manager search and host selection.
- 19 Atomic label rename/recolor and collision rejection.
- 19 Delete affected-count inspection, confirmation and assignment removal.
- 19 Catalog snapshot/delta synchronization and offline host projection.
- 20 JSON project/workspace registry load and serialized mutation.
- 20 Workspace/project list filters, sort, pagination and empty-project rows.
- 20 Owned subscriptions, bootstrap delta queue, generation/sequence resume.
- 20 App cached snapshot/hydration and stale-response/archive suppression.
- 21 Project root observation and bounded periodic reconciliation.
- 21 Missing-directory safe archive and unreachable-root preservation.
- 21 Git metadata changes with immutable identity/membership.
- 22 Aggregated workspace status and transition/activity timestamps.
- 22 Pending initial-agent running overlay.
- 22 Mark read except permission attention.
- 22 Mark unread via eligible finished root agent and pane unfocus.
- 23 Workspace title override; SDK/CLI reset, UI empty-name restriction.
- 23 Shared workspace pin/unpin and duplicate-toggle guard.
- 24 Cross-host project grouping and placement rows.
- 24 Collapse/expand and empty-project New Workspace row.
- 24 Project settings/window/folder/remove actions.
- 24 Workspace copy/rename/read/pin/label/folder/archive actions.
- 24 Branch/host/PR/check/service/label metadata row.
- 24 Manual/created/last-prompt order, filters and loading states.
- 25 Workspace script/service list and health tones.
- 25 Start and script terminal reveal.
- 25 Stop and state-observed restart.
- 25 SDK/CLI named script list/start/stop status contract.
- 25 Service URL route preference/copy and sidebar health.
- 26 New-tab launcher built-ins, plugin panels and terminal profiles.
- 26 Tab target identity and workspace-local persistence key.
- 26 Agent/terminal hydration-safe reconciliation and subagent ownership.
- 27 Context/tab menu copy/resume/id/path/open-with actions.
- 27 Agent/terminal tab rename and agent reload.
- 27 Root-agent archive-on-close versus child-agent layout-only close.
- 27 Terminal kill confirmation and passive/dirty-panel close.
- 27 Close left/right/above/below/others and batched lifecycle handling.
- 28 CLI project create/list/rename/reset/delete.
- 28 CLI workspace create modes/title/visibility, list, rename/reset, setup and archive.
- 28 CLI worktree list/create and whole-worktree archive.
- 28 CLI validation/host targeting and missing restore/icon CLI surfaces.
- 29 Pane split/focus/move/reorder/close and depth/size limits.
- 29 Preferred main/side opens and modified-preview protection.
- 29 Explorer Changes/PR placement and one-time background PR tab.
- 29 Compact sheets, promotion, parent relationship and dirty-state retention.
- 29 Validated layout persistence/migrations.
- 30 Route/active workspace selection and warm-deck retention.
- 30 Workspace/agent/terminal/browser/files/changes keyboard entry points.
- 30 Platform-specific workspace/tab jump/cycle/close shortcuts.
- 30 Pane keyboard operations, focus gates and user overrides.
- 31 Desktop editor/file-manager bridge and local-placement gate.
- 31 Active file/line or selected-directory open target.
- 31 Forge web links and project Open in new window.
- 32 First-context provisional/generated workspace title.
- 32 One-shot branch-off placeholder rename and collision suffixes.
- 32 Intervening user rename protection and metadata failure behavior.
- 33 Host Settings Projects list, host-specific rows and empty/loading/error behavior.
- 33 No-editable-target settings route and Back to Projects.
- 33 Open Project home Add/Import/Providers/Pair device links and host errors.

### from parts/p2-git.md

- Protocol: checkout_status_request/response/checkout_status_update
- Protocol: checkout.diff.get.request/response (one-shot diff)
- Protocol: subscribe_checkout_diff_request/response, checkout_diff_update, unsubscribe_checkout_diff_request
- Protocol: checkout.refresh.request/response
- Protocol: checkout_commit_request/response
- Protocol: checkout_merge_request/response (merge into base, merge|squash)
- Protocol: checkout_merge_from_base_request/response
- Protocol: checkout_pull_request/checkout_pull_response
- Protocol: checkout_push_request/response
- Protocol: checkout.discard_changes.request/response
- Protocol: checkout.commits.list.request/response, checkout.commits.file_diff.request/response
- Protocol: checkout_switch_branch_request/response, checkout.rename_branch.request/response
- Protocol: validate_branch_request/response, branch_suggestions_request/response
- Protocol: stash_save/pop/list request/response
- Protocol: authorization scopes per message; CheckoutError codes; ParsedDiffFile/DiffHunk/DiffLine schemas; CheckoutDiffCompare; ProjectCheckoutLite; WorkspaceGitRuntime payload
- Server: CheckoutSession read side (status, branch validate, suggestions, diff read/subscribe, refresh)
- Server: CheckoutSession write side (commit, discard, switch/rename branch, merge, merge from base, pull, push, stash)
- Server: commit message generation (git-metadata-generator.ts)
- Server: GitMutationService (notifyGitMutation, checkoutExistingBranch, createBranchFromBase)
- Server: status-projection.ts (snapshot -> checkout status payload)
- Server: CheckoutDiffManager (targets, debounce, fingerprints, working-tree watch)
- Server: CheckoutDiffCache (LRU 64, 15 s / 2 s windows, invalidation)
- Server: getCheckoutDiff computation and size limits (checkout-git.ts)
- Server: commits list and per-commit file diff (checkout-git.ts)
- Server: branch suggestions / resolveBranchCheckout / checkoutResolvedBranch
- Server: WorkspaceGitService snapshots, refresh pipeline, auxiliary caches
- Server: working tree + repo metadata watchers, degraded polling, recovery ladder
- Server: background git fetch (workspace-git-fetch.ts)
- Server: git-metadata-event-rules.ts, git-observation-paths.ts, workspace-git-metadata.ts
- Server: WorkspaceGitObserverService (checkout_status_update + workspace_update fan-out)
- Client: DaemonClient git methods and capability flags
- App: checkout status query + push cache (use-status-query, checkout-status-cache, query-keys)
- App: checkout diff subscription (use-diff-query, push-router, diff-sharing, diff-order)
- App: working diff comparison state (uncommitted/base/all) and useWorkingDiff
- App: commits query, commit file diff queries, useCommitDiffFiles
- App: ChangesSurface / diff pane (header, mode menu, options, refresh, empty/loading/error/too-large states)
- App: working_diff, changes_tree, commit_diff panels and persisted panel state / preferences
- App: changed-files tree, folder rows, file header, file context menu
- App: jump-to-file sheet (compact)
- App: commits section (graph rows, collapse persistence)
- App: discard/revert file changes (uncommitted mode only)
- App: branch switcher with stash-and-switch and stash restore
- App: git actions policy (primary/secondary selection, availability messages)
- App: git actions store (pending/success states) and useGitActions (toasts, ship default, archive promotion)
- App: GitActionsSplitButton / WorkspaceActions / menu-only variant
- App: worktree archive warning
- App: diff document model (virtualized rows, split/unified, wrap, collapse, anchors)
- App: diff document web canvas surface (selection, copy, context menu, sticky headers)
- App: diff document native Skia slab surface
- App: diff too-large / binary states
- App: keyboard shortcut for Changes tab

### from parts/p3-forge-review.md

- Forge manifest (5 forges, neutral fallback, sign-in recipes, check traits)
- Forge registry / resolver / host probing (cloud match, CLI auth probe, SSH alias resolution, caches)
- Forge CLI runner and error classes (gh/glab/tea, timeouts, auth/missing/command errors)
- ForgeService adapter contract (reads, writes, polling, invalidation, ForgeReadOptions force/reason)
- GitHub adapter (gh): list/view/search/timeline/check details/create/merge/auto-merge/repo search, batched GraphQL poller + rate-limit guard
- GitLab adapter (glab): MR lookup, approvals, discussions, pipeline stages/jobs, create/merge/auto-merge, guards
- Gitea/Forgejo/Codeberg adapter (tea): PR list/view, timeline, commit status/Actions, merge, no auto-merge, family detection
- Generic non-GitHub PR-status poller (20 s / 120 s / 300 s backoff)
- Current-branch PR status (snapshot, `checkout_pr_status_*`, `checkout_status_update.prStatus`, caching, hooks, authState)
- Workspace descriptor `githubRuntime` + `forge`; sidebar PR badge, CI indicator, hover card, menu entry
- PR actions: Create PR (with LLM-generated title/body), merge (3 methods), enable/disable auto-merge, View PR, ship default, primary-action policy
- Merge capability per forge (github/gitlab/gitea) and server-side merge guards
- Pull/Push RPCs (named `checkout_pull_request`, not PR checkout)
- PR tab (panel kind, auto-open, open-location setting, content states, discovery/demand)
- PR pane: header/state, toolbar, checks section (ring, groups, presentation classes), GitLab pipeline + approvals, activity timeline (threads, review nesting, collapse), kebab menus
- PR timeline RPC (`pull_request_timeline_*`) and unsupported-daemon fallback
- Check details RPC (`checkout.forge.get_check_details.*`, legacy github twin), failed jobs/log tails, annotations
- Add-to-chat context attachments for comments/reviews/threads/checks
- Forge search RPC (`forge.search.*`, legacy `github_search_*`) and query hook
- Composer forge auto-attach from pasted URLs; forge URL/ref parsing; forge URL builders ("Open on <forge>")
- New-workspace PR picker and picker-owned PR attachment
- Create worktree/workspace from PR (checkoutSource, intent resolution, fetch refs, fork branch naming, push remotes, metadata hint)
- MCP workspace-create `checkout-pr` mode; plugin worktree hook intent kinds
- GitHub repository search + clone RPCs and add-project UI; CLI `clone`
- CLI: `worktree create --mode checkout-pr`, `workspace create --mode checkout-pr --forge`, `run --worktree-mode checkout-pr`
- Review: draft store (persisted), draft keys per mode/base/whitespace, inline editor + threads + geometry, compact comment sheet, keyboard shortcuts
- Review attachment snapshot (context lines, hunk header) and workspace attachment publishing
- Review feedback send workflow (recipients, disabled reasons, steer/interrupt, partial clear), success toast
- Server rendering of review/forge attachments as agent prompt text
- Auto-archive on merge (connection only)

### from parts/p4-files.md

- Path safety, binary detection, limits (shared rules, server `file-explorer/service.ts`)
- Protocol: file explorer read RPC (`file_explorer_request/response`)
  - Binary file-transfer frames (`FileBegin/FileChunk/FileEnd`)
- File explorer panel (tree UI, sort, hidden files, refresh, context menu)
- Entry mutations: create / rename (git mv) / duplicate / delete (`fs.entry.*`)
- File download (token + `/api/files/download`, or session socket, download store)
- File upload (`file.upload.request`, chunked frames, `FileUploadStore`)
- Live file subscription (`fs.file.subscribe/update/unsubscribe`, per-file watcher with poll fallback)
- Files panel registration and open-file routing (`file-open`, preview target resolution, open-in-side setting)
- File tab pane (stat, panel choice, modified/close-discard flow)
- FileHandle / file session (`PluginFile` API, versioned writes, deletion confirmation)
- File panel resolution, associations, picker, open-with menu, Settings > Files, plugin hosting
- Source panel: modes/budgets, CodeMirror editor, Vim, read-only view, toolbar
  - Editor save flow and conflict handling (`FileEditorModel`, autosave 800 ms, conflict alert)
  - Server write path (`fs.file.write`, 1 MiB cap, atomic rename, revision check)
  - Find in file (web)
  - Markdown preview (front matter table)
  - HTML preview (sandboxed iframe + CSP)
  - Image viewer
  - DocumentFrame (PDF/sandbox rule, plugin-only)
- Open in editor / reveal in file manager (desktop bridge, editor-target registry, planner, preferred editor, forge link)
  - Legacy daemon editor RPCs (stubbed)
- Server directory observer (`file-observer`, feeds git)
- Directory sync (sequenced agent/workspace/project directory replica protocol)
- Directory suggestions and project icon RPCs
- Private files and atomic file write helpers
- LSP: not found

## 4 Terminals, scripts and browser

### from area-4-terminals-scripts-browser.md

1. Terminal PTY session (spawn, env, shell integration, protocol replies, title, exit info, kill)
2. Terminal manager + detached terminal worker (pool, records, reaper, incompatibility handling)
3. Terminal WS protocol: list/subscribe directories/create/rename/subscribe/input/kill/capture
4. Binary terminal stream (slots, opcodes, snapshot/replay/restore, backpressure catch-up, coalescer)
5. Terminal size ownership (claim/update)
6. Terminal activity tracker, activity HTTP endpoint, agent hooks, `enableTerminalAgentHooks`
7. Terminal attention notifications (daemon fan-out, push, OS notification, attention clear)
8. Terminal tab lifecycle in the app (create/queue/list sync/rename/close/confirm/reconcile)
9. TerminalPane (stream controller, loading/error states, input queue, modifiers)
10. Terminal resize and focus claim logic
11. Terminal paste (bracketed paste, image paste), file paste upload queue, file drop routing
12. Terminal local file links and external links
13. xterm.js web emulator (addons, key handling, fonts, scrollback setting)
14. Native grid terminal renderer (selection handles, scroll, hidden input)
15. Terminal find
16. Virtual key bar / soft keyboard controls
17. Terminal profiles (defaults, prompt sentinel, adoption, icons, settings UI, launch)
18. Agent terminal tools and `paseo terminal` CLI
19. Worktree auto terminals (`worktree.terminals`)
20. New-workspace terminal launch target
21. Layout model (split tree, tabs, persistence, placements)
22. Split container rendering, resize handles, pane focus, maximize, focus mode
23. Panel registry/manifest/descriptor contract and panel kinds
24. Tab drag and drop, drop zones, tab reorder, move to pane, keyboard pane commands
25. Mobile three-panel shell (motion model, gestures, thresholds, gesture blocking)
26. Compact workspace sheets and layout modes
27. `paseo.json` scripts/services/servicePorts configuration and project settings editor
28. Script/service launch, stop, restart, exit handling, runtime store
29. Service env and port plan/allocation (range, portScript, free port)
30. Port discovery (not found)
31. Service proxy: hostnames, routes, HTTP/WebSocket proxying, 404/502 behaviour, standalone listener, public base URL, branch rename re-route
32. Script health monitor
33. Script status projection and `script_status_update` delivery
34. Scripts menu UI (rows, link selector, route preference store, open service URL behaviour)
35. Managed-process ledger
36. In-app browser pane (toolbar, device presets, store, resident webviews, profile, shortcuts, new-tab requests, browser data clear)
37. Browser element annotate/screenshot attachments
38. Agent browser tools (22 tools, daemon broker, host registration, error codes, config switch)
39. Desktop browser automation host and Electron/CDP executor
40. Preview of services in the app (via "View service" -> browser tab; no separate preview feature found)

## 5 Hosts, devices and desktop

### from area-5-hosts-devices-desktop.md

1. Daemon supervisor + worker process model (fork, heartbeat, crash restart, lifecycle intents)
2. Daemon CLI lifecycle commands (start/run/status/stop/restart/reload/pair/config/set-password)
3. PID/instance lock `paseo.pid` (boot id, heartbeat, stale detection)
4. Daemon identity: `server-id`, `daemon-keypair.json`, hostname, version
5. Persisted config `config.json` (schema, defaults, env/CLI precedence, JSON schema generation)
6. DaemonConfigStore (live patch, reload, reloadable paths, override pinning) and `get/set_daemon_config`, `daemon.config.reload`, `daemon_config_changed`
7. Daemon authentication: password (bcrypt), HTTP bearer, WS subprotocol/hello auth, `hello.rejected`, close codes
8. Local credential (`local-credential`, desktop-managed auto-auth)
9. Permissions model and session authorization (9 daemon permissions)
10. Network gates: Host header allowlist, CORS, WS origin check, trusted proxies
11. WebSocket transport: hello/server_info, session grace, socket lease, client keepalive/reconnect, ping/pong latency, chunked transport
12. Outbound proxy support (env proxies for fetch and WS)
13. Relay transport (daemon outbound control/data sockets, relay v1/v2, Cloudflare Durable Object relay server)
14. Relay end-to-end encryption (Noise `protected` vs `legacy`, `requireProtectedSecurity`, app-side security pinning)
15. Pairing offers (ConnectionOffer v2, `#offer=` URL, `relay://`/`#connect=`, `daemon.get_pairing_offer`)
16. Pair device UI (enable relay consent, QR, copy link) and CLI `paseo daemon pair` QR
17. Add connection method chooser and welcome-screen actions
18. Paste pairing link modal and password prompt
19. QR scan screen (`/pair-scan`, scan model) and F-Droid stubs
20. Host confirmation sheet (new host / changed connection, key fingerprint)
21. App host registry (`@paseo:daemon-registry`, HostProfile, connection kinds, merge rules, routed connections)
22. Host runtime controller (status machine, probing, best-connection selection, adaptive switch, auth failure state)
23. Host boot/bootstrap (localhost probe, env override, initial hint, desktop daemon registration)
24. Host switching/pickers (sidebar HostPicker, Settings host switcher, HostChooserModal, new-workspace host picker)
25. Host settings page (status badges, connection error, appearance: rename/color/badge, remove host, restart daemon, update daemon)
26. Host connections page (latency rows, remove connection)
27. Host appearance (name, color, badge display) and sidebar host badges
28. Add host: Direct connection modal (host/port/SSL/password/advanced URI)
29. Host target grammar (`parseHostTarget`: tcp, ws, unix, pipe, ssh, relay, offer URL)
30. Add host: Remote SSH (desktop; ssh `-W` tunnel to existing daemon, installs nothing)
31. Daemon restart (RPC `restart_server_request`, keep/kill terminals, wait-for-replacement)
32. Daemon shutdown RPC (`shutdown_server_request`)
33. Daemon self-update (`daemon.update.request`, progress, npm global install, origin validation, restart)
34. Daemon status RPC (`daemon.get_status`) and `/api/health`, `/api/status`
35. Connected hosts (daemon-to-daemon `hosts.json`, hosts.* RPCs, routing, identity pinning, limits)
36. Connected hosts CLI (`paseo host ...`) and app discovery of routed hosts
37. Desktop app shell (Electron packaging, targets, `paseo://` scheme, custom `paseo://app` renderer protocol)
38. Desktop windows (state persistence, new window, chrome modes, single instance)
39. Desktop application menu and context menus
40. Deep links (`paseo://h/<serverId>/agent/<agentId>`) and open-project launches
41. Desktop CLI passthrough and CLI install (`~/.local/bin/paseo`, shell rc PATH)
42. Desktop login-shell environment inheritance
43. Desktop preload bridge (`window.paseoDesktop`, IPC `paseo:invoke`, events)
44. Desktop built-in daemon management (start/stop/restart, ownership, version mismatch, keep-running-after-quit, pause management)
45. Desktop local daemon transport (unix socket/pipe via main process) and Local Daemon settings card (logs, CLI status, stop)
46. Startup splash/blocker states for the managed daemon
47. Desktop quit lifecycle (stop daemon, install update on quit, signals)
48. Desktop settings file (`desktop-settings.json`) and migrations
49. Desktop notifications and permissions settings sections
50. Desktop app updates (electron-updater, channels stable/beta, staged rollout, quit-time install, callouts, Rosetta callout)
51. Changelog sheet (What's new)
52. Onboarding (steps, decision rules, orchestration choices, re-run)
53. App diagnostic sheet (client + desktop + per-host report, redaction)
54. Daemon diagnostics RPC (`diagnostics.request`) and report sections
55. Provider diagnostics RPC (`provider_diagnostic_request`)
56. Process diagnostics and daemon log (`daemon.log`) viewing
57. App native watchdog / render trace (`PaseoNativeTrace`) and macOS compositor watchdog
58. Native shims (iOS hardware keyboard submit, F-Droid camera/notification stubs)
59. Bundled web UI hosting by the daemon (`features.webUi`)
60. Not found: system tray, device list/revocation UI, cross-host settings/file sync

## 6 Automation, overviews, settings, plugins

### from .area6-parts/A.md

Schedules
- Schedule data model + status/cadence/target/run schema (`proto/schedule/types.ts`)
- Schedule on-disk store (`<paseoHome>/schedules/<id>.json`, atomic writes, per-schedule serialisation, invalid-file skipping)
- Cron parser/validator (5 fields, 0-6 dow) and next-run computation (timezone, 366-day lookahead); `every` legacy cadence; `everyMsToFiveFieldCron`
- ScheduleService tick loop (1 s), claim-and-advance, overlap skip, maxRuns/expiry completion, run recovery after daemon restart, orphan sweep, completeForAgent on archive
- Run execution: agent-target (steer) and new-agent (workspace per run, archive on finish, isolation local/worktree, output capture)
- Schedule ops: create (runOnCreate), createOrReplace (idempotent by name+target), update (+ newAgentConfig patch), pause, resume, delete, run-once, inspect, logs
- Schedule RPC messages (`schedule/*` + `/response`, `rpc_error` code `schedule_request_failed`) and `DaemonClient` schedule methods
- Schedules screen (`/schedules`, sidebar row, command-center entry, Active/Ended filter, host filter, empty/loading/error/host-error banner states)
- Aggregated schedules across hosts (react-query, per-host errors, reconnect invalidation)
- Schedule row (derived state badge, title/target/meta line, kebab menu: Resume/Pause/Run now/Edit/Delete, delete confirm, optimistic pause/resume/delete)
- Schedule form sheet (create/edit new-agent schedule, heartbeat cadence-only edit; host/project/model/thinking/mode/isolation/archive-on-finish/max runs; submit gating)
- Cadence editor (presets, cron text, validation, preview, device timezone)
- Schedule CLI (`paseo schedule create|ls|inspect|logs|pause|resume|delete|run-once|update`, duration parsing, legacy `--target`)
- Heartbeats (agent-target schedules; `paseo heartbeat create|update|delete`; MCP `create_heartbeat`/`delete_heartbeat`; UI wording and restrictions)
Saved prompts
- Saved prompts / prompt library: not found
MCP
- Agent MCP endpoint `/mcp/agents` (streamable HTTP, stateless, capability-token auth, `callerAgentId` scoping)
- MCP injection into agent configs (`mcpServers.paseo`, env vars, strip on persist) and native delivery for OpenCode/omp
- Paseo tools visibility: settings card (`mcp.injectIntoAgents`), `--no-mcp`/`--no-inject-mcp`, per-provider `paseoTools {enabled, disabledTools}`
- Tool catalog scoping (agent vs top-level), `serverId` host fan-out, plugin tools, result serialisation
- MCP tools: agents (12), workspaces/scripts (7), terminals (5), schedules/heartbeats (11), providers/hosts/voice (6), browser (22)
Orchestration skills
- Skills settings section (install switch, Update, uninstall confirm, picker sheet, destructive-removal confirm)
- Onboarding "orchestration" step (tools, skills, CLI)
- Skills protocol (`agent.skills.*` requests/responses) + client methods
- Skills controller/queue, status diff, sync with managed-file manifest, crash-safe transactions, startup auto-update
- Selection store (`agents.skills.selection`, default all) and legacy desktop selection migration
- Bundled skills: paseo, paseo-advisor, paseo-committee, paseo-handoff, paseo-help, paseo-plugin; install targets `~/.agents/skills`, `~/.claude/skills` (+ legacy `~/.codex/skills` cleanup)

### from .area6-parts/B.md

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

### from .area6-parts/C.md

- Sidebar shell (LeftSidebar, layout, resize, collapse)
- What the user sees
- How it works
- States, limits
- Sidebar header nav rows (sidebar-nav)
- What the user sees
- How it works
- Settings section "Sidebar"
- Workspace list: grouping, filtering, sorting, pinning
- What the user sees
- How it works
- Sidebar display-preferences menu
- What the user sees
- How it works
- Project row and workspace row
- What the user sees (project row, project grouping)
- What the user sees (workspace row)
- Sidebar callouts
- What the user sees
- How it works
- Routes that matter for navigation
- What the user sees / how it works
- Keyboard shortcuts: engine, bindings, dispatch
- What the user sees
- How it works
- Binding to action routing (`routeKeyboardShortcut`)
- Default shortcut table (binding id, action, combo; `when` summary)
- Shortcut help dialog
- Settings > Shortcuts (rebinding)
- Keyboard shift (mobile on-screen keyboard)
- Command center
- How it works
- Registered contribution sources (what items exist)

- Sidebar persistent width and collapsed state
- Sidebar project/status grouping and custom project ordering
- Workspace title/branch source and trailing diff/timestamp choice
- Host/project/label filters including Unlabelled
- Workspace pin, copy path/branch, rename/read/unread/archive menus
- Project action menus and metadata status rows
- Sidebar setup/changelog/host-permission callouts
- Keyboard binding scopes, contexts and override persistence
- Reserved/native editing and IME shortcut exclusions
- Shortcut help and rebinding conflict/reset states
- Mobile keyboard shift behavior
- Command center search ranking, group navigation and nested pages
- Command center root/workspace/host/project action registrations
- Plugin command-center contribution integration

### from .area6-parts/D.md

- Settings shell and navigation; app settings store; General; Sending; Chat; Terminal; Editor; Files; Open location; Browser data; Integrations; Notifications (desktop); Permissions (desktop); Diagnostics; About and updates; Desktop settings file; daemon config read/patch/reload; Agents page cards (Paseo tools, Browser tools, Append system prompt, Restart recovery, Idle runtime cleanup, Skills, Profiles); Workspaces page (auto-archive); Terminals page (profiles, agent hooks); Metadata generation; Providers; Voice; Usage page; Connections; Add host flows; Pair device; Host overview (appearance, local daemon, update daemon, remove host); Restart daemon; Project settings.

### from .area6-parts/E.md

- Plugin model, trust and `pluginsEnabled`; manifest; directory/git/npm/registry install; reviewed updates; reload/enable/disable/remove; logs; subprocess runtime; plugin RPC; plugin settings; lifecycle hooks; agent tools; provider plugins; usage-source plugins; client contributions (screen, sidebar header/footer, settings screen, workspace/agent/file panels, command center, slash commands, header buttons, composer pills, attachment sources, themes, timeline transformer/renderer, translations, playAudio); registry + catalog sync + bundle cache; Plugins settings page; CLI plugin commands; built-in plugins.

## 8 Contracts found missing in the first pass

### from gaps.md

- Permission cards and plan/mode approval
- Agent question forms and answer encoding
- Nonblocking Codex questions and durable pending requests
- Permission event fan-out, cancellation and mode policy
- Agent attention policy — corrections and missing boundary details
- CLI permission listing, allow and deny
- Host agent profiles: create, edit, remove and reorder
- Applying profiles to drafts and existing agents
- Legacy favorites migration and account boundary
- Agent skills: coverage boundary and provider-native skills
- Agent titles versus workspace generated titles
- Metadata generation selection and project prompt overrides
- Built-in appearance themes and system theme
- Plugin-contributed themes
- Fonts, text sizes, content width, syntax palette and preview
- Localization mechanism and language selection
- Onboarding state machine, completion and reopening
- Onboarding step effects and project-history suggestion integration
- Validated local storage, settings migrations and hydration safety
- Replica cache: offline paint, row storage, budgets and invalidation
- Performance controls and local render tracing
- Workspace label journal limits and uncertain-storage recovery
- Subagent track, child navigation and archive/detach actions
- Hub CLI login, credentials and guided onboarding
- Hub daemon relationship, grants, reconnect and revocation
- Hub-owned agent execution and control
- Hub triggers as code: export, deploy and project bundles
- Web overlay layering, focus trap and shortcut isolation

- Permission tool/plan/mode cards and provider-defined actions
- Plan markdown and implementation follow-up turn
- Question navigation, radio/checkbox, custom answer, preview and dismiss semantics
- Question response failure/retry and header-keyed encoding
- Codex asynchronous questions: active-turn steer and idle follow-up
- Runtime permission maps, persistence metadata and offline stripping
- Cross-client request/resolution events, direct acknowledgement and cancellation
- Presence selection, delegated callback suppression and unread corrections
- CLI permit listing, batch allow default, deny flags and confirmation errors
- Profile CRUD, order, notes, glyph/color and catalog-backed form
- Profile draft apply, live bundle apply, invalid-mode omission and partial failure
- Legacy favorite migration markers, deduplication and catalog retry
- Provider account/usage boundary and absent core account switcher
- Bundled skill install versus native provider skill autocomplete
- Agent provisional/manual titles versus generated workspace names
- Metadata Automatic/Preferred candidate order and fallback
- Project title/branch/commit/PR instruction overrides
- Built-in theme picker, system adaptation and Android system bars
- Plugin theme validation, multi-host deduplication and missing-theme fallback
- Font-family validation, per-axis text sizes, width reset, syntax theme and preview
- Offline locale backend, system tag resolution and plugin text boundary
- Onboarding gate/completion/reopen and step visit tokens
- Onboarding step effects, early project suggestions and late-result guarding
- Validated JSON/Zustand storage, hydration protection and settings migration markers
- SQLite/IndexedDB replica rows, checkpoint repair, timeline/byte budgets and eviction
- Browser render profiling, native trace gates and explicit query freshness
- Workspace label journal catch-up, compound commit and uncertain-storage block
- Managed/provider subagent track, parent pill/callout and nested timelines
- Archive finished progress/retry, local provider hide, detach and tab-close policy
- Hub browser login, per-origin credentials and explicit automation grant
- Hub relationship state, permission update, liveness/retry and revocation
- Hub execution idempotency, authority generation, owned events and control
- Hub trigger export, validate-only deploy, sequential install and legacy bundles
- Web overlay relative layers, focus trap/restore, IME and DOM popover Escape
