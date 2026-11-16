# Web page parity

Status: A1, paired page corrections and A2a accepted; feedback package narrow checks pass, whole/browser/captures pending, 2026-11-15. ADR 0075.

## Scope and evidence

The browser client remains `web/` behind the local bridge (ADR 0017),
retained under ADR 0036 while ADR 0035's native client develops. ADR 0020
was never accepted and is historical, replaced by ADR 0035. This track changes no native source and adds no protocol.
Original inventory row origins: `7f87d41c` (protocol 21). Node availability
columns are updated through merged `f453ec2e` (protocol 23, schema 17); native source snapshot:
`1fc0f66a`; Paseo 0.15.4: `1c00b365`. Native working drafts and unrelated
branches are excluded: presence in a branch is not an integrated screen.

This inventory is a source comparison, not a runtime or visual acceptance
claim. **Present** below means the named source/call exists; partial and
missing behavior is explicit. Native paths start at
`spikes/native/app/src/areas/` unless stated; web feature paths start at
`web/src/features/`, and `data/` paths start at `web/src/`. Reference keys
resolve to exact Paseo app files/line ranges in the final source index.
A reference marked “none” means no matching Paseo app surface was found in
the surveyed source, not proof of absence across every extension. Features
unique to another reference are retained rather than invented as Paseo UI.

Coverage: one row for each of the **93** union feature rows in milestone 11
(lines 93–230), then the **15** milestone-12 area rows, **nine** wire gaps
(the two repeated historical tables share one row with both origins), and
**five** UI defects. Track-assignment and technical type/limit tables are
implementation evidence, not additional product feature rows. Their product
features are represented by these rows. Historical built/not-built markers
in those plans do not override the current snapshots.

## Existing assets and decision

**Modify existing assets.** Reuse typed `Fleet`/`Outcome`, generated wire
types, Settings frame and machine picker, route registry, UI kit, styles
and design tokens, per-feature data modules and the test fleet. Pure
report decisions/formatting belong in `web/src/data/`; components and
hooks in `features/`; registration only in routes/settings. The alternative
of a new query/state/UI library adds a second lifecycle and look without
solving a missing capability. Standard `Intl` and existing React/hooks
are sufficient: no new dependency, external endpoint, credential reader,
new wire number or native modification.

## Milestone 11 feature matrix

