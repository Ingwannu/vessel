# p1 — Projects, workspaces and worktrees (Paseo 0.15.4)

Source: `/home/dev/refs/paseo-latest`, commit `1c00b365c9e0b74c1580eba33d905d693125ae21`, inspected 2026-11-15. Source-only survey; no builds, tests, daemon calls or live UI verification. Paths below are relative to `packages/`: **APP** `app/src`, **SRV** `server/src/server`, **UTIL** `server/src/utils`, **P** `protocol/src/messages.ts`, **C** `client/src/daemon-client.ts`. All request/response RPCs below carry `requestId`; correlated responses echo it (push events use subscription/sequence identity). Unless overridden, SDK session RPC timeout is `DEFAULT_SESSION_RPC_TIMEOUT_MS=60000` (`C:1125`). Defaults below are source defaults, not a claim about an operator's installed configuration. Git operations, forge/PR behavior and files are owned by [p2](p2-git.md), [p3](p3-forge-review.md), [p4](p4-files.md).

The implementation uses Expo/React Native + Zustand/React Query, a typed WebSocket SDK, Zod protocol schemas and Node daemon services. Host-local JSON registries are placement authority; app layout is device-local. This separation matters for a Rust port: a workspace is an identity and lifecycle container, not a directory or a Git branch. Multiple records may share one cwd; project membership and workspace execution cwd need not equal the project root.

## 01. Add Project flow and host selection

### Host and method selection

- Entry: `APP/hooks/use-open-add-project.ts`, `components/add-project-flow-host.tsx`, `components/add-project-flow.tsx:327`; modal request lives in `stores/add-project-flow-store.ts:18` (monotonic request id, optional preferred host).
- `add-project-flow/model.ts:73` builds a page stack: `host`, `method`, `directory-search`, `github-search`, `github-location`, `new-directory-parent`, `new-directory-name`. Preferred/only host skips the host page; late host discovery can resolve the initial host page. Back pops a page, then closes. Per-host/parent name drafts and per-host/repository clone-location drafts survive back navigation.
- `components/add-project-flow.tsx:344` offers online hosts only. `projectAdd` **and** `stableProjectIdentity` gate Add Project; method capabilities separately gate `githubClone`, GitHub repository search and create-directory. Unsupported methods explain that the host needs updating.
- Methods (`add-project-flow/options.ts:36`): **Search for directory**, **Browse**, **Clone from GitHub**, **New directory**. No create mutation happens merely by opening or selecting a host.

### Search and navigation states

- Directory/GitHub queries request up to 30 rows and debounce 250 ms (`add-project-flow.tsx:411`); GitHub search asks for 30. Search has loading, empty, query-failure and page-specific error states; mutation disables repeated submission and shows adding/cloning/creating progress (`:174`, `:856`, `:944`). Arrow Up/Down wraps and skips disabled rows (`model.ts:276`); Enter selects/submits, Escape goes back then closes (`add-project-flow.tsx:793`).
- Platforms: cross-platform modal; Browse depends on the Electron/local-host directory bridge (`:518`), not generic browser filesystem access. Native/compact navigation shows the agent panel on completion (`:472`).
- Connections: host connection/capabilities → project registration → new-workspace screen; no agent or workspace is created by project registration itself.

## 02. Register an existing directory; compatibility Open Project

### Register project without creating a workspace

- Directory selection calls `useOpenProject` (`APP/hooks/use-open-project.ts:13`) → `openProjectDirectly` (`hooks/open-project.ts`) → **`C.addProject(cwd)`** (`C:3011`) → `project.add.request {cwd}` (`P:2669`). The hook's historical name does not mean it uses `open_project_request`.
- `SRV/session.ts:7072` expands `~`, requires a directory, and calls `WorkspaceProvisioningService.findOrCreateProjectForDirectory` (`session/workspace-provisioning/workspace-provisioning-service.ts:201`). The project registry idempotently matches the active exact lexical root. Symlinks are not resolved into another project identity.
- Result: `project.add.response {project|null,error,errorCode?}`; failure includes `directory_not_found`. Registry mutation publishes `project.update`; app also accepts the returned snapshot into host runtime and sets workspace hydration state. `add-project-flow.tsx:464` routes to a project-targeted new-workspace draft.
- Storage: `$PASEO_HOME/projects/projects.json`; project has `projectId`, exact `rootPath`, `kind`, `displayName`, `customName`, `projectKey`, icon revision and timestamps. Archived-only matching projects do not supply an active id.

### Compatibility workspace open

- Compatibility/API distinction: `C.openProject(cwd)` (`C:3000`) → `open_project_request` → `SRV/session.ts:6983` finds/creates a **workspace** by directory, syncs Git observers, pushes workspace update and returns its descriptor. Missing directory fails before provisioning. Do not use this deduplicating path to implement the fresh-workspace button.
- Platforms: selected path belongs to the selected daemon, including remote hosts. Connections: project registry, directory suggestions ([files](p4-files.md)), sidebar empty projects, workspace provisioning.

## 03. New directory (and the absence of a project Git-init action)

### Create directory and roll back failed registration

- UI: Add Project → New directory → parent → name (`APP/components/add-project-flow.tsx:737`), retaining typed names per host/parent. Suggested parents include parents of existing projects plus `~/dev`, `~/Developer`, `~/src`, `~/projects`, `~/workspace`, `~` (`add-project-flow/options.ts:153`).
- `C.createProjectDirectory({parentPath,name})` (`C:3022`) → `project.create_directory.request` (`P:2675`) → `SRV/session.ts:7135` → `project-directory-service.ts:47`.
- Validates nonempty single name, not `.`/`..`, no leading/trailing whitespace, NUL, separator, absolute path or Windows drive prefix; requires an existing directory parent and lexical containment. Creates one directory with nonrecursive `mkdir`, then registers it. If registration fails, attempts `rmdir` rollback; reports rollback failure separately rather than pretending registration succeeded.
- Response includes `directoryPath|null`, `project|null`, `error`, `errorCode`. Service distinguishes invalid name/parent, missing parent, access denied, existing target, create and registration failures (`project-directory-service.ts:98`, `:126`, `:164`). UI accepts snapshot and opens the new-workspace screen.

### Git init boundary

- **Git init not found in this flow:** `add-project-flow/options.ts`, `components/add-project-flow.tsx`, `project-directory-service.ts` and project RPC handlers create an empty directory, not a repository. Existing non-Git roots can later be reclassified by reconciliation when Git metadata appears; Git commands belong to [p2](p2-git.md).
- Platforms: server filesystem semantics; Electron Browse may create folders in the OS dialog, while this RPC supports remote/mobile. Connections: project registry and subsequent workspace launch; no setup command or agent is started here.

## 04. Clone a repository into a project

### Repository discovery and manual remote input

- UI: Add Project → Clone from GitHub → repository search/manual URL → parent directory (`APP/components/add-project-flow.tsx:449`, `:563`). Search errors do not remove manual URL choices.
- Manual full Git remotes yield one choice; `owner/repo` yields HTTPS and SSH choices (`add-project-flow/options.ts:95`). Despite the UI/RPC name, complete Git remote URLs are accepted by the remote parser; account search is GitHub-specific.
- `C.searchGithubRepositories({query,limit})` (`C:3036`) → `workspace.github.search_repositories.request` (`P:2691`, limit 1–50) → `SRV/session.ts:7186` → GitHub service at host home. Returns success, unavailable/`gh_missing`, unauthenticated, or error; authentication hint is `gh auth login` on the host. Forge internals: [p3](p3-forge-review.md).

### Clone and registration

- `C.cloneGithubProject({repo,cloneProtocol?,targetDirectory})` (`C:3052`) → `project.github.clone.request` (`P:2701`) → `session.ts:7261`. `targetDirectory` is the **parent**, final path adds repository name. Containment and an already-existing checkout are checked first.
- Server creates parent recursively, clones to `.paseo-clone-*` staging, then renames to final checkout; clone failure removes staging. Git command timeout is `5 * 60 * 1000`, maximum captured output `1024 * 1024` (`session.ts:7293`). A post-clone registry failure is not a promise to remove the completed clone.
- Returns `{repo,checkoutPath,project,error}`; app accepts the project and opens a new-workspace draft. Last clone parent is remembered by host in this flow's module; existing target choices display **Already exists** and are disabled.
- Platforms: all clients initiate host-side clone; remote credentials and git executable belong to daemon host. Connections: project registry, GitHub discovery, new-workspace launch, sidebar; no agent created yet.

## 05. Suggested projects from provider history

### History discovery

- Entry: onboarding Projects step (`APP/onboarding/internal/steps/projects-step.tsx:1`); history begins loading when onboarding opens (`onboarding-modal.tsx:35`). Controller `project-suggestions/internal/use-project-suggestions.ts:48`; list UI `project-suggestions/internal/project-suggestions.tsx:32`.
- `use-provider-suggestions.ts:28` gates on `projectSuggestions`, discovers enabled providers, queries each independently with `C.listProjectSuggestions({providers:[provider]})` (`C:2986`) → `project.suggestions.request` → `SRV/session.ts:6559` → `project-suggestions.ts:47`. Provider failure contributes an empty result; slow providers do not hide already answered rows. No retries; `SUGGESTIONS_STALE_TIME_MS=5 min`.
- Server scans provider sessions (`SESSION_SCAN_LIMIT=500`), filters to `since` (default `DEFAULT_WINDOW_MS=30 days`), excludes metadata-generation sessions; resolves cwd to main repository/worktree root or directory (`CHECKOUT_READ_CONCURRENCY=8`). Excludes nonexistent directories, active projects, home and filesystem root. Default return `DEFAULT_LIMIT=20`; malformed since → `invalid_since`.

### Selection and incremental addition

- App merges roots, sums session counts, unions provider ids and sorts by latest activity. Default checked iff ≥ `PRESELECT_MIN_SESSIONS=3` and activity within `PRESELECT_WINDOW_MS=30 days` (`suggestions.ts:5`). User toggles override future provider answers.
- Add checked runs sequential project registrations (feature 02); each failure remains on its row. Selection freezes while adding. If a provider adds newly checked rows during submission, onboarding stays on the step instead of silently skipping them (`use-project-suggestions.ts:87`).
- States: unsupported/loading/ready, reading, adding, checked/unchecked, per-row add failure, no suggestions. Suggestions themselves are not persisted projects until accepted.
- Platforms: same onboarding controller on native/web/desktop. Connections: provider discovery/importable history, onboarding, project registration; this does not import those agent sessions.

## 06. Project name and icon editing

