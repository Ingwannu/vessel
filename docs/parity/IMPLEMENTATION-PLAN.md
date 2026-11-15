# Vessel: Paseo 100% implementation plan (coordinator, 2026-11-15 05:5x UTC)

Operator's goal: Paseo parity today (by 24:00 KST = 15:00 UTC), built skeleton-first and then filled
in, with the strictest Rust and code quality. Parity is the floor; extras Paseo lacks (PDF and SVG
viewers, language-server features) are welcome after parity in their area.

Documents this plan stands on (read the ones your package names):

| What | Where |
|---|---|
| Every Paseo feature, how it works end to end (app → client → protocol → server → storage → events), limits, platforms, connections | `~/refs/notes/paseo-features/` survey files (table in §6) |
| Feature index against Vessel (ID, survey ref, Paseo messages, Vessel node / app / page) | `CATALOGUE.md` (being generated) |
| Shared entities and cross-area links | `CONNECTIONS.md` (being generated) |
| Vessel today: two lines, crates, app areas | `vessel-today.md` |
| Rules every worker follows | `~/refs/notes/briefs/m12-grok-common.md` A1-A14, `/tmp/work/m12-parity-plan/WORKER.md` |
| Paseo source | `~/refs/paseo-latest` (0.15.4) |

## 1. Step 0 (running): one app base with the current node

The native app line sat 246 commits behind the node line (protocol 15 vs 23). Worker **B0** merges the
node line (head d954fa3a) and the reviewed workspace stack (to c408b653) into `backend/m12-app-base`
and adapts `spikes/native` (the lead reviews the head). Every app package merges that head (A8) when
B0 announces it, and from then on builds against protocol 23. Until then: build what does not need the
missing calls, with the node boundary as a typed trait tested against an in-memory fake.

## 2. How Vessel is built (the structure every package follows)

Node side (`crates/`): one crate per responsibility (vessel-proto wire contract, vessel-rpc frames,
vessel-client typed client, vessel-node daemon, vessel-session actor per session, vessel-store SQLite,
vessel-workspace, vessel-forge, vessel-harness, vessel-agent, vessel-mcp, vessel-lsp, vessel-cli,
vessel-web …). New calls go into the **one method table** (`vessel-proto/src/rpc.rs`, `method_table!`
after Quality B, with a pending row until the integrator assigns the protocol number). New storage
needs the integrator's schema number.

App side (`spikes/native`), four layers, each package adds its own module in each layer:

| Layer | Crate | Holds | Must not |
|---|---|---|---|
| view | `view/src/<feature>/` | pure state, state machines, sorting, formatting, words (FTL keys), tests | touch GPUI or the network |
| link | `link/src/<feature>.rs` (+ folder) | node calls for the feature, typed results, deadlines, reconnect handling (`connection_ready` fan-out in `link/nodes.rs`) | put feature logic in `link/src/lib.rs` (registration lines only) |
| app | `app/src/areas/<feature>/` | the GPUI screen: entity, draw, rows, actions | put logic in `app/src/shell.rs` (registration lines only; event dispatch lives in `shell/events.rs`) |
| kit | `kit/src/` | shared controls and tokens | gain a control without the coordinator's word |

Strict Rust, enforced by the gate (fmt, clippy -D warnings with the workspace lint set, size budget 500
lines per file, boundaries, tokens-only colours and sizes) and by review:
- Types carry meaning: newtypes for ids and units, enums for states (no booleans pairs, no stringly
  values across a boundary), `#[non_exhaustive]` only where the wire requires it, exhaustive matches.
- No `unwrap`, `expect`, `panic!`, `todo!` or `unimplemented!` in product code; errors are typed
  (`thiserror`) and carry their cause; no silent defaults: a refusal is a typed refusal shown to the user.
- Every limit (bytes, counts, timeouts, retries) is one named constant, cited from Paseo where Paseo has
  one.
- Async: every wait has a finite deadline; actors own their state; no shared mutable state across tasks.
- Tests that fail without the change; pure logic tested in view; link tested against a fake node;
  screens proven by captures (EN/KO, dark/light, 1280 and 390, 800x600 when layout matters).
- Every difference from Paseo is written in the package plan with its reason.

## 3. Skeleton first, then fill in (the method for every package)

1. **Skeleton commit (first 30-60 minutes)**: for every feature of the package (its catalogue rows):
   the view types and state enum, the link command / event variants with typed results, the area module
   with its entity and an honest placeholder draw ("not built yet" is not allowed on screen: the entry
   point stays hidden until the feature works), registration lines, and failing or ignored tests named
   after each feature. It must compile and pass clippy. This fixes the shape early so packages do not
   collide.
