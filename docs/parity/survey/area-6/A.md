# Area 6 - PART A: Schedules, heartbeats, saved prompts, MCP tools, orchestration skills

Source root: `~/refs/paseo-latest/packages` (Paseo 0.15.4). Paths below are relative to it unless prefixed.
Abbreviations: `proto` = packages/protocol/src, `srv` = packages/server/src, `app` = packages/app/src, `cli` = packages/cli/src, `client` = packages/client/src.

## Schedules (data model, storage, cadence)

**What the user sees**: schedules exist as JSON files on the daemon host; the app lists them (see "Schedules screen"), CLI `paseo schedule ...`, agents via MCP `*_schedule` tools.

**Data model** (`proto/schedule/types.ts`):
- `ScheduleStatusSchema` = `"active" | "paused" | "completed"` (types.ts:4).
- `ScheduleCadenceSchema` (types.ts:7-17), discriminated on `type`:
  - `{type:"every", everyMs: int>0}` (legacy rolling interval; server still executes it: `srv/server/schedule/cron.ts:90` "COMPAT(scheduleEveryMs) ... remove after 2027-01-17").
  - `{type:"cron", expression: string trim min1, timezone?: string trim min1}`.
- `ScheduleTargetSchema` (types.ts:20-42):
  - `{type:"agent", agentId: guid}` -> existing agent (UI calls it a "heartbeat", `app/utils/schedule-format.ts:31`).
  - `{type:"new-agent", config:{provider (AgentProviderSchema), cwd (min1), modeId?, model?, thinkingOptionId?, archiveOnFinish?: bool, isolation?: "local"|"worktree", title?: string|null, providerOptions?: record, featureValues?: record, systemPrompt?: string, mcpServers?: record}}`.
  - There is NO "project/workspace" target type. A "project" choice in the UI is just a `cwd` (the project's `repoRoot`) plus `isolation` ("local" = directory workspace, "worktree" = new Paseo worktree workspace per run); see "Schedule form".
- `ScheduleRunSchema` (types.ts:45-55): `{id, scheduledFor, startedAt, endedAt|null, status:"running"|"succeeded"|"failed", agentId: guid|null, workspaceId?: string|null, output|null, error|null}`.
- `StoredScheduleSchema` (types.ts:58-73): `{id, name|null, prompt (min1), cadence, target, status, createdAt, updatedAt, nextRunAt|null, lastRunAt|null, pausedAt|null, expiresAt|null, maxRuns: int>0|null, runs: ScheduleRun[]}`. `ScheduleSummarySchema` = same minus `runs` (types.ts:76).
- `CreateScheduleInput` / `UpdateScheduleInput` / `UpdateScheduleNewAgentConfig` (types.ts:81-109). `ScheduleExecutionResult {agentId|null, output|null}`.

**Storage** (`srv/server/schedule/store.ts`): directory `<paseoHome>/schedules/` (`service.ts:292`, `paseoHome` default `$PASEO_HOME` or `~/.paseo`, `srv/server/paseo-home.ts:15`). One file per schedule: `<id>.json`, pretty-printed JSON (2-space) written atomically through `writeJsonFileAtomic` (`srv/server/atomic-file.ts:70`; temp file + rename; Windows rename retried up to 1000 ms with 10..100 ms backoff). Id = `randomBytes(4).toString("hex")` (8 hex chars, store.ts:12). `list()` (store.ts:123) reads every `*.json`, validates with `StoredScheduleSchema`, skips + logs once per file invalid ones ("Skipping invalid schedule file"), sorts by `createdAt`. All mutations serialised per schedule id (`serializeScheduleMutation`, store.ts:231) and per (name,target) identity (`upsertByNameAndTarget`, store.ts:188). Runs history is stored inside the schedule file (`runs[]`, never trimmed: not found any cap; looked in service.ts, store.ts).

**Cadence helpers**:
- 5-field cron only. `parseCronExpression` (`proto/schedule/cron-expression.ts:107`): fields minute 0-59, hour 0-23, day-of-month 1-31, month 1-12, day-of-week 0-6 (no 7, no names, no `@aliases`); supports `*`, `a-b`, lists `,`, steps `/n` on `*` and ranges (not on a single value). Errors e.g. `Cron expressions must have 5 fields`, `Invalid cron <field> range|value|step|field`. Day-of-month and day-of-week are ANDed (all five must match; not standard cron OR).
- `computeNextRunAt(cadence, after)` (`srv/server/schedule/cron.ts:88`): `every` -> `after + everyMs`; cron -> scan minute by minute from next whole UTC minute, up to `limit = 366*24*60` minutes (cron.ts:97), else throws `Unable to compute next run time for cron expression`. Timezone via `Intl.DateTimeFormat` with `hourCycle h23`; no tz = UTC. `validateScheduleCadence` also asserts tz (`Invalid cron time zone: <tz>`).
- `everyMsToFiveFieldCron(everyMs)` (`proto/schedule/cadence.ts:5`): converts a rolling interval to an equivalent cron only when calendar boundaries do not change it (minutes dividing 60, 60 min, whole hours dividing 24, 24h); else `null`.

## Schedule service (scheduler loop and run execution)

**How it works** (`srv/server/schedule/service.ts`):
- Created in `srv/server/bootstrap.ts:1447` with `paseoHome, logger, agentManager, agentStorage, createAgent, createDirectoryWorkspace, createPaseoWorktreeWorkspace, archiveWorkspace`; `await scheduleService.start()` (bootstrap.ts:1457), `stop()` on shutdown (bootstrap.ts:2014). `agentManager.setAgentArchivedCallback` -> `scheduleService.completeForAgent(agentId)` (bootstrap.ts:1459-1465): when an agent is archived, all its non-completed `agent`-target schedules become `completed`.
- `start()` (service.ts:303): `recoverInterruptedRuns()` then `sweepOrphanedSchedules()`, then `setInterval(tick, SCHEDULE_TICK_INTERVAL_MS = 1000)` (service.ts:29), timer unref'd.
  - Recovery (service.ts:613): any `running` run -> `failed`, `endedAt=now`, `error="Daemon restarted before the scheduled run completed"`; interrupted new-agent workspace archived if `archiveOnFinish ?? true`; overdue `nextRunAt` skipped forward (no catch-up/backfill of missed slots); may complete schedule if limits reached.
  - Sweep (service.ts:701): `agent` target whose agent record is missing or archived -> `completed`.
- `tick()` (service.ts:570): for each `active` schedule with `nextRunAt`: skip if `runningScheduleIds` has it (**overlap rule: at most one in-flight run per schedule; a due slot is just skipped/deferred while running**); if `shouldCompleteSchedule` (expired: `expiresAt <= now`, or completed runs >= `maxRuns`) -> complete; if `nextRunAt > now` skip; else `runSchedule`.
- `claimDueRun` (service.ts:775): atomically appends a `running` run (id = `randomUUID()`, `scheduledFor = nextRunAt`) and advances `nextRunAt` (`computeNextRunAt` then `advanceNextRunAtPast(now)`), or `null` if this is the final run (`countCompletedRuns+1 >= maxRuns`). `countCompletedRuns` counts runs whose status != running (failed runs count against maxRuns).
- `finishRun` (service.ts:839): sets run status/endedAt/agentId/output/error, `lastRunAt`; `ScheduleTargetGoneError` -> schedule completed immediately (even manual runs); manual runs never advance cadence; paused schedule gets `nextRunAt=null`; else re-checks completion and moves overdue cursor.
- `runOnce(id)` (service.ts:558): errors `Schedule <id> is already completed` / `already running` (`ScheduleStateConflictError`); appends a manual run (`scheduledFor=startedAt=now`) and executes immediately without touching cadence; also allowed on paused schedules.
- `pause` (service.ts:418): completed -> conflict error; paused -> no-op; sets `status=paused, nextRunAt=null, pausedAt=now`. `resume` (service.ts:438): `status=active`, `nextRunAt = computeNextRunAt(cadence, now)` (next future slot; never runs immediately).
- `create` (service.ts:325): trims prompt (`Schedule prompt is required`), validates cadence, `runOnCreate = input.runOnCreate ?? (cadence.type === "every")` (service.ts:347): when true `nextRunAt = now` (fires on the next tick), else next cron slot. `expiresAt` stored as given (ISO string), `maxRuns` via `normalizeMaxRuns` (`maxRuns must be a positive integer`).
- `createOrReplace` (service.ts:369): idempotent upsert keyed by (trimmed name, target identity) among non-completed schedules, used by the MCP write path (babysit-pr re-registering its heartbeat); update path resets `status=active, pausedAt=null`, recomputes `nextRunAt`, replaces `expiresAt`/`maxRuns`. Unnamed -> always creates.
- `update` (service.ts:458): patch of `prompt`, `name` (trim -> null if empty), `cadence` (a cron without timezone keeps the existing timezone: `mergeScheduleCadenceTimezone`; recomputes `nextRunAt` only if status active, else null), `newAgentConfig` (only valid for new-agent targets: error `new-agent config updates are only valid for new-agent target schedules`; `applyNewAgentConfig` service.ts:90 trims provider/cwd and deletes model/modeId/thinkingOptionId when null/empty), `maxRuns`, `expiresAt`. Editing does not change `status` (a `completed` schedule stays completed; no un-complete path found).
- `delete(id)`: removes file; no state check, no error if missing.
- `logs(id)`: returns runs sorted by `startedAt` ascending.

**Run execution** (`executeSchedule`, service.ts:927):
- Target `agent` (heartbeat): prompt = `formatSystemNotificationPrompt("Schedule \"<name>\" fired (id=<id>, run=<runId>).\n<prompt>")` (service.ts:75); errors `ScheduleTargetGoneError` if agent record missing/archived; `ensureAgentLoaded`; error `Agent <id> already has an active run` if `hasInFlightRun`; `startAgentRun(..., {replaceRunning:true, activeTurnBehavior:"steer"})`; then `waitForAgentEvent(waitForActive:true)`; failure if waiting on permission (`Scheduled agent <id> is waiting for permission`) or status error. Output = agent's `lastMessage`.
- Target `new-agent`: `assertNewAgentCwdDirectory(cwd)` (ENOENT/not a directory -> `ScheduleTargetGoneError`); creates a workspace per run: isolation `local` (default) -> `createDirectoryWorkspace`, `worktree` -> `createPaseoWorktreeWorkspace` (service.ts:1062); records `workspaceId` on the run (`recordRunWorkspace`); `createAgent({kind:"mcp", provider: provider[/model], labels {"paseo.schedule-id","paseo.schedule-run"}, unattended:true, promptFailure:"return-error", notifyOnFinish:false, mode, thinking, features})`; then `agentManager.runAgent(agent.id, prompt)`; waits for completion; output = `buildRunOutput` (lastMessage, else final text, else curated timeline). In `finally`, workspace archived when `agentId===null || (archiveOnFinish ?? true)` (service.ts:156) -> default is archive the run workspace (and agent) after each run.
- Title for run agent: `resolveCreateAgentTitles({configTitle, initialPrompt})`.

**States/limits**: constants `SCHEDULE_TICK_INTERVAL_MS=1000`; cron lookahead 366 days; error classes `ScheduleTargetGoneError`, `ScheduleNotFoundError` (`Schedule not found: <id>`), `ScheduleStateConflictError`.

**Platforms**: daemon-only (Node); app/CLI/MCP are clients.

**Connections**: agent archive (completes schedules), workspace create/archive (worktree isolation), agent manager (`createAgent`, `runAgent`, steer), system-notification prompt wrapper, permissions (a run pausing on a permission request fails), timeline curation (`curateAgentActivity`).

## Schedule RPC (protocol, session, client)

**Protocol** (`proto/schedule/rpc-schemas.ts`, re-exported into `proto/messages.ts:84`): requests (all carry `requestId`):
| type | fields |
|---|---|
| `schedule/create` | `prompt` (min1), `name?`, `cadence`, `target` = `self{agentId}` | `agent{agentId}` | `new-agent{config}`, `maxRuns?` (int>0), `expiresAt?` (string), `runOnCreate?` (bool) (rpc-schemas.ts:27) |
| `schedule/list` | - |
| `schedule/inspect` | `scheduleId` |
| `schedule/logs` | `scheduleId` |
| `schedule/pause` / `schedule/resume` / `schedule/delete` / `schedule/run-once` | `scheduleId` |
| `schedule/update` | `scheduleId`, `name?: string|null`, `prompt?`, `cadence?`, `newAgentConfig?{provider?, model?|null, modeId?|null, thinkingOptionId?|null, archiveOnFinish?, isolation?, cwd?}`, `maxRuns?: int>0|null`, `expiresAt?: string|null` (rpc-schemas.ts:90) |
Responses `<type>/response` with `payload{requestId, error|null, ...}`: create/pause/resume -> `schedule: ScheduleSummary|null`; list -> `schedules: ScheduleSummary[]`; inspect/run-once/update -> `schedule: StoredSchedule|null` (with runs); logs -> `runs: ScheduleRun[]`; delete -> `scheduleId`. `self` target (legacy) is converted to `agent` in the handler.

**Server**: `srv/server/session.ts:3219` `dispatchScheduleMessage` -> `ScheduleSession` (`srv/server/session/schedule/schedule-session.ts`): `handleScheduleCreateRequest` :67, `List` :97, `Inspect` :115, `Logs` :133, `Pause` :151, `Resume` :169, `Delete` :187, `RunOnce` :205, `Update` :223. Each wraps the service call; summary responses strip `runs` (`toScheduleSummary` :26). On any exception emits `rpc_error {requestId, requestType, error: message, code: "schedule_request_failed"}` (:56) instead of putting the error in the payload (payload.error is always null on success path).
- No push event/subscription for schedules exists (looked in proto/messages.ts, session.ts, schedule-session.ts): the app polls via `schedule/list`.

**Client** (`client/daemon-client.ts`): `scheduleCreate` :6628, `scheduleList` :6645, `scheduleInspect` :6655, `scheduleLogs` :6666, `schedulePause` :6677, `scheduleResume` :6688, `scheduleDelete` :6699, `scheduleRunOnce` :6710, `scheduleUpdate` :6721; each uses `sendCorrelatedSessionRequest` with `responseType`; option types `CreateScheduleOptions` :888, `UpdateScheduleOptions` :939. Create only sends `name/maxRuns/expiresAt/runOnCreate` when set.

## Schedules screen (app)

**What the user sees**: route `/schedules` (`app/app/schedules.tsx` wraps `SchedulesScreen` in `HostRouteBootstrapBoundary`; `app/_layout.tsx:809,841`; route builder `utils/host-routes.ts:428`). Entry points: sidebar header nav row "schedules" (`sidebar-nav/model.ts:11`, `components/sidebar/sidebar-nav-rows.tsx:179`, label key `sidebar.sections.schedules`), Command Center item `id:"schedules"` with keywords schedules/scheduled/automation/recurring (`command-center/root-registration.tsx:231`). Screen `screens/schedules-screen.tsx`: renders blank `View` unless `useIsFocused()` (:56); `MenuHeader title="Schedules"`.
- Layout: filter row = optional `HostFilter` (only if `hosts.length > 1`) + `SegmentedControl` (`Active` = bucket "runnable" testID `schedules-filter-active`, `Ended` = bucket "ended" `schedules-filter-ended`) + `New schedule` button (`schedules-new`); scroll list `schedules-list`.
- Body states (`screens/schedules-screen-state.ts`): `loading` (spinner; also "connecting"), `load-error` ("Unable to load schedules" + "Try again" `schedules-retry`), `empty` (no schedules at all: CalendarClock icon, "No active schedules", "Schedules run agents on a cadence.", link "See docs" -> `https://paseo.sh/docs/schedules`, button `schedules-empty-new`), `content`. Filter-empty states: Active tab empty -> same empty card; Ended tab empty -> "No ended schedules". Host errors banner (`schedules-host-errors`): one line per failing host `"<serverName>: Could not load schedules"`.
- List sorted by `createdAt` descending (:152). Rows in one settings-style card (`components/schedules/schedules-table.tsx`).

### Aggregated schedules across hosts
`hooks/use-schedules.ts`: `useSchedules()` -> react-query (`useFetchQuery`) key `["schedules", sortedServerIds.join("|"), connectionStatusKey]`, `staleTimeMs: 5_000`, `dataShape:"list"`. `schedules/aggregated-schedules.ts` `fetchAggregatedSchedules`: for every host whose runtime snapshot `connectionStatus==="online"` with a client, `client.scheduleList()` in parallel; tags each row `serverId`, `serverName`; offline hosts skipped; per-host failure -> `hostErrors`; if every attempted host failed and no schedules -> throws `ALL_SCHEDULE_HOSTS_FAILED_MESSAGE = "No connected hosts could load schedules"`; if nothing askable and some hosts `connecting|idle` -> `{status:"connecting"}`. Refetch triggers: any mutation `onSettled` invalidates `["schedules"]` (`hooks/use-schedule-mutations.ts:192`); host reconnect (`runtime/host-runtime.ts:2559` invalidates `schedulesQueryBaseKey` on transition to online). No polling interval, no daemon push.

### Row display, derived state, and row actions
- `schedules/schedule-derivation.ts` `resolveSchedule`: derived state precedence `expired` (`expiresAt <= now`) > `targetGone` (agent target not in the client agent directory, only when that host's `hasEverLoadedAgentDirectory`) > `finished` (status completed) > `paused` > `active`; bucket `runnable` = active|paused, `ended` = expired|finished|targetGone. Badge (`schedule-row.tsx:78`): Active (success), Paused/Expired/Finished (muted), "Target gone" (error).
- Title: `resolveScheduleTitle` (`utils/schedule-format.ts:35`) = name, else new-agent `config.title`, else first non-empty prompt line, else `Untitled heartbeat|schedule`. `scheduleProductName`: target `agent` => "Heartbeat", else "Schedule".
- Target line: agent title or "Untitled agent"/"Agent unavailable" for heartbeats; for new-agent: matched project name (by `serverId:cwd`) else shortened cwd (`schedule-project-targets.ts:396`); provider glyph.
- Meta line (`buildMeta` schedule-row.tsx:99): `[host name if >1 host ·] <cadence label> · Created <ago> · Last run <ago>|Never run · Next run in Nm/Nh/Nd|soon (only when active)`. `formatCadence` (`schedule-format.ts:86`): `Every N minute(s)/hour(s)/day(s)` for legacy `every`; `describeCron` humanises only fixed-time shapes (`Every minute`, `Every hour`, `Every hour at :MM`, `Daily|Weekdays|Weekends|<Day>s at HH:MM <tz>`), else raw expression.
- Kebab menu (`schedule-row.tsx:262-330` cadence actions + Edit/Delete; tapping the row = Edit): for new-agent schedules shows ONE of `Resume schedule` (when status paused, pending label "Resuming...", testID `schedule-menu-resume-<id>`) or `Pause schedule` (otherwise, "Pausing...", disabled when completed), plus `Run now` ("Starting..."), then Edit and Delete. `canRun = target.type==="new-agent" && state in (active|paused)` (disables them when ended). **For heartbeat (agent-target) rows the pause/resume/run-now items are not rendered at all** (`schedule-row.tsx:275` returns null); only Edit and Delete exist. Delete asks `confirmDialog({title:"Delete schedule|heartbeat", message:'Delete "<title>"? This cannot be undone.', confirmLabel:"Delete", destructive:true})` (`schedules-table.tsx:114`). Per-row pending flags (`pause|resume|runNow|delete`); errors swallowed (no toast; mutation rolls back and refetches).
- Mutations (`hooks/use-schedule-mutations.ts`): pause/resume do optimistic status flip (`optimisticallySetStatus`), delete optimistic removal, all with snapshot rollback on error; create/update/run-now just invalidate. All use `useSessionStore.getState().sessions[serverId].client`; error `common.errors.daemonClientUnavailable` when missing.
- No UI to view run history/logs, `expiresAt`, or `runOnCreate`: not found (looked in components/schedules/*, screens/schedules-screen.tsx). Run history is CLI/MCP only.

## Schedule form (create / edit sheet)

**What the user sees**: `components/schedules/schedule-form-sheet.tsx` (`AdaptiveModalSheet`, testID `schedule-form-sheet`). Title `New schedule` | `Edit schedule` | `Edit heartbeat`; footer `Cancel` + `Create schedule`/`Save changes` (`schedule-form-submit`).
- Heartbeat edit (target `agent`): only read-only `Target` field (agent title) + `CadenceEditor`; submit sends `scheduleUpdate{id, cadence}` (`submitAgentTarget` :316). Name/prompt/maxRuns not editable here.
- New-agent: fields in order: `Name` (optional; also used as agent `title` on create), `Prompt` ("What should the agent do each run?"), `Max runs` (placeholder "Unlimited"; `parseMaxRuns` keeps only positive ints, else null = clear), `Host` (select; create only when >1 host, `resolveCreateServerId` :82), `Project` (select of project roots on that host; `showProjectField = edit || host selected`), `Model` (provider/model selector; shown once a project is chosen), `Thinking` (only if the model has thinking options), `Mode` (only if provider has modes), `Isolation` (Local/Worktree; only when selected project is a git repo and host `supportsWorkspaceMultiplicity`: `canUseWorktreeIsolation`), `Archive on finish` switch (default true; shown when host supports workspace multiplicity), then `CadenceEditor`.
- `CadenceEditor` (`components/schedules/cadence-editor.tsx`): preset select `CADENCE_PRESET_OPTIONS` (`schedules/schedule-cadence-options.ts:19`): Every minute `* * * * *`, Every hour `0 * * * *`, Daily 9:00 `0 9 * * *`, Weekdays 9:00 `0 9 * * 1-5`, Mondays 9:00 `0 9 * * 1`, else "Custom cron"; plus a monospace cron text field (placeholder `0 9 * * *`, testID `cadence-cron-expression`), inline error from `validateCron` (`utils/schedule-format.ts:155`, "Enter a cron expression" or `Invalid <field> ...`) or human preview (`describeCron`). Timezone is not user-editable: the device timezone (`getDeviceTimeZone`) is attached for new schedules; existing cron keeps its tz (`nextCronCadence` `utils/schedule-cadence-policy.ts`), legacy `every` converted via `normalizeScheduleFormCadence` -> cron (`schedule-cadence-options.ts:194`). Default cadence in form model `{type:"every", everyMs:3_600_000}` (`schedule-form-model.ts:~143`) normalised to cron on open; `DEFAULT_TIMEZONE="UTC"`.
- Create (`submitNewAgent` :327): `scheduleCreate{prompt, name?, cadence (must be cron: requireCronCadence), target:{type:"new-agent", config:{provider, cwd, model?, modeId?, thinkingOptionId?, archiveOnFinish?, isolation?, title: name}}, maxRuns?}`; never sends `expiresAt` or `runOnCreate` (so cron schedules first fire at the next slot). Edit sends `scheduleUpdate{id,name|null,prompt,cadence?,newAgentConfig{provider,model|null,modeId|null,thinkingOptionId|null,cwd,archiveOnFinish?,isolation?},maxRuns}` (maxRuns null clears).
- Submit gating (`schedule-form-model.ts` `resolveCanSubmit`): heartbeat: needs a cadence; new-agent: non-empty prompt, workingDir set, a matched project on create, a valid selected provider/model; plus `cadenceError === null` and not submitting. Errors from the daemon appear in red under the fields (`submitError`).
- Form state machine: `schedules/schedule-form-model.ts` (external-store model: `getState/subscribe/setHost/setProject/setModel/setThinking/setSessionMode/setName/setPrompt/setMaxRuns/setCadence/setIsolation/setArchiveOnFinish/setSubmitError/applyProviderSnapshot`), hook `schedules/use-schedule-form-model.ts`, provider list/model resolution via `schedules/use-schedule-form-provider-snapshot.ts` (reuses create-agent form resolution `provider-selection/resolve-agent-form`). Selecting provider/model persists last choice to form preferences (`persistPreferences` :294, `updateSelectionPreferences` incl. `isolation`).
- Project options (`schedules/schedule-project-targets.ts:352` `buildScheduleProjectTargets`): one per online host of each project from `useProjects()` with non-empty `repoRoot`; `isGit = Boolean(host.gitRuntime)`.

**Platforms**: same React Native screen on web/desktop/mobile; hover highlight on rows only when not compact (`useIsCompactFormFactor`); sheet is adaptive (bottom sheet on compact).

**Connections**: host/daemon list (`useHosts`), projects (`useProjects`), agent directory (`useAggregatedAgents includeArchived`), provider/model catalog and form preferences, command center, sidebar nav.

## Schedule CLI (`paseo schedule ...`)

Registered in `cli/cli.ts:147`; all commands take `--json` and daemon host options (`addJsonAndDaemonHostOptions`; `--host`). `cli/commands/schedule/index.ts`.
- `create <prompt>`: `--every <duration>` (compiled to cron through `everyMsToFiveFieldCron`; error `UNREPRESENTABLE_CADENCE` "cannot be represented faithfully by five-field cron ... Use --cron"), `--cron <expr>`, `--timezone <iana>` (only with `--cron`, default UTC), `--name`, `--provider <provider|provider/model>`, `--mode`, `--thinking <id>`, `--cwd <path>` (default current dir; **required when `--host` endpoint given**: `MISSING_CWD`), `--run-now` (sends `runOnCreate:true`; omitted => `runOnCreate:false` from CLI), `--max-runs <n>`, `--expires-in <duration>` (`parseDuration`: `5m 30s 1h 2h30m 1d`, bare number = seconds; `cli/utils/duration.ts`), hidden `--target` (`new-agent` | `self` (needs `PASEO_AGENT_ID`; COMPAT(scheduleSelfTarget), remove after 2027-01-17) | `<agentId>`). Exactly one of `--every`/`--cron` required (`INVALID_CADENCE`). Output columns ID, NAME, CADENCE, TARGET, STATUS, NEXT RUN. Code `shared.ts:275` `parseScheduleCreateInput`.
- `ls`: lists only `new-agent` targets (heartbeats filtered out, `ls.ts`). `inspect <id>`: key/value rows (Id, Name, Prompt, Cadence, Target, Status, CreatedAt, UpdatedAt, NextRunAt, LastRunAt, PausedAt, ExpiresAt, MaxRuns, RunCount); `--json` returns full record. `logs <id>`: rows RUN ID, STATUS, STARTED, AGENT (7 chars), OUTPUT, ERROR. `pause|resume|run-once <id>` call `requireNewAgentSchedule` first (error for heartbeats). `delete <id>`: no target check; outputs `{id,status:"deleted"}`. `update <id>`: `--every|--cron|--timezone|--name ("" clears)|--prompt|--provider|--model|--mode|--cwd|--max-runs <n>|--no-max-runs|--expires-in|--no-expires-in`; `NO_UPDATES` if nothing given; conflicting flags error (`CONFLICTING_MAX_RUNS`, `CONFLICTING_EXPIRES`); requires new-agent target; no `--thinking`/`--isolation`/`--archive-on-finish` in update.
- Error codes: `SCHEDULE_CREATE_FAILED|LIST_FAILED|UPDATE_FAILED|DELETE_FAILED|LOGS_FAILED`, `DAEMON_NOT_RUNNING`.

## Heartbeats

A heartbeat = a schedule with `target:{type:"agent", agentId}` (recurring prompt injected into an existing agent). Not a separate store/protocol: same `schedule/*` messages, same `<paseoHome>/schedules/*.json`. UI wording only: `scheduleProductName` (`app/utils/schedule-format.ts:31`), "Edit heartbeat" sheet.
- CLI `paseo heartbeat` (`cli/commands/heartbeat/index.ts`, registered `cli.ts:148`): must run inside an agent (`PASEO_AGENT_ID`, else `Heartbeat commands must run inside a Paseo agent`). `create <prompt> --cron <expr> [--timezone] [--name] [--max-runs] [--expires-in]` (target = caller agent); `update <id> --cron [--timezone]`; `delete <id>`; update/delete verify ownership (`Heartbeat <id> does not belong to agent <agentId>`, index.ts:43). No list/pause/resume in CLI.
- Execution: see service `executeSchedule` agent branch (steers into running turn: `replaceRunning:true, activeTurnBehavior:"steer"`; fails if agent already has in-flight run or awaits permission). Auto-completes when the agent is archived/deleted (`completeForAgent`, sweep at startup, `ScheduleTargetGoneError`).
- MCP tools `create_heartbeat`/`delete_heartbeat` (see MCP section).
- Internal unrelated "heartbeat" names (not features): client liveness heartbeat `client/daemon-client.ts:2620`, Android `HeartbeatWatchdog.kt`.

## Saved prompts / prompt library

Not found. Searched `packages/app/src`, `server/src`, `protocol/src`, `client/src`, `cli/src`, `plugin` (case-insensitive) for `saved prompt`, `savedPrompt`, `prompt library`, `promptLibrary`, `prompt snippet`, `prompt template`; only unrelated hits: `server/src/server/agent/providers/codex-app-server-agent.ts:858 expandCodexCustomPrompt` (Codex's own custom-prompt slash commands expanded for the Codex provider, a provider command feature, not a Paseo library) and `protocol/src/terminal-profiles.ts:238` (a CLI flag string). Prompt text that persists in Paseo: schedule `prompt` field, `agents.resumePrompt` daemon setting (`persisted-config.ts:353`), `appendSystemPrompt` card (`AppendSystemPromptCard`, host page). None of those is a user prompt library.

## MCP server exposed to agents (transport, auth, injection, scoping)

**What the user sees**: Settings > Host page card "Paseo tools" (switch labelled `settings.host.orchestration.enableTools.title`, hint `...enableTools.hint`) = `PaseoToolsCard` (`app/screens/settings/paseo-tools-card.tsx:9`, testID `host-page-inject-mcp-card`, mounted `screens/settings/host-page.tsx:294`; returns null when host disconnected). The same row (`PaseoToolsRow`, :33) is reused in onboarding step `onboarding/internal/steps/orchestration-step.tsx:16`. Switch value = `config.mcp.injectIntoAgents !== false` (undefined shows ON), toggling calls `patchConfig({mcp:{injectIntoAgents: next}})` (`useDaemonConfig(serverId)`). There is no per-tool UI for `disabledTools` in the app (grep `paseoTools|disabledTools` in app/src: only i18n/none). Per-provider policy is config-file only.
- CLI/daemon flags: `--no-mcp` (-> `daemon.mcp.enabled=false`) and `--no-inject-mcp` (-> `daemon.mcp.injectIntoAgents=false`) (`cli/commands/daemon/local-daemon.ts:88-89`, `srv/server/daemon-worker.ts:113-121`). Persisted config keys `daemon.mcp.enabled` / `daemon.mcp.injectIntoAgents` (`srv/server/persisted-config.ts:255-259`, daemon-config-store paths `daemon-config-store.ts:184-185,212-213`, live-patchable fields `mcp.enabled`, `mcp.injectIntoAgents`). Env `MCP_DEBUG=1` -> `config.mcpDebug` logs each MCP request (authorization masked as "[redacted]", bootstrap.ts:256,1665-1671).
- Default discrepancy (not resolved by reading): `config.ts:552-554` resolves `mcpEnabled ?? true` and `mcpInjectIntoAgents ... ?? false`, but `bootstrap.ts:584` seeds the mutable config with `injectIntoAgents: config.mcpInjectIntoAgents ?? true`, and `bootstrap.ts:1604` uses `config.mcpInjectIntoAgents !== false`; the app treats unset as ON. Since `loadConfig` already returns a boolean (false when unset), effective boot default per code = OFF unless persisted true; verify empirically before porting.

**Transport and endpoint** (`srv/server/bootstrap.ts:1609-1700`, `srv/server/agent/mcp-server.ts`):
- Streamable-HTTP MCP at `POST /mcp/agents` on the daemon's main TCP listener (`createAgentMcpBaseUrl` bootstrap.ts:274, only when listening on TCP; none for unix socket). GET/DELETE routes mounted but return 405 JSON-RPC error (`-32000 "Method not allowed"`). Stateless: a fresh `McpServer` (`name:"agent-mcp", version:"2.0.0"`, mcp-server.ts:33) + `StreamableHTTPServerTransport({sessionIdGenerator: undefined, enableDnsRebindingProtection:false})` is built per HTTP request and closed on response close (no session map).
- Disabled -> `404 {error:"Agent MCP endpoint disabled"}` when `mcpEnabled` false.
- Auth: route is exempt from the global bearer middleware (`srv/server/auth.ts:115 SELF_AUTHENTICATING_ROUTES`) and authorises itself with `isAgentMcpRequestAuthorized` (auth.ts:134): if no daemon password configured -> open; else accepts either the per-daemon-run capability token (`agentMcpAuthToken = randomUUID()`, bootstrap.ts:712, constant-time compare) or a valid daemon-password bearer. Token is only injected into local agent configs, never sent to remote clients. Failure -> `401 {error:"Unauthorized"}`.
- Per-agent scoping: caller identity = query param `?callerAgentId=<agentId>` (bootstrap.ts:1692); `createAgentToolHostDependencies({callerAgentId, paseoToolPolicy: agentManager.getPaseoToolPolicy(callerAgentId)})` builds a catalog bound to that agent. Without callerAgentId the catalog is "top-level" scope (used by external/daemon clients): tool schemas/behaviour differ (see below). The id is not cryptographically bound to the token (any holder of token/password can claim any callerAgentId).
- Native (non-HTTP) delivery: providers with `capabilities.supportsNativePaseoTools` (opencode with bridge `opencode-agent.ts:1423`, opencode v2 `opencode/v2/agent.ts:62`, omp `omp/agent.ts:481`) get the same catalog in-process via `launchContext.paseoTools` (`agent-manager.ts:5620-5635`) and the injected `paseo` HTTP server is stripped (`resolveProviderLaunchConfig` agent-manager.ts:5650); OpenCode gets a manifest through `OpenCodeBridge.setManifestCatalog` (`agent/provider-runtime.ts:41`, `bootstrap.ts:1598-1602`); omp bridges via `omp/mcp-bridge.ts`, `omp/host-tools.ts`. Plugin provider capability false (`plugin-provider.ts:1812`).

**Injection into agent config** (`srv/server/agent/runtime-mcp-config.ts`, `agent-manager.ts:5530`): `prepareSessionConfig` -> `withRuntimePaseoMcpServer({config, agentId, mcpBaseUrl, mcpAuthToken})` adds `mcpServers.paseo = {type:"http", url: "<base>/mcp/agents?callerAgentId=<agentId>", headers:{Authorization:"Bearer <token>"} (only if token)}` (`PASEO_MCP_SERVER_NAME = "paseo"`). The injected entry is NOT persisted: `stripInternalPaseoMcpServer` removes any `paseo` http/sse server whose path is `/mcp/agents` from the stored config (so URLs/tokens are rebuilt every launch). A user-defined `paseo` server wins (not overwritten). Injection only when `paseoToolsEnabled && policy.enabled !== false && mcpBaseUrl`. Also every agent process gets env `PASEO_AGENT_ID` and `PASEO_AGENT_CWD` (`agent-manager.ts:5611-5616`; used by `paseo heartbeat`/`--target self`). Live toggle: `daemonConfigStore.onFieldChange("mcp.enabled"|"mcp.injectIntoAgents")` updates `setMcpBaseUrl`, `setPaseoToolsEnabled`, provider catalog (bootstrap.ts:1769-1783); already-running sessions keep their launch config until reloaded.

**Tool visibility policy**: `ProviderPaseoToolsPolicy {enabled?: boolean; disabledTools?: string[]}` (`proto/provider-config.ts:31`), stored per provider in `providers.<id>.paseoTools` (provider override, provider-config.ts:63) and resolved by `resolvePaseoToolPolicy(providerId, daemonConfigStore.get().providers)` (`srv/server/agent/paseo-tool-policy.ts:7`, wired bootstrap.ts:1025). `isPaseoToolEnabled(policy, name)`: `speak` always allowed; `enabled===false` disables all; else `!disabledTools.includes(name)`. Applied inside `registerTool` (paseo-tools.ts:737-741: disabled tools are simply never registered, still added to `builtinNames` so plugin tools cannot collide) and to plugin tools (`plugin-tools.ts:83`). Policy is stored per agent at launch (`getPaseoToolPolicy(agentId)`, agent-manager.ts:963).

**Catalog scoping**: `createPaseoToolCatalog` (paseo-tools.ts:610): scope = `callerAgentId ? "agent" : "top-level"`; `createPaseoAgentToolManifest` (:614) builds an agent-scope catalog for plugin registration. Differences by scope: `create_agent`/`send_agent_prompt` `notifyOnFinish` default true (agent) vs false (top-level); `send_agent_prompt` `activeTurnBehavior` default steer (agent) vs interrupt; `create_schedule` provider optional in agent scope (inherits caller provider/model/mode/cwd/thinking/providerOptions/featureValues/systemPrompt/mcpServers, `buildCallerAgentScheduleConfig` :950) but required top-level; cwd resolution inherits caller cwd and honours `lockedCwd`/`allowCustomCwd` from `resolveCallerContext` (voice agents); heartbeats agent-scope only; browser tools need a workspace. Plugin tools (`agentManager.getPluginTools(callerAgentId)`) are appended last in agent scope only (`plugin-tools.ts`), name collision with built-ins throws.

**Result format** (`agent/tools/paseo-tool-serialization.ts`, `mcp-server.ts:17`): tools return `{content: [], structuredContent}`; `addModelVisibleStructuredContent` fills `content` with a text block = pretty JSON, prefixed by `<key>_count=N` and `<key>_ids=a,b` for top-level arrays of objects with `id`. Errors: `PaseoToolInputError` (status 400, includes zod issues) for invalid input/not found; unconfigured services throw `Error("... is not configured")`. Input validated with zod `safeParseAsync` (`parseToolInput` paseo-tools.ts:640; raw shapes are wrapped `z.object(shape).passthrough()`).

**Host fan-out** (`hostTools` map paseo-tools.ts:709): `create_agent, create_workspace, list_workspaces, send_agent_prompt, get_agent_status, get_agent_activity, list_agents` get an extra optional `serverId` param ("Destination server ID from list_hosts; omit for this host") and are executed through `createHostResolver` (`agent/host-operations/index.ts:195`) against another connected daemon's `DaemonClient`; response `structuredContent` gets `serverId` added; remote errors come back as `{isError:true, structuredContent:{serverId,error,code?,outcome?}}`; remote finish-notification subscriptions are registered locally (`prepareRemoteNotification` :667 / `registerRemoteCreation` :684, `finishSubscriptions().register`).

## MCP tool catalogue (`srv/server/agent/tools/paseo-tools.ts` unless noted)

Line = `registerTool(` call (handler follows immediately). "A"=agent-scope only, "T"=top-level differs. All outputs are `structuredContent` (+ auto text).

### Agents
| tool | params (type, default) | returns | line |
|---|---|---|---|
| `create_agent` | `title` string 1-60 req; `provider` "provider/model" req (e.g. `codex/gpt-5.4`); `initialPrompt` string req; `labels?` record<string,string>; `settings?` {modeId?, thinkingOptionId?, features?: record} strict; `workspaceId?` string (agent: defaults to caller workspace; top-level: creates new local workspace); `notifyOnFinish?` bool (default true agent / false top-level). Legacy accepted but hidden: `relationship{kind subagent|detached}`, `workspace{kind current|existing|create}`, `cwd`, `mode`, `thinking`, `features` (COMPAT nestedCreateAgentPlacement, remove after 2027-01-17); worktree placement args rejected ("use create_workspace first") | `{agentId, type(provider), status, cwd, workspaceId?, currentModeId|null, availableModes[], guidance?}` returns immediately (async); labels get `paseo.parent-server-id` for subagents; child created via `createAgentCommand({kind:"mcp", promptFailure:"throw", callerAgentId,...})` | 1589 (handler 1607) |
| `send_agent_prompt` | `agentId` string; `prompt` string; `sessionMode?` string; `activeTurnBehavior?` "steer"\|"interrupt" (steer for agent callers, interrupt otherwise); `notifyOnFinish?` bool (true agent / false top) | agent callers: `{success, deliveryStatus:"queued", messageId, status, guidance}` (queued through `agentMessages().enqueue`, delivered after two quiet seconds per recipient, source `{kind:"agent-message", agentId, title}`); top-level: `{success, disposition:"turn_started"\|"out_of_band"\|"steered", status, guidance}` after `waitForAgentRunStartWithTimeout` | 1944 |
| `get_agent_status` | `agentId` | `{status, snapshot: AgentSnapshotPayload}` (live or stored; internal agents -> not found) | 2077 |
| `get_agent_activity` | `agentId`; `limit?` number (most recent first) | `{agentId, updateCount, currentModeId|null, content: curated text}` (`ensureAgentLoaded`, `summarizeAgentActivity`) | 3151 |
| `list_agents` | `includeBackground?` false; `includeArchived?` false; `cwd?` (default caller cwd for agent callers); `sinceHours?` int 1..720 default 48; `statuses?` AgentStatus[]; `limit?` int 1..200 default 50 | `{agents: AgentListItemPayload[]}` | 2127 |
| `cancel_agent` | `agentId` | `{success}` (abort run, keep agent) | 2191 |
| `archive_agent` | `agentId` | `{success:true}` (soft delete; triggers schedule completion) | 2215 |
| `kill_agent` | `agentId` | `{success:true}` (`closeAgentCommand`, permanent) | 2244 |
| `update_agent` | `agentId`; `name?`; `labels?`; `settings?` {modeId?, model?|null, thinkingOptionId?|null, features?} strict | `{success}` | 2265 |
| `set_agent_mode` | `agentId`, `modeId` | `{success, newMode}` | 3190 |
| `list_pending_permissions` | none | `{permissions:[{agentId,status,request}]}` across all agents (sanitized request) | 3214 |
| `respond_to_permission` | `agentId`, `requestId`, `response` (AgentPermissionResponse) | `{success}` | 3248 |

There is no `wait_for_agent` tool (not found; waiting is replaced by finish-notification via `notifyOnFinish`, and guidance text says "Do not poll for status"). Looked in paseo-tools.ts and browser-tools/tools.ts.

### Workspaces and scripts
| tool | params | returns | line |
|---|---|---|---|
| `create_workspace` | `isolation` "local"\|"worktree" req; `path?` (default caller workspace; local adopts an existing dir, never creates); `projectId?`; `title?`; `background?` bool (hides from lists/sidebar; inherits caller workspace's value); worktree-only: `mode?` "branch-off"(default)\|"checkout-branch"\|"checkout-pr", `worktreeSlug?`, `branchName?`, `baseBranch?`, `branch?`, `prNumber?` int>0, `forge?` (worktree options with local -> error "Worktree options require isolation worktree") | `{workspaceId, projectId, cwd, isolation, kind:"directory"\|"local_checkout"\|"worktree", title|null, background?}` | 1381 |
| `list_workspaces` | `includeBackground?` false | `{workspaces: WorkspaceAutomationSummary[]}` (non-archived) | 1521 |
| `archive_workspace` | `workspaceId` | `{workspaceId, archivedAgentIds[], removedDirectory}` (needs github/git services; `archiveByScope`) | 1545 |
| `rename_workspace` | `workspaceId?` (default caller's current workspace), `title` | `{success, workspaceId, title}` | 2307 |
| `list_workspace_scripts` | `workspaceId` | `{scripts: WorkspaceScriptPayload[]}` (lifecycle, port, proxy URL, health, terminalId) | 2367 |
| `start_workspace_script` | `workspaceId`, `scriptName` (paseo.json script) | `{script}` | 2391 |
| `stop_workspace_script` | `workspaceId`, `scriptName` | `{script}` | 2418 |

### Terminals
| tool | params | returns | line |
|---|---|---|---|
| `list_terminals` | `cwd?` (default caller cwd), `all?` bool | `{terminals:[{id,name,cwd}]}` | 2444 |
| `create_terminal` | `cwd?`, `name?` | `{id,name,cwd}` (terminal belongs to caller's workspace; else minted via `ensureWorkspaceForCreate`) | 2492 |
| `kill_terminal` | `terminalId` | `{success}` | 2531 |
| `capture_terminal` | `terminalId`; `start?`, `end?` number; `scrollback?` bool (start=0); `stripAnsi?` default true | `{terminalId, lines[], totalLines}` | 2562 |
| `send_terminal_keys` | `terminalId`; `keys` string (tokens Enter Tab Escape Space BSpace Up Down Left Right Home End Insert Delete PageUp PageDown F1-F12, prefixes `C-` `M-` `S-`); `literal?` bool (send raw) | `{success}` (encodes via `encodeTerminalKeyInput` honouring the terminal's input mode from `getReplayPreamble`) | 2606 |

### Schedules and heartbeats (see Schedules sections)
| tool | params | returns | line |
|---|---|---|---|
| `create_schedule` | `prompt` req; `cron` req (5-field, validated incl. next-run computation); `timezone?` IANA; `name?`; `provider?` ("provider" or "provider/model"; optional in agent scope = inherit caller, required top-level); `cwd?` (default caller cwd / process.cwd()); `isolation?` local\|worktree; `maxRuns?` int>0; `expiresIn?` duration string | `ScheduleSummary`; calls `scheduleService.createOrReplace` (idempotent upsert by name+target) with `target:{type:"new-agent"}` | 2647 |
| `create_heartbeat` | `prompt`, `cron`, `timezone?`, `name?`, `maxRuns?`, `expiresIn?` | `ScheduleSummary` (target = caller agent; error `create_heartbeat requires an agent-scoped session`; `createOrReplace`) | 2699 |
| `delete_heartbeat` | `id` | `{success}` (must be caller's own: `Heartbeat <id> does not belong to caller <agent>`) | 2750 |
| `list_schedules` | none | `{schedules: ScheduleSummary[]}` only `new-agent` targets (heartbeats hidden) | 2771 |
| `inspect_schedule` | `id` | `StoredSchedule` incl. runs (new-agent only, else "Schedule not found") | 2796 |
| `pause_schedule` / `resume_schedule` / `delete_schedule` | `id` | `{success}` | 2819 / 2845 / 2871 |
| `update_schedule` | `id`; `cron?`; `timezone?` (needs cron); `name?|null`; `prompt?`; `maxRuns?|null`; `provider?`; `model?|null`; `mode?|null`; `cwd?`; `expiresIn?`; `clearExpires?`; hidden COMPAT `every` (remove after 2027-01-17) | `StoredSchedule` (no isolation/archiveOnFinish/thinking patch) | 2897 |
| `schedule_logs` | `id` | `{runs: ScheduleRun[]}` | 2969 |
| `run_schedule_once` | `id` | `StoredSchedule` | 2995 |
Duration strings for `expiresIn` (`parseDurationString` `agent/mcp-shared.ts:237`): bare number = seconds, else units `s m h` only (no `d`, unlike CLI which also accepts `d`).

### Providers, models, profiles, hosts
| tool | params | returns | line |
|---|---|---|---|
| `list_providers` | none | `{providers: ProviderSummary[]}` (id,label,description,enabled,modes,status,error?) via `providerSnapshotManager.listProviders({wait:true})` | 3016 |
| `list_models` | `provider` | `{provider, models: AgentModel[]}` | 3037 |
| `list_profiles` | none | `{profiles: AgentProfile[]}` from `daemonConfigStore.get().agentProfiles` (empty if none); no `profile` param on `create_agent`: copy fields | 3065 |
| `inspect_provider` | `provider` (or provider/model); `cwd?`; `settings?` {modeId?, model?, thinkingOptionId?, features?} | `{provider,label,description,enabled,status,modes,selectedModel,features[]}`; errors if disabled/unavailable | 3089 |
| `list_hosts` | none | `HostDirectory {self{serverId,label}, hosts[{registrationId,serverId,label,source,metadata,enabled,status connecting\|online\|offline\|error\|disabled,targetDescription,lastOnlineAt,nextRetryAt,error}]}` (`proto/hosts.ts`) | 1367 |
| `speak` | `text` string 1-4000 | `{ok}`; only registered when `voiceOnly \|\| enableVoiceTools \|\| callerContext.enableVoiceTools`; always allowed by policy; blocks until TTS playback done | 1314 |

### Browser tools (`srv/server/browser-tools/tools.ts`; only when `browserToolsEnabled && broker`)
Gate: `browserToolsPolicy.isEnabled()` = `daemon.browserTools.enabled === true` (default false, `config.ts:530`, `browser-tools/policy.ts`); registered from `paseo-tools.ts:1359`. Executes via `BrowserToolsBroker.execute` (`browser-tools/broker.ts`) which forwards to a connected browser-automation host (desktop app) with default timeout `DEFAULT_BROWSER_TOOLS_TIMEOUT_MS = 15_000` (broker.ts:46). `browserId` validated by `BrowserAutomationBrowserIdSchema` (`proto/browser-automation/rpc-schemas.ts:50`); refs `@e<N>` from latest `browser_snapshot` (expire on page change); URLs must be http(s) (scheme-less = http). Tools needing workspace: `browser_list_tabs`, `browser_new_tab` (error "needs a workspace").
| tool (tools.ts line) | params | |
|---|---|---|
| `browser_list_tabs` (67) | none | tabs across hosts for the agent's workspace |
| `browser_new_tab` (94) | `url?` | background tab, returns browserId |
| `browser_snapshot` (123) | `browserId` | model-readable page snapshot with refs |
| `browser_click` (151) | `ref`, `browserId`, `button?` left\|right\|middle (default left), `doubleClick?` (false), `modifiers?` Alt\|Control\|Meta\|Shift[] | |
| `browser_fill` (187) | `ref`, `value`, `browserId` | |
| `browser_wait` (219) | exactly one of `text`/`url`; `timeoutMs?` int 1..30000 (default 5s on host); `browserId` | |
| `browser_type` (249) | `text`, `ref?`, `browserId` | |
| `browser_keypress` (281) | `key`, `ref?`, `browserId` | |
| `browser_navigate` (313) | `url`, `browserId` | |
| `browser_back` / `browser_forward` / `browser_reload` (loop 340, registerTool 363) | `browserId` | |
| `browser_screenshot` (389) | `browserId`, `fullPage?` default false | PNG |
| `browser_upload` (419) | `ref`, `filePaths` string[] min1, `browserId` | |
| `browser_hover` (loop 451, registerTool 460) | `ref`, `browserId` | |
| `browser_select` (487) | `ref`, `value`, `browserId` | |
| `browser_drag` (519) | `sourceRef`, `targetRef`, `browserId` | |
| `browser_logs` (551) | `maxEntries?` int 1..200, `browserId` | |
| `browser_evaluate` (581) | `function` string, `ref?`, `browserId` | |
| `browser_scroll` (613) | `browserId`, `ref?`, `deltaX`, `deltaY` | |
| `browser_resize` (647) | `browserId`, `width`, `height` (positive ints) | |
| `browser_close_tab` (679) | `browserId` | |

Counts: 41 built-in non-browser tools (40 registered at paseo-tools.ts 2-space indent incl. `list_hosts`, plus conditional `speak`) + 22 browser tools (20 `registerTool` calls, three of them loops) + plugin tools. Other MCP servers in the repo: none exposed (`McpServer` is only constructed in `agent/mcp-server.ts`; other files consume `mcpServers` provider config).

## Orchestration skills (install, selection, sync)

**What the user sees**:
- Settings > Host > "Skills" section (`AgentSkillsSection`, `app/agent-skills/index.tsx:177`, mounted `screens/settings/host-page.tsx:305`; section title `settings.host.skills.sectionTitle`, docs link `https://paseo.sh/docs/skills` (`SKILLS_DOCS_URL` :155)). One card (`host-agent-skills-card`) with `AgentSkillsRow` (:236): switch (installed = `status.state !== "not-installed"`: ON runs `reconcile`, OFF runs `uninstall` after a destructive confirm), gear button (opens `SkillSelectionSheet`, disabled while status null/working), and an `Update` button when `status.state==="drift"` and there are non-delete ops (confirm dialog lists ops sorted add, update, delete as `<verb> <name>`). Switch disabled when nothing selected (`!value && !hasSelectedSkills(status)`). When the host is not connected -> card `host-agent-skills-unavailable` with `settings.host.skills.unavailable`; connected but host lacks the feature -> `settings.host.skills.unsupported`. Refetches status on screen focus (`useFocusEffect`).
- Picker (`agent-skills/selection-sheet.tsx`, testID `skill-selection-sheet`): "all skills" switch (`skill-selection-all`) + one switch row per `available` skill (disabled when all/saving) + Cancel / Save (`skill-selection-save`). Draft seeded only when the sheet opens. Leaving `all` selects every available skill as the starting `custom` list. Save: calls `onSave(draft)`; if the host replies `confirmationRequired.removals`, shows destructive confirm (`removeTitle`/`removeMessage` listing names), then retries with `confirmedRemovals = removals`; a second confirmationRequired or an exception -> inline error `settings.host.skills.saveFailed`.
- Onboarding step "orchestration" (`onboarding/internal/model.ts:3-8`, step order appearance, providers, projects, orchestration, [phone]; `orchestration-step.tsx`, `orchestration-setup.ts`): three switches all ON by default: Paseo tools (`config.mcp.injectIntoAgents`), Skills (only offered when host connected and supports), CLI (desktop only, `useCliInstall`). Nothing touches the host until Continue -> `apply()` writes only choices that differ from known state (`patchConfig({mcp:{injectIntoAgents}})`, `reconcile()`/`uninstall()`, CLI install); resolves true only if all succeeded, failures toasted (`settings.host.orchestration.enableTools.updateFailed`, ...).
- Hook `useAgentSkills(serverId)` (`agent-skills/use-agent-skills.ts`): react-query key `["host", serverId, "agent-skills"]`, `staleTimeMs: 0`, `retry:false`, `enabled: supported && client`, `supported = useHostFeature(serverId, "skillManagement")` (server advertises `skillManagement: true` in server info, `websocket-server.ts:2069`; COMPAT gate "remove after 2027-08-16", `proto/messages.ts:3752`). Toasts on errors (`statusFailed`, `updateFailed`, `uninstallFailed`, `saveSelectionFailed`). Uninstall always confirmed first (`uninstallTitle`/`uninstallMessage`: deletes whole skill folders including user additions).
- Legacy desktop migration `LegacyAgentSkillsMigration` (`agent-skills/legacy-migration.tsx`, mounted `app/_layout.tsx:80`): for desktop-managed local daemon, reads the old desktop-stored selection (`readLegacySkillSelection`), sends `importLegacyAgentSkillsSelection`, then `deleteLegacySkillSelection`. COMPAT(desktopSkillSelectionMigration) remove after 2027-02-16.

**Protocol** (`proto/messages.ts:1631-1690` requests, `:7004-7035` responses; requests are `.strict()`, all carry `requestId`):
| request type | extra fields | response type payload |
|---|---|---|
| `agent.skills.get_status.request` | - | `agent.skills.get_status.response` `{requestId, state: "not-installed"\|"up-to-date"\|"drift", ops: [{kind: add\|update\|delete, name}], available: string[], installed: string[], selection}` |
| `agent.skills.reconcile.request` | - | `agent.skills.reconcile.response` (same status payload) |
| `agent.skills.uninstall.request` | - | `agent.skills.uninstall.response` (same status payload) |
| `agent.skills.save_selection.request` | `selection`, `confirmedRemovals?: string[]` | `agent.skills.save_selection.response` status payload + `confirmationRequired: {removals: string[]} \| null` |
| `agent.skills.import_legacy_selection.request` | `selection` | `agent.skills.import_legacy_selection.response` `{requestId, imported: bool, selection}` |
`AgentSkillSelection` (`proto/agent-profile.ts:32`) = `{mode:"all"}` \| `{mode:"custom", skills: string[]}`.
Client methods (`client/daemon-client.ts`): `getAgentSkillsStatus` :6024, `reconcileAgentSkills` :6032, `uninstallAgentSkills` :6040, `saveAgentSkillsSelection(selection, confirmedRemovals?)` :6048, `importLegacyAgentSkillsSelection` :6064. Server dispatch `srv/server/session.ts:2491 dispatchOrchestrationSkillsMessage` (only when `orchestrationSkills` provided; handlers `emit` the response; errors propagate to the generic session error path; not found a dedicated rpc_error code).

**Server modules** (`srv/server/orchestration-skills/`):
- `index.ts` `createOrchestrationSkills(configStore)`: facade `getStatus, reconcile (=controller.update), uninstall, saveSelection(selection, confirmedRemovals=[]), importLegacySelectionIfUnset, autoUpdate`. Wired `bootstrap.ts:668`; **at every daemon start** `orchestrationSkills.autoUpdate()` runs in the background (errors logged "Failed to maintain orchestration skills at startup"), so installed+drifted skills are silently repaired on upgrade; additions of new skills happen only under `mode:"all"`; deletions never automatic.
- `internal/paths.ts`: `resolveSkillTargets(home=os.homedir())` -> `{sourceDir: bundled skills dir, agentsDir: ~/.agents/skills, claudeDir: ~/.claude/skills, codexDir: ~/.codex/skills}`; `resolveBundledSkillsDir` tries `<server>/dist/server/skills` (3 levels above module, = copy made by the server build script `package.json build:lib: cpSync('../../skills','dist/server/skills')`) then repo-root `skills/` (6 levels up).
- `internal/selection-store.ts`: selection persisted in daemon config at `agents.skills.selection` (`persisted-config.ts:356`, set through `daemonConfigStore.setAgentSkillSelection` `daemon-config-store.ts:409`, exposed in daemon config message `skills.selection` `proto/messages.ts:248`). `coerceSkillSelection`: anything invalid -> `{mode:"all"}` (default); custom lists are trimmed, de-duplicated, sorted. `isSet()` = selection key defined.
- `internal/controller.ts`: single writer; all operations run through one promise queue (`serialize`). Each op first `recoverInterruptedSkillTransactions` then acts. `status()` = converge(getSkillsStatus) (read only), `update()`/`install()` = `updateSkills` (non-destructive), `uninstall()`, `autoUpdate()`. `saveSelection` algorithm (controller.ts:161): compute plan from fresh scan; if any `delete` op not in `confirmedRemovals` -> return current state with `confirmationRequired.removals` and change nothing; else open a transaction, apply the frozen plan minus deletes (deletes are pre-staged by the transaction), if resulting `ops.length===0` persist the selection then commit; otherwise roll back, rescan and retry up to `MAX_SAVE_ATTEMPTS = 3`, then throw "Skills changed repeatedly while the selection was being saved".
- `internal/operations.ts`: `SkillsStatus {state, ops, available, installed}`. Catalog = subdirectories of the bundled dir (`listBundledSkills`), so `all` picks up new skills with no code change. `LEGACY_SKILL_NAMES = ["paseo-chat","paseo-epic","paseo-orchestrate","paseo-orchestrator"]` are never selectable but always scanned so old copies get cleaned. `diff`: desired skill missing in at least one of agents/claude targets -> `add`; present but any bundle file's sha256 differs -> `update` (extra user files ignored); undesired but present -> `delete`. Codex dir handled separately: a codex copy that is not the same realpath as the agents/claude copy is a "legacy Codex copy" -> `delete` if undesired, `update` (retire) if it has a managed manifest. State: `not-installed` (no managed skill dir in any of the 3 homes), `up-to-date` (no ops), else `drift`. `uninstallSkills` removes every managed name from all three dirs (recursive rm). `autoUpdateInstalledSkills` only acts when `drift`, never deletes.
- `internal/sync.ts`: installs by copying the bundle skill dir into `~/.agents/skills/<name>` and `~/.claude/skills/<name>` (Codex also reads `.agents/skills`, so no dedicated Codex copy is written; an old Codex copy is retired by `retireManagedSkill`, which removes only files whose hash still matches the manifest). Per-directory ownership manifest `.paseo-managed-files.json` `{version:1, files:{rel: sha256}}` (`MANAGED_FILES_MANIFEST`): files in the previous manifest that disappeared from the bundle are removed only if unmodified; user files are never touched; empty parents pruned. Safety: refuses symlinks and paths escaping the skill directory (`Cannot sync through symbolic link`, `Cannot sync managed path outside skill directory`).
- `internal/transaction.ts`: crash-safe save. Staging dir `.paseo-skills-transaction-<uuid>` created beside the skills trees (`dirname(agentsDir)`, i.e. `~/.agents/`), with `transaction.json` `{owner:"paseo-skills-transaction", version:1, phase:"capturing"|"converging", previousSelection, nextSelection, entries:[{livePath, kind, backupPath}]}` and `backup/` copies of only the directories the frozen plan can mutate; `commit()` drops staging, `rollback()` restores exactly (quarantines newly appeared dirs under `.paseo-skills-recovered-*`). `recoverInterruptedSkillTransactions(targets, committedSelection)` (called before every op): if committed selection equals the manifest's `nextSelection` -> discard; if equals `previousSelection` -> restore then discard; otherwise throws `Cannot safely recover interrupted skills transaction at <dir>`. Directories without a valid manifest are never touched.

**Skills that ship** (repo `skills/<name>/SKILL.md`, bundled into `dist/server/skills`): `paseo` (152 lines, "Paseo reference for managing hosts, projects, workspaces, workspace scripts, agents, schedules, and heartbeats"), `paseo-advisor` (user-invocable; `[--profile <name>] <question or topic>`), `paseo-committee` (user-invocable), `paseo-handoff` (user-invocable), `paseo-help`, `paseo-plugin` (766 lines). No `paseo-loop`, `paseo-orchestrator`, `paseo-chat`, `paseo-epic` (those are the legacy names cleaned on update). Installed on disk at `~/.agents/skills/<name>/` and `~/.claude/skills/<name>/` (each with `.paseo-managed-files.json`); `~/.codex/skills/<name>` only legacy copies are cleaned. Repo also has `.claude/skills` and `.agents/skills` dirs (dev tooling, not shipped by this mechanism).

**States/limits**: `SkillsState` three values above; `MAX_SAVE_ATTEMPTS=3`; native errors surface as toasts; unsupported daemon (no `skillManagement` feature) hides the controls.
**Platforms**: web/desktop/mobile same UI; CLI install row only on desktop; legacy migration only for desktop-managed daemon.
**Connections**: daemon config store (`agents.skills.selection`, `mcp.injectIntoAgents` in onboarding), Paseo MCP tools (`list_profiles` mention only; skills teach agents to use MCP/CLI), agent providers read the skill dirs themselves (Claude `~/.claude/skills`, others `~/.agents/skills`), onboarding, host settings page, CLI install (desktop).

## Cross-area connections

- Schedules -> agents: `new-agent` runs call `createAgent` (kind `mcp`, labels `paseo.schedule-id`/`paseo.schedule-run`, `unattended:true`) and `runAgent`; heartbeats call `startAgentRun` with steer on an existing agent and use the system-notification prompt wrapper (`formatSystemNotificationPrompt`); agent archive (`setAgentArchivedCallback`) completes schedules; permission requests make a run fail.
- Schedules -> workspaces: per-run workspace via `createDirectoryWorkspace` / `createPaseoWorktreeWorkspace`; archived after the run unless `archiveOnFinish===false` (new-agent only); recovery after daemon restart archives interrupted run workspaces.
- Schedules -> providers/models: form model reuses create-agent provider snapshot/model catalog and form preferences (`create-agent-preferences`, `provider-selection`); `thinkingOptionId`, `modeId`, `featureValues` stored in target config.
- Schedules -> projects/hosts: project targets from `useProjects`, host list from `useHosts`/host runtime; aggregated across hosts client-side (no server aggregation); invalidated on host reconnect.
- MCP `create_schedule`/`create_heartbeat`/etc. share `ScheduleService` (`createOrReplace` idempotency) with the app and CLI; CLI/MCP list hide heartbeats (`new-agent` filter) while the app list shows both.
- MCP tools -> agent manager (`createAgentCommand`, `agentMessages().enqueue`, `finishSubscriptions().register` for notifyOnFinish callbacks), permissions (`list_pending_permissions`, `respond_to_permission`), terminals (`terminalManager`), workspace scripts service, workspace registry/archive service, provider snapshot manager (`list_providers/list_models/inspect_provider`), daemon config (`list_profiles` reads `agentProfiles`; policy from `providers.<id>.paseoTools`), connected hosts (`list_hosts`, `serverId` fan-out via `DaemonClient`), browser automation broker (browser tools -> desktop browser host), plugins (plugin-declared tools appended; `plugin-tools.ts`), voice (`speak`).
- MCP injection -> every provider's launch config (`mcpServers.paseo`), env `PASEO_AGENT_ID`/`PASEO_AGENT_CWD`, daemon password auth (`/mcp/agents` self-authenticating), OpenCode bridge manifest, omp native host tools.
- Orchestration skills -> daemon config (`agents.skills.selection`), filesystem homes of Claude/Codex/agents-compatible CLIs, onboarding, settings host page, desktop legacy store.
- Heartbeat CLI/MCP -> caller identity from env `PASEO_AGENT_ID` / `callerAgentId` query param.
- Settings: `config.mcp.injectIntoAgents`, `config.browserTools.enabled` (BrowserToolsOptInCard `screens/settings/browser-tools-card.tsx:15`), `providers.<id>.paseoTools`, `agents.skills.selection`.

## Inventory

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