### Name override

- Entry: project edit sheet (`APP/components/project-edit-sheet.tsx:105`), project settings (`screens/project-settings-screen.tsx:230`). `projects/edit-form.ts` models pending name/icon edits; save steps run name then icon (`project-edit-sheet.tsx:238`) and report step-specific failures.
- `C.renameProject(projectId,customName)` (`C:3448`) → `project.rename.request {projectId,customName:string|null}` → `SRV/session.ts:3631`. Trims name; blank/null resets to derived `displayName`; missing id returns **Project not found**. Registry `customName` changes, not directory path or stable project id.

### Icon upload, URL import, automatic mode and cache

- `C.setProjectIcon(projectId,source)` (`C:3468`) → `project.icon.set.request` → `session.ts:3718` → `UTIL/project-custom-icon.ts:22`. Sources are upload base64 or automatic; custom data lives in `$PASEO_HOME/projects/icons/<sha256(projectId)>.bin` (`:137`), and registry records `customIconRevision`. Automatic removes override and resumes root discovery.
- URL import happens **on the client** (`APP/projects/import-project-icon.ts:63`): HTTP(S) without credentials; direct image or HTML link-icon candidates + favicon fallback. `MAX_ICON_BYTES=512 KiB`, `MAX_HTML_BYTES=512 KiB`, `MAX_ICON_CANDIDATES=8`. Browser fetch/CORS therefore differs from daemon-side file discovery.
- Server accepts signature-identified PNG/JPEG/GIF/WebP/ICO (not SVG), nonempty ≤512 KiB, square image ≤1024×1024 (`UTIL/project-custom-icon.ts:142`, `:163`); unsupported/invalid image and nonsquare/oversize fail. Per-project saves serialize.
- Both mutations publish `project.update` and descriptors for affected workspaces (`session.ts:3694`, `:3734`). `C.getProjectIcon(projectId)` (`C:5470`) → `project.icon.get.request` returns `{projectId,icon|null,error}`; effective revision hashes automatic/custom bytes and MIME so absence is also cacheable.
- App `projects/icons.ts`, `icon-cache.ts`, `icon-cache-storage.web.ts`/`.ts` own revision-keyed icon caching (`MAX_ENTRIES=512`, `icon-cache.ts:9`); web uses IndexedDB `paseo-project-icon-cache` / `key-value`, native AsyncStorage. Automatic discovery has separate `MAX_ICON_SIZE=32 KiB` (`UTIL/project-icon.ts:71`); placeholder is the project initial. Name/icon are host-specific even when sidebar groups projects across hosts.
- Platforms: client URL fetch and file picking depend on platform implementation; registry persistence is daemon-local. Connections: project settings, multi-host project grouping, sidebar and workspace header visuals.

## 07. Remove a project

### Removal entry and host preflight

- Entry: sidebar project Remove action and destructive confirmation (`APP/components/sidebar-workspace-list.tsx:1710`); `APP/projects/project-remove.ts:32` checks every host's `projectRemove` capability; `:79` resolves all clients before issuing any remove. Missing capability → `needs_host_update`; missing client → `host_disconnected`.

### Archive contents and remove registry membership

- `C.removeProject(projectId)` (`C:3480`) → `project.remove.request {projectId}` → `SRV/session.ts:3773`. Active child workspaces are marked archiving; each has agents archived/terminals killed and its record archived. The project record and custom icon are removed.
- **This path preserves directories**: it calls `archiveWorkspaceContents` and `archiveWorkspaceRecord`, not the managed worktree disk-removal workflow. Do not substitute workspace/worktree deletion semantics here.
- Response `{accepted,removedWorkspaceIds,error}` plus workspace removals carrying `removedProjectId` clears app project rows, including empty projects (`session.ts:3833`). Failed removal emits activity error and accepted=false; sequential server work is not a transaction that restores already archived children.
- Grouped-project removal fans out with `Promise.allSettled`; one host failing yields failed host ids while successful removals remain (`project-remove.ts:104`). Capability/client preflight does not promise cross-host atomicity.
- Platforms: shared UI; affected paths are on each target host. Connections: archive agents/terminals, project registry, icon cache, sidebar selection/redirect. Project removal is not filesystem deletion.

## 08. Project settings and conflict-aware `paseo.json` editing

### Load and authorize project config

- Entry: `/settings/hosts/[serverId]/projects/[projectId]` → `APP/screens/project-settings-screen.tsx:100`; multi-host project selects the concrete host/root before editing. Reads are gated by connection/project availability (`:218`, `:325`). Missing config is an editable empty config, not a transport failure.
- `C.readProjectConfig(repoRoot)` (`C:5721`) → `read_project_config_request {repoRoot}` (`P:1695`) → `SRV/session/project-config/project-config-session.ts:40`; write `C.writeProjectConfig({repoRoot,config,expectedRevision})` (`C:5732`) → `write_project_config_request` (`P:1701`) → same service `:83`.
- Only currently active registered project roots qualify; alias/trailing-slash resolution is rechecked against registry after filesystem IO (`project-config-session.ts:178`). Reads return `{ok,config|null,revision|null,hasUncommittedWorktreeSetupChanges?}`; writes return saved config/revision or structured failure.

### Revision check and save failures

- Storage is `<project root>/paseo.json`, `MAX_PASEO_CONFIG_BYTES=256 KiB`; revision is **mtimeMs + size**, not a hash (`UTIL/paseo-config-file.ts:23`, `:46`, `:246`). Writes create an exclusive temp file then compare expected/current revision and rename; code explicitly acknowledges the optimistic stat/rename race (`:262`).
- UI read failures: project missing, invalid config, transport; reload action (`project-settings-screen.tsx:410`). Write failures: `stale_project_config` (save blocked until Reload), `write_failed` (Try again/Reload), other structured codes (`:529`, `:770`). Successful save replaces query data and toasts saved.
- Form keys include host/root/revision (`:384`), so a new authoritative revision resets the draft. Unknown config fields survive form projection (`APP/utils/project-config-form.ts`), and string versus array lifecycle shape is retained.
- Platforms: shared settings UI; config is on selected daemon, not client filesystem. Connections: worktree bootstrap, scripts/services, metadata generation, Git commit state. Uncommitted setup changes get an explicit warning because future worktrees may check out another version.

## 09. Project lifecycle, scripts and metadata settings

### Setup and teardown configuration

- UI sections `APP/screens/project-settings-screen.tsx:690`: Worktree setup (`npm install` placeholder), teardown (`docker compose down`), script list/Add/Edit/Remove, metadata prompts for `branchName`, `commitMessage`, `pullRequest` (`utils/project-config-form.ts:9`). These use feature 08's config RPC, not separate daemon settings.
- `protocol/src/paseo-config-schema.ts:46` permits `worktree.setup` and `.teardown` as string or string array; runtime normalization filters blank commands and defaults absent lists to `[]`. `worktree.terminals`, `worktree.servicePorts` and unknown keys pass through; service port range is inclusive 1–65535 and/or `portScript` (`:5`). The settings form does not expose every pass-through key.

### Script and metadata editing; setup callout

- Named `scripts` store `type`, `command`, `port`, preserving extra keys. Script editing validates required name/command and port/type; empty new entries are removed on cancel, remove uses confirmation, and invalid scripts disable overall Save (`project-settings-screen.tsx:560`, `:591`, `:634`, `:1037`).
- Metadata schema also supports `title.instructions`, but this form's editable list contains only the three keys above. Overrides are instructions for existing metadata generation, not executable setup commands. See [p2](p2-git.md) and [p3](p3-forge-review.md) for commit/PR generation consumers.
- Setup-missing callout: `APP/components/worktree-setup-callout-policy.ts:28` accepts an active Git project only; successful read with no nonblank setup commands shows a settings link. Dismissal key is `worktree-setup-missing:<serverId>:<projectId>`, priority 100 (`:55`). Read failure/non-Git does not imply missing setup.
- Script execution is workspace-scoped and separate from saving config; see feature 25. All platforms edit the same schema; commands execute on host.

## 10. New Workspace screen: project, host, isolation and checkout target

### Project, host and isolation selection

- Entry: sidebar/project New Workspace, global new-workspace action (`APP/hooks/use-global-new-workspace-action.ts`), `screens/new-workspace-screen.tsx:849`, form `screens/new-workspace/form/index.tsx`; project combobox includes Add Project (`form/options.tsx:78`), and exposes keyboard action `workspace.project.pick` (`new-workspace-screen.tsx:183`).
- Selection ownership: `screens/new-workspace/project-selection.ts` resolves explicit route context, remembered project and late project hydration without overriding manual selection; `new-workspace-initial-context.ts` resolves remembered host/project context; `stores/last-workspace-selection.ts:8` separately persists last route selection `{serverId,workspaceId}` at `paseo:last-workspace-route-selection`. Host selector limits placement to hosts where the chosen project exists.
- Isolation defaults to remembered preference or **local** (`new-workspace-screen.tsx:195`). Worktree is effective only when current project can create one; switching to a non-Git/unsupported target does not magically turn the folder into Git. Local creates a new workspace record backed by the selected source directory.

### Branch/PR picker and attachment ownership

- Worktree ref picker (`new-workspace-picker-item.ts`, `new-workspace-picker-state.ts`) maps every branch row to `action:branch-off`; change-request rows map to `action:checkout` (`new-workspace-picker-item.ts:181`). Direct existing-branch checkout is exposed by API/CLI, not this picker. Branch and PR/MR search/resolution are linked to [p2](p2-git.md) and [p3](p3-forge-review.md); the selected result becomes wire `action/refName/checkoutSource`. Default base is current branch upstreamRef, else explicit local `refs/heads/<currentBranch>`; no current branch returns no default. Local/origin rows split when divergence differs or is unknown, and show local ahead/behind. Branch query limit=20, staleTime=15000 ms (`new-workspace-screen.tsx:1047`).
- `createMultiplicityWorkspace` (`new-workspace-screen.tsx:283`) requires a host-local project id and calls `C.createWorkspace` with `{idempotencyKey,source,agent?,firstAgentContext?}`; worktree source carries cwd, projectId, generated worktreeSlug and picker request, local source `{kind:'directory',path,projectId}`.
- Picker attachment coupling (`new-workspace-picker-state.ts:56`): switching selection replaces only picker-owned PR attachment, keeps user-added context; changing project/host clears all PR attachments. One detected-PR signal allows one automatic PR selection; manual selection disables it.
- States: host disconnected, choose project/host, missing model/composer, checkout loading, pending empty/chat/terminal submission, inline error plus toast (`:1023`, `:1233`, `:1290`). Prompt remains for retry. Workspace multiplicity is capability-gated.
- Platforms: compact panel navigation versus wide form, shared placement contract. Connections: host runtime, project registry, remembered form preferences, provider/model controls, draft attachments and forks, Git/forge selection.