2. **Fill in**, feature by feature, in the catalogue order; each feature visible as soon as it works.
3. **Finish**: captures, one whole native gate (A13; plus root gate A12 if crates/ or web/ changed),
   one review submission, then the next package.

## 4. Packages and owners

Existing tracks keep their scope (left table); new package workers take the rest (right table).
Survey refs are file#section in `~/refs/notes/paseo-features/`.

### 4a. Existing tracks, scope confirmed against the survey

| Owner | Scope | Survey refs |
|---|---|---|
| G1 | git panel, changes / diff, commit, branches, stashes; PRs: status, create, merge, auto-merge, checks, PR tab, forge search, PR to worktree, review comments to the agent | parts/p2-git.md, parts/p3-forge-review.md |
| G2 | schedules, heartbeats (agent-target schedules), saved prompts (beyond Paseo) | .area6-parts/A.md#Schedules…, #Heartbeats |
| G3 | agents / providers / models pages, provider diagnostics, ACP registry add, install and sign-in, profiles UI (after ADR 0076 calls) | parts/B-providers.md, gaps.md#Host agent profiles… |
| G4 | devices, pairing (link, QR, code), sync across machines, shared Alert | area-5#Pairing…, #App host registry, #Connected hosts |
| K | themes, fonts, text sizes, content width, syntax palette | gaps.md#Built-in appearance themes…, #Fonts… |
| L | translations as data files, language selection | gaps.md#Localization… |
| S2 | sidebar (scroll repair first), grouping/sorting/filter/pin, callouts, keyboard shortcut engine, command center | .area6-parts/C.md |
| E2 | Settings shell and all Settings / host pages not owned elsewhere | .area6-parts/D.md |
| C2 | terminal in the app (T1 repair, T2 shell terminals beside a session, find, scrollback), agent's own terminal | area-4#Terminal… |
| N1 | inbox, push, diagnostics (node) | .area6-parts/B.md#Notification routing, #Mobile push |
| N2 | the web page: usage, parity, toasts/dialog fixes | .area6-parts/B.md#Usage… (page) |
| N3 | wire gaps, named launch profiles (ADR 0076, schema 19) | gaps.md#Host agent profiles… |
| N5 | side question, several models at once, delivered files, restart recovery (node) | area-2, parts/A-lifecycle.md |
| lifecycle / titles / dictation / workspace / nodectl | node sides of labels-logs-reload, generated titles, dictation, scripts-services-ports, node as a service (+ node update from the app: ADR) | parts/A-lifecycle.md, area-2#Dictation, area-4#Workspace scripts…, area-5#Daemon… |
| W1 | message search across sessions | parts/C-timeline.md#Chat find, area-7 search messages |
| W2 | import from the agent's history (session.create resume, ADR 0053), move a session to another machine | parts/A-lifecycle.md#Import…, #Export / handoff |
| W3 | handoff to another agent (new agent + first message, Paseo's handoff) | parts/A-lifecycle.md#Export / handoff, parts/D-messaging.md |
| W4 | side question panel; several models; deliverables | area-2, N5's node slices |
| W5 | usage page in the app, per-agent usage meter, limit notice | .area6-parts/B.md#Account usage…#Per-agent usage |
| W6 | attachments (P's patch), `@` files, file/context into chat, dictation controls later | area-2#Attachments, #File and context input…, #Dictation |
| W7 | inbox in the app, attention / unread, OS notifications, sounds, notification settings, notification open routing | .area6-parts/B.md#Notification…#Notification sounds |
| W8 | scripts and services, ports, preview opening | area-4#Workspace scripts and services, parts/p1#Workspace scripts/services… |
| W9 | checkpoints and file restore (ADR 0077 first) | parts/A-lifecycle.md#Rewind |

### 4b. New package workers (started with this plan)

| WP | Scope | Survey refs | Kind |
|---|---|---|---|
| W10 timeline | every timeline item kind and its renderer, tool-call display model, plan / todo / reasoning / diff in stream, capability gates, permission cards incl. plan/mode approval, question forms, nonblocking Codex questions, queued messages, steer / interrupt, message receipts | parts/C-timeline.md, parts/D-messaging.md, gaps.md#Permission cards…#Nonblocking Codex questions… | app (+node gaps reported) |
| W11 projects | add project (register dir, new dir, clone), suggested projects, rename / icon, remove, project settings and `paseo.json`-equivalent (`vessel.toml`), new workspace screen (project, host, isolation), worktree modes, setup logs and trust release, archive / bulk archive / unarchive workspaces, auto-archive after merge, host projects home | parts/p1-projects-workspaces.md sections 1-17, 32-33 | app + node gaps |
| W12 layout | workspace tabs (catalog, identity, retention, menus, rename/reload/copy), split panes, side pane, explorer, compact sheets, workspace navigation and keyboard, open in editor / file manager | parts/p1#Workspace tab catalog…#Open workspace in editor…, area-4#Panels, tabs and split containers, #Mobile / compact panels | app |
| W13 files | file explorer tree, create / rename / duplicate / delete, download, upload, live file subscription, file tabs and panel resolution, source editor (view, edit, save, find), directory suggestions; beyond Paseo after parity: PDF viewer, SVG viewer, LSP hover / definition / diagnostics through vessel-lsp | parts/p4-files.md | app + node gaps |
| W14 browser | in-app browser panel and agents driving it (Paseo's 22 browser tools) | area-4#Preview and the in-app browser…, .area6-parts/A.md#MCP tool catalogue | ADR first (engine choice), then node + app |
| W15 desktop | windows, menus, deep links, tray, managed local node, app update UI parity, changelog "What's new", onboarding state machine and steps | area-5#Desktop app…#Onboarding, gaps.md#Onboarding… | app |
| W16 plugins | plugin model and trust, install sources, lifecycle, subprocess runtime, server and client contribution points, catalog, Settings › Plugins, CLI | .area6-parts/E.md, gaps.md#Plugin-contributed themes | ADR first, then node + app |
| W17 cli | `vessel` CLI parity with Paseo's 117 command paths (names, flags, output formats, exit codes) | area-7-cli-protocol.md (CLI part) | node |
| W18 mcp | MCP tools parity (41 core tools: create, prompt, wait via notify, read output, permissions, schedules …), tool visibility policy, orchestration skills install / selection | .area6-parts/A.md#MCP…#Orchestration skills | node |
| W19 voice | spoken conversation (realtime voice overlay, two-way audio) | area-2#Realtime voice… | ADR first, then node + app |
| W21 sessions | session lifecycle in the app: archive / unarchive / close / delete, resume / reload, detach a subagent, subagent tree under its parent, labels UI, generated titles shown, deep links, agent list and history screen | parts/A-lifecycle.md, gaps.md#Subagent track… | app |
| W22 providers | provider coverage on the node: Claude, Codex, OpenCode, ACP, Pi / OMP; mode tables, launch config, provider snapshot, create-config resolution, unattended mode | parts/B-providers.md | node |

Not planned without the operator's word: Paseo Hub (gaps.md#Hub…: Paseo's hosted cloud service:
login, grants, hub-owned agents, triggers as code); the phone app (a separate product).

## 5. Order, review, merge

- Reviewers: V1 (strict app tracks G1-G4, K, L, S2, E2), V3 (node tracks and node work of any package),
  V4 (W1-W22 app work). One review per package (A14); security rows per commit.
- Merges: app packages → the native lead after review, in review order; node work → the integrator.
  Registration-line conflicts in `shell.rs`, `shell/events.rs`, `link/lib.rs` are the lead's to resolve.
- Builds: the lead cuts a build whenever five or more packages have landed (R builds, the coordinator
  smoke-tests on a real node, R publishes).
- Disk: narrow checks with `CARGO_PROFILE_DEV_DEBUG=line-tables-only` to keep target folders small.

## 6. Survey files

| Area | Files |
|---|---|
| A agents / sessions / timeline / messaging / providers / approvals | parts/A-lifecycle.md, parts/B-providers.md, parts/C-timeline.md, parts/D-messaging.md, gaps.md (first 12 sections) |
| B composer and input | area-2-composer-input.md |
| C-F projects, workspaces, git, forge, files | parts/p1-projects-workspaces.md, parts/p2-git.md, parts/p3-forge-review.md, parts/p4-files.md |
| G terminals, panels, scripts, browser | area-4-terminals-scripts-browser.md |
| H hosts, devices, desktop | area-5-hosts-devices-desktop.md |
| I-M automation, notifications, usage, sidebar, settings, plugins | .area6-parts/A.md … E.md, gaps.md (themes, fonts, i18n, onboarding, storage) |
| CLI and protocol | area-7-cli-protocol.md (117 CLI paths, 457 session messages) |