| ID / source row / feature | Node has it: actual calls or boundary | Page has it: existing file | Native app has it: committed source only | Paseo web/app source | What the page lacks / next action |
| --- | --- | --- | --- | --- | --- |
| F01 / m11:93 / Start a session with an agent, model, effort, permissions and a prompt | session.create; harness.options | compose/new-session.tsx; agent-menu.tsx | draft/mod.rs; draft/pickers.rs | [P01](#reference-p01); [P03](#reference-p03) | Verify remembered option, effort and permission parity; retain existing flow. |
| F02 / m11:94 / A session as a conversation or as the agent's own terminal | session.create structured/terminal | session/structured-session.tsx; terminal-view.tsx | session.rs; terminal surface registration needs runtime verification | [P23](#reference-p23); [P42](#reference-p42) | Creation exists; a switch of an existing session needs separately supported wire. |
| F03 / m11:95 / Change model, effort or permissions on a running session | session.options; session.set_option | session/agent-controls.tsx | composer/running.rs | [P03](#reference-p03) | Audit effort/fast/permission choices against actual provider options; no inferred capabilities. |
| F04 / m11:96 / Queue messages while a turn runs; edit, reorder, remove, send now | session.queue; session.prompt | session/queue-dock.tsx; data/queue-view.ts | conversation/queue.rs | [P04](#reference-p04) | Existing queue; compare edit/reorder/send-now cases before marking parity. |
| F05 / m11:97 / Steer the running turn without stopping it | session.prompt delivery=steer | session/composer.tsx; data/delivery.ts | composer/running.rs | [P02](#reference-p02) | Existing delivery selector; provider refusal must remain visible. |
| F06 / m11:98 / Interrupt a turn | session.send cancel_turn | session/composer.tsx | conversation/actions.rs | [P02](#reference-p02) | Retain turn identity protection and IME-safe key handling. |
| F07 / m11:99 / Start an ended session again with its conversation | session.resume | session/ended-banner.tsx; use-mark-actions.tsx | conversation/ended.rs | [P01](#reference-p01) | Existing resume; compare failure and restored draft behavior. |
| F08 / m11:100 / Rename | session.rename | session/session-title.tsx | conversation/rename.rs | [P05](#reference-p05) | Existing inline rename; no new wire needed. |
| F09 / m11:101 / Pin, archive and bring back, delete, mark unread | session.mark; session.delete | session/use-mark-actions.tsx; board/archive.tsx | archive.rs; conversation/actions.rs | [P05](#reference-p05) | Existing marks/archive; deletion and unread races need regression coverage. |
| F10 / m11:102 / Questions the agent asks, answered in the session | session.send answer_question | session/question-dock.tsx; question-field.tsx | conversation/question.rs | [P07](#reference-p07) | Existing structured questions; validate all provider choice shapes. |
| F11 / m11:103 / Context meter; compact | usage_changed; session.prompt /compact where advertised | session/context-ring.tsx; data/context-meter.ts | conversation/meter.rs | [P08](#reference-p08) | Existing meter; never confuse context occupancy with billed usage. |
| F12 / m11:104 / Plans shown as the agent revises them | agent plan updates in session.read/follow | session/inspector-summary.tsx | conversation/plan.rs | [P09](#reference-p09) | Web plan is in Summary; compare inline card/progress behavior before moving it. |
| F13 / m11:105 / Edit an earlier message and continue from there (rewind) | session.rewind (conversation only) | No rewind control; session/timeline-item.tsx is insertion seam | conversation/rewind.rs | [P10](#reference-p10) | Add capability-bound rewind with actual server boundary, confirmation and draft restore; no file rollback promise. |
| F14 / m11:106 / Fork a conversation into a new session | session.fork | No fork control; session/session-actions.tsx seam | No registered fork surface found | [P11](#reference-p11) | Add a real fork action only for advertised agent capability; never substitute fresh empty chat. |
| F15 / m11:107 / Import conversations from the agent's own history | agent.history; session.create resume | No import surface; compose/new-session.tsx seam | No registered import surface found | [P12](#reference-p12) | Paged provider history, safe resume, per-provider errors and no duplicate live claim. |
| F16 / m11:108 / Find in a session | session.read/follow; client-side find | session/find-bar.tsx; use-search.ts | conversation/find.rs; outline.rs | [P13](#reference-p13) | Existing virtualized find/outline; preserve 20k-entry navigation proof. |
| F17 / m11:109 / Search every session's messages | session.search | No global message-search screen; palette/palette.tsx finds names | conversation/search.rs | [P14](#reference-p14) | Add bounded message search and cursor continuation; distinguish name search from transcript search. |
| F18 / m11:110 / Park an idle agent process and wake it on the next prompt | session.park; session.wake; node.settings | No explicit park/wake controls; ordinary prompt wakes | No explicit park/wake surface found | [P24](#reference-p24) | Draw parked state and working actions; stored history is not a live runtime. |
| F19 / m11:111 / Sessions started by a session, shown under it | SessionInfo.created_by; session.list | board/card-origin.tsx; data/tree.ts | board/card.rs; sidebar.rs | [P15](#reference-p15) | Existing creator attribution; complete nested child navigation after source comparison. |
| F20 / m11:112 / Change the agent mid-session with a handoff | Same-agent session.move_profile only; no cross-agent handoff on base | session/move-profile.tsx (account move only) | No cross-agent handoff surface found | [P15](#reference-p15) | Blocked for cross-agent handoff; do not label account move as agent switch. |
| F21 / m11:113 / Switch one session between conversation and terminal | No existing-session interface-switch call on base | terminal-view.tsx is creation-selected, not a switch | No registered interface-switch control found | [P43](#reference-p43) | Blocked until assigned wire is merged; separate terminal workspace differs from switching one agent. |
| F22 / m11:114 / A side question that stays out of the history | session.side_open; session.side_prompt; session.side_reset | No side-question surface | No registered side-question surface found | Not found in surveyed Paseo app; wider-reference feature retained. | Add isolated side panel with actual node side calls; no history injection. |
| F23 / m11:115 / One prompt to several models at once | CLI orchestration through project/worktree/session calls; no batch RPC | No comparison flow | No registered comparison flow found | Not found in surveyed Paseo app; wider-reference feature retained. | Possible client orchestration later; preserve partial effects and explicit setup approval. |
| F24 / m11:116 / Usage-limit notice: wait for the reset or fall back to another model | usage.query21; usage.limits/lookup23; no automatic recovery | No notice; session/composer-docks.tsx seam | No usage-limit dock in committed conversation/dock.rs | [P26](#reference-p26); [P27](#reference-p27); [P29](#reference-p29) | Assigned limits facts are available; factual reset notice remains A4. No stopped-turn diagnosis or inert wait/switch action. |
| F25 / m11:117 / Files the session delivers, HTML shown in the conversation | attachment.read for user uploads; no delivered-artifact contract on base | session/prompt-files.tsx handles uploads, not delivered HTML | No delivered-artifact surface found | [P22](#reference-p22) | Blocked for unassigned delivered facts; preserve raw HTML-off policy and sandbox requirements. |
| F26 / m11:123 / Attach images and files | attachment.begin/write/finish/read | compose/attachment-strip.tsx; session/attach-button.tsx | composer/offers.rs | [P02](#reference-p02); [P07](#reference-p07) | Existing uploads; compare paste/drop/type/refusal states, never upload silently on failure. |
| F27 / m11:124 / `@` a file of the project | file.find; file.read | compose/use-file-search.ts; data/suggest.ts | workspace/code/search.rs | [P22](#reference-p22) | Existing project file mention; compare outside-root and stale-file cases. |
| F28 / m11:125 / `/` the agent's commands and skills | commands_changed; session.prompt | compose/use-suggest.tsx; data/commands.ts | composer/slash.rs | [P02](#reference-p02) | Existing command picker; only advertised commands, not invented commands. |
| F29 / m11:126 / Drafts kept per session | Bridge preferences and local draft state, not a node RPC | compose/use-draft.ts; data/drafts.ts | composer/drafts.rs | [P01](#reference-p01) | Existing drafts; preserve per-session identity across machine/route changes. |
| F30 / m11:127 / Up and Down recall sent messages | Client-only input history | data/recall.ts; data/compose-keys.ts | composer/drafts.rs (exact recall parity not verified) | [P16](#reference-p16) | Existing web recall; native parity not established by file presence. |
| F31 / m11:128 / Enter or Mod+Enter sends, by choice | Bridge preferences, no new wire | settings/general.tsx; data/compose-keys.ts | settings/general.rs | [P17](#reference-p17) | Existing send-key choice; preserve composition and modifier rules. |
| F32 / m11:129 / A selection in a diff or a file, sent as a comment | file.read/git.diff plus attachment/prompt | workspace/selection-ask.tsx; data/workspace/reference.ts | workspace/code/actions.rs | [P19](#reference-p19) | Existing selection comment; exact line/root/unsaved-file behavior still to compare. |
| F33 / m11:130 / Dictation | No dictation calls on base | No dictation surface | No registered dictation surface found | [P34](#reference-p34) | Wait for assigned dictation wire; no microphone or new external endpoint in this track. |
| F34 / m11:131 / Spoken conversation | No realtime voice contract on base | No voice surface | No registered voice surface found | [P35](#reference-p35) | Deferred; wider than page parity and requires explicit provider/network scope. |
| F35 / m11:137 / Agents it knows | harness.list; agent.list; agent.status | machines/agents-section.tsx | settings/agents.rs | [P03](#reference-p03); [P24](#reference-p24) | Existing dynamic rows; audit provider names/defaults without assuming branch-only drivers merged. |
| F36 / m11:138 / Any agent that speaks ACP, by command or from the ACP registry | agent.list/add/remove/catalogue/install | Read-only added-agent listing in machines/agents-section.tsx | settings/agents.rs (catalog/install completeness not verified) | [P24](#reference-p24); [P33](#reference-p33) | Add catalog/add/install progress from existing calls; no package installation during UI discovery. |
| F37 / m11:139 / Is it installed, is it signed in; install and sign in from the page | agent.status; agent.install; agent.install_output | machines/agent-detail.tsx; agent-row.tsx, no install workflow | settings/agents.rs; full sign-in workflow unverified | [P24](#reference-p24) | Expose bounded readiness/install results; never ask browser for local credentials. |
| F38 / m11:140 / Several accounts of one agent | profile.list; session.move_profile | settings/models.tsx; session/move-profile.tsx | composer/offers.rs; settings/agents.rs | [P03](#reference-p03) | Existing profile display/move; distinguish same account aliases from separate accounts. |
| F39 / m11:141 / Defaults per agent | Client preference defaults plus profile configuration | data/defaults.ts; settings/models.tsx | draft/book.rs; settings/agents.rs partial | [P03](#reference-p03) | Per-browser remembered choices exist; native default/readiness parity needs explicit audit. |
| F40 / m11:142 / Every agent on every model | model.list; configured model route at launch | settings/models.tsx; data/models.ts | settings/sections.rs has no Models renderer | [P03](#reference-p03) | Existing route/profile listing, not arbitrary route editing; no fabricated prices/models. |
| F41 / m11:148 / Approve or refuse in the session, with the agent's own choices | session.send answer_permission | session/permission-dock.tsx | conversation/permission.rs | [P06](#reference-p06) | Existing choices; permission-card text/layout follow actual request, with scoped failure. |
| F42 / m11:149 / Everything waiting on you, from every machine, in one place | inbox.list/get/mark22; existing session.send validates answers | board/lane.tsx plus individual session docks | board/mod.rs; conversation/dock.rs, no inbox renderer | [P06](#reference-p06); [P32](#reference-p32) | Assigned inbox contract is available; dedicated fleet list/answer-in-place remains B1/B2, including read-only refusal. |
| F43 / m11:150 / Answer from the command line or from another agent | CLI session answer and agent tools; session.send | session/question-dock.tsx; permission-dock.tsx | conversation/permission.rs; question.rs | [P06](#reference-p06); [P07](#reference-p07) | No separate CLI screen required; retain identical resolved request identities. |
| F44 / m11:156 / Projects: add a folder, clone, start new | project.list/get/add/clone/init/remove | projects/add-project-dialog.tsx; projects-section.tsx | workspace/projects.rs | [P18](#reference-p18) | Existing add/clone/init; clone/setup failures and cancellation need visual review. |
| F45 / m11:157 / A worktree and branch per session | worktree.list/create/output/remove | compose/start-worktree.ts; workspace/setup-card.tsx | workspace/start_card.rs | [P18](#reference-p18) | Existing per-session worktree start; do not bypass setup approval. |
| F46 / m11:158 / Per-project setup and teardown commands, scripts, environment | project.commands/approve; worktree.setup; script.list/run | projects/project-commands.tsx; script-list.tsx (approval only) | workspace/approval.rs | [P18](#reference-p18); [P23](#reference-p23) | Approval display exists; script execution/service controls missing in web. |
| F47 / m11:159 / Diff: working, staged, everything since the base branch | git.status/changes/diff/show/log | workspace/changes-tab.tsx; scope-menu.tsx | workspace/code/changes.rs | [P19](#reference-p19) | Existing working/staged/base scopes; compare default and empty/error behavior. |
| F48 / m11:160 / File tree, viewer, editor, open by name, search in files | file.list/read/write/find/create_dir/rename/delete | workspace/files-tab.tsx; file-tree-view.tsx; file-view.tsx | workspace/code/files.rs; viewer.rs | [P22](#reference-p22) | Existing file flow; upload/download/preview breadth differs, no wholesale new renderer. |
| F49 / m11:161 / Commit, push, pull, branches, stash, discard | git.commit/push/pull/switch/stash/discard, etc. | workspace/git-bar.tsx; commit-dialog.tsx; stash-dialog.tsx | git/menu.rs; commit.rs; stashes.rs | [P20](#reference-p20) | Existing operations; copy native reviewed commit wording and retain refusal detail. |
| F50 / m11:162 / Open a pull request | pr.open | No PR create control in workspace/git-bar.tsx | pulls/form.rs | [P20](#reference-p20); [P21](#reference-p21) | Add title/body/base form using actual server outcomes; generated text not promised. |
| F51 / m11:163 / Pull request facts (checks, reviews, mergeable) on the board | SessionInfo board/PR facts; pr.get/list | board/session-card.tsx shows status lane, no detailed PR card | pulls/card.rs; facts.rs | [P21](#reference-p21); [P37](#reference-p37) | Add checks/review summary, distinguish no PR from unavailable/stale data. |
| F52 / m11:164 / Merge a pull request | pr.merge | No PR merge control | pulls/facts.rs (action completeness unverified) | [P21](#reference-p21) | Add typed refusal/confirmation and refresh facts after success. |
| F53 / m11:165 / Failing checks and review comments sent to the session | pr.feedback | No feedback settings/action | No registered feedback preferences found | Not found in surveyed Paseo app; wider-reference feature retained. | Add explicit feedback enablement; no automatic prompt without configured consent. |
| F54 / m11:166 / A checkpoint per turn; restore the files | worktree.snapshots/drop_snapshot for cleanup; no turn file-restore RPC | projects/project-snapshots.tsx lists cleanup snapshots | workspace/remove.rs; no turn restore surface found | [P10](#reference-p10) | Do not call cleanup snapshot a per-turn checkpoint or promise file rewind. |
| F55 / m11:167 / A reviewing agent | No reviewing-agent RPC on base | No reviewing-agent surface | No registered reviewing-agent surface found | Not found in surveyed Paseo app; wider-reference feature retained. | Blocked; no fake Start review button. |
| F56 / m11:173 / The agent's terminal | session.create terminal; session.send; stream | session/terminal-view.tsx | session.rs terminal wiring; runtime not verified | [P23](#reference-p23); [P42](#reference-p42) | Existing web terminal; native runtime/UI completeness not inferred. |
| F57 / m11:174 / Shell terminals beside a session | terminal.create/list/read/subscribe/send/kill/close | No sibling-shell UI; existing terminal-view only agent session | No sibling-shell renderer registered in pinned head | [P23](#reference-p23) | Add session terminal tabs and lifecycle, bounded scrollback; no new wire needed. |
| F58 / m11:175 / Project scripts and services, their ports found, a preview opened | script.list/run/restart; port.list | projects/script-list.tsx only command approval | No registered scripts/ports execution panel found | [P23](#reference-p23) | Add script/service state then discovered-port action; validate target and do not auto-open ports. |
| F59 / m11:176 / Find in a terminal; scrollback size | Terminal records client-side; no new wire for find | session/terminal-view.tsx has xterm; no search control | No verified terminal find surface | [P23](#reference-p23) | Add search affordance and bounded scrollback setting from existing terminal runtime. |
| F60 / m11:177 / Agents create, read and type into terminals | terminal.* exposed through agent tools | No separate UI needed for agent tools; sibling terminals absent | No registered agent-terminal management surface found | [P23](#reference-p23) | Show created shells/read-only state when sibling-terminal UI lands. |
| F61 / m11:178 / A browser panel | Browser wire not on base | No browser panel | No registered browser renderer found | [P41](#reference-p41) | Blocked until assigned browser capability; keep node opt-in and host restrictions. |
| F62 / m11:179 / Agents drive the browser | Browser automation wire not on base | No browser-driving UI | No registered browser-driving renderer found | [P41](#reference-p41) | Blocked; no broad browser credentials or local Chrome access. |
| F63 / m11:185 / Several machines in one page | Bridge book and fleet.nodes/session.list | shell/sidebar-machines.tsx; board/board-page.tsx | sidebar.rs; board/mod.rs | [P24](#reference-p24) | Existing fleet scope; preserve machine identity and disconnected state. |
| F64 / m11:186 / Pairing by link or code | device.invite plus bridge pairing ops | machines/add-machine.tsx; settings/invite-card.tsx | pair/render.rs; settings/devices.rs | [P24](#reference-p24) | Existing pairing; no secret in URL captures/logs, no preview pairing. |
| F65 / m11:187 / A relay, end-to-end encrypted, with no account | Secure transport below page via local bridge | data/fleet.ts; bridge/client.ts | pair/render.rs and link transport | [P24](#reference-p24) | Retain bridge boundary; Paseo app transport implementation not audited here. |
| F66 / m11:188 / SSH, installing the runtime on the host | CLI SSH connector/install, bridge book | machines/add-machine.tsx documents connect path | No SSH provisioning UI verified in pinned app | [P24](#reference-p24) | No node-management call on base; do not offer inert remote install/update. |
| F67 / m11:189 / Move a session to another machine | session.export/import; transfer.read/close | No cross-machine transfer UI; move-profile is local account only | No cross-machine transfer UI found | Not found in surveyed Paseo app; wider-reference feature retained. | Add explicit transfer preview/progress later, preserve file-loss constraints. |
| F68 / m11:190 / Agents start sessions on other machines | session.create via fleet.forward; agent tools | board/card-origin.tsx; sidebar source attribution | board/card.rs | [P15](#reference-p15) | Display children/remote origin; no extra privileged agent tools from browser. |
| F69 / m11:191 / Instructions, skills, MCP servers and secrets shared across machines | fleet.status/published/sync/members; secret.list | settings/sync-section.tsx (published read) | settings/sync.rs | Not found in surveyed Paseo app; wider-reference feature retained. | Wire join/leave/sync actions missing on page; never expose secret values. |
| F70 / m11:192 / Spread new sessions over machines by load | No spread-by-load RPC on base | No load placement control | No registered load-placement UI found | Not found in surveyed Paseo app; wider-reference feature retained. | Blocked until actual scheduler/placement behavior exists. |
| F71 / m11:193 / A phone app | Node transport shared; phone packaging is outside web RPC | Responsive shell and settings CSS, not an installed phone app | Native phone build is separate; no runtime verified | [P24](#reference-p24); [P36](#reference-p36) | Responsive web regression only; do not claim mobile-app/push parity. |
| F72 / m11:194 / Push notifications | push.key/subscribe/unsubscribe/presence22; paired-device leases | features/notifications/use-notices.ts is desktop notification only | No registered push settings found | [P36](#reference-p36) | Node contract is available; browser service-worker/subscription workflow is absent. Paseo web intentionally no-ops push; native differs. |
| F73 / m11:200 / Agents start, prompt, wait on and stop sessions | session.create/prompt/read/kill and agent tools | Existing normal session controls | conversation/actions.rs | [P15](#reference-p15) | No separate orchestration page required; preserve scope intersection. |
| F74 / m11:201 / The starter is told when a session it started finishes | session notification delivery and starter events | board/card-origin.tsx; no persistent starter-notice page | No verified starter-notice surface | [P15](#reference-p15) | Show actual completion notification when inbox wiring exists; no invented completion. |
| F75 / m11:202 / Schedules: every so often or at a time, a new session or a prompt | schedule.list/get/create/update/pause/resume/run/delete/runs/preview | No schedules page or data module | No schedule renderer in committed settings/sections.rs | [P25](#reference-p25) | Largest ready common gap: list+human cadence first, editor/preview next, actions/runs separately. |
| F76 / m11:203 / A recurring prompt to oneself | schedule.create with session target/recurring cadence | No recurring-prompt UI | No registered recurring-prompt UI found | [P25](#reference-p25) | Reuse schedule editor; keep timezone/due/ended-target rules from node. |
| F77 / m11:204 / Saved prompts | prompt.list/create/update/delete | No saved-prompts page | No registered saved-prompts renderer found | Not found in surveyed Paseo app; wider-reference feature retained. | Add bounded picker/editor; no claim Paseo has an identical named surface. |
| F78 / m11:205 / A webhook starts a session | hook calls not on base | No webhook UI | No registered webhook renderer found | [P25](#reference-p25) | Blocked until hooks merged; receiving endpoint/secret generation never moves into page. |
| F79 / m11:211 / A board by status | session.list board projection | board/board-page.tsx; data/board.ts | board/mod.rs | Not found in surveyed Paseo app; wider-reference feature retained. | Existing board; Paseo has status/sidebar views rather than this board surface. |
| F80 / m11:212 / Sidebar: group, sort, filter, pin | Session marks; bridge preferences | shell/sidebar-view-menu.tsx; data/sidebar.ts | sidebar.rs | [P05](#reference-p05); [P37](#reference-p37) | Ended default, stored off and archive exclusion covered by the paired follow-up; broader grouping/pinning/nesting parity remains unverified. |
| F81 / m11:213 / A notification inbox that survives a reload | inbox.list/get/mark22 with durable shared read state | notifications/use-notices.ts is transient; no durable inbox | No inbox renderer registered | [P32](#reference-p32); [P06](#reference-p06) | Implement B1 durable list/read cursor, loading/offline/partial/refusal states; no local fake inbox. |
| F82 / m11:214 / Desktop notification and sound | Client preferences; browser Notification API | notifications/use-notices.ts; sound.ts; settings/notifications-section.tsx | toasts.rs; no OS-notification integration verified | [P32](#reference-p32) | Existing web desktop alerts/sound; compare focus/privacy defaults before changing. |
| F83 / m11:215 / Usage: tokens, cost, each account's limits | usage.query21; usage.limits/lookup23 | usage/usage-section.tsx A1 accepted; scope-picker.tsx A2a verified checkpoint; routed turn/proxy counts retained | conversation/footer.rs/meter.rs; settings Usage=None | [P26](#reference-p26); [P27](#reference-p27); [P28](#reference-p28); [P29](#reference-p29); [P44](#reference-p44) | A1 machine day/week is accepted. A2a agent/profile selection is accepted after its round-trip correction; A2b session reports and A3/A4 limit controls/notices remain. |
| F84 / m11:216 / Pull requests page | pr.list/get | No dedicated pull-requests route | pulls/mod.rs session dialogs only | [P21](#reference-p21) | Add project-aware list and facts after schedules; do not claim all-PR coverage from watched sessions only. |
| F85 / m11:222 / Light, dark, system | Bridge preference theme | settings/appearance.tsx; data/theme.ts | settings/appearance.rs | [P38](#reference-p38) | Existing three schemes; preserve system/reduced-motion and tokens. |
| F86 / m11:223 / More themes | Client themes; no node RPC required | Only light/dark/system tokens | settings/appearance.rs three choices; extra theme branch not integrated | [P38](#reference-p38) | Wait for shared theme decision/data; do not invent new purple/gradient theme. |
| F87 / m11:224 / Font sizes, content width, how much of a turn is shown | Bridge preferences; local rendering | settings/appearance.tsx; data/prefs.ts | settings/appearance.rs | [P30](#reference-p30) | Existing sizes/width/turn display; no global redesign. |
| F88 / m11:225 / Shortcuts, rebindable | Client shortcut registry/preferences | settings/shortcuts-section.tsx; data/shortcuts/registry.ts | shortcuts/page.rs | [P31](#reference-p31) | Existing rebindings; preserve IME and platform modifiers. |
| F89 / m11:226 / Languages | Client language resources; not node wire | English labels; no page locale selector | settings/language.rs; Fluent-backed text source | [P39](#reference-p39) | Shared translation integration is separate; first feature must not claim whole-page Korean parity. |
| F90 / m11:227 / Command palette | Client command registry | palette/palette.tsx; data/palette.ts | palette.rs | [P13](#reference-p13); [P31](#reference-p31) | Existing command palette; add working routes only, not future capabilities. |
| F91 / m11:228 / Update from the page | node.stop3 and CLI node update; no remote-update RPC | settings/about.tsx has versions, not update action | update.rs app updater, not node update | [P24](#reference-p24) | Node update UI remains blocked by its missing update contract; app updater and node.stop do not supply remote update. |
| F92 / m11:229 / Diagnostics | node.info; staged diagnostic types/collector, no assigned report RPC | settings/about.tsx; no diagnostics report | settings/sections.rs Diagnostics=None | [P40](#reference-p40) | Basic versions exist; diagnostics screen needs assigned report transport. Staged collector/redaction values alone are not callable. |
| F93 / m11:230 / Plugins | Plugin management wire not on base | No plugins section | settings/sections.rs Plugins=None | [P33](#reference-p33) | Blocked; no arbitrary script/plugin loading in browser. |

## Milestone 12 feature rows

The references below are the same pinned source index; the F ids preserve
the finer-grained feature rows rather than reclassifying historical track
assignments as current implementation proof.

| Source row / area | Node has it: calls / boundary | Page has it: file | Native app has it | Paseo web/app source | What the page lacks |
| --- | --- | --- | --- | --- | --- |
| m12:360 / Conversation (F01–F25) | session.prompt/queue/conversation/mark, usage_changed | session/composer.tsx; timeline-item.tsx; inspector-summary.tsx | conversation/{queue,question,meter,plan,rewind}.rs | [P02](#reference-p02) [P04](#reference-p04) [P06](#reference-p06) [P08](#reference-p08) [P10](#reference-p10) | Global search, forks, usage totals; provider-dependent capabilities must stay explicit. |
| m12:361 / Agent terminal and sibling shells (F02,F56–F60) | session stream; terminal.* | session/terminal-view.tsx | session.rs; sibling shell not registered | [P23](#reference-p23) [P42](#reference-p42) | Shell tabs, scripts/ports and terminal find. |
| m12:362 / Composer (F26–F32) | attachment.*, file.find, advertised commands | compose/prompt-box.tsx; session/composer.tsx | composer/{drafts,offers,slash}.rs | [P02](#reference-p02) [P16](#reference-p16) [P17](#reference-p17) | Check edge cases; retain existing attachment/draft/recall behavior. |
| m12:363 / Agents/defaults (F35–F40) | harness.list/options; agent.status/list/catalogue/install | machines/agents-section.tsx; settings/models.tsx | settings/agents.rs; Models unregistered | [P03](#reference-p03) [P24](#reference-p24) | Registry editing/install progress and exact readiness; route editing is not implied. |
| m12:364 / Approvals/inbox (F41–F43,F81) | session.send; inbox.list/get/mark22 | session/permission-dock.tsx; board/lane.tsx | conversation/permission.rs; no inbox registered | [P06](#reference-p06) [P07](#reference-p07) | Assigned durable fleet inbox calls are available; inline B1/B2 decisions remain. |
| m12:365 / Projects/git/PRs (F44–F55,F84) | project.*, worktree.*, git.*, file.*, pr.* | projects/projects-section.tsx; workspace/panel.tsx | workspace/code/mod.rs; git/mod.rs; pulls/mod.rs | [P18](#reference-p18) [P19](#reference-p19) [P20](#reference-p20) [P21](#reference-p21) [P22](#reference-p22) | PR create/facts/merge and explicitly partial watched-session list; project-wide enumeration and turn checkpoints still require node behavior. |
| m12:366 / Scripts/services/ports/browser (F58,F61–F62) | script.list/run/restart; port.list; no browser calls | projects/script-list.tsx approval list only | No execution/browser panel registered | [P23](#reference-p23) [P41](#reference-p41) | Run/service/port controls; browser blocked until assigned wire. |
| m12:367 / Machines/devices/sync (F63–F69) | device.*; fleet.status/published/join/leave/sync | machines/machines-section.tsx; settings/{devices-section,sync-section}.tsx | pair/render.rs; settings/{devices,sync}.rs | [P24](#reference-p24) | Wire existing fleet Admin actions and self-scopes; no credential exposure. |
| m12:368 / Automation (F75–F78) | schedule.*; prompt.*; hooks absent | No automation route | No schedule renderer in pinned settings/sections.rs | [P25](#reference-p25) | Largest unimplemented ready family: list then edit/preview/actions/runs; hooks wait. |
| m12:369 / Board/sidebar/palette/notices (F79–F82,F90) | session board/marks plus client preferences | board/board-page.tsx; shell/sidebar.tsx; palette/palette.tsx | board/mod.rs; sidebar.rs; palette.rs | [P05](#reference-p05) [P32](#reference-p32) [P37](#reference-p37) | Durable inbox; regress existing controls rather than replacing their design. |
| m12:370 / Settings (F85–F89,F91–F92) | Existing preferences; node.info/settings/stop; staged diagnostics, no assigned report/update RPC | settings/{appearance,shortcuts-section,about}.tsx | settings/{appearance,language}.rs; update.rs | [P24](#reference-p24) [P30](#reference-p30) [P31](#reference-p31) [P38](#reference-p38) [P39](#reference-p39) [P40](#reference-p40) | Translation/themes remain; diagnostics and remote update still need callable assigned contracts. |
| m12:371 / History/lifecycle (F13–F18,F20) | session.rewind/fork/search/park/wake; agent.history; no cross-agent handoff | No these action surfaces; session resume exists | conversation/rewind.rs; search.rs partial | [P10](#reference-p10) [P11](#reference-p11) [P12](#reference-p12) [P14](#reference-p14) [P24](#reference-p24) | Wire existing history/lifecycle actions incrementally; no file restore promise. |
| m12:372 / Usage/limits (F24,F83) | usage.query21; usage.limits/lookup23 | A1 usage/usage-section.tsx; routed inspector/turn counts retained | Usage unregistered; conversation meter/footer only | [P26](#reference-p26) [P27](#reference-p27) [P28](#reference-p28) [P29](#reference-p29) [P44](#reference-p44) | A1 totals accepted; A2a agent/profile selection is accepted; next A3/A4 consent/windows/factual notices on assigned limits calls. |
| m12:373 / Phone/push (F71–F72) | Shared node transport; push.key/subscribe/unsubscribe/presence22 | Responsive page only | Phone build/runtime unverified | [P24](#reference-p24) [P36](#reference-p36) | Do not claim installed-app/push parity from a narrow viewport. |
| m12:374 / Browser as surface | Local bridge serves page; GPUI browser is separate spike | app.tsx; routes/; loopback bridge | Wasm experiment is not connected-client parity proof | [P13](#reference-p13) [P23](#reference-p23) [P24](#reference-p24) | This track maintains the existing page; no new native/Wasm/browser engine. |

## Milestone 12 wire-gap rows

Both historical occurrences are retained in the source-row column. “Ready”
here means the checked base supplies the contract, not that a page was drawn.

| Source rows / feature | Node has it: calls | Page has it: file | Native app has it | Paseo web/app source | What the page lacks |
| --- | --- | --- | --- | --- | --- |
| m12:444,617 / PRs without a live session | Not available: pr.list {} returns bounded watched-session entries only | No PR route; workspace/git-bar.tsx seam | pulls/mod.rs session dialog only | [P21](#reference-p21) | Project/no-live-session enumeration waits for its separately assigned node contract to merge. A partial watched-session list must be labelled exactly. |
| m12:445,618 / Webhook control/listener | Not on this base | No webhook route | No registered hook UI | [P25](#reference-p25) | Blocked; a schedule list is not webhook support. |
| m12:446,619 / Machine timezone | node.info timezone | No schedule UI; data/nodes.ts transports info | Schedule renderer absent in pinned head | [P25](#reference-p25) | Use node timezone in future cadence/preview, never browser timezone by accident. |
| m12:447,620 / Preview next runs | schedule.preview | No schedule form | Schedule renderer absent in pinned head | [P25](#reference-p25) | Editor must use server preview; no client cron approximation. |
| m12:448,621 / Fleet join/leave/sync/applied state | fleet.join/leave/sync/status | settings/sync-section.tsx published/read only | settings/sync.rs has actions; runtime unverified | [P24](#reference-p24) | Add existing Admin actions, Read status, and explicit destination. |
| m12:449,622 / Fixture registry address | Not confirmed from code in this slice | No registry UI; machines/agents-section.tsx lists existing | settings/agents.rs; registry path not verified | [P24](#reference-p24) | Verify registry override before registry e2e; never call the live catalogue in tests. |
| m12:450,623 / Caller own scopes | device.self protocol20 | No data helper/action gating from device.self found | Runtime scope UI not verified | [P24](#reference-p24) | Capability-aware controls; unknown is not granted, server remains authoritative. |
| m12:451,624 / Unreachable pairing invitation | device.invite rejects no advertised routes | settings/invite-card.tsx | settings/devices.rs | [P24](#reference-p24) | Preserve typed setup guidance and no invitation when impossible. |
| m12:452,625 / Too-old/too-new agent states | agent.status; exact readiness flow not runtime checked here | machines/agent-row.tsx; data/agent-status.ts | settings/agents.rs | [P24](#reference-p24) | Do not manufacture reachable states from the presence of an enum; fixture/readiness audit needed. |

## Milestone 12 UI-defect rows

These are cross-surface checks, not new permission to modify the native kit.

| Source row / check | Node has it | Page has it: file | Native app has it | Paseo source | What the page lacks / must verify |
| --- | --- | --- | --- | --- | --- |
| m12:479 / semantic badges, alerts, destructive/split buttons | No new wire | `web/src/ui/kit/` controls exist | Native kit controls are separate; not changed | P20, P26 | Reuse kit and existing status tokens; no new look. |
| m12:480 / dialog content/actions stay reachable | No new wire | `web/src/ui/kit/dialog.tsx`; workspace dialogs | Native dialog implementation not runtime checked | P20, P21 | Capture long failures at 390 and small desktop; no height waiver. |
| m12:481 / multiline commit/body fields | git.commit/pr.open | `workspace/commit-dialog.tsx` | `git/commit.rs` | P20 | Compare labels, line breaks and destructive destinations in a later isolated slice. |
| m12:482 / equal-length output replacement | Terminal/output events | `session/terminal-view.tsx` | Native kit output not checked here | P23 | Existing terminal/output regression, not this first accounting slice. |
| m12:483–484 / Korean font fallback and IME | No new wire | `web/src/ui/fonts.css`, compose input | Native X11/Windows behavior remains outside this track | P17, P39 | Pretendard precedes generic mono/serif; validate composition in browser, no OS IME claim. |

## Ordered implementation families and small commits

The first three families are **A Usage and limit notices**, **B Inbox**,
**C Schedules**. The inbox and account-limit calls are now on the merged node head.
Every family still obeys the dependency boundary: never register a
call/type absent from that head, invent a
protocol number or bypass the unassigned-protocol contract. If blocked, continue only a ready sub-slice
without presenting a dead control. Each code sub-slice targets a few
hundred production lines, with separate pure unit cases and one actual
fake-fleet browser flow; split further at a real seam when it grows.

| Order | Small user-visible slice | Scope / approximate production budget | Unit cases and actual-node browser acceptance | Dependency |
| --- | --- | --- | --- | --- |
| A1 | Settings → Usage: current machine's reported totals | Typed usage.query wrapper/model, Settings registration and existing row cards; 250–350 lines | Zero vs unknown, cost withheld, optional cache/reasoning coverage, overflow/incomplete; open Usage on fake node, real turn reports, day/week and Refresh | Ready on base, no wire change |
| A2 | Per-agent/profile and session accounting | Reuse A1 report rows/requests, selection and session summary; 200–300 lines | Historical profile None semantics, session-only period/cursor, sparse pages, selector races; change machine/agent and open actual session totals | usage.query only |
| A3 | Saved-login account limits/opt-in | Own model/control component and existing Settings rows; 250–350 lines | Off/ready/stale/unassociated/cooldown/unknown periods, raw finite pct clamped, zero/missing balance; synthetic login fixture only, admin vs read-only/operate | Ready on merged f453ec2e; use assigned limits/lookup calls |
| A4 | Session factual limit notice | One shared model and composer dock registration; 150–250 lines | Multiple full windows/unknown resets/paid eligibility/no false cause; fake session notice and working refresh action only | Ready limits facts; no recovery API invented |
| B1 | Inbox list and unread | Typed inbox API wrapper + list model + nav/route; 250–350 lines | Reload/cursor/offline/partial machine errors, no local-only fabricated history; actual node creates durable items | Ready on merged f453ec2e; inspect exact generated types first |
| B2 | Decide from inbox | Reuse permission/question docks, own request-bound state; 200–300 lines | Resolved/stale/race/read-only refusal; answer one real fake permission and question from list | Accepted B1 and node ownership model |
| C1 | Schedules list in human words | Data wrapper/formatting + a list route using current rows; 250–350 lines | Host/status/due/ended target/empty/error; real scheduled item visible, reload retains it | Existing schedule.list/get/runs; no new wire |
| C2 | Schedule create/edit with server preview | Draft model and dialog; 250–350 lines | Cadence bounds, node timezone/DST preview, refusal; create/edit on fake node and compare returned next runs | Existing schedule.preview/create/update |
| C3 | Pause/resume/run/delete and runs | Actions and bounded run rows; 200–300 lines | Busy/forbidden/changed/failed run; action changes real node state, reload verifies it | Existing calls; same scopes as node |
| D1 | Watched-session PR list/facts then actions | Split partial read list/facts, create, merge into ≤350-line slices | Watched-session coverage label, stale/error/read-only and bounded omissions; actual isolated forge fixture, no live forge | Existing pr.list {} / session-bound pr.get/open/merge only; project/no-live-session acceptance waits for the separately assigned project-enumeration contract to merge |
| E1 | Search/history/lifecycle | One feature per ≤300-line slice: global search; history import; fork; park/wake; rewind | Real bounded cursors and capability refusal, navigation to actual record, no invented file rollback | Existing base calls only |
| F1 | Sibling shells/scripts/ports | Split tabs/lifecycle, scripts/services, port navigation into ≤350-line slices | Own fake terminal bytes/end state, setup approval, port limits, strict URLs; no live preview access | Existing terminal/script/port calls |
| G1 | Agent setup/defaults and exact settings parity | Separate catalog/readiness/install/defaults/wording slices, each ≤350 lines | Long/none/error/read-only states and real fixture installer; no credentials entered or copied by page | Existing calls; no unassigned wire |
| Held | Push, browser, dictation, plugins, diagnostics/node update, handoff/delivered facts | No registration, fake actions or numbers while absent from node head | Re-plan against merged contract, security/disclosure and real-shaped fixtures before implementing | Assigned merged node wire first |

C is the largest ready common feature family missing entirely from this
page: schedule management, recurring prompts and their next-run/timezone
controls cover several union/area/wire-gap rows. PRs and terminals are
also substantial gaps, but those extend existing project/terminal surfaces;
the schedule route is a wholly absent, independently testable first step.
This prioritization is a source-based scope choice, not a measured demand
ranking or a claim that every broad row fits one small patch.

## Acceptance for the first ready implementation

1. Open Settings → Usage using the existing navigation and machine picker.
   A disconnected or old machine shows its real unavailable/upgrade state.
2. Read rolling 24-hour or seven-day **reported** totals from usage.query.
   Unknown counts stay unknown; reported zero remains visible. Reasoning
   and both cache fields retain their report coverage. Never add context
   occupancy or routed proxy counts to the ledger total.
3. Cost withheld is distinct from not reported. Keep currencies/units
   separate, label accounting start/retention/incomplete/overflow and
   unaccounted costs; never infer a price or assign a cumulative baseline
   to today's spend. No samples cursor is needed for the first machine page.
4. Change period/machine or Refresh and read the new real node response.
   An old in-flight response cannot replace the newly selected identity.
   Fetch only the visible selected machine, bound concurrent work, and
   retain errors explicitly. No background vendor polling.
5. No account bars, credential opt-in row or wait/continue action is drawn
   before its real contract is merged. A full bar later is a factual
   account observation, never proof of a stopped turn. Automatic recovery
   and account switching remain absent unless separately implemented.
6. Reuse existing row/card, Button/Select/Banner/Skeleton and token CSS.
   No new theme, purple, gradients, fonts or generic-first mono/serif.
   New transitions use existing tokens and respect reduced motion.

Differences from Paseo are explicit: Vessel keeps reported token/day/week
accounting as well as the provider card; it never invents prices; saved-login
lookup is default-off for the credential boundary (ADR 0042); old peers get
a typed unavailable state; reference rewind file modes/recovery actions are
not offered where Vessel does not implement them. Source is read as a
behavioral reference, not copied code/assets. Existing page English labels
are retained initially; this is not a claim of translated-page parity.

## Checks and limits of evidence

Documentation-only contract: fmt/check and available size/boundary checks;
no compile/test rerun for documents. The original inventory base lacked
root `scripts/` size/boundary programs and `docs/QUALITY.md`; that limitation
applies only to the retained initial documentation check. The subsequent
node-head merge supplies them, and they apply to all following checks. Native gate/token checks do
not apply: no native source is touched.

For each code slice: one detached gate through one bounded build slot, own target
and npm install, private mode-0700 SSD TMPDIR, load/memory before and after,
both test-binary prebuilds (testkit bins and vesseld) before tests in the same
environment, fmt, whole Clippy/Rust tests, TypeScript freshness, and web
lint/types/unit/build/full headless e2e. Preserve contemporary pre/post
selected-input identities, actual changed-crate compilation, served-bundle
hashes and verified-PID cleanup. No deadline increases, ignored suite parts
or unchanged successful reruns. Source-only inventory is not any such run.

Rendered acceptance uses the repository Playwright suite (the task's
authorized fake fleet). Browser plugin is not available in this session;
regular Playwright is the supported path. Traces/video/automatic failure
screenshots stay off because WS/URL data can carry credentials. Purposeful
content-only captures use the separate screens path outside tracked source,
with desktop/390px, both schemes, small windows and console/identity/flow
checks. No token or pairing URL in captures/output. The existing page has
no language selector; numeric locale checks can cover English/Korean but
do not prove whole-page translation. Native runtime, live providers, real
credentials/Keychain, OS IME, push, all other platforms and every later
feature's full behavior/defaults are **not verified** by this inventory.
Many reference entries establish surface/type wiring only: their complete
interaction/default/error behavior must be re-read for the implementing
sub-slice before it can be called parity. No independent security or
licence audit, deployment, live permission change or repository push.

## Resume point

The parity contract passed documentation review (`39fcda13`). Node head
`a365d295` was merged without conflicts (`5790cdd3`). The corrected A1
source `1907bd0a` passed the whole merged-tree gate: Rust 1,963 passed /
zero failed / 12 ignored, TypeScript freshness two passed, frontend units
428 passed, full browser E2E 63 passed, and sixteen capture cases passed.
All format, size, boundaries, Clippy, web lint, typecheck and bundle steps
passed. Execution identity, served assets and cleanup are recorded below.
This bounded A1 source/runtime/capture scope passed implementation review
and maintainer acceptance. It is not whole-page parity or combined
integration approval.

A1 adds machine reported accounting in Settings: pick a machine and the
past day/week, read totals and coverage, then explicitly refresh. It adds
no wire method, field, protocol/schema number, crate edge or native change.
The earlier failed whole run and capture-bootstrap diagnostic remain in
the historical sections below. The complete-list palette correction adds
only the new Usage entry between Devices and Sync; every prior entry and
its order remain. No deadline, assertion or quality exception was loosened.

Next is A2's existing-contract agent/profile and session accounting, split
into small visible increments. Account limits and inbox still wait for
their separately assigned merged contracts. No live account, vendor,
native runtime or other-platform check is claimed by this page work.

## Paseo source index

All entries below refer to Paseo 0.15.4, commit
`1c00b365c9e0b74c1580eba33d905d693125ae21`. Ranges identify what was read,
not a declaration that the entire file was behaviorally audited.

### Reference p01

`packages/app/src/composer/draft/create-flow.ts:111–146` — Draft creation has independent pending/error state.

### Reference p02

`packages/app/src/composer/index.tsx:1560–1593` — Steer and interrupt are explicit submission modes.

### Reference p03

`packages/app/src/composer/agent-controls/index.tsx:101–162` — Provider/model/thinking/mode/features controls.

### Reference p04

`packages/app/src/composer/index.tsx:1949–1978` — Queued send-now and editing.

### Reference p05

`packages/app/src/components/sidebar/sidebar-workspace-menu.tsx:85–124` — Workspace rename, archive, pin and unread actions.

### Reference p06

`packages/app/src/agent-stream/view.tsx:143–156` — Pending permission cards inside the chat.

### Reference p07

`packages/app/src/components/question-form-card.tsx:1–38` — Question form renderer and structured choices.

### Reference p08

`packages/app/src/components/context-window-meter.tsx:34–71` — Valid context usage and session cost, bounded percentage.

### Reference p09

`packages/app/src/components/plan-card.tsx:1–38` — Plan presentation is its own component.

### Reference p10

`packages/app/src/components/rewind/rewind-menu.tsx:47–81` — Rewind choices come from capabilities; locked while pending.

### Reference p11

`packages/app/src/agent-stream/turn-footer.tsx:27–59` — Completed and in-flight fork boundaries differ.

### Reference p12

`packages/app/src/components/import-session-sheet.tsx:25–54` — Provider import has paging, errors and provider filtering.

### Reference p13

`packages/app/src/agent-stream/chat-find/index.web.tsx:1–36` — Web chat find is a separate surface.

### Reference p14

`packages/app/src/screens/sessions-screen.tsx:73–115` — History search and host selection.

### Reference p15

`packages/app/src/subagents/parent/callout.tsx:1–38` — Parent/child agent linkage.

### Reference p16

`packages/app/src/composer/recall.ts:21–51` — Arrow history starts empty and stops after edits.

### Reference p17

`packages/app/src/screens/settings/general/sending-section.tsx:35–63` — Send shortcut and default delivery settings.

### Reference p18

`packages/app/src/screens/projects-screen.tsx:60–95` — Per-host project list and scoped failures.

### Reference p19

`packages/app/src/git/diff-pane.tsx:33–59` — Changes/commits and review feedback panels.

### Reference p20

`packages/app/src/git/actions-split-button.tsx:78–99` — Primary git action resolves pending/success labels.

### Reference p21

`packages/app/src/git/pull-request-panel/pane.tsx:1–42` — Pull request pane and its sections.

### Reference p22

`packages/app/src/components/file-explorer-pane.tsx:1–40` — File tree/viewer host.

### Reference p23

`packages/app/src/components/terminal-pane.tsx:1–46` — Terminal find/input/paste controls.

### Reference p24

`packages/app/src/screens/settings/host-page.tsx:45–71` — Host runtime/providers/usage and lifecycle.

### Reference p25

`packages/app/src/screens/schedules-screen.tsx:94–135` — Host/status filters and create/edit state.

### Reference p26

`packages/app/src/usage/usage-section.tsx:54–91` — Off/unavailable/loading/error/empty/report states.

### Reference p27

`packages/app/src/usage/card.tsx:80–165` — Provider header, plan, refresh, separate windows/balances and stale age.

### Reference p28

`packages/app/src/usage/format.ts:29–38` — Finite provider percentages are clamped for display.

### Reference p29

`packages/app/src/usage/format.ts:103–139` — Relative reset and local clock labels.

### Reference p30

`packages/app/src/screens/settings/appearance/appearance-section.tsx:55–85` — Typography rows commit user values.

### Reference p31

`packages/app/src/screens/settings/keyboard-shortcuts-section.tsx:17–38` — Override/normalization registry and platform rules.

### Reference p32

`packages/app/src/utils/os-notifications.ts:23–64` — Desktop and browser notification paths are separate.

### Reference p33

`packages/app/src/screens/settings/plugins-page.tsx:24–45` — Plugin state and explicit reload/enable/disable/remove actions.

### Reference p34

`packages/app/src/components/dictation-controls.tsx:11–46` — Recording/processing/accept/retry control states.

### Reference p35

`packages/app/src/voice/realtime-voice-config.ts:1–14` — Realtime voice has dedicated configuration.

### Reference p36

`packages/app/src/push-notifications/index.web.ts:1–9` — Web push is a no-op; native push is separate.

### Reference p37

`packages/app/src/components/sidebar/display-preferences/menu.tsx:1–38` — Sidebar display settings surface.

### Reference p38

`packages/app/src/appearance/theme-picker.tsx:1–40` — Theme selection surface.

### Reference p39

`packages/app/src/i18n/locales.ts:1–38` — Locale list is data, not display text switches.

### Reference p40

`packages/app/src/components/app-diagnostic-host.tsx:1–18` — Diagnostics host is distinct from content.

### Reference p41

`packages/app/src/screens/settings/browser-tools-card.tsx:1–45` — Browser tools have explicit opt-in configuration.

### Reference p42

`packages/app/src/screens/new-workspace-terminal.ts:1–45` — Terminal workspace flow differs from agent chat.

### Reference p43

`packages/app/src/composer/input-mode.ts:1–45` — Composer terminal input mode.

### Reference p44

`packages/app/src/usage/queries.ts:134–160` — Fetch only connected supported enabled hosts; explicit refresh.

## Inventory source-review correction

The first source review returned the PR-list dependency: on the pinned
node, `ListPullRequests {}` (`crates/vessel-proto/src/forge/calls.rs:18–21`)
is forwarded to the watcher (`crates/vessel-node/src/service/forge.rs:31–36`),
which enumerates its held session map and truncates to its watch bound
(`crates/vessel-node/src/forge/watcher.rs:432–445`). Filtering those entries
cannot discover unobserved project PRs. The wire-gap row and D1 now agree
with F84: a labelled watched-session partial list is ready, project/no-live-
session coverage is held for the separately assigned node extension after
merge. This page track adds no replacement wire or client-side discovery.

ADR 0020 is historical and was never accepted; current page retention is
ADR 0036, with the native choice in ADR 0035. The correction changes only
these inventory/history documents. A1's assigned usage-query scope is
unchanged. Documentation correction review is pending; no feature code
was written before correcting the returned contract. No UI/runtime or
whole-table behavioral parity approval is implied.

## First accounting implementation checkpoint

The documentation correction passed contract review. A1 now has an
unverified implementation draft: usage-query call edge, a single-flight
latest-selection reader, pure token/cost/coverage facts and the Settings
section. Scoped type checking and eight pure tests passed before the
latest lint corrections; lint found four issues (an intentionally unsafe
integer literal in a test and three concise void arrows). These were
corrected without a rule exemption. Lint has not yet been rerun, and the
implementation has no whole gate, browser flow or capture evidence yet.
No limits/inbox/future call is registered. A new checked node head is to
be merged at this clean checkpoint; its quality rules apply before further
implementation and the whole merged tree must pass.

## Merged node head and current quality boundary

The accepted documentation commits remain unchanged in history. Draft
implementation checkpoint `2f169260` was followed by merge `5790cdd3`,
which brings node head `a365d295` without conflicts or rebase. The node
head's protocol/method tables, schema/migrations, tests and document entries
were retained. Comparing the two node snapshots shows no RPC table or
protocol-constant change: usage.query remains assigned at 21; limits, inbox
and project-wide PR enumeration are still unavailable here. The feature
rows therefore retain their accepted capability classification.

The merged head adds `docs/QUALITY.md`, strict AGENTS rules and root size/
boundary checks. Those now govern the page slice. No root check, schema
number, test expectation or quality allow-list is changed by this track.
Store schema 15 is the head's, not an added page migration. The whole
merged implementation gate is still owed, including test-binary prebuilds
and all Rust/web steps. A known shared side-question forwarded test race
is outside the page implementation; any actual failure must be retained
and its one instructed exact rerun reported separately, never erased.


## A1 implementation and verification boundary (source checkpoint)

Modification/reuse: the existing bridge call, Outcome guidance, Store,
Settings frame, machine picker, row list, value, select and button controls
are reused. New `data/usage` modules isolate this query's single-flight
lifetime and its coverage/unit formatting; a long-poll reader would add
background work to an explicitly refreshed ledger. Native Intl replaces
no library and adds no dependency. Existing English page labels remain;
number/date formatting follows the viewer locale. This is a deliberate
incremental difference from Paseo's translated account-window cards:
only the assigned machine-accounting contract is currently available.

Production code is split along call, request-lifetime, pure facts and
rendering responsibilities (each below the page file budget). The first
estimate of 250–350 lines proved too small for explicit optional coverage,
redaction, unsafe-browser-integer handling and identity-safe reads; this
bounded slice retains those behaviors rather than dropping them to fit
an estimate. The session/profile extension remains separate.

The real-node browser case uses the existing ACP fake's usage-reasoning
response, compares displayed values with the sanitized actual query reply,
changes day/week, adds a real second turn and refreshes the ledger. Pure
cases cover zero/missing, per-field partial reports, separate units and
tiny costs, overflow/retention gaps, redaction, locale and selection races.
Intentional screenshots use only isolated synthetic content, in both
system themes and four viewport sizes; Korean locale formats numbers and
dates but does not translate the page's English labels.

Historical A1 source checkpoint: source was prepared for its immutable merged-tree gate. Earlier
focused type/unit runs passed; lint exposed ordinary test/style issues
which were corrected without exemptions. Whole Rust/web, real-node e2e,
served assets and rendered captures are still unverified at this source
checkpoint. No limits/inbox/new wire/native change or live service action
is included. The next step is the whole gate and recording its actual
results, including any shared-test failure and the instructed exact rerun.


## First whole accounting run and retained failure (historical)

The first immutable implementation run stopped at the command palette
unit assertion for the complete Settings list, which did not yet include
Usage: 427 passed and one failed. Its successful Rust and web checks
remain scoped evidence; the incomplete whole run remains a failure. The complete Settings-list assertion gains the new product entry
without dropping or moving any pre-existing entry. Actual browser flow,
intentional captures, final asset freshness and a new whole changed-tree
run are required before calling this slice validated.


## Isolated browser diagnostic and capture correction (historical)

On the same immutable source checkpoint, a separate diagnostic built the
browser binaries, then passed the real-node usage flow: selected machine,
day/week, reported reasoning/cache zero, a second actual fake-agent turn,
and refreshed ledger values. The flow passed once; this does not turn the
stopped whole gate into a success. Twelve of sixteen intentional captures
passed. All four 390-pixel cases stopped at the shared desktop bootstrap
helper's Sidebar visibility assertion, before measuring Usage layout.

The capture setup now authenticates and seeds at the existing desktop
size, then resizes to each requested viewport and keeps the report-visible
and no-horizontal-overflow assertions. The common bootstrap helper and
all deadlines remain unchanged. Captures use the established machine-name
mask convention. This proves neither mobile cold-start behavior nor a
translated page: Korean locale applies to numbers/dates; labels stay English.

Both diagnostic fleets recorded matching hashes for all eighteen static
assets referenced by their actual index. An extra audit assertion comparing
the per-request painted HTML with the unpainted bundle file failed; this
was an audit assumption, not a static-asset mismatch. `crates/vessel-web/src/files.rs`
documents the request painting. The original observations remain retained;
subsequent freshness checks compare static assets and explicitly exclude
that HTML byte comparison. No live service or access permission changed.
A complete gate on the corrected tree, including all browser cases and
all sixteen corrected captures, remains required.


The corrected capture cases reuse the existing `e2e/shot.ts` helper,
including its machine-name mask and cosmetic frame settling after real
visibility/font readiness. Each of the sixteen locale/theme/viewport
cases records an overview and a scrolled coverage view (32 images), so
cost and all provenance dates can be inspected inside the actual scroll
container. The usage flow's no-raw-ISO assertion now matches any RFC3339
date prefix rather than a fixed fixture timestamp. These strengthen the
evidence; no product state, deadline or existing assertion was weakened.


After the explicit palette-list correction, all 428 frontend unit tests
passed in the narrow run. This does not erase the stopped whole run or
execute the strengthened browser/capture cases. Narrow type/lint checks
and the complete corrected-tree gate remain pending at this checkpoint.


## Corrected A1 whole gate and review handoff

Executed source: `1907bd0ae1bf34ec81cb2ec15eeac74d035abed8`, including
A1's earlier checkpoints `2f169260` and `7a46c043` and the merged node
quality head. The final whole run started 2026-11-14 22:45 UTC and ended
23:52 UTC. One enclosing build slot held both testkit/daemon prebuilds,
all Rust steps and browser steps. Test data used a private mode-0700 SSD
directory, two Rust test threads and the unchanged test deadlines. Load
averages before/after were 18.21 and 12.02 (one-minute readings).

| Check | Actual corrected-tree result |
| --- | --- |
| Both test-binary prebuilds, format, Rust size/boundaries, full Clippy | Passed; no exception or boundary-map change |
| Full Rust workspace, including known shared side-question cases | 112 summaries: 1,963 passed, zero failed, 12 ignored |
| Generated TypeScript freshness | Two passed; no generated wire change |
| Web lint, fonts, typecheck, bundle | Passed; 132 font stacks checked |
| Web unit tests | 428 passed across 83 files |
| Fresh binaries after the new web bundle | Built before browser tests; changed web/node/CLI crates recompiled |
| Full isolated-fleet browser E2E | 63 passed, zero failed |
| Usage capture cases | 16 passed, zero failed; 32 content-only images |

Pre/post manifests selected 2,430 inputs and matched exactly:
`c410e4598060b1a193fbef3c0ef8f2b00afb753722b45b3d8085022bed539d44`.
All 2,261 tracked contents were independently compared with executed Git
blobs. The additional inputs include the private build configuration,
bundle files and the installation lock/link identity; external dependency
and toolchain caches, OS state and non-input artifacts are excluded.
This is selected-input provenance, not a supply-chain attestation.

Both final fleets served all eighteen selected static index-graph assets
with hashes equal to the freshly built bundle. Request-painted HTML is
explicitly excluded from byte comparison, as described in the diagnostic
section. Actual index references still determine the checked static graph.

All 32 final Usage images were manually viewed: English/Korean numeric
locale, light/dark, 1280×900, 390×844, 800×600 and 1024×700, each with an
overview and scrolled coverage view. Period/Refresh controls stay within
the viewport, reported zero and cost units remain visible, and the three
provenance dates use readable locale formatting. English labels remain.
The small-window coverage view scrolls inside the existing Settings
container beneath its sticky header. A read-only image audit recorded
zero OCR failures and no own-machine-name match. This does not certify
mobile cold-start, physical devices, contrast/accessibility or translation.

All ten recorded final fleet PIDs had exited; no owned runtime remained.
The private SSD test directory was emptied after execution. No live service,
permission, vendor/account, native source, deployment or push action took
place. The failed first whole run and its 427/1 frontend result, the 12/4
capture diagnostic and the original HTML-audit error remain adverse
historical evidence. They are corrected by this new changed-tree run,
not relabelled as successes or waived. Implementation review and any
combined-line gate remain separate from this result.


## Sidebar and viewport follow-up

Before extending accounting, two small page corrections follow as separate
commits. First, an ended conversation remains in the sidebar by default:
reuse the bridge's field-aware preference reader and the page's existing
filter rather than migrating or overwriting a stored choice. Missing
`sidebar.ended` means on; stored false stays false; archived remains out.
ADR 0036 records the reason. The second correction will address following
the timeline's end when the viewport and composer dock change together,
keeping the existing browser assertion and deadlines intact. The A1 review
handoff above remains tied to its immutable executed source.


The default-on correction's first focused frontend run exposed four old
live-only test cases that had relied implicitly on the old off default
(17 passed, four failed). Those four view arguments now explicitly choose
ended=false; their expected rows, order, names and assertions are kept.
Separate added cases verify the new default, retained off choice and
archive exclusion. The first bridge-unit run passed 54 and failed one
new assertion that searched pretty-printed JSON without accounting for
whitespace; it now uses that test module's existing compact comparison.
This was a new test's assumption, not a stored-false product failure.
The real-node browser flow and whole follow-up gate are still pending.


After correcting that new test's whitespace assumption, scoped bridge
Clippy passed, all 55 bridge unit tests passed, all 430 frontend unit tests
passed, and web typecheck plus changed-file ESLint passed. The added real
browser case checks ended visibility, acknowledged off, reload retention,
turning on again and archive exclusion. It has not run at this checkpoint;
the two corrections' whole gate and rendered evidence are still required.
The older collapsed-rail Settings/theme-mark clipping at 800×600 is a
separate later parity item; these corrections do not claim to repair it.


### Viewport/dock correction and reproducible ordering

The old watch recalculated end-following from the new gap on every scroll.
If layout's queued scroll arrived before ResizeObserver, a shorter view
made that gap look like a reader scrolling up. The resize callback then
saw following=false; row measurement only held a pin, and the layout
correction was also conditional on following. Nothing restored the end.

A synthetic React fixture using the actual timeline window reproduced
that ordering explicitly: after a simultaneous width/dock change and a
scroll delivered before resize, the old watch stayed 100 pixels from the
end, with its last line below the dock. A normal uncontrolled resize in
that fixture passed. This is controlled-order evidence, not a claim that
the historical eight-pixel failure was reproduced naturally. The staged
correction returned the controlled case to a zero gap, while an explicit
upward reader stayed at top=0 through another resize (gap 101 pixels).

Modify/reuse: keep the existing observers, virtual window, pin ownership
and scroll threshold. A pure geometry decision preserves prior end intent
across layout; the DOM follower uses it for both container resize and row
measurement. An upward move outside the existing near-end band still wins; a browser
clamp within that band retains end intent. Event/observer registration is split from that decision to
keep functions small and cleanup explicit. No CSS offset, timer, polling
repair, retry, new dependency or wire change is introduced.

The original phone question test is unchanged. Added pure cases cover
scroll-before-resize, later wrapped-row growth, genuine upward reading,
content clamping and the existing near-end threshold. The added actual
page case forces the callback order, then waits for a real resize delivery
to verify an upward reader remains put. Separate light/dark captures cover
the new ended default and the actual question dock at 1280×900, 390×844,
800×600 and 1024×700. These browser
cases/captures and the whole paired-correction gate remain unverified at
this source checkpoint. The first correction is committed as `3d888d1b`.


The viewport checkpoint passes all 434 frontend unit tests, including its
four added pure ordering/reading cases. An initial typecheck passed; the
subsequently expanded capture matrix and exhaustive DOM decision branch
are checked again by the pending bundle's typecheck and the whole gate.
Format, size, boundary and diff checks passed. Full web lint/build is
queued; no actual-node or rendered follow-up success is claimed yet.


The narrow full web lint caught two new static errors before the whole
queue: a geometry read discarded with a needless void operator and an
exported follower missing its explicit return type. The follower now has
a named interface. The actual-page fixture returns its measured geometry
and asserts a positive, shortened viewport with a pre-delivery gap beyond
the shared near-end threshold, making its controlled ordering meaningful.
The original phone assertion and deadline remain unchanged.


After those static corrections, all 434 frontend units, the complete page
and script typecheck, full web lint (including 132 font stacks) and the
production bundle pass on the final correction tree. The bundle typecheck
covers the final ten-image capture matrix and the explicit DOM decision
interface. No actual-node browser/capture or whole-gate success is inferred
from these narrow checks; the next run freezes both corrections together.


## Paired sidebar/viewport gate and review handoff

Executed source: `9a21b8df32bb7cb35b6d038ecba4e05b1a57049a`, comprising
`3d888d1b` (ended default), `6126d8be` (layout/end intent), and `9a21b8df`
(the explicit fixture geometry and follower return type). These remain
separate commits for their separate reasons. This is a page/bridge correction;
there is no node protocol, store schema, generated wire, dependency or native
source change. ADR 0036 records the default's shared behavior contract.

[Decision Log]
- Purpose and intent: retain stopped conversations until archiving and preserve a
  reader at the timeline end across viewport/dock layout.
- Existing constraints: an explicit off preference must survive; the virtual
  window owns pins and uses an existing near-end band; scroll may arrive before
  ResizeObserver.
- Alternatives considered: rewriting persisted preferences, resize-only
  correction, a timed retry, and a CSS offset.
- Choice: reuse field-aware preference defaults and existing filters; share one
  pure geometry decision across scroll, container resize and row measurement.
- Why: absent fields inherit the new default without replacing a saved choice;
  correcting the event-order decision fixes the lost end intent without a timer or
  changing dock geometry.
- Costs and consequences: automatic browser clamping inside the existing near-end
  band follows the end; an upward reader outside that band and a held pin still
  win. No new dependency, wire field or native behavior is introduced.


For the retention behavior, the pinned Paseo source is
`packages/app/src/hooks/use-aggregated-agents.ts:22–28,76–100` (archive-only
exclusion), with the sidebar's retained done bucket in
`hooks/sidebar-status-view-model.ts:6–16,27–53`. The aggregate hook is not
claimed to be the current sidebar's data source. This changes no row/card
look. The layout correction repairs Vessel's `978d0dc9` watch; it does not
claim to copy a Paseo DOM ResizeObserver implementation.

The four previously implicit live-only sidebar tests now explicitly supply
ended=false, as authorized: the two sidebarMachines cases, status grouping,
and keyboard order. A content audit proves those four call arguments are
that file's only changes; names, expected rows/order and assertions remain
unchanged. The entire original `conversation.spec.ts`, including its phone
case and deadline, is byte-for-byte unchanged.

The whole run executed 2026-11-15 01:09–02:48 UTC. It had been started before
the remote-gate amendment and was retained as instructed. One ordinary slot
held both A5 prebuilds and the complete gate, with four build jobs, two Rust
test threads and private mode-0700 SSD test data. One-minute load before/after
was 14.09 / 7.46. No test deadline, retry count or assertion was relaxed.

| Check | Actual result on the paired correction source |
| --- | --- |
| Both A5 prebuilds, format, Rust size/boundaries, whole Clippy | All exit 0 |
| Rust workspace | 112 summaries: 1,965 passed, zero failed, 12 ignored; exit 0 |
| TypeScript declaration freshness | Two passed; no generated change |
| Web lint/fonts, page/script typecheck, production bundle | All exit 0; 132 font stacks |
| Web unit | 434 passed across 85 files |
| Post-bundle binaries | Web/node/CLI compiled before browser execution |
| Full isolated-fleet browser suite | 65 passed, zero failed, including the unchanged phone case and both new regressions |
| Follow-up captures | Two cases passed, ten original images |

All stages, the source pin and the enclosing whole exit are zero. The pre/post
selection of 2,439 inputs matched exactly:
`61e829476e61d05fbe31058e0ea8ca4efbf99f235b0e53845d50c9996c39a6b8`.
All 2,270 tracked input contents match the executed commit's Git blobs.
The remaining selected inputs include the local build configuration, npm
installation identity and built bundle. External dependency/toolchain/OS
caches and non-input artifacts are excluded: this is selected-input evidence,
not a supply-chain attestation.

An independent narrow remote run of the same commit passed the original phone
case and the two new regressions, three passed / zero failed. It is additional
runtime evidence, not a second whole run. Future whole gates use the remote
tool; captures remain local. No unchanged whole gate was repeated.

Both whole-run fleets served eighteen static graph assets each with hashes
equal to the new bundle. The additional successful local console/capture run
also served the same eighteen assets. Per-request painted HTML remains
excluded from byte comparison. That extra run passed two cases and ten images,
with zero page/console errors in each case; its source and external-harness
pre/post identities matched. Its first harness attempt failed before test
collection because the temporary folder lacked ES module mode. That failure
and its logs remain retained; the corrected external harness is a separate
run, with no product-source change or whole-gate retry.

All twenty original images were personally opened after read-only PNG/OCR
checks: no OCR failure or own-machine-name match. These are short-name,
normal synthetic English states, light/dark: an ended conversation at
1280×900, and the question dock at 1280×900, 390×844, 800×600 and 1024×700.
The stopped conversation stays listed even when the machine's live count is
zero; the waiting line stays above the question dock. The separate browser
regression also proves saved off survives reload and archive removes the row,
and the forced ordering case preserves an upward reader.

These frames retain an existing transient kill toast over the lower composer,
and the known collapsed-rail Settings-mark clipping is not repaired. They
are not general layout, full translation, cold-phone-bootstrap, physical-device,
OS input-method, accessibility, native or cross-platform approval. The old
uncontrolled eight-pixel occurrence was not reproduced naturally; the controlled
old-watch failure and corrected ordering evidence are described above.

All fifteen recorded successful fleet process identities and eight owned helper
identities had exited; the failed external harness's five recorded PIDs had
already exited separately. No owned target runtime or private-SSD cwd remained.
Nineteen private SSD entries were removed, leaving zero. No extra cleanup
signal was needed; no live service or permission changed. No real credentials, vendor account,
live preview, deploy or push was used.

The focused 17/4 sidebar run, the 54/1 new whitespace assertion, the two new
static-lint findings and the failed optional harness remain historical evidence.
They are not relabelled as successes. Review and latest-node combined integration
approval remain separate from this clean bounded-source gate.

Resume: submit these two corrections together for review. The next ready usage
work is A2: first agent/profile selection, then a separately bounded session
summary/cursor slice if required by size. Both use existing usage.query only.
Account limits, inbox and project-wide PR enumeration remain gated by the exact
assigned contracts on a subsequently merged green node head. No new wire number
is introduced by this page track.


## A2a checkpoint: agent/profile scope

The paired correction stack passed independent source/whole/submitted-normal-
frame review and was accepted for those captures. Its live-count-zero ended
example is the light frame; the dark frame includes an earlier waiting
session. This does not extend the earlier native/platform/layout exclusions.

A2 is split at an actual seam: A2a is agent/profile selection; session totals
and cursor pages remain A2b. Reuse the typed call, one active plus latest
reader, existing agent identity/naming helpers and Settings Row/Select.
The node remains authoritative for every selected number. No new wire,
method, number, schema, dependency, native module or vendor access is added.

The catalogue holds only names/partial metadata from the last accepted
all-agent report on one machine. It is replaced, not accumulated, and is
cleared on a move, stop, disconnect or unmount. Choosing an agent does not
replace it with that filtered report. The UI explains that names come from
the last All agents report, so no claim that every name has current-period
usage is made. The selected historical name remains visible through an
empty later interval. The node's grouping limit can omit names; the partial
hint and accounting-coverage facts are retained.

Agent query profile=None is All profiles, including unnamed historical
reports. A reported group profile=None is not a name and does not become a
fake current-default or individually queryable option. Named profiles keep
their recorded names. Scope keys reuse agentKey; private queued requests
copy the scope and added-agent object. Render-time identity includes scope,
so another agent/profile's totals do not flash before the next effect.
No totals are added, priced or reconstructed in the browser.

The first narrow run passed all 442 frontend units (86 files), page/script
typecheck, then stopped at one new optional-chain lint finding in the test
observer. That expression is corrected without a lint exception; the failed
log is retained. The observer binds each sanitized usage result to its
request id/scope without retaining authentication frames or full messages.
The stronger typed-input fixtures and whole-state assertions remain part
of this checkpoint; no browser or whole-gate pass is claimed yet.

Added actual-fleet acceptance seeds default and named-profile ACP reports,
selects All profiles and the named profile, compares displayed input with
the actual query result, changes day/week and moves machine without stale
names/numbers. Captures use a valid 50-character profile in the fake node's
own profiles.toml, reload through the existing CLI and real retained reports.
Eight planned locale/theme/desktop/phone cases produce 24 views: full long
choice, selected scope and coverage. English labels remain intentional;
locale changes number/date formatting as in A1. Trigger and open-list labels
wrap inside this feature; no shared kit source is changed.

Before the next whole run, merge the announced green node head at a clean
checkpoint, preserving its protocol/schema and cases under the merge rules.
The next whole gate is remote; captures stay local. Actual browser/capture,
whole gate, source manifests, served assets, cleanup and review are pending.

Next after A2a, as one separate page-fix slice: narrow-screen toast placement
from Paseo; new-session autofocus behind an open dialog; Escape before the
dialog takes focus. The last two begin with failing page tests. The incoming
head's readiness waits remain unchanged unless their owner agrees to relax
them after the product fixes. The older collapsed rail clipping and A2b
session accounting remain later work; none is folded into A2a.


After the optional-chain correction, the second narrow run passes all 442
frontend units, both typechecks, full lint/fonts/controls/size and the
production bundle. Only documentation and a scope comment were updated
after that run; no production expression or test assertion changed.
This remains a source checkpoint: real-node browser flows, 24 capture
views and the remote whole gate are unverified. The green node merge
point announced during A2a is ec5b1162; no new RPC method is added here.


### Green shared-accessor head merged before A2a gate

The source checkpoint `31920bf7` is preserved. The uncommitted attempt to
merge the superseded ec5b1162 head was discarded before committing; no
source checkpoint or history was rewritten. Merge `62570304` instead brings
`f453ec2e`, including the shared method accessor and the accepted paired
page correction. It has no conflicts. All 332 incoming non-document entries
match that named head by mode/blob; README and ARCHITECTURE auto-merge both
sets of additions. Six native entries are incoming unchanged, not authored
or validated by this page track.

Incoming protocol/schema/constants, migrations, older-peer/method tables,
generated TypeScript and existing test cases remain unchanged. The root
sequence now also requires forge-fixture Clippy/tests; the remote plan
includes both, both nested prebuilds, TS regeneration and all web stages.
Format/size/boundaries and authored-delta diff checks pass. The abandoned
old-head merge's broad diff check reported inherited generated-TypeScript
trailing whitespace; no generated file was hand-edited to remove it.

Availability cells for limits, inbox and push are updated from rpc.rs's
assigned contracts; their page screens are still pending. Project-wide PR
values are staged with transport absent, so the previous watched-session
restriction remains. Diagnostics still has no assigned report RPC. No new
method or wire field is introduced by A2a, which uses existing usage.query.
The next step is actual scoped-browser acceptance on the committed merged
tree, then its first complete remote gate and local captures.


### A2a verified checkpoint: reported agent/profile selection

Executed source: `d34e86dbdb5f81f9057a8c30b1d73c29f22b075b`, after
source checkpoint `31920bf7` and merge `62570304` of the green shared-accessor
node head. A2a adds no Rust, native, dependency, RPC, wire field, protocol
or schema change. It uses the existing usage.query contract. The earlier
source-checkpoint paragraphs above are history, not the final run's result.

The complete remote run exits zero at every one of its 18 steps: bundle,
format/size/boundaries, default/TypeScript/forge Clippy, TypeScript generation,
fresh binaries, both nested prebuilds, workspace tests and every web stage,
then the forge fixture. Raw Rust summaries total 2,205 passed, zero failed,
12 ignored across 114 sections; the separate forge fixture passes four.
TypeScript generation/freshness runs four checks in two test sections.
Web lint/types/fonts pass, unit tests pass 450 in 87 files, and the entire
browser suite passes 66. Its run is
`webparity-20261115T043236Z-d34e86db`; start/end load is 6.55/10.15.
The remote tool retains the immutable commit/checkout and step logs; no
custom remote pre/post source manifest is claimed. The earlier narrow real
browser run on the same commit passes the new scoped flow and unchanged
A1 flow, two cases, before the first whole run. The first narrow frontend
lint failure remains retained as history; no exemption or deadline changed.

Local visual verification is a separate capture run, with a fresh bundle
and binaries: eight cases pass and produce 24 inspected original images.
They cover en-US/ko-KR number/date locales, light/dark, 1280x900/390x844,
and choices/selected/coverage views. A valid 50-character recorded profile
wraps in both the open list and selected trigger. All eight cases observe
zero page/console errors; 18 actual served static assets match the bundle.
Contemporary local pre/post selected inputs are identical: 2,607 inputs,
identity `a2ccd9cfda83bffde0c34c480a46e717f7bac70ba093d4b6cc9015760a67bc01`;
all 2,438 tracked contents match the executed commit. Dependency/toolchain
caches are outside that attestation. Five fleet processes and three helpers
are gone, and the private SSD test folder is empty. No live service, account,
credential lookup, permission change or deployment was involved.

These images show normal synthetic long-profile reports, after desktop
authentication and resize. English labels remain; locale coverage is not
complete translation. Rendered redaction/partial/error/offline/empty/many
or added-agent states, cold phone startup, physical devices, accessibility,
OS IME and native/combined integration are not verified by this checkpoint.
Source, whole-run and bounded visual evidence are submitted for review;
no reviewer approval or complete web parity is inferred here.

Work proceeds by feature package: narrow checks for intermediate commits,
one whole gate and one combined review/capture submission at package end.
The next package is the three assigned toast/modal defects listed above;
its focus regressions begin with failing page tests. Their source is not
part of this executed A2a checkpoint.

### A2a review correction: forget scopes on a machine round trip

Review found that the first implementation derived All agents on A to B
without persisting the new target. Returning from B to A restored A's old
named profile. The original green run and 24 images remain evidence only
for their immutable source; this correction does not reinterpret them.

The pure usageTargetFor transition forgets the scope whenever the machine
changes. The view persists that transition during render, before effects
can issue a request for the old profile. A later effect reset was rejected
because it could send that filtered query before resetting. The interval
is separate state and stays selected. The unit case covers A/profile to B
to A and stable-machine identity; the actual-page case now returns to A,
checks a machine-scope query, All agents and the retained weekly interval.
Narrow checks and the changed-package whole gate are pending here. The
following toast/modal test checkpoint is preserved in history and parked
with an ordinary revert, so its unfinished tests are not mixed into A2a.


### A2a corrected package verification

Executed correction: `4939635c789779ede57abf0ce14e3f4145241ce5`.
Narrow type/lint checks and 20 usage units pass. The changed-package run
`webparity-20261115T053111Z-4939635c` passes all 18 steps, including all
three Clippy configurations, both nested prebuilds, generation freshness,
all web stages and the separate forge fixture. Raw workspace summaries
remain 2,205 passed, zero failed, 12 ignored across 114 sections; forge
passes four, and TypeScript checks pass four across two sections. Web
units pass 451 in 87 files; the complete browser suite passes 66, including
the extended machine round trip and retained weekly interval. Start/end
load is 6.31/11.42. The rebuilt web, node and CLI are recorded by the run.

The extended browser assertion was also executed against the retained
original D34 product, deliberately using its old binaries and bundle.
It fails at the return to A: the observed scope is the old claude-qwen
profile instead of machine. The 166 selected bundle entries match D34's
contemporary capture manifest, and the binary hashes match its retained
runtime. This is an expected old-product counterexample, not a failed
corrected-source gate; that distinction is retained with the evidence.
Its fake processes are gone and the private SSD test folder is empty.

No appearance changed in this state correction. The 24 inspected original
D34 images keep their original source and normal-state limitations; they
are not relabelled as captures of 4939635c. No repeat unaffected matrix or
new visual/translation/cold-phone/native/combined claim is made. The remote
tool's commit/checkout and step logs identify this new execution; a custom
remote pre/post manifest is not claimed. The source return awaits review
closure; integration approval is separate.


## Feedback package: narrow toasts and modal ownership (test checkpoint)

Reuse the existing toast host, Base UI dialog and new-session lifecycle.
The reported problems are lower toasts covering the composer/question
buttons, a late-mounted new prompt taking an open dialog's focus, and
Escape before a dialog's initial focus. The focus cases begin as page
regressions before a product change. One holds only the real lazy new-page
asset until the modal owns focus; the other holds the initial-focus frame
and sends a real keyboard Escape. The toast fixture uses the existing
Copy session id action while a real fake-agent question remains open;
it checks the same toast host's separation from prompt and question buttons.
Its temporary clipboard-write permission belongs only to that disposable
browser context and synthetic id. No service or OS permission is changed.
The original test-only checkpoint is preserved, and then restored after
the A2a review correction. Its first baseline had two real toast geometry
failures (bottom 828 exceeds composer top 699), an incorrect assumption
that the first focused control was the paste field, and a passing ordinary
pre-focus Escape case. The focus fixture now checks ownership of the whole
dialog, waits for the new view to render and waits for route handlers when
releasing its held asset; the original failure is not called a product proof.

Separate old-product diagnostics confirm late-mounted prompt focus leaves
the dialog, and confirm a distinct early-Escape defect: the background
ShortcutRecorder's window capture listener prevents and stops the key before
the modal receives it. Its isolated recorder/palette page case fails before
any fix. Ordinary palette/menu/first-connected snapshot cases already pass;
there is no reason to add another Escape listener to the dialog. Temporary
frame control fixes the event order; these are controlled regressions rather
than a claim that every historical occurrence was naturally reproduced.
The three fixes, changed-package gate and affected captures remain pending.


### Feedback implementation checkpoint

`ui/page-focus` guards background draft focus and shortcut recording against
the kit's current modal. `use-draft-presence` preserves the draft lifetime
while moving the mount focus into that policy; the new-session file remains
within its budget. Base UI still owns dismissal, composition, nested controls
and focus return. The recorder's pending edit pauses instead of consuming
keys from a modal, including before initial focus. No new control or second
overlay stack was added.

The toast viewport is top-centred below the header across sizes, because a
wide lower-right toast can also reach the composer. Existing wide card width
is preserved; narrow width follows Paseo. The scrolling stack is bounded,
text wraps and shadow padding is retained. The browser cases cover 390,
800 and 1280 widths in both themes, individual clearance and eight actual
UI-generated messages with access to the last message by scrolling. Color,
status roles and notification lifetimes remain unchanged. Both typechecks, full lint/fonts/controls/size and the bundle pass. The first
frontend run also passes all 451 pure units; later edits only share browser
fixtures and hide the empty toast host. Corrected browser behavior, affected
captures and the complete package gate remain pending at this checkpoint.

### Feedback boundary correction and base test receipt

The first feedback package execution passed all nine new browser cases,
the pure units and its 24 captures, but its whole gate was red on the
existing history/security storage check: the sidebar painted before the
verified welcome stored its tab-session receipt. The separately reviewed
test correction waits for the same receipt as the existing page opener.
It preserves every original assertion, reload and replay check and adds a
bounded real-handshake delay test. No product authentication change,
deadline increase or retry is part of that correction.

Inspection also found a narrower toast defect: the initial card cleared
the header, but its scroll viewport began seven pixels inside that header.
A separate actual-page assertion fails at host top 44 versus header bottom
51. The host now uses the existing larger spacing token and its shared
browser fixture checks the host itself. Earlier images and runs remain
evidence of their original source; they are not approvals of this fix.
Affected captures and the changed-tree whole gate are still owed.

The assigned node quality base is merged without conflicts. All incoming
non-document changed blobs are preserved exactly, including generated
types and their formatting; both existing README and architecture texts
are retained. This page package adds no method, protocol or schema number.

### Additional short-phone acceptance boundary

The immutable `1a8a63ab` whole run passes every selected stage: 2,267 Rust
tests with twelve ignored, four TypeScript checks, 451 web units, 76
browser cases and four forge fixtures. Its local nine focus/toast cases
and twelve capture cases also pass. All 24 original normal-state images
were opened; their 2,644 selected inputs match before and after, and all
2,475 tracked contents match that commit. Its ten recorded fleet processes
and two observer/runner identities are gone; private fixture data is empty.

That evidence does not close the package. A subsequent actual 390 by 600
page probe finds the eight-toast stack intercepting the question's collapse
button at x328/y170 despite leaving Next clear. The earlier cases checked
Next and did not cover this shorter phone. The retained hit test fails on
that immutable product. The host now follows the actual prompt dock's
boundary, and new resize/fold/click cases guard the missed interaction.
Changed-source browser, captures and whole evidence remain pending.

The shared package base is merged after the earlier whole run completes;
all incoming changed blobs except the auto-merged architecture document
remain exact, including the incoming native base. This page correction
authors no native change or native approval. No earlier failure or normal
matrix is relabelled as evidence of the new clearance behavior.

### Final contract-base composition

The corrected feedback renderer is carried onto node head `ba45a6e3` in a
separate checkout while its local capture checkout stays frozen. The only
merge conflict is the architecture document: the head's process-mechanism
paragraph and the page-feedback section are both retained unchanged.
Exact incoming-blob claims apply to code and tests; the architecture, this
plan and ADR 0075 retain head text plus local page additions. The plan
and ADR match Git's automatic merge; the architecture conflict retains
both incoming and local paragraphs unchanged. Their full differences
against the head include earlier wording and status updates, so they are
not described as exclusively additive. The earlier audit used that overly
broad comparison and its shell continued after the failed check. A separate
merge-tree reconstruction verifies the actual resolutions and all 106
incoming non-document changed blobs. This corrects the evidence claim
without changing executable code.
Captures still identify their original renderer commit; the merged tree
requires its own whole gate, and no rendered latest-head claim is inferred.


### Menu ownership follow-up

The latest-head whole run on `774f41b8` is red: 2,307 Rust tests pass
(twelve ignored), four TypeScript checks, 464 web units and four forge
fixtures pass; browser cases are 76 passed and two failed. The unchanged
Display grouping case loses its Machine menu item while clicking, and the
following reconnect case cannot find its initial Connected mark. Neither
failure is waived, and no head assertion, deadline or retry is changed.

A new controlled browser test against the retained renderer first puts
keyboard focus inside Display, then releases the actual lazy new-session
asset. The menu stays painted but loses focus. The modal-only guard was
incomplete; the same public role/open boundary now includes menus. The
regression shares the existing real lazy-route harness with the dialog
case. This proves a menu ownership defect, without claiming it alone
explains both intermittent whole-run failures. Changed-source checks and
whole evidence remain pending.


### Feedback package executed result

Executed source `dac4965996491e214bc6f5391df6cce8e012359e` passes all
eighteen whole-gate stages on the merged contract base. Independent raw-log
sums are 116 Rust summaries, 2,307 passed / zero failed / twelve ignored;
two TypeScript-generation summaries total four passed, with no diff; 464
web unit tests, 79 browser cases and four forge fixtures pass. Format,
boundary/size checks, all three Clippy modes, lint, typecheck, fonts and
fresh bundle/binary builds pass. Both fixture prebuilds precede tests in
the same environment. The changed web, node and CLI crates actually compile.
A separate six-case browser run passes the dialog/menu/early-Escape/recorder
regressions and the unchanged Display and reconnect cases.

The new controlled menu counterfactual fails on the retained old renderer:
Display first owns keyboard focus; releasing the held lazy page leaves the
menu painted and activates Prompt instead. Its frozen source and asset
record are retained. The corrected whole run closes the current gate; it
does not erase the earlier security-receipt failure, the short-phone hit
failure after the earlier green run, the capture-title collection failure,
or the two `774f41b8` browser failures. Existing head assertions, deadlines
and retries remain unchanged.

The visual evidence is deliberately separate. Sixteen capture cases produce
32 originals across en-US/ko-KR, light/dark, and 1280x900, 390x844, 390x600,
800x600, showing a normal copy toast and a scrolled stack. Every original
was opened and rehashed; all sixteen capture cases check horizontal
overflow, page errors and console errors. Their renderer
is `7e3418d8`; harness `37a723ba` changes only the duplicate viewport title.
All 3,385 selected inputs match before/after; 3,216 tracked contents match
that harness commit. The eighteen served static assets match its bundle.
That matrix proves its submitted short-text states, not a new latest-head
render or a rendered long error, permission card, cold phone bootstrap,
physical device, IME or accessibility audit. Browser locale varies; the
existing English UI labels are not a complete Korean translation. The toast
renderer and tests
remain unchanged by the later menu-only correction.

Five capture fleet identities were recorded and are gone. The two capture
helper launch snapshots were missed; only current PID absence is claimed
for those helpers. Current owned-runtime/browser-profile scans are empty;
private fixture data is empty with the original owner and mode 0700. The
final cleanup sends no additional signals after normal test teardown. No
live services, OS permission changes, real agent prompts, real accounts,
vendor account calls, native authored changes or deployment are part of this
work.
The remote whole is identified by its immutable commit, checkout/spec and
logs; no bespoke remote pre/post source manifest is claimed.

### Resume point after feedback

Done: the feedback source above and its full gate; review submission carries
the exact source, historical adverse evidence and original capture scope.
Review/integration approval is separate from this author's executed result.
No new node method, field, protocol or store schema is authored here.

Next: the assigned F3 web usage package, preserving reported accounting and
context when external lookup is off. The source owner's report/inspect/
source-consent calls remain pending until a reviewed, assigned node head
contains them. Do not consume drafts, invent a number or simulate working
quota data in the browser. Same-binding success, refresh origin, explicit
source disclosure and unsupported session associations must remain typed.
Pure formatting/time preparation has not been installed into this tree and
is not part of this package's gate or review.


### Returned narrow-width correction

Review of `dac496599` finds one remaining reference mismatch. Paseo's
`components/toast-host.tsx:283–295` applies 16px container insets before
its 92%/480px width boundary. The earlier viewport-relative width measured
358.796875px at 390px; a new real-page counterfactual expects the literal
329.36px and fails on that retained renderer. This is a porting error,
not a deliberate difference. The narrow card now uses the inset-relative
formula, with the literal measured assertion shared by its browser and
capture cases. The declared existing 320px wide-layout choice is unchanged.

The earlier whole result and inspected images remain evidence of their
original source. Corrected-source checks and the affected narrow image
states are owed before this return closes; they are not inferred from the
old gate. The correction is isolated on the exact review candidate rather
than the next feature's moving base.

### Narrow-width correction: executed result

Source `811c18d53a8443fe94e3a0aa6817e61b9be99fb9` fixes the returned width.
The independent literal is 329.36px at a 390px viewport; the retained old
renderer fails it at 358.796875px. No existing assertion, retry or deadline
is weakened.

Whole-web run `webparity-20261115T143312Z-811c18d5` passes eight stages:
fresh bundle/binaries, both fixture prebuilds, lint, both typechecks, 464
units in 87 files and fonts. Its full E2E remains 78 passed / one failed.
All twelve feedback focus/toast cases pass. The sole timeout is the
unchanged Usage scope test at `openUi`'s verified-tab-receipt wait, before
any usage seeding, navigation or scope assertion. Its retained DOM reports
“The UI was restarted or its identity could not be verified”. This is an
actual identity-verification refusal, not a merely slow receipt.

The explicitly authorized single same-source replay,
`webparity-20261115T153059Z-811c18d5`, passes all five stages and that exact
case: 7.0s for the case, 26.3s including the fixture. It rebuilds the same
source's web/CLI and runs both prebuilds. Whole-run load is 34.06 at start,
40.43 at end; replay load is 35.07 at start, 35.26 at end. Acceptance counts
this one named intermittent under the explicit ruling; neither the first
failure nor its observed refusal is erased. A second unchanged replay is
not permitted. The investigation below stays open whatever that pass shows.

The source differs from the preceding executed feedback source only in the
stylesheet, shared feedback helper and two documents. Rust, manifests,
lockfiles, toolchains and gate scripts are byte-identical. The prior exact
2,307 passed / zero failed / twelve ignored Rust result is retained for that
unchanged source under the changed-web verification scope. It is not a new
Root run; latest-line combined integration remains separate.

A successful managed export builds the exact source's CLI, node and eight
fake executables. Source/features/recipes, hashes, sizes and owner/modes are
verified before use; the modified web and CLI crates actually compile.
Eight real-page capture cases pass across en-US/ko-KR browser locales,
light/dark and 390x844/390x600. All sixteen normal/stack original PNGs are
opened and rehashed. Width, header/host clearance, prompt/question-button
clearance, horizontal overflow and page/console errors are checked. The
stack stays bounded above the question. This supersedes only the earlier
narrow-width visual claim; earlier wide frames retain their original source.

All 3,239 selected capture inputs match before/after, including exported
binaries and their manifest; 3,235 tracked contents independently match Git.
Local capture performs no Rust compilation or bundle build. Eighteen served
static hashes are observations tied to the verified CLI, not an inferred
comparison with local dist. Both helper exe/start identities are recorded
immediately at launch, as are five runtime identities after setup. All seven
are gone, the owned-runtime scan is empty and the private mode-0700 fixture
root is emptied by its original owner after normal teardown. No additional
signals or live-service permission changes are made. Completed export/
log OUT top directories are restored to private 0700 without changing
source, log or binary bytes.

The older scratch counterfactual pins tracked helpers and the old bundle,
but excludes its external spec/config files. Its raw measured failure is
retained without a stronger executed-test identity claim. The corrected
actual-page test and config are tracked and pinned before/after. The earlier
helper-snapshot limitation and all earlier adverse results remain history;
this new capture does not repair their historical provenance.

Browser locale is not complete UI translation. Desktop authentication then
resize is not cold-phone bootstrap. No physical device, long-error or
permission-card rendering, OS IME, accessibility, native app, live vendor,
real account, deployment or combined latest-line approval is claimed.

### Web UI identity refusal investigation

| Investigation | State | Evidence | Next |
|---|---|---|---|
| WEB-IDENTITY-1: intermittent web UI identity refusal under build-VM load | Open; cause unknown. Investigate after the feedback package; no further unchanged replays. | Source `811c18d53`; whole `webparity-20261115T143312Z-811c18d5` (78 passed / one timeout); authorized single replay `webparity-20261115T153059Z-811c18d5` (one passed; 7.0s case, 26.3s including fixture). Error-context SHA256 `d5667e687c89c749f2099486637274d897e513c613cc08343de913e0494fb783`. | Follow the one-use bootstrap, challenge/hello/welcome proof, socket refusal/drop and receipt-persistence paths from source; identify the cause before changing behavior. |

The retained DOM says “The UI was restarted or its identity could not be
verified”. This is an actual identity-verification refusal, not evidence
that the receipt was merely slow. The helper times out waiting for the
verified tab receipt before any usage seeding or scope interaction. The
helper and usage test are byte-identical to the preceding green feedback
source. High build-machine load is observed; a causal explanation is still
unverified. The approved error context is preserved privately without
frames, capabilities or stored credential values.

The authorized replay is evidence for the package's named intermittent
only if it passes. It never erases the original failure or closes this
investigation. If it fails, the package remains blocked and no further
unchanged replay is permitted. Subsequent work traces the receipt path and
uses a controlled regression from the confirmed cause, not repeated
unexplained browser runs. This investigation precedes the next usage-page
implementation; the new account-report contract remains gated on its
reviewed, assigned and merged node source.


## F3 account usage: presentation skeleton

The prior auth timeout correction is integrated; its final bounded runtime
review uses the existing integrated whole with the named open `/new`
intermittent. The next assigned web package is the Usage parity consumer.
Its report/inspect/source-consent calls are not yet reviewed, assigned and
merged. They remain inactive; no provisional protocol number is invented.

The first implementation is pure presentation under `data/account-usage`:
finite clamped percentage, independent used/remaining preference, reported
source tone, separately formatted balance units, validated instants, reset
versus exhaustion states, and successful snapshot age. Source references and
reuse decisions are recorded in ADR 0075. The files do not define a wire DTO,
register a UI, read credentials, fetch a provider or infer account association.
Tests cover unknown versus zero, finite bounds, source thresholds, locale
formatting, two-unit floor/boundaries and retained past risk. Their execution
is recorded at the implementation checkpoint; no feature gate or visual pass
is claimed by this skeleton.

After the assigned source is ready, add its typed adapter and a selection-owned
bounded reader, preserving exact machine/session/report binding. Reuse the
existing Settings and kit controls for cards, source consent, modal/sidebar
summary and session quota notice. Cached Read never becomes vendor polling;
Refresh is explicit and distinct from a foreign cached Refreshing snapshot.
Global Off clears external-account presentation while reported ledger/context
stays available. Host errors retain complete data when present; agent errors
hide partial account data. A per-report failure retains the original successful
timestamp and does not retain past risk or summary eligibility.

Use the managed real-node fixture after exporting the exact reviewed, assigned,
merged combined source. Seed a success before holding its actual reader; always
release the synthetic hold before shutdown. No browser-authored quota payload,
old-backend/new-page mixture, real sign-in or vendor probe substitutes for this.
The one package whole and affected real-browser matrix come after the consumer
is complete, not once per small presentation commit.

Initial pure presentation checks: two test files, eleven passing tests; both
TypeScript configurations and the changed-file ESLint pass. Prettier and the
diff check pass. These narrow checks compile no Rust and are not the one
feature-package whole, a screen capture, or a report-consumer runtime test.

The shared value is then extracted to dependency-free `quota-percent`, with
its unchanged data API re-exported and a leaf import rule. Its constructor
body is byte-identical; the clamp test and infinity assertion move with it.
The first refactor check catches one unused moved-test import; removing that
import yields both typechecks, changed ESLint and all eleven tests passing
across three files. The original failed check remains recorded. No screen,
wire adapter or package whole is activated by this structural change.