## 11. Launch empty workspace, chat, terminal or terminal profile

### Import and launch-target selection

- Import session button (`APP/screens/new-workspace/import-session-button.tsx:12`) opens `useImportSession` for selected host (`new-workspace-screen.tsx:1523`); compact header button versus wide pill below composer (hidden in fullscreen). Import details belong to [A-lifecycle.md](A-lifecycle.md).
- Same screen offers launch target Chat or terminal profile; `APP/new-workspace-launch/target.ts:9` reserves profile id **blank** for default shell. Missing persisted profile falls back to Chat. Target and isolation are saved in form preferences.

### Empty, chat and terminal creation

- Empty chat submission (`new-workspace-screen.tsx:1295`, `screens/new-workspace-empty.ts`) provisions a workspace without an agent. Normal chat builds initial agent config `{provider,cwd,modeId,model,thinkingOptionId,featureValues}`, prompt, encoded images/attachments and stable `<draftId>:initial-message` id (`:426`).
- Combined workspace+agent creation uses `C.createWorkspace` rather than independently racing two creations. On first creation event containing workspace, app accepts it as running and hands its draft to workspace UI (`:478`); draft submission tracks the eventual agent result/retry.
- `isStillOnCreateScreen` protects navigation: if user left, creation finishes in background, clears consumed draft and releases pending lock; it does not steal the current route (`:1303`, `:1410`). Here “background” is a **submission outcome**, distinct from persisted hidden workspace `background`.
- Terminal path (`:1368`, `screens/new-workspace-terminal.ts`) provisions workspace then `C.createTerminal(workspaceDirectory,name,undefined,{command,args,workspaceId})`, updates terminal query and navigates to the terminal. Blank shell may receive typed input; profile substitutes prompt in argv. A profile without prompt placeholder shows a read-only resolved command and explicit launch button (`new-workspace-launch/composer-state.ts:51`). Terminal launch has no chat attachments/model prompt semantics.
- Failures release pending state, show error/toast and permit retry. Terminal that finished launch while screen moved remains standalone and will be reconciled into tabs on next visit.
- Platforms: same launch contract; web restores input focus keyed by target. Connections: agent lifecycle ([A](A-lifecycle.md)), providers ([B](B-providers.md)), composer ([D](D-messaging.md)), attachments/files ([p4](p4-files.md)), terminal subsystem and workspace tab reconciliation.

## 12. Workspace creation API, idempotency and placement authority

### Creation contract and placement

- `C.createWorkspace` (`C:5024`) delegates to `creations.createWorkspace`; current operation transport sends `workspace.create.request` and observes `workspace.create.update` / `agent.create.update` (`C:3241`). Operation RPC timeout is 0 there; do not apply ordinary short RPC timeout to the whole provisioning operation.
- Wire (`P:2718`): `workspaceId?` matching `wks_[a-f0-9]{16}`, `idempotencyKey?` (1–512 chars), `title?`, `firstAgentContext?`, `agent?`, `subscribe?`, `background?`, `callerAgentId?`, `source` discriminated as directory or worktree. Worktree source may resolve cwd from projectId.
- `SRV/session.ts:4230` hands intent/key to creation service, checks agent creation authority, provisions workspace, then maps initial agent cwd relative to source into new workspace. Agent cwd outside source is rejected. Response carries workspace, agent, setup result/error and request id; operation updates support early workspace visibility.
- `session.ts:6856` rejects public `internal` workspaces and agent-level `background`; hidden visibility belongs to workspace. `workspace.create.before` plugin hook transforms provision input once per operation, not on every idempotent retry.
- Local handler `:6880` checks directory existence and calls `createWorkspaceForDirectory` (`session/workspace-provisioning/workspace-provisioning-service.ts:238`), always allocating a fresh record even if cwd already has workspaces. Worktree handler `:6935` requires cwd or projectId and uses worktree workflow.
- `background` inherits caller workspace if omitted (otherwise false); missing caller workspace errors (`workspace-provisioning-service.ts:227`). It hides default discovery/sidebar without changing durable content ownership. SDK feature gate is `requireBackgroundWorkspaces` (`C:5030`).

### Durable receipts and reconnect recovery

- Creation lifecycle (`P:5027`; `SRV/creation/index.ts:64`) phases: accepted/workspace_ready/agent_ready/prompt_started/completed/failed; optional failedStage workspace/agent/prompt and outcomeUnknown. Durable `$PASEO_HOME/creations/<sha256(kind,key)>.json` receipts plus resource `.claim` files (0600) preserve ids before side effects (`SRV/websocket-server.ts:741`, `creation/index.ts:253`). Changed intent with same key rejects `<kind>_request_key_conflict`; conflicting id rejects `<kind>_id_conflict`. In-flight receipt after interruption becomes outcomeUnknown and is not blindly replayed.
- SDK reconnect (`client/src/creation/index.ts:200`) observes `creation.subscribe.request {kind,idempotencyKey}` (`P:1822`) instead of duplicating creation; only missing receipt resubmits. Revisioned updates drive existing operation; legacy fallback is capability-gated on `creationLifecycle`.
- Storage: `$PASEO_HOME/projects/workspaces.json`, not SQLite. Record includes immutable placement identity (`workspaceId,projectId,cwd`), kind, displayName/title, branch/worktreeRoot/baseBranch/mainRepoRoot/ownership, timestamps/archive, pin, labels, untrusted source and background (`SRV/workspace-registry.ts:51`).
- Connections: creation receipts/agent startup, plugins, project registry, Git observer, status and workspace updates; local cwd sharing must preserve workspace-scoped terminals/agents.

## 13. Worktree creation modes, paths and ownership

### Mode resolution

- User entries: New Workspace worktree isolation (feature 10), workspace/worktree CLI (feature 28). SDK `C.createPaseoWorktree` (`C:5001`) preserves older `create_paseo_worktree_request`; modern workspace creation also reaches `SRV/worktree-session.ts:341` → `paseo-worktree-service.ts:68` → `worktree-core.ts`.
- Intent resolver `SRV/resolve-worktree-creation-intent.ts:65`: explicit `action:'branch-off'` uses refName or default branch and branchName/worktreeSlug/`worktree`; `action:'checkout'` requires refName or change request. No action + refName means **branch-off**, not checkout. Change request input wins over legacy `githubPrNumber`.
- Checkout modes internally: `branch-off`, `checkout-branch`, `checkout-change-request`. Forge mismatch and unsupported cross-repository checkout fail explicitly; missing target maps to `missing_checkout_target`, checked-out branch to `branch_already_checked_out`, missing branch to `unknown_branch`, other failures to `unknown` (`worktree-errors.ts:4`). Forge/Git mechanics: [p2](p2-git.md), [p3](p3-forge-review.md).

### Backing path, hooks, metadata and rollback

- Default worktree root is `$PASEO_HOME/worktrees/<projectHash>/<slug>`; configured `worktreesRoot` expands `~`, resolves absolute directly or relative to Paseo home (`UTIL/worktree.ts:840`). Project hash is based on Git common dir. Workspace's exact subdirectory is mapped into the new backing tree; missing subproject fails (`paseo-worktree-service.ts:91`).
- Core applies transforming `worktree.create.before` hook to source/slug/path (`SRV/worktree-core.ts:137`), allowing plugin-placed trees outside the default root, then newly created tree invokes waited `worktree.create.after` hook, marks first-agent branch-name eligibility, seeds `paseo.json` by COPYFILE_EXCL only if destination lacks it (source missing also ignored; `UTIL/worktree.ts:789`), then persists workspace with exact backing `worktreeRoot`, execution `cwd`, source main root and comparison base (`:81–129`). Source project membership is retained.
- Metadata under Git worktree metadata `paseo/worktree.json` carries version 1/2, base ref/name, PR lookup target, first-agent rename pending/attempted and runtime port (`UTIL/worktree-metadata.ts:8`). `MAX_WORKTREE_METADATA_BYTES=64 KiB`. Exact base is retained for comparisons/recovery; bare branch display is not a substitute.
- If post-create hook/placement/registration fails, newly created tree is rolled back without teardown; preexisting trees are not rolled back (`paseo-worktree-service.ts:141`). Worktree ownership is proven from managed root, metadata or persisted record before deletion.
- Cross-repo checkout persists `untrustedSource {kind:'change_request',forge,number,headRepository}`; bootstrap and later automation honor this gate. All filesystem/Git work happens on daemon host.

## 14. Setup execution, logs and explicit trust release

### Background setup execution

- Worktree workflow returns after creation, then schedules branch naming/Git warmup/setup (`SRV/worktree-session.ts:341`). Setup is asynchronous; worktree existence does not mean dependencies installed. `shouldBootstrap=false` branch emits completed without rebootstrap, but current `worktree-core.ts:155` returns created=true; this branch is a service compatibility/injection path, not a documented UI reuse mode.
- `runWorktreeSetupInBackground` (`:507`) reads setup from **workspace cwd** `paseo.json`, resolves runtime env and registers it with terminals, runs commands sequentially, then opens configured automatic terminals. Failure stops further setup and publishes failed; workspace is retained.
- Environment (`UTIL/worktree.ts:701`): `PASEO_SOURCE_CHECKOUT_PATH`, legacy `PASEO_ROOT_PATH`, `PASEO_WORKTREE_PATH`, `PASEO_BRANCH_NAME`, `PASEO_WORKTREE_PORT`. Command shell selection is platform-aware; not executed by the app. Runtime port is recorded in worktree metadata.

### Progress, blocked automation and cancellation

