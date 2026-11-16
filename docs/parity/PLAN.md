# Vessel: Paseo 0.15.4 parity, the implementation plan and the assignment (2026-11-15)

Written by the coordinator at the operator's order: document every Paseo feature (done: `FEATURES.md`,
260 features / about 700 sub-features, details in the survey files named there), find how they connect
(`CONNECTIONS.md`), then write how to build them (this file), how they are assigned (section 5), and run
it: **skeleton first, then fill in and fix**. Goal (operator, 2026-11-15): Paseo 100 % today (by midnight
Korea time). Beyond Paseo is welcome after parity (PDF / SVG viewers, LSP, saved prompts, moving a
session; marked "beyond").

## 1. Where Vessel stands

See `vessel-today.md`. In short: the node (22 crates) already has most of Paseo's server side; the gap is
mostly the native app's screens and flows. The app line sits on old root crates; **Step 0** (worker B0,
running) merges the node line into the app line (`backend/m12-app-base`). Every package rebases on B0's
head when it is announced.

## 2. Architecture every package follows (strictest Rust, one shape everywhere)

The native app (`spikes/native`) has four crates. A feature is one vertical slice through all four:

| Layer | Crate / path | What goes there | Rules |
|---|---|---|---|
| Words | `view/src/text/` FTL data (L's format) | every user-visible string, EN + KO | no string literal in a screen |
| State and rules | `view/src/<feature>/` | the feature's types, a state machine as an enum, pure functions, sorting, formatting, validation | no GPUI, no I/O; 100 % unit-testable; every state the Paseo survey lists is a variant |
| Node boundary | `link/src/<feature>.rs` (+ `<feature>/tests.rs`) | the feature's `Command` sub-enum and `Event` sub-enum, the calls through `vessel-client`, typed results, one named deadline per call, correlation tickets | registration lines only in `link/src/lib.rs`; the shared `connection_ready` fan-out in `link/src/nodes.rs` for reconnect notices (G2's) |
| Screen | `app/src/areas/<feature>/` (`mod.rs` entity, `draw.rs`, `rows.rs` …) | GPUI drawing from the view state, actions dispatched as Commands | registration lines only in `app/src/shell.rs` and its event child module; existing kit controls only |
| Controls | `kit/src/` | only if no existing control fits; ask the coordinator first | one renderer per control |

Type rules (enforced by clippy settings and review, no exceptions): no `unwrap` / `expect` / `panic!` /
`todo!` / `unimplemented!` in product code; no `as` casts that can truncate (use `try_from`); no `String`
or `u64` crossing a module boundary where a newtype or enum belongs (ids, paths, ports, durations, counts
are newtypes with validating constructors); errors are enums with `thiserror`, never `Box<dyn Error>` in
product code; every numeric limit a named `const` used once; `#[must_use]` on builders and results;
exhaustive matches without `_` arms over the feature's own enums; files under 500 lines; no `pub` wider
than needed (`pub(crate)` / `pub(super)` by default).

Tests: every behaviour has a test that fails without it; view state machines get table tests over every
transition; link calls get tests against the in-memory fake node (`vessel-testkit`); screens get
captures at the end of the package.

Node side, when a package needs a call that does not exist: the node work goes to the node track named in
the table; new calls register in the one method table (Quality B, at the integrator) with a pending row
until the integrator assigns the number. Never a dummy version, never a second table.

## 3. Skeleton first, then fill in (how a package runs)

1. **Skeleton (first hour of a package):** the view enum of every state the survey lists, the link
   Command / Event enums with typed results, the screen drawing every state from fixture data, the
   registration lines, words keys. Compiles, narrow checks green, committed. This is the "frame".
2. **Fill in:** wire the real calls, behaviour per the survey, tests per behaviour, one commit per
   sub-feature. Ship a working first version of the whole package before polishing any part.
3. **Close:** captures (sizes 1280x900 / 390x844, 800x600 where layout matters; dark and light; EN and
   KO), one whole native gate (A13; root gate too if crates/ changed), one review submission, the lead's
   merge. Then the next package.

Rules from the brief stay: A8 merges, A10 staged node code, A11-A14 build and review rules, own worktree
and cargo target, never the live preview, no secrets, `date -u` before times.

## 4. Spec for each package = the survey sections