- Push `workspace_setup_progress {workspaceId,status,detail,error,blockedSource?,subscriptionId?}` (`P:4542`) has **running/completed/failed/blocked**. Detail includes command results/logs. `MAX_WORKTREE_SETUP_COMMAND_OUTPUT_BYTES=64 KiB` per command with middle truncation marker; carriage-return output is normalized (`SRV/worktree-bootstrap.ts:33`). Auto-terminal readiness timeout `WORKTREE_BOOTSTRAP_TERMINAL_READY_TIMEOUT_MS=1500`.
- Setup tab `APP/panels/setup-panel.tsx:265` shows waiting spinner (no snapshot/running without commands), completed/no commands, failed error, blocked source warning and Run setup when supported. Command rows show running/check/error icon, command, duration (ms/s/m+s), selectable log; current running or last command auto-expands, manual collapse wins (`:164`, `:195`, `:408`). It subscribes only while panel active, app visible and workspaceSetupObservation supported; unsupported host gets update warning and observation failures get alert.
- App `stores/workspace-setup-store.ts:63` stores snapshots by host+workspace; failed/blocked setup claims a setup-tab surface once, later nonfailed status clears that claim. Setup UI is visible for supported nonnull state or an unsupported-host explanation (`:29`).
- Subscription: `C.observeWorkspaceSetup`/subscription surface around `C:3143` → `workspace.setup.subscribe.request {workspaceId}` → `SRV/session.ts:2387`/setup observation; initial nullable snapshot plus owned updates. Legacy fetch `C.fetchWorkspaceSetupStatus` → `workspace_setup_status_request` (`P:2626`) returns current nullable snapshot.
- Cross-repo automation block publishes blocked and skips automatic setup/name/bootstrap (`worktree-session.ts:360`). Explicit `C.runWorkspaceSetup(workspaceId)` (`C:3177`) → `workspace.setup.run.request` → `worktree-session.ts:452` clears the block and starts bootstrap only if clearing changed it; returns `{started,error}`. This is **not a generic rerun-failed-setup RPC**: already unblocked returns started=false.
- No explicit wall-clock setup/teardown command timeout is configured in `UTIL/worktree.ts:412/:444`; do not confuse 1500 ms auto-terminal readiness with setup timeout. Streamed setup abort uses tree kill graceful=1000 ms, force=1000 ms (`:530`).
- `WorkspaceSetupRuntime.stop` aborts and waits for current run during manual workspace archive (`SRV/workspace-setup-runtime.ts:27`). Setup snapshots/runtime are transient; persisted untrustedSource reconstructs blocked state after reconnect/restart.
- Connections: project config, terminals/services, setup tab, attention/creation status, metadata naming, plugin hooks. Platforms share progress protocol; daemon shell implementation determines commands.

## 15. Workspace archive, bulk archive and managed disk removal

### Archive UI and helper boundaries

- UI: sidebar/workspace header Archive → `APP/workspace/use-workspace-archive.ts:39`; risky worktree warning delegates to [p2 archive warning](p2-git.md). `workspace-archive.ts:90` hides the workspace optimistically, suppresses stale re-upserts via pending marker, restores prior snapshot on failure. Bulk helper `:103` uses allSettled and reports individual host/workspace failures; no production caller of `archiveWorkspacesOptimistically` or `selectProjectWorkspacesToArchive` was found under APP, so this is helper capacity, not a verified bulk-workspace UI entry.

### Lifecycle teardown, scope and disk removal

- `C.archiveWorkspace(workspaceId)` (`C:3128`) → `archive_workspace_request {workspaceId}` (`P:2710`) → `SRV/session.ts:7566` → `workspace-archive-service.ts:157` `archiveByScope({kind:'workspace',workspaceId})`.
- Sequence: stop setup, mark archiving/push, archive workspace-owned live and stored agents, kill workspace terminals, archive record, conditionally teardown/delete backing, finally clear transient archiving and republish. Agent/terminal teardown failures are logged and continued; completion is not proof every subprocess gracefully exited (`workspace-archive-service.ts:482`). Terminal kill uses graceful 2000 ms + force 1500 ms (`:620`).
- Local directories and unowned trees remain. Owned backing is removed only after its last active workspace reference disappears; shared cwd or shared backing subdirectories must not delete a sibling's files. Ownership proof failure reports directoryLeftBehind path/reason internally.
- Teardown reads config at exact archived workspace cwd and deduplicates equivalent teardown targets. Manual session archive supplies `assertWorkspaceAutomationAllowed` to exclude untrusted targets; optional dependencies differ for automatic archive (feature 16). A teardown failure leaves the already archived workspace's directory; disk removal failure likewise leaves it (`workspace-archive-service.ts:345`). Do not assume archive is reversible filesystem trash.
- Older worktree archive RPC `paseo_worktree_archive_request` (`P:2579`) accepts `worktreePath?/repoRoot?/branchName?/workspaceId?/scope?`; scope defaults **workspace**, explicit **worktree** archives all records at backing. Legacy `deleteWorktreeFromDisk` is retained but ignored; deletion derives from ownership and references.
- Disk removal (`UTIL/worktree.ts:1156`) uses forced Git worktree remove (120000 ms), filesystem retry removal and best-effort prune (30000 ms). **No branch deletion occurs here**, despite CLI archive help claiming associated branch removal. RPC success does not expose removedDirectory or prove every teardown step succeeded.
- Archiving final workspace keeps its project active; sidebar renders empty project + New Workspace. App success purges workspace-local draft/attachment/UI state and redirects active route through archive-navigation policy (`use-workspace-archive.ts:15`, `utils/workspace-archive-navigation.ts`).
- Platforms: same confirmation and state model; affected directories live on selected host. Connections: agents, terminals, setup cancellation, scripts, tabs/drafts, sidebar/attention, Git ownership.

## 16. Automatic archive after merge

### Host toggle and merge observation

- Entry: host settings switch (`APP/screens/settings/host-page.tsx:910`, `:933`). Host config key `daemon.autoArchiveAfterMerge`, wire mutable field `autoArchiveAfterMerge`, default **false** (`P:237`, `SRV/daemon-config-store.ts:193`). Uses daemon-config read/patch; this is not a project `paseo.json` key.
- `SRV/auto-archive-on-merge/index.ts:26` observes WorkspaceGitService snapshots. It must first see the **same URL open**, then merged; merely finding a historical merged PR does not trigger. `OPEN_PULL_REQUEST_LATCH_MAX=1024` LRU; disabled clears latch, per-cwd in-flight set prevents concurrent duplicate work.

### Archive guards and consumed merge identity

- Before archive it reads a fresh snapshot and revalidates merged URL/latch, enumerates active workspaces with exact resolved cwd, then calls `archiveIfSafe` per workspace (`archive-if-safe.ts:51`).
- Skip when `isDirty === true`, or numeric `aheadOfOrigin > 0`, ownership not allowed, or stored `autoArchivedChangeRequestUrl` already equals PR URL. **Unknown dirty/ahead values are not explicitly rejected by these predicates**; do not describe the code as requiring proven clean and fully pushed.
- Uses workspace-scope archive machinery, preserving last-reference disk rules; stores consumed merged URL in workspace record. Restore retains/sets this marker to avoid immediately autoarchiving the same restored workspace again.
- Invocation boundary: `auto-archive-on-merge/archive-if-safe.ts:85` does **not** supply optional `stopWorkspaceSetup` or `assertWorkspaceAutomationAllowed` to archiveByScope, unlike manual `SRV/session.ts:7589`. Do not attribute those manual archive protections to automatic archive from the shared function name alone.
- No RPC originates from the observer, and app need not be foreground; registry/workspace updates propagate resulting removal. Failures log and skip; disabling is host-wide.
- Connections: [forge PR state](p3-forge-review.md), [Git snapshots](p2-git.md), host settings, archive lifecycle, restoration. Platforms: server behavior shared by every client.

## 17. Archived workspace inspection, unarchive and worktree restoration

### Recovery inspection

- Entry: visiting missing/archived workspace route → `APP/screens/workspace/workspace-route-state.ts:31`, state views, `workspace-recovery/use-workspace-recovery.ts:30`. Requires `server_info.features.workspaceRecovery`; cached existing workspace can still render reconnecting while offline.
- `C.inspectWorkspaceRecovery(workspaceId)` (`C:3537`) → `workspace.recovery.inspect.request {workspaceId}` → `SRV/session.ts:3990` → `session/workspace-recovery/workspace-recovery-service.ts:72`. Returns authoritative recoverable action or unavailable reason; client does not infer recovery from path shape.
- Unavailable reasons: `workspace_not_found`, `workspace_not_archived`, `project_not_found`, `workspace_directory_missing` (non-worktree), `project_directory_missing`. Existing archived cwd → **unarchive**; missing worktree cwd with available source repo → **restore**.

### Restore execution and route presentation

- `C.restoreWorkspace(workspaceId)` (`C:3552`) → `workspace.recovery.restore.request` → `session.ts:4003`; service **re-inspects** before mutation. Restores worktree from persisted branch/base (or `restored/<8 UUID chars>` from base), prunes stale registrations, recreates exact backing path and maps relative subproject cwd (`workspace-recovery-service.ts:150`). Wrong/missing restored subdirectory rolls back newly created tree without teardown.
- Then `workspaceProvisioning.ensureWorkspaceRecordUnarchived` clears archive, refreshes placement and restores project if needed, retains consumed merge marker, and pushes workspace descriptor. Recovery then synchronizes Git observer and emits workspace (`SRV/session.ts:1538`); no setup/auto-terminal rerun is called by this recovery chain. This is workspace recovery, not automatically resuming all archived agents; their lifecycle remains separate.
- UI state model (`APP/workspace-recovery/model.ts:60`): idle, checking, needsHostUpgrade, recoverable ready/restoring/failed, unsupportedAction, unavailable, inspectionFailed. Inspection stale window 5000 ms, retry=false; explicit retry supported. Restore shows two animation frames then `MIN_RECOVERY_LOADING_MS=350` before RPC (`use-workspace-recovery.ts:10`).
- Route states additionally distinguish ready/reconnecting/unreachable/loading/missing/archived/recoveryUnavailable/recoveryInspectionFailed. New unknown recovery actions ask to update Paseo instead of treating them as restore.
- Platforms: shared recovery UI; filesystem/Git restore server-side. Connections: archived history/deep links, project registry, Git branch/base, merge-consumption marker, tabs and host reconnect.

## 18. Assign, create and remove workspace labels

### Label picker and assignment

- Entry: workspace row/header Labels submenu (`APP/components/sidebar/sidebar-workspace-menu.tsx:245`; `workspace-labels/picker.tsx:72`). Picker searches definitions, checks existing assignments and opens Create label with color selection (`:110`, `:209`). This mutates workspace labels, not agent labels.
- `WorkspaceLabelPickerModel` (`workspace-labels/internal/workflow-model.ts:34`) owns pending/error state; `C.setWorkspaceLabel({workspaceId,label:{name,color},assigned})` (`C:2907`) → `workspace.label.assignment.set.request` (`P:1056`) → `SRV/session.ts:6670` → `WorkspaceLabelService.setAssignment` (`workspace-labels/internal/service.ts:85`).
- Server rejects missing/archived workspace, collapses whitespace and trims label name; lookup is case-insensitive (`protocol/src/workspace-labels.ts:18`). Empty → `label_name_empty`; no maximum name length/count found in the wire schema or `requireName` (`service.ts:298`).
- Allowed colors: violet, sky, emerald, orange, pink, indigo, teal, red, amber, blue (`protocol/src/workspace-labels.ts:1`). Existing host definition's spelling/color wins; assignment of unknown name creates definition only when assigned=true. Unassigning unknown label does not create catalog entry.

### Committed assignment and host catalog colors

- Response `{label,workspaceLabels}`; workspace mutation emits fresh descriptor; new definition publishes `workspace.label.update` upsert. Workspace stores ordered **names**, catalog owns colors. Empty assignment list becomes omitted/undefined in registry.
- Storage: `$PASEO_HOME/projects/workspaces.json` assignments plus `workspace-labels.json` and recoverable `workspace-labels.transaction.json` compound commit (`SRV/workspace-labels/internal/catalog-store.ts:89`). Publication follows commit; uncertain durable state blocks mutations rather than inventing success.
- Row colors resolve only from that workspace's own host catalog (`APP/workspace-labels/index.ts:103`); same name on two hosts may have different colors. Offline cached catalog remains for row rendering; first-load unknown names are omitted.
- Platforms: dropdown page / compact sheet with same model; feature gate `workspaceLabels`. Connections: sidebar label chips/filter, host replica, workspace registry, manager (feature 19), command center search.

## 19. Label manager, rename/recolor/delete and catalog synchronization

### Manager edits and deletion

- Entry: `APP/workspace-labels/manager-modal.tsx:52`; search list → edit name and swatch → Save/Delete (`:151`, `:295`). `WorkspaceLabelManagerModel` owns host target, draft, search, pending/error and delete inspection (`internal/workflow-model.ts:129`).
- `C.updateWorkspaceLabel({name,newName?,color?})` (`C:2928`) → `workspace.label.update.request` (`P:1068`) → `SRV/session.ts:6684` → service `:141`: rename+color are **one atomic catalog commit**. Case-only rename allowed; collision with another normalized name → `label_name_taken`, missing label → `label_not_found`. Rename rewrites workspace assignments; recolor touches catalog only.
- `C.inspectWorkspaceLabelDelete({name})` (`C:2954`) → `workspace.label.delete.inspect.request` → `session.ts:6712` counts affected workspaces for confirmation. `C.deleteWorkspaceLabel({name})` → `workspace.label.delete.request` → `session.ts:6698` removes catalog definition and all matching assignments. Missing label returns affected count 0; count is advisory, not a lock on future changes.

### Catalog replication and cross-host projection

- `C.listWorkspaceLabels({sync?})` / `observeWorkspaceLabels()` (`C:2879`, `:2896`) → `workspace.label.list.request {subscribe?,sync?:{generation,afterSeq}}` → `session.ts:6612`. Returns labels + sync `{mode:'snapshot'|'changes',generation,headSeq,removals}` (`P:4420`). Push `workspace.label.update` has upsert `{label,previousName?}` or remove `{name}`, generation and positive seq.
- `APP/workspace-labels/index.ts:145` maintains per-host `HostWorkspaceLabelReplica`, releases owned subscription on disconnect, applies snapshot/updates, refetches after replica mismatch. Cross-host projection merges **online** catalogs; target host wins, otherwise lexicographic serverId, then labels sort by case-insensitive name (`internal/merge.ts:9`). Default sequence journalLimit=256; expired/different generation returns full snapshot (`SRV/workspace-labels/internal/sequence.ts:36`).
- Manager state includes offline/online/unsupported host and per-host load failure. Persisted sidebar label filter is reconciled only when connected catalogs are authoritative; failed first fetch is not evidence a label was deleted (`index.ts:35`).
- Storage uses the compound journal in feature 18 to recover catalog/workspace consistency at initialization. Mutation and publication sequencing is host-local; no cross-host transaction is implied by merged display.
- Platforms: adaptive modal/sheet, shared model. Connections: workspace label assignments, sidebar filters/command center, host connection and owned subscription/directory sequencing.

## 20. Workspace and project registry updates to the app

### Registry persistence

- User effect: newly added/renamed/archived workspaces and empty projects appear across connected clients without reopening a screen. `SRV/bootstrap.ts:942` instantiates project/workspace JSON registries; `workspace-registry.ts:273` validates arrays with Zod and caches them. ENOENT becomes empty; other load errors are logged (not silently declared valid).
- Registry writes stage a Map under a mutation queue, persist via atomic JSON write, then commit cache and notify listeners (`workspace-registry.ts:306`). Project allocation separately serializes active-root dedupe (`:394`); generated ids preserve old compatible ids rather than rekeying them.

### List, subscribe, resume and app reconciliation

- App `runtime/host-runtime.ts:2756` → `runtime/directory-sync/index.ts:601` → `C.observeWorkspaces`/`fetchWorkspaces` (`C:2848`) and `C.listProjects` (`C:2967`). Wire `fetch_workspaces_request` (`P:1329`) supports query/projectId filter, sort by status_priority/activity_at/name/project_id, page limit 1–200, subscription, sync cursor and includeBackground. Legacy idPrefix/includeInternal are parse-only.
- `SRV/session.ts:6433` begins subscription **before** snapshot, queues concurrent mutations, emits `fetch_workspaces_response {entries,emptyProjects,pageInfo,sync?,subscriptionId?}`, then flushes bootstrap updates. Empty projects are first-page-only in ordinary pagination (`P:4369`); `project.list.request` separately lists active projects (`session.ts:6526`).
- `session.ts:1735`, `:1786` subscribes registry mutations; archive/remove drops Git observer, upsert syncs observer and emits `workspace_update`. Project mutation `:1847` republishes dependent descriptors and project changes, including projects without workspace rows.
- Wire update (`P:4390`): upsert `{workspace}`, remove `{id,emptyProject?,removedProjectId?}`, optional subscriptionId/generation/seq; `project.update` upsert/remove is independent (`P:4487`). Sync supports snapshot/changes and removals; reasons include no_cursor/generation_changed/cursor_expired/changes_too_large (`P:1289`).
- `WorkspaceDirectoryReplica` (`APP/runtime/directory-sync/workspace-replica.ts:36`) normalizes into per-host Zustand maps, applies project updates to dependent workspace display fields, script deltas, and setup status. Snapshot plus buffered deltas commits atomically from the controller perspective; cached map seeds initial UI before hydration.
- Creation responses only seed missing workspace rows (`:84`), so late creation progress cannot overwrite newer authoritative directory state. Local pending archive suppresses stale resurrection. Full successful snapshot sets hydrated flag; cached presence alone is different.
- Platforms: same replication model; local cache/layout storage is platform-adapted. Connections: every project/workspace action, sidebar, status, Git observers, script health, reconnect, creation receipts. No SQL tables exist for these registries.

## 21. Filesystem reconciliation and stable identity

### Filesystem observation

- User effect: external Git init/remote changes update project kind/equivalence and checkout metadata; deleted worktrees become archived when safely determined. Service is `SRV/workspace-reconciliation-service.ts:121`, boot integration `workspace-registry-bootstrap.ts:62`; it does not require the user to reload the project.
- Active project roots are watched nonrecursively; fallback periodic rescan `DEFAULT_RESCAN_INTERVAL_MS=5 min`, debounce `DEFAULT_DEBOUNCE_MS=100 ms`; directory inspection `INSPECTION_CONCURRENCY=2`, `DEFAULT_INSPECTION_TIMEOUT_MS=5000`, yielding every 32 completions (`:22`, `:664`).
- Metadata-only pass (`:210`) never archives missing records. Full pass (`:250`) archives missing workspace cwd only while its project root is reachable; missing/unreadable root is not proof all its workspaces were deleted (`:281`). Rechecks live registry identity/archive before commit.

### Identity-preserving metadata and archive transitions

- Immutable policy in `workspace-registry-model.ts`: reconciliation preserves project root/id/default name, workspace projectId/cwd/displayName/baseBranch. Mutable Git-derived kind, branch/backing/ownership and `projectKey` may refresh. `customName` and workspace `title` remain user overrides.
- `projectKey` (`SRV/project-key.ts`) groups logical projects across hosts, using normalized remote when available else local root; it is persisted metadata separate from host-local projectId. App must not derive identity from today's remote/branch name.
- Missing worktree record preserves its original backing/source/base so recovery works after deletion; source tree `cwd` and project root are not interchangeable. Exact subprojects remain exact execution directories.
- Registry mutation events carry results through feature 20; no new app RPC starts this background pass. UI may show archived recovery on stale deep link and updated project grouping after key changes.
- Platforms: daemon filesystem/watch differences, same bounded fallback/identity policy. Connections: [Git checkout metadata](p2-git.md), empty-project persistence, sidebar grouping, archive/recovery, provider imports/legacy bootstrap.

## 22. Workspace status, activity, attention and read/unread

### Status aggregation

- Entry: workspace status glyph/sidebar sorting/header plus Mark as read/Mark as unread (`APP/components/sidebar/sidebar-workspace-menu.tsx:208`; `hooks/use-workspace-read-state.ts:16`). Status buckets are **needs_input, failed, running, attention, done** (`P:1321`).
- `SRV/workspace-directory.ts:216` assembles record descriptors from workspace-root agents, running subagent contributions and workspace-owned terminal activity. Highest-priority bucket wins; an agent status does **not** fan out to same-cwd sibling workspaces (`:366`). Empty workspace derives done with creation timestamp until activity exists.
- `statusEnteredAt` is an aggregate transition timestamp, not merely newest message time (`:445`); activityAt/sort and last prompt timestamps are distinct. The server bucket-history cache is process-local and resets on cold start (`workspace-directory.ts:174`). Pending initial agent creation overlays done as running in `APP/workspace-status/index.ts:13`; existing non-done server status is authoritative. Root activity can also override done.

### Mark read and mark unread