A worker's specification is the survey file sections its package names (for example "area-4 §21-26").
Those sections give, per feature: what the user sees, the end-to-end flow (app → client → protocol →
server → storage → events), states, errors, limits and defaults, platform differences, connections.
Port behaviour and limits as they are; state each deliberate difference in the package's plan file
(`docs/plans/wp-<id>.md` in the worker's tree) with its reason. Connections to other packages are in
`CONNECTIONS.md`: when a package touches an entity another package owns, it uses that package's link /
view API, never a copy.

## 5. Packages and owners

Catalogue references: `1/A` = parts/A-lifecycle.md, `1/B` = B-providers, `1/C` = C-timeline, `1/D` =
D-messaging, `2` = area-2, `3/p1..p4` = parts/p1..p4, `4` = area-4, `5` = area-5, `6/A..E` =
.area6-parts/A..E, `G` = gaps.md. Numbers are the inventory numbers in `FEATURES.md`.

| Pkg | Scope (catalogue) | Owner | Starts |
|---|---|---|---|
| **B0** | Step 0: node line into the app line, protocol 23 in the app | B0 | running |
| **A1** | New session / new workspace screen and every create flow: 1/A 14-18, 3/p1 10-14, 1/D composer create handoff | W3 | now |
| **A2** | Archive / unarchive / delete / close, archived callout, Sessions screen, workspace archive and restore of worktrees: 1/A 21-22, 24, 45; 3/p1 15-17 | W1 | after its search package |
| **A3** | Message and timeline search, chat find, outline, prompt index: 1/C 17-18, 48-49 | W1 | now |
| **A4** | Import from the agent's own history, subagents (track, child navigation, detach), fork / rewind parity check; beyond: move a session: 1/A 29-34, 37; G "Subagent track"; 1/C 20 | W2 | now |
| **A5** | Attention, unread, notification routing, in-app OS notifications, sounds, push, badges, toasts, inbox: 1/A 40-41; 6/B notifications part; 4 §7 | W7 | now |
| **A6** | Labels for sessions and workspaces, label manager: 1/A 39; 3/p1 18-19 | W10 (new) | now |
| **A7** | Timeline parity: every item kind and detail body, plan / todo / compaction / turn footer / usage meter, tool grouping, turn collapse, older history, prompt jump: 1/C 1-3, 21-26, 37-64 | C2 | after T1 |
| **A8** | Permissions and questions parity, plan approval and follow-up, mode policy, CLI permit: G first five sections | W11 (new) | now |
| **A9** | Providers and models in the app: provider list, diagnostics, catalogue, ACP install, modes / effort tables per provider: 1/B; 5 §55 | G3 | now (with its Models screen) |
| **A10** | Agent profiles (host profiles CRUD, apply to drafts and live agents, favourites migration): G "Host agent profiles" .. "Legacy favorites" | N3 (node, ADR 0076) then G3 | ADR now |
| **B1** | Composer: attachments of every kind, paste / drop / upload, pills, tray, lightbox, workspace and forge attachments, `@` mentions: 2 §10-11, 16-24 | W6 | now |
| **B2** | Composer shell and commands: modes, fullscreen editor, history recall, client / plugin / provider slash commands and skills, shortcuts, send settings, assistant file links / images / selection copy: 2 §1, 3-9, 25-27, 29-30; 1/D | W12 (new) | now |
| **B3** | Dictation (mic, overlay, streaming, providers, model library): 2 §12-13 | dictation (node) then W12 | after its wiring |
| **B4** | Realtime voice and audio: 2 §14-15 | W13 (new) | ADR now |
| **C1** | Add project (register, create dir, clone, GitHub search, suggestions, icons), project settings page, open in editor / file manager: 3/p1 01-09, 31, 33 | W14 (new) | now |
| **C2** | Workspace tabs, panes, split layout, drag and drop, keyboard pane commands, compact sheets: 3/p1 26-30; 4 §21-26 | W15 (new) | now |
| **C3** | Git panel parity: diff surfaces, commits, branch switcher with stash, discard, git actions policy, commit message generation: 3/p2 | G1 | now |
| **C4** | Forge and PR pane parity: checks, pipelines, timeline, add-to-chat, PR picker, create from PR, review drafts and inline comments, review feedback: 3/p3 | G1 then W16 for review drafts | after C3 |
| **C5** | Files: explorer, mutations, download / upload, live file, file tabs, editor with save / conflict, find in file, markdown / HTML / image preview; beyond: PDF and SVG viewers: 3/p4 | W16 (new) | now |
| **C6** | Beyond: language-server features in the editor (vessel-lsp exists on the node) | W17 (new) | after C5's editor |
| **D1** | Terminals: everything in 4 §1-20 | C2 | T1 repair now, then T2 |
| **D2** | Scripts and services, ports, service proxy and routes, health, scripts menu, worktree setup progress: 4 §27-35; 3/p1 14, 25 | W8 | now |
| **D3** | In-app browser pane, browser element attachments, agent browser tools: 4 §36-40 | W18 (new) | ADR now |
| **E1** | Hosts and connections: host registry, runtime / best connection, add host (direct, SSH), host settings and appearance, connections page, connected hosts: 5 §11-36 | E2 (with G4 for pairing) | now |
| **E2** | Pairing and devices: offers, QR, paste link, host confirmation, device list / revoke: 5 §15-20 | G4 | now |
| **E3** | Desktop shell parity: windows, menus, deep links, CLI install, login-shell env, single instance, quit lifecycle, desktop settings, app updates, changelog: 5 §37-51 | W19 (new) | now |
| **E4** | Onboarding, diagnostics (app, daemon, provider, process, logs): 5 §52-56; G onboarding sections | N1 (node) + W20 (new, app) | now |
| **F1** | Schedules and heartbeats: 6/A schedules part | G2 | running |
| **F2** | MCP tools and orchestration skills parity (41 + 22 tools, skills install / update): 6/A MCP and skills parts | W21 (new, node) | now |
| **F3** | Usage: account reports, cards, sidebar summary, modal, per-agent meter, CLI: 6/B usage part | W5 | now |
| **F4** | Sidebar, command center, keyboard shortcuts engine, help and rebinding: 6/C | S2 (sidebar) + W22 (new: command center, shortcuts) | now |
| **F5** | Settings pages parity (every page in 6/D) and appearance, fonts, text sizes, themes, localization: 6/D; G themes / fonts / localization | E2 (pages) + K (themes, fonts) + L (words) | now |
| **F6** | Plugins: model, install, runtime, RPC, every client contribution point: 6/E | W23 (new) | ADR now |
| **N** | Node work the packages above need: lifecycle (labels, reload), titles, workspace, nodectl, dictation, N1 inbox / push / diagnostics, N2 usage, N3 wire gaps and profiles, N5 side question / several models / deliverables / restart recovery | existing node tracks | running |
| **Q** | Out of parity unless the operator says otherwise: Paseo Hub (hosted login, triggers as code): G hub sections. Vessel's fleet covers host-to-host. | — | ask the operator |
| **Beyond** | saved prompts screen (node `prompt.*`), PDF / SVG viewers, LSP, move a session, side question (GOAT), several models at once | G2 / W16 / W17 / W2 / W4 | after parity of their area |

Checkpoints and restoring files (W9) need a node contract first: ADR 0077 (W9 writes it, V3 reviews).

## 6. Reviews, merges, tracking

- Reviewers: V1 (G1-G4, K, L, S2, E2, C2), V3 (node tracks, B0's root side), V4 (W1-W23).
- The native lead merges finished packages into the app line in the order they pass and builds a test
  label when 3-4 packages have landed; R builds, the coordinator smoke-tests on a real node, R publishes.
- Tracking: `TRACKER.md` beside this file lists every catalogue feature with its package, owner and state
  (missing / skeleton / built / passed / merged). A worker updates its rows when its package passes.
  "How far are we" = merged rows / all rows.

## 7. Owner corrections (coordinator, 2026-11-15 05:58 UTC): one writer per package

Two coordinator processes ran for a few minutes and assigned some packages twice. The duplicate process
is stopped. This table is final; it overrides section 5 where they differ. PLAN.md and TRACKER.md stay
the canonical plan; IMPLEMENTATION-PLAN.md keeps only its rulings (both sides per package, ADR numbers).

| Pkg | Single owner (agent id) | Note |
|---|---|---|
| A1 new session / new workspace / create flows | W3 1ea2064e | |
| A2 archive / unarchive / delete / close, Sessions screen, workspace archive and restore, deep links to a session | W21 0c674d40 | W1 does A3 only |
| A3 search, chat find, outline, prompt index | W1 7f54c040 | |
| A4 import, subagents (track, child navigation, detach), fork / rewind check, move a session | W2 3fe89166 | |
| A5 attention, notifications, inbox, sounds, push in the app | W7 e4bfffc1 | owns its node gaps (recipient claim, terminal attention) |
| A6 labels for sessions and workspaces, label manager | W10b 62128a09 | lifecycle d6d78e1f owns session-label node calls |
| A7 timeline parity (all item kinds, details, plan / todo / compaction, turn footer) | W10 9dc6bb3a | node side: W23 17459095 (ADR 0084) |
| A8 permissions and questions, plan approval, mode policy, CLI permit | W4 a95c5748 | W10 does not touch permission / question files |
| A9 providers and models in the app | G3 d645571c | node side: W22 53d744fd (ADR 0081) |
| A10 agent profiles | N3 813788de (node, ADR 0076) then G3 | |
| B1 attachments, `@` mentions | W6 1ee9cf54 | |
| B2 composer shell, commands, send settings | W12b bf9fa3e4 | owns the command-kind node gap (ADR 0085) |
| B3 dictation | dictation e0f46912 (node) then W12b | |
| B4 realtime voice | W19 2149d9bc (ADR 0080) | |
| C1 add project, project settings, open in editor / file manager, plus p1 registry / status / rename / pin rows | W11 cea3a6e5 (ADR 0082) | 11:50 ruling: actual empty-workspace FS provisioning + receipt/launch adapter moves to A1/W3, using reviewed W11 Store. W11 retains canonical registry/start leases, project/config admission, native projection next, nested reroot, PR mode, unregister and legacy destructive-route fencing; file boundary in project-management package notes. Dependency status13:1x: identity and registry/store row213 scoped PASS258 (two named base-test limits retained), integrated pending gate47; doc-only0140 follows. Native projection next, source skeleton only until proto reaches app line, then checks/V1. Full C1 remains incomplete. |
| C2 workspace tabs, panes, split layout, compact sheets | W12 40936242 | pane / tab contract for W13, C2, W14 |
| C3 git, C4 forge and PRs | G1 7f706a0e | |
| C5 files, editor, previews, PDF / SVG; C6 LSP | W13 f88a3eca (ADR 0083) | |
| D1 terminals | C2 263bf0b4 | |
| D2 scripts, services, ports, proxy, setup progress | W8 9d8b74a1 | |
| D3 in-app browser, browser tools | W14 95b0af81 (ADR 0078) | |
| E1 hosts and connections (registry, add host direct / SSH, connections page, connected hosts) | app side (client book multi-route/upsert API, host overview, connections, add direct / SSH screens): E2 a864bd30; node host registry + CLI: W17 2ac4e6fc; G4 consumes the book API for pairing (07:2x ruling) | |
| E2 pairing and devices | G4 d87efb9f | |
| E3a desktop windows, menus, OS deep links, CLI install, single instance, app updates, changelog | W19b dd0931ff (own tree wp-desktop-shell) | |
| E3b onboarding, local-node controller, quit lifecycle | W15 a49b707d (tree wp-desktop) | |
| E4 diagnostics in the app | W20 3df23e9e with N1 84843f39 (node) | E2 hands its Diagnostics page to W20 |
| F1 schedules, heartbeats, saved prompts | G2 340fd692 | |
| F2 MCP tools and orchestration skills | W18 09b1f793 (ADR 0065) | |
| F3 usage | W5 f37ab4a0 | may add one shared kit usage meter |
| F4 sidebar | S2 2ef83c79 | |
| F4 command center, keyboard shortcuts engine, help and rebinding | W22b e1b6ac0b | |
| F5 settings pages | E2 a864bd30; themes / fonts K 1fbfb416; localization L 4aafd116 | |
| F6 plugins | W16 6d6d77d6 (ADR 0079 PASS) | |
| CLI parity | W17 2ac4e6fc | |
| checkpoints | W9 9da4c992 (ADR 0077 PASS) | |
| side question, several models, deliverables | N5 4f384cde (node), app later W4 | |

Archived as duplicates: ff2d50a2, 6da9ce28, d067b604 (had started wp-panes: W12 reads it and keeps what fits), f9628205, d283b71b,
b5675d71, 973740ba.

ADR numbers in use for the parity packages: 0076 profiles (N3), 0077 checkpoints (W9), 0078 browser (W14),
0079 plugins (W16), 0080 voice (W19), 0081 providers (W22), 0082 project / workspace registry and
WorkspaceId (W11), 0083 files (W13), 0084 timeline protocol (W23), 0085 command kind (W12b), 0086 workspace
labels (W10b), 0087 git node gaps (G1), 0088 forge node gaps (G1), 0089 notification recipient claim and terminal attention (W7), 0090 subagent parentage and detach (W2), 0091 creation receipts and terminal profiles (W3), 0092 shortcut dispatcher and palette contributions (W22b). 0093 worktree setup observation and cancellation (W8), 0094 package contents (CLI + node beside the app) and whole-set self-update (R), 0095 terminal screen engine crate vessel-terminal-screen (C2), 0096 relay consent from the app: node relay settings read and Admin enable (G4), 0097 typed multi-route client book, upsert, pinned identity and partial-credential outcomes (E2), 0098 resume interrupted turns at node start (N5; renumbered from a 0059 reservation that collided with the app line). 0099 release build on the build VM with a placeholder feed address patched and signed on dev-host (R). 0100 HTML rendered ranges and optional highlight foreground in the shared text engine (B2/W12b, 18:4x). 0101 shared locale number formatting: full-finite amounts with units and exact unsigned counts (L, 18:5x). 0102 shared masked single-line field and secret entry policy (G2, coordinator ed1d1f71, 2026-11-16). 0103 ended-agent quiescence proof for restore: typed wait outcome, descendant settlement, complete output drain, forgotten agent debt (GKS lifecycle, coordinator, 2026-11-16). 0104 tracked DNS resolution lifetime: one shared primitive that keeps native resolution work owned until it actually completes, consumed by NodeLink relay dialing (ADR 0096) and HTTP clients through reqwest's dns_resolver hook (dictation endpoints) (G4, coordinator, 2026-11-16). Next free: 0105. Workers ask the coordinator before taking a number.

Baseline measured by CATALOGUE.md (06:0x UTC): 1,419 Paseo feature items; native app present 16, partial
792, none 433, unclear 178.
- Worker ids: /tmp/work/m12-parity-plan/AGENTS.txt (full ids; send_agent_prompt needs them).
- Shared seam owner (06:2x UTC): AgentEvent::required_protocol() -> Option<ProtocolVersion> (the A6 accessor shape for agent events, on top of lifecycle's SessionEvent accessor 9480754d after Quality B) is written by W2 (3fe89166) alone, as its own small commit with a test over every variant, reviewed by V3. W23 and any other package use it and do not write their own.

## 8. Rulings 2026-11-15 06:30-06:45 UTC (coordinator)

- Bases: app part = the lead's package base 8d5513e4 (backend/m12-package-base); node part = node head
  29eebb8b (Quality B, V1 source pass + gate 37). B0 merges the node head into the package base as
  backend/m12-app-base2; the lead fast-forwards the package base to it when green.
- Worktree setup observation (per-command results, durations, blocked source, cancellation): W8, both
  sides, ADR 0093. The workspace track be3ba172 keeps allocator, env, proxy, routes, health.
- A10 configuration seam in vessel-agent (per-process configuration owner behind AgentInput, serializing
  single options and bundles, completion adapters in driver arms) and its vessel-session extensions: N3.
  W22 keeps vendor parsers, offered choices, provider policy, permission defaults, login files, argv.
- Self-process facts (pid, parent, RSS, VmSize from /proc/self/status; other OS Unsupported): vessel-proc,
  written by N1, landing with its D4 caller.
- Shared node seams from W17: rpc.rs Request/UnexpectedReply -> rpc/contract.rs and node boot -> node/setup.rs
  as separate behaviour-preserving commits merged by the integrator ahead of W17's feature.
- kit::combobox gets a typed Content width (G2, for HostFilter); HostErrors radius corrected to R2 = 8 (G4).
- nodectl 2h: source to V3; the terminals delete Conflict race is a base defect the integrator owns.
- Voice reload (06:4x): W22 owns the AgentSetup transient system-prompt override field and the per-harness
  capability (typed refusal when unsupported, no fallback); lifecycle owns the reload orchestration; W19
  owns the voice prompt and restore state. Order W22 -> lifecycle -> W19.
- Package contents (06:5x): the app package carries vessel CLI and vesseld (Paseo's desktop runs a managed
  local daemon and ships its CLI); whole-set self-update; ADR 0094 (R); shipped after W15's local-node
  controller. rpc.rs size: the integrator moves method_table! into rpc/table.rs as the lasting seam;
  W17's contract extraction 1c8d0b93 meanwhile. kit TextField: W4 extracts field/events.rs (shared seam
  to the lead early) and adds typed FieldId with source-aware callbacks.
- Terminal screen engine (07:0x): one pure crate vessel-terminal-screen owned by C2 (ADR 0095), the only
  parser for node capture, D1 restore/snapshot and the native view; engine chosen by a measured corpus
  (W17's vt100 adapter vs C2's parser); W17 consumes it for CLI capture.
- Multi-session panes (07:1x): W12 owns the SessionId-keyed resident registry and selection/focus (contract first, V1); A7, W12b, W21, W6 adapt their parts behind it.
- Durable process identity (07:4x): nodectl owns the vessel-proc identity proof primitive (pid + start time + boot id, verify Same/Gone/Reused); W22 owns the provider helper ledger and restart reap on top; ProcessLifetime (workspace track) stays in-process only.
- vessel-proc Child order (09:2x): W16 wait_checked (checked group cleanup) first, then nodectl identity::spawn; ProcessLifetime already in.
- Workspace provisioning (11:5x): W3 owns the empty-workspace FS provisioning + receipt/launch adapter on W11's reviewed Store; W11 keeps identity/registry/leases/admission/projection.