- `useWorkspaceReadState` only enables clear for attention/failed, unread for done + `workspaceMarkUnread`. `C.clearWorkspaceAttention` → `workspace.clear_attention.request {workspaceId:string|string[]}` → `SRV/session.ts:7634`; clears owned live/stored agent attention **except permission attention**, and returns per-workspace results/pushes updates.
- `C.markWorkspaceUnread` → `workspace.mark_unread.request {workspaceId}` → `session.ts:7770`; chooses a finished workspace-root agent, preferring newest activity, then restores attention. No suitable finished agent → error. Candidates must be unarchived, idle/closed, without requiresAttention or pendingPermissions and visible provider; ties use id. Empty workspace cannot manufacture unread agent activity.
- App `workspace/mark-unread.ts:5` unfocuses the pane **before** RPC so ongoing view tracking cannot instantly consume the mark; restores focus token on failure. `workspace/focus.tsx:27` handles temporary unfocus/restoration for overlays.
- Attention is persisted through agent state, not a standalone workspace unread boolean. Workspace descriptor update feeds status to all devices; client viewing state controls clearing. Permission/question resolution still belongs to the agent workflow.
- Platforms: status and menu shared; focus/visibility model adapts panes/compact UI. Connections: [agent lifecycle](A-lifecycle.md), [timeline](C-timeline.md), permissions/questions, terminal activity, sidebar sort, badges/notifications.

## 23. Workspace rename and pin/unpin

### Workspace title

- Entry: sidebar context/kebab, workspace header and command center actions. `APP/components/workspace-rename-modal.tsx:32` uses AdaptiveRenameModal initialized from title or name; errors remain inline and modal stays open. The modal uses shared required-name validation (`components/rename-modal.tsx:68`), so blank submission is blocked despite callback converting blank to null. Reset is available through SDK/null or CLI `--reset`, not this modal.
- `C.setWorkspaceTitle(workspaceId,title|null)` (`C:3497`) → `workspace.title.set.request` → `SRV/session.ts:3872`; writes registry `title`/updatedAt, accepted response and workspace update. This does not rename the Git branch, directory, project or agent tab.

### Pin and keyboard ownership

- Pin action `APP/hooks/use-sidebar-workspace-pin.ts:23` → `C.setWorkspacePinned(workspaceId,pinned)` (`C:3517`) → `workspace.pin.set.request` → `session.ts:3944`; pinned=true records current ISO timestamp, false null. Missing workspace/host or RPC failure shows error; authoritative update moves row to/from pinned section.
- Module-level pending workspace-key set blocks opposite concurrent toggles from independent menu/keyboard controllers (`use-sidebar-workspace-pin.ts:20`), cleared on settled. Pin is host-persisted and shared, while sidebar ordering/collapse are device preferences.
- Global pin shortcut controller `hooks/use-global-workspace-pin-action.ts:10` targets active route; it does not require a mounted sidebar row. Rename host `components/workspace-rename-host.tsx` similarly separates modal ownership from row lifetime.
- UI returns derived displayName when title null; project rename is feature 06. Reconciliation preserves user title and pin metadata. SDK accepted=false is an error, not successful optimistic rename.
- Platforms: adaptive modal and touch menu, keyboard on supported surfaces. Connections: sidebar pinned projection, command center, registry updates, active navigation; independent of agent-tab pin for subagent visibility.

## 24. Sidebar project/workspace rows, ordering and navigation

### Project/workspace row projection

- Entry/render: `APP/components/sidebar-workspace-list.tsx:852` project header, `components/sidebar/sidebar-workspace-row.tsx`, row content; host projects → `projects/workspace-structure.ts:54` → `hooks/sidebar-workspaces-view-model.ts:270` → sidebar projection.
- Project view groups equivalent projects across hosts by persisted projectKey; host-local ids stay separate placements. Duplicate logical roots on one host receive separate placement keys (`workspace-structure.ts:128` onward). Names use custom overrides; icons target a concrete host, and host badges disambiguate placement.
- Project expand/collapse persists locally (`stores/sidebar-collapsed-sections-store.ts`); header shows aggregate status and New Workspace action when a host supports worktrees or `workspaceMultiplicity` (`utils/sidebar-project-row-model.ts:75`). Empty project renders New Workspace ghost row (`sidebar-workspace-list.tsx:785`), not a fake workspace.

### Menus, metadata and visible ordering

- Project menu (`:529`): Open settings, Electron Open in new window, local Open folder, Remove project (feature 07). New window path is resolved from local daemon placement, not arbitrary remote root (`utils/sidebar-project-row-model.ts:65`).
- Workspace menu (`sidebar-workspace-menu.tsx:146`): Copy path, Copy branch (when available), Rename, Mark as read/unread, Pin/Unpin, Labels, local Open folder, Archive. Pending creation shows creating; archiving disables/replaces action state. Row title and branch metadata are separate (`sidebar-workspace-title.ts`).
- Meta row `components/sidebar/workspace-meta-row/index.tsx` combines branch/project/host, forge/check indicators, running service health and label chips; status glyph consumes effective workspace status. PR actions link to [p3](p3-forge-review.md); branch copy/status to [p2](p2-git.md).
- Sorting includes manual, created, lastPrompt (`workspace-structure.ts:33`), with stable key tie-break; stored ordering retains known keys and adds new ones (`sidebar-workspaces-view-model.ts:474`, `:560`). Project/status/label filters change visible projection and numbered shortcut targets, not registry membership.
- Loading distinguishes first hydration from refreshing while rows exist (`sidebar-workspaces-view-model.ts:597`). Clicking resolves `{serverId,workspaceId}` and active tab; archive uses nearby-workspace/empty-project fallback navigation rather than cwd guessing.
- Platforms: web context-menu/hover affordances; native/compact controls stay accessible and menus use sheets (`sidebar-workspace-menu.tsx:298`). Electron-only window/folder actions; offline hosts retain cached representation but mutations need connected client. Connections: all project/workspace actions, host runtime, status, sidebar settings, shortcut target ordering.

## 25. Workspace scripts/services, start/stop/restart and URL selection

### UI script and service actions

- Entry: workspace Scripts button/menu and script terminal panels (`APP/screens/workspace/scripts/workspace-scripts-button.tsx`, `workspace-scripts-menu-pages.tsx`); configuration comes from feature 09. Named scripts and services are distinct types, not synonyms.
- **Current UI path** `use-workspace-script-actions.ts:51`: start → `C.startWorkspaceScript(workspaceId,scriptName)` (`C:3069`) → legacy `start_workspace_script_request` (`P:3070`) → `SRV/session.ts:7348` → `workspace-scripts-service.ts:244`; returns terminalId and opens its tab when present. Stop uses **`C.killTerminal(terminalId)`**, not the newer script-stop RPC.
- Restart records script name in a pending set, kills its terminal, then waits until descriptor reports lifecycle no longer running before restarting (`use-workspace-script-actions.ts:132`). A failed stop removes pending restart and shows toast. Missing daemon/terminal, unavailable script or failed launch surface errors.

### Named APIs, runtime status and service routes

- Typed tool/CLI surface: `C.listWorkspaceScripts`, `startWorkspaceScriptWithStatus`, `stopWorkspaceScript` (`C:3087–3124`) → `workspace.script.list/start/stop.request {workspaceId,scriptName?}` (`P:3076`) → `SRV/session.ts:7352/7379/7408` → service `list/launch/stop` (`workspace-scripts-service.ts:166/201/216`). Stop awaits supervised terminal exit; absent/not-running terminal is explicit error.
- Service resolves exact workspace cwd/config, checks automation trust, spawns via `spawnWorkspaceScript`, allocates/records ports and proxy routes, and owns exit listener cleanup. `WorkspaceScriptRuntimeStore` is an in-memory map keyed **workspaceId + scriptName**, so two records sharing cwd remain independent.
- Status push `script_status_update {workspaceId,workspaceDirectory,scripts}` (`P:4534`) reaches all clients; app replica patches scripts array. Payload (`P:4072`) has name/type (`service` wire fallback), hostname/port, local/public/legacy proxy URL, lifecycle running/stopped, nullable health healthy/unhealthy, exitCode and terminalId.
- UI tones (`scripts/script-status.ts:19`): stopped idle, running script blue, service healthy green/unhealthy red/unknown running; any unhealthy service dominates summary, healthy only if all running services report healthy. Sidebar reports **services only**, preferring failing one (`components/sidebar/workspace-meta-row/service-summary.ts`).
- URL targets derive from active connection + server-provided routes (`utils/workspace-script-links.ts`); user chooses route and copies URL, preferred kind stored per server (`workspace-service-routes/store.ts`). This is not permission to fabricate localhost for a remote host.
- Platforms: shared script actions; URL opening/terminal renderer differs by client. Connections: project config, setup/auto-terminals, workspace ownership/archive, terminal lifecycle, service proxy/health, sidebar status. Deep proxy implementation is outside this area.

## 26. Workspace tab catalog, identity and automatic reconciliation

### Launcher and tab target identity

- Entry: New tab (+/menu), blank-tab launcher and compact tab switcher (`APP/screens/workspace/workspace-new-tab-menu.tsx`, `workspace-tabs/launcher/index.tsx:107`, `screens/workspace/compact-workspace-header/tab-switcher.tsx`). Catalog groups built-ins, plugin panels and terminal profiles with Edit profiles link.
- Primary order **agent, terminal, changes, diff, files, browser, pullRequest**; supporting order **changes, terminal, diff, files, agent, browser, pullRequest** (`workspace-tabs/launcher/internal/catalog.ts:1`). Changes/PR/browser obey capability/context flags, terminals can disable, compact hides standalone Diff because Changes shows inline diff (`launcher/index.tsx:168`). Plugin panel availability checks manifest location/host.
- Target union (`workspace-tabs/model.ts:34`): new_tab, draft, agent, provider_subagent, terminal, browser, changes_tree, files, pull_request, file, working_diff, plugin, setup, commit_diff. Draft stores provider/cwd/mode/model/thinking/features; tab includes createdAt and optional JSON panel state.
- Identity is target-based (`workspace-tabs/identity.ts`): agent id, terminal id, browser id, setup workspace id, commit SHA, plugin+panel+context+agent; file path + location normalization. New tabs get unique instance ids, not deterministic target ids. Workspace persistence key **serverId:workspaceId**, never cwd.

### Hydration-safe reconciliation

- `screens/workspace/workspace-screen.tsx:1523` reconciles layout against hydrated agent/terminal snapshots. `workspace-tabs/agent-visibility.ts:20` only uses workspace-owned nonarchived agents, automatically opens roots; foreign workspace agents are excluded. Archived agent tabs prune only after agent hydration (`:128`), preventing temporary startup data gaps from wiping layout.
- Standalone terminals auto-open; pending terminal/draft creates preserve interim state. Same-workspace child sessions are shown through subagent policy rather than blindly auto-opening every child; cross-workspace roots follow ownership, not cwd.
- Open/reveal/background is local `useWorkspaceLayoutStore.openTab` (`stores/workspace-layout-store.ts:100`); opening passive content sends no “create tab” RPC. Launching a new agent/terminal calls that subsystem; tab state then follows returned/streamed entity.
- Platforms: wide horizontal tabs, compact switcher/sheet tabs; supporting panels filtered by current host/platform manifest. Connections: [agents](A-lifecycle.md), [Git](p2-git.md), [PR](p3-forge-review.md), [files](p4-files.md), terminal/browser/plugins and setup snapshots.

## 27. Tab menus, rename/reload/copy and lifecycle-aware close

### Tab menu actions

- Menu builder `APP/screens/workspace/workspace-tab-menu.ts:181` supplies agent Copy resume command/Copy agent ID, terminal Copy terminal ID, file Copy path/Open with…, agent/terminal Rename, Close left/right/others, agent Reload, Close. At first/last/only tab, impossible bulk actions disable. Mobile labels are Close above/below instead of left/right.
- Rename `use-workspace-tab-rename.tsx:32` → `C.updateAgent(id,{name})` or `C.renameTerminal({terminalId,title})`; trims, updates agent/terminal query and leaves failures in modal. UI maxLength=200 (`:129`). This changes backing entity title, not workspace title.

### Lifecycle close and bulk close

- Single terminal close (`workspace-screen.tsx:1904`) confirms destructive action, optimistically removes cache/layout and calls terminal kill; failure invalidates query. Root agent close (`:1942`) confirms if running, removes tab, archives agent; archive lifecycle belongs to [A](A-lifecycle.md).
- Child-agent close can be **layout-only** under `resolveCloseAgentTabPolicy`: await `C.updateAgent(...,{labels:{[getOpenAgentTabLabel(clientId)]:'false'}})`, re-evaluate policy after response, then hide tab. Failure leaves tab and shows toast. It must not archive the child just because its tab was closed.
- Passive tabs close locally through panel cleanup/close guards (`:2016`, `:2048`); dirty file close semantics are owned by [p4](p4-files.md). Setup tab close does not cancel daemon setup.
- Bulk close (`workspace-bulk-close.ts:63`) partitions archive-agent, layout-only-agent, terminal and passive tabs; confirmation explicitly counts destructive content. `C.closeItems({agentIds,terminalIds})` sends one batch, while layout-only agent tabs use their separate label update. UI closes relevant layouts; async batch failure is logged, not a transactional UI rollback (`:132`).
- Reload is agent reload, not daemon restart; copy resume command is provider-specific agent action. File panels may offer multiple renderers; no additional project/workspace mutation is involved.
- Platforms: desktop context menu/horizontal location, mobile menu/vertical locations, same lifecycle boundaries. Connections: agent archive/reload, per-client open-tab labels, terminals, unsaved-file guards, local layout persistence.

## 28. CLI project, workspace and worktree commands

### Project CLI

- `cli/src/commands/project/index.ts:9`: `paseo project create [path]`, `ls`, `rename <project-id> [name] [--reset]`, `delete <project-id>`; shared JSON/daemon-host options. Create registers a directory via `C.addProject`, does not initialize Git; delete uses project-remove semantics (feature 07).
- Project-create path handling `project/create.ts`: local daemon defaults to caller cwd/resolves local relative path; explicit remote host requires an explicit path so it is interpreted by target host. Rename/reset validates arguments and uses `C.renameProject`; no icon CLI was found in project command registration.

### Workspace CLI and worktree CLI

- `cli/src/commands/workspace/index.ts:11`: `create --isolation local|worktree`, `ls [--background]`, `rename <workspace-id> [title] [--reset]`, `archive <workspace-id>`, `setup <workspace-id>`. Create options include `--path`, `--project`, `--title`, `--mode`, `--worktree-slug`, `--new-branch`, `--base`, `--branch`, `--pr-number`, `--forge`, `--background/--no-background`.
- `workspace/create.ts:125` maps isolation to `C.createWorkspace`; worktree mode defaults branch-off. Local rejects all worktree-only flags; checkout-branch requires branch; checkout-pr requires positive integer PR number and rejects conflicting branch/base flags. Without path: local uses cwd, worktree uses project root if project given else cwd.
- Caller agent id is resolved and passed to inherit background. Returned table fields are workspace id/project/name/isolation/cwd; background listing adds visibility column (`workspace/shared.ts:4`). `workspace/ls.ts:34` paginates active records; does not list archived rows by default.
- Workspace setup gates `workspaceSetupRun` and uses `C.runWorkspaceSetup` (`workspace/setup.ts:32`), reporting allowed/started state; not an unconditional rerun. Rename title reset does not rename branch. Archive uses workspace id, not ambiguous cwd.
- `cli/src/commands/worktree/index.ts:9`: `worktree ls [--cwd]`, `create --mode branch-off|checkout-branch|checkout-pr` with relevant branch/base/PR flags, `archive <name> [--cwd]`. List uses `C.getPaseoWorktreeList`, create older `C.createPaseoWorktree`; archive resolves name/path then calls `C.archivePaseoWorktree({worktreePath,scope:'worktree'})` (`worktree/archive.ts:109`). Thus it affects every active workspace on that backing, unlike `workspace archive`.
- SDK list wire `paseo_worktree_list_request {cwd?,repoRoot?}` → `SRV/session.ts:5275` → `worktree-session.ts:141`; create/archive handlers `:256/:193` use shared services. List response includes managed paths/branch data; errors are structured/printed through CLI output wrapper.
- Platforms: commands run where CLI is installed; daemon target owns paths/processes. No workspace restore subcommand found in `workspace/index.ts` at this revision; recovery exists via SDK/UI. Connections: same registry and lifecycle APIs as UI, host targeting, MCP caller inheritance.

## 29. Split panes, side pane, Explorer and compact sheets

### Pane layout and drag

- Entry: tab drag/reorder, split/focus/move/close pane actions, Explorer toggle and supporting-view open (`APP/screens/workspace/workspace-screen.tsx:2620`, `workspace-tabs/open-beside.ts:90`, `open-supporting-view.ts:44`). All layout changes are client-local; no daemon split-pane RPC.
- `stores/workspace-layout-store.ts:117` modes: **compact**, **single-pane**, **split-panes**; publisher chooses compact first, otherwise `supportsDesktopPaneSplits()` (`workspace-tabs/layout-mode-publisher.tsx:5`). Split capability is `isWeb`; native tablets can be wide yet single-pane (compact breakpoint below md=600 points); do not equate width with desktop split capability.
- Layout is tree of horizontal/vertical groups and panes with tab order/focused tab/focused pane. `MAX_TREE_DEPTH=5` preserves four user split levels below Explorer bookkeeping root (`workspace-layout-store.ts:230`); `MIN_SPLIT_SIZE=0.1` (`workspace-layout-constants.ts:1`). Drag policy permits same-pane reorder without cross-pane support, rejects split/cross-pane drop when disabled (`components/workspace-tab-drag-policy.ts:1`).

### Supporting-view placement, sheets and persistence

- Side-open preferences route implicit targets to main/side while revealing existing target where user left it (`workspace-tabs/open-beside.ts:104`); parentTabId can preserve originating pane. Preview replacement only reuses unmodified file preview or working-diff target (`:63`); dirty file is not discarded.
- Changes/composer Changes open Explorer on compact/single-pane; wide first composer action may reveal Changes Explorer and subsequent action opens preferred Diff. PR destination is explorer/main/side (`open-supporting-view.ts:64`); automatic PR tab detection is once-per-workspace background insertion, not forced selection (`:98`). Git/PR contents: [p2](p2-git.md), [p3](p3-forge-review.md).
- Compact reveal uses target-aware sheet policy (`workspace-tabs/compact-sheet.ts:20`); unknown agent defaults root/tab, known subagent can go to sheet. Sheet promotion preserves tab id/state. Dismissal protects modified work and uses parent-child sheet relationships (`workspace-layout-store.ts:841`, `:891`, `:917`).
- Explorer is a persisted shell docked outside workspace split rendering; closing/hiding it preserves the shell so toggle can restore it. Ordinary pane close removes pane after caller handles tab teardown. Layout focus restoration tokens prevent overlays from accidentally claiming read/view focus.
- Persistence: validated AsyncStorage entry **workspace-layout-state**, schema version/migrations (`workspace-layout-store.ts:2292`; `workspace-layout-storage.ts:101`), with layout trees, split sizes, explorer width, pane ids, pinned agents and PR-autoopen marker; `WORKSPACE_LAYOUT_PERSIST_VERSION=2`. Commit-diff tabs specifically strip before persistence to avoid restoring rebased SHAs (`workspace-layout-store.ts:997`); sheets are outside persisted slices.
- Platforms: web drag implementation (`components/workspace-tab-drag-scope.web.tsx`) vs native fallback; compact sheet vs wide dock/splits. Connections: panels/plugins, file unsaved state, Git/PR navigation, focus/attention and device settings.

## 30. Workspace navigation, tab retention and keyboard commands

### Route selection and retention

- Route `/h/[serverId]/workspace/[workspaceId]` has explicit host+workspace identity; `APP/navigation/workspace-route-navigation.ts`, `stores/navigation-active-workspace-store/navigation.ts`, `workspace-tabs/navigation.ts` resolve selection/focused tab. Sidebar and command-center navigation use the same active selection.
- Warm mounted deck (`screens/workspace/workspace-deck-retention.ts:3`): web keeps up to `WORKSPACE_DECK_MAX_MOUNTED_WORKSPACES=10`, inactive TTL `WORKSPACE_DECK_INACTIVE_TTL_MS=10 min`; native limit=1. Active always retained, inactive missing workspace prunes only after hydration. Stable render ordering avoids remounting all panes when active selection changes.

### Workspace, tab and pane shortcuts

- Default shortcuts source `APP/keyboard/keyboard-shortcuts.ts:257`: Add/Open Project **Cmd/Ctrl+O**, New Workspace **Cmd/Ctrl+N**, Archive **Cmd/Ctrl+Shift+Backspace**, Pin **Cmd/Ctrl+Shift+P**. Historical binding id/help name “new-agent” still dispatches project opening; current combo is O, not Shift+O.
- New tab **Cmd/Ctrl+T**; new agent **Cmd/Ctrl+Shift+A**, terminal **Cmd/Ctrl+Shift+T**, browser **Cmd/Ctrl+Shift+B**, Changes **Cmd/Ctrl+Shift+G**, Files **Cmd/Ctrl+Shift+E** (`:391–492`, `:842`). Cmd/Ctrl+P normally searches files; new-workspace route intercepts project picker.
- Close tab: Electron **Cmd/Ctrl+W**, browser **Alt+Shift+W** (`:503`). Workspace jump 1–9: Electron Mac Cmd+digit, Electron non-Mac Ctrl+digit, browser Alt+digit. Tab jump: Electron Mac Cmd+Alt+digit, non-Mac Alt+digit, browser Alt+Shift+digit (`:538–605`).
- Previous/next workspace: Electron Cmd/Ctrl+[ / ], browser Alt+[ / ]; previous/next tab **Alt+Shift+[ / ]** (`:620–706`). Targets follow current sidebar visible ordering; shortcut subscriber maintains ordered workspace target map rather than enumerating raw registries.
- Mac defaults: split right **Cmd+\\**, split down **Cmd+Shift+\\**, pane focus **Cmd+Shift+arrows**, move tab **Cmd+Alt+Shift+arrows**, close pane **Cmd+Shift+W** (`:720–830`). Non-Mac bindings for these pane operations were **not found** in this binding list; command actions still exist.
- Dispatcher scopes actions to active workspace/sidebar and blocks many shortcuts while command center or terminal/editable fields own focus; per-user overrides alter displayed hints (`command-center/workspace-shortcuts.ts:12`). Keyboard action alone does not grant a disconnected target mutation; existing controllers enforce availability.
- Platforms: browser reserves native-tab shortcuts, Electron uses conventional chords, touch uses explicit controls; native warm retention is intentionally smaller. Connections: route restore, sidebar filtering, command center, focus/read tracking, tabs and host reconnect.

## 31. Open workspace in editor/file manager or another desktop window

### Open-target planning

- Entry: workspace header Open in… (`APP/workspace/open-in-editor/button.tsx`), sidebar Open folder and project Open in new window (`components/sidebar-workspace-list.tsx:550`). Copy path/branch is clipboard-only (`hooks/use-workspace-clipboard-actions.ts`).
- Planner `workspace/open-in-editor/planner.ts:59` only produces desktop targets when bridge available **and local execution**. Active file and selected directory are resolved relative to workspace; invalid/escaping path does not become a desktop action. File opening details: [p4](p4-files.md).

### Desktop bridge, forge targets and preferences

- Current implementation calls Electron editor bridge **`listTargets/openTarget`**, not daemon editor RPC: `workspace/desktop-open-targets.ts:62/:72`. Arguments `{editorId,workspacePath,filePath?,line?,column?}`; target enumeration includes workspace-specific targets such as Xcode only when workspacePath provided. Query staleTime=60000 ms, retry=false.
- `OpenInFileManagerMenuItem` (`workspace/open-in-file-manager/menu-item.tsx:26`) requires Electron, nonempty local path and available file-manager target; absence hides action. Open failure toasts. Caller resolves local project/host placement before supplying path.
- Forge web targets may still exist for remote/web/mobile workspaces: neutral forge presentation builds branch/tree/blob URL from checkout remote, branch and optional line range (`planner.ts:107`). No desktop local path is sent to a remote daemon to open the user's editor; see [p3](p3-forge-review.md).
- Project Open in new window passes resolved local project path to desktop bridge (`sidebar-workspace-list.tsx:550`). It opens another UI window; it does not create a new registry workspace or clone.
- Legacy `list_available_editors_request`/`open_in_editor_request` remain parse-compatible (`P:2646`) but are not the current app's editor workflow. No durable workspace mutation occurs; preferred editor persists per host/project at `@paseo:preferred-editor:<JSON [serverId,projectId]>`, with legacy global fallback and first available target fallback (`APP/hooks/use-preferred-editor.ts:10`).
- Platforms: desktop actions Electron/local only; browser/mobile retain eligible forge links/clipboard. Connections: desktop shell bridge, filesystem location safety, project placement, active tab and Git remote metadata.

## 32. Workspace title and first-agent branch auto-naming

### Provisional and generated title

- User effect: newly created workspace first shows explicit title or prompt-derived provisional title, then metadata-generated human title; a branch-off worktree may rename its placeholder branch. This is separate from manual workspace Rename and Git Rename branch.
- Entry comes from first launch context (`APP/screens/new-workspace-screen.tsx:153`, `:304`), including terminal prompt naming without initial agent (`screens/new-workspace-terminal.ts:57`). Fork `chat_history` attachments are excluded from workspace naming seed while still available to the new agent (`screens/new-workspace-fork-context.ts:11`). Wire `firstAgentContext {prompt?,attachments}` accompanies workspace creation; initial agent prompt/attachments take precedence in combined creation (`SRV/session.ts:4261`).
- `SRV/workspace-auto-name.ts:68` schedules worktree naming; `:88` directory title naming. Uses `worktree-branch-name-generator.ts`/metadata generation with current provider/model selection; generated `{title,branch}` updates registry, Git mutation notifies checkout observers after branch change (`:142`). Metadata-generation sessions are excluded from project suggestions.

### One-shot placeholder branch naming

- Eligibility marker is written only for newly created **branch-off** tree (`paseo-worktree-service.ts:278`). `attemptFirstAgentBranchAutoName` (`:180`) requires nonempty naming seed + version-2 metadata pending; it marks attempted **before** generation and rechecks current branch still equals placeholder before rename.
- Invalid generation, absent metadata, manually switched/renamed branch or empty seed do not force a rename. Collision lookup tries desired then suffixes 2 through `MAX_BRANCH_NAME_SUFFIX_ATTEMPTS=50` (`:254`). Worktree path remains its slug; branch and path are not the same identity.
- Applying generated title only overwrites absent title or the original provisional prompt title (`workspace-auto-name.ts:181`); a user's intervening rename wins. Directory workspace changes title only; no fictitious branch is created.
- Failures are background logged, not workspace-creation rollback. Cross-repo untrusted automation skips automatic naming with setup workflow; explicit title stays usable. Base comparison ref remains the captured creation base.
- Platforms: daemon background work, all clients see registry/checkout updates. Connections: metadata provider/model, first prompt/attachments, project metadata instructions, Git rename ([p2](p2-git.md)), project-suggestion filtering and sidebar title.

## 33. Host projects list and Open Project home screen

### Host project settings directory

- Entry: host Settings → Projects, route `APP/app/settings/hosts/[serverId]/projects/index.tsx:7` → `screens/projects-screen.tsx:24`. Each row shows host-specific project name/icon and opens exact host/project settings (`projects-screen.tsx:116`), retaining projects with zero workspaces.
- `APP/hooks/use-projects.ts:164` acquires directory demand per known host, derives project summaries from host runtime/session replica (`:94`), and releases demand on unmount. It does not maintain a second authoritative project database or issue a bespoke list on each render.
- End-to-end listing is feature 20's `C.listProjects`/workspace observation → daemon project registry → project.update/workspace_update → replica → `useProjects`; selection itself is local routing, not a project mutation.
- Loading with no rows shows spinner, loaded empty shows empty-project message; populated list shows host-scoped load-error banner plus existing rows (`projects-screen.tsx:52`). **When zero rows, early empty return precedes error banner**, so do not claim the banner always appears on failure.
- Project detail rejects absent target/client/offline placement with NoEditableTarget; an optional Back to Projects action uses settings navigation (`project-settings-screen.tsx:124`). Read/query/edit states are feature 08; no implicit fallback edits another host.

### Home screen navigation

- Open Project home (`APP/screens/open-project-screen.tsx:26`) shows Add project, Import session, Set up providers and Pair device only when local daemon id exists; host last errors render with host label (`:126`). Providers invokes host chooser; import opens its sheet; pairing opens local-device modal.
- Wide home opens desktop agent-list panel on entry; compact uses common menu header/layout (`open-project-screen.tsx:39`). Community links are navigation only. Home does not register cwd just because it renders.
- Platforms: shared host list/home; local-device pairing tile depends on local daemon presence, list rows use shared touch/web press behavior. Connections: project settings, onboarding/provider setup, session import ([A-lifecycle.md](A-lifecycle.md)), host pairing/connection errors and sidebar replica.

## Cross-area connections

- **Agents/sessions**: workspace id is ownership for agent create/import/archive, terminal ownership and activity aggregation; combined create carries provider settings and first message. Closing root-agent tabs archives; child-agent tabs can only change client-open labels. [A-lifecycle.md](A-lifecycle.md), [B-providers.md](B-providers.md).
- **Timeline/composer/attachments**: draft-to-created-workspace handoff, fork history attachment, file/image review context, provisional title seed, pending initial create overlay and attention/read state. [C-timeline.md](C-timeline.md), [D-messaging.md](D-messaging.md), [p4-files.md](p4-files.md).
- **Git**: checkout source/base ref, worktree branch/cwd/backing ownership, exact subproject mapping, risky archive confirmation, dirty/ahead snapshots, metadata auto-name, Changes/Diff panel destinations. Git internals and operations are only in [p2-git.md](p2-git.md).
- **Forge/PR**: repository discovery, branch versus change-request worktree input, cross-repo trust block, PR URL open→merged latch, PR tab auto-open, forge links, merged URL consumed marker across restore. Provider-specific search/check/review/merge behavior is only in [p3-forge-review.md](p3-forge-review.md).
- **Files/editor/transfer**: directory suggestions and selection, imported icon bytes, workspace file targets/line ranges, preview dirty guards, Open with…, desktop editor bridge and archived backing removal. Editing/transfer contracts are only in [p4-files.md](p4-files.md).
- **Hosts/daemon/settings**: all entity references pair host id with project/workspace id; online/capability gates, background discovery, owned subscriptions and generation/seq resume, daemon autoArchiveAfterMerge, terminal profiles/service-port policy. Device-local preferences include last selection, launch target, layout, collapse/order, keyboard overrides and service route choice.
- **Terminals/services**: initial terminal launch, worktree auto-terminals/setup env, named script supervision, ports/URLs/health, activity contribution, stop/restart and archive kill escalation. Same cwd is not same owner; workspace id scopes each resource.
- **Permissions/questions/attention**: needs_input status derives agent permissions/questions; Mark read does not clear permission attention. Cross-repo untrustedSource blocks automatic setup/scripts; explicit setup release changes that persisted gate. No workspace label or pin operation resolves an agent question.
- **Plugins/metadata providers**: workspace.create.before and worktree create hooks, metadata-generated title/branch, plugin workspace/agent panels and allowed pane locations. Hook failure can roll back a newly created worktree; metadata failure does not roll back workspace creation.
- **Navigation/command center/desktop**: active workspace identity, route recovery, archive redirect, numbered sidebar targets, tab focus/side-sheet placement, external editor/window bridge. Local tab arrangement is not broadcast as daemon workspace state.

## Inventory

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
