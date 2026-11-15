# New sessions and workspaces

Status: skeleton in progress. This package replaces the former handoff
assignment; `wp-a4.md` records why that assignment did not match Paseo.

## Reference and existing assets

Paseo 0.15.4 source inspected, paths relative to its source tree:

- `packages/app/src/composer/draft/create-flow.ts:25-81,203-248`: editing /
  creating states, validation before dispatch, duplicate-submit protection,
  retained attempt on failure, and handoff of the first optimistic message.
- `packages/app/src/composer/submission/writer.ts:66-72` and
  `packages/app/src/stores/session-store.ts:1188-1227`: the first user row
  moves to the newly created agent, rather than switching an existing agent.
- `packages/app/src/composer/draft/workspace-tab.tsx:118-193,468-495`:
  create with provider/model/options, draft idempotency key and first message.
- `packages/app/src/screens/new-workspace-empty.ts:5-36`: empty text with
  no attachments creates a workspace without an agent.
- `packages/app/src/screens/new-workspace-terminal.ts:39-109`: ensure an
  empty workspace, launch the selected profile or default shell, substitute
  prompt into profile argv, or type the blank shell's command followed by CR.
- `packages/app/src/new-workspace-launch/target.ts:9-59`: Chat default,
  blank-shell choice, removed profile falls back to Chat.
- `packages/app/src/new-workspace-launch/composer-state.ts:51-100`:
  no-placeholder profiles display the resolved command read-only.
- `packages/server/src/server/agent/create-agent/create.ts:156-214`:
  registration commits creation; first-prompt failure belongs to that agent.
- `packages/protocol/src/terminal-profiles.ts:11-34,45-104`: prompt sentinel,
  default agent profiles, all-occurrence substitution and empty prompt-only
  argument omission. More server receipt and form references are recorded
  with their implementation before those parts are filled in.

Existing native assets: draft provider/model/mode controls, project/local/
worktree/base pickers, attachment tray/pump, worktree approvals/setup walk,
optimistic conversation, typed session/terminal/project calls, Fluent catalogs
and the kit. Reuse those; do not create another composer, picker, terminal
renderer, attachment pump, workspace registry or layout model.

## Decision log

- Purpose and intent: one reliable creation flow for empty workspaces, chats
  and shells, with retained input and navigation-safe asynchronous completion.
- Existing implementation and constraints: the draft handles only chats;
  direct session creation and first-prompt delivery share a generic failure,
  permitting duplicate sessions after a prompt failure. Workspace identity,
  receipts and terminal profiles need additive node work.
- Alternatives considered: treat cwd/worktree/session as workspace identity;
  launch terminal profiles as harness sessions; reuse the existing explicit
  registry, terminal and draft boundaries.
- Selected approach: **modify existing assets**, add a pure typed creation
  state machine and correlated link boundary, then extend creation on the
  node using the registry of ADR 0082 and existing session/terminal actors.
- Why: stable ownership cannot be inferred from cwd; starting an agent is
  distinct from delivery of its first prompt; a completion cannot steal a
  route after the person leaves.
- Advantages, costs and effects: existing controls and runtime ownership stay
  authoritative; asynchronous creation needs receipt persistence and typed
  reconciliation. No new external library/API is planned.

## Structure and ownership

- `view/src/creation/`: correlation tickets, exhaustive phases, stage-specific
  failure/outcome-unknown state, submission kind and navigation lease.
- `link/src/creation/`: typed commands/events, existing-call orchestration,
  deadlines and in-memory transport tests. New node methods wait for the
  central pending-method table and assigned protocol number.
- `app/src/areas/draft/create.rs`, `status.rs`: actions and rendering; extend
  existing `draft` state and reuse project/place controls.
- `view/locales/{en,ko}/creation.ftl`: all new labels and explanations.
- Node package: creation receipt contract/service/store and terminal profile
  values/settings/launch. Independent node worktree starts at `d954fa3a`.
- ADR 0082 owns `WorkspaceId`, registry snapshots, resource membership and
  idempotent empty local/worktree provisioning. Creation reserves the workspace
  id before invoking that API; it does not persist a second registry.
- Layout owns opening `Target::Terminal`, terminal area owns its resident
  store/follow, attachments owns prepared payload/tray handoff, project owner
  owns registry persistence. Shared shell/link files get registration only.

## Differences and compatibility

Vessel persists in its existing SQLite store rather than Paseo's JSON files.
Receipts must retain the same key/intent/phase/outcome-unknown behavior, not
the reference's storage format. The node controls filesystem, shell and
approval authority; the app never executes a configured profile itself.
No pending call is advertised before the integrator assigns its protocol.
Older peers receive an explicit unsupported result, never a fake success.

An absent receipt after reconnect is different from an interrupted in-flight
side effect. Only the former can start a new operation. A registered agent
survives a first-prompt failure; retry must not create a replacement agent.
The local state skeleton provides these distinctions while the additive
workspace/provisioning contract is being completed.

## Verification and resume point

Required tests: every pure transition, duplicate submit, stale/out-of-order
events, completion after leaving/reentering, known refusal versus uncertain
outcome, input preservation, profile substitution and bounds, receipt intent
conflict/restart/concurrency, first-prompt failure after registration, empty
workspace and each launch target. Fakes only; no real agent or preview.

Native base8d5513e4 is merged. Pure Ticket/Flow tests3823224d passed five cases;
initial app/link/view skeleton Clippy found five import/qualification issues,
now fixed for the next narrow check. This is not a whole package gate.

The staged W6 range througha01781ed is integrated for combined checks, not
claimed ready. New-session submission now captures a Ticket, options and
placement before dispatch; worktree preparation delegates resource creation
to the same link boundary. Registration is retained before first input,
known first-input retries address that SessionId, and unknown outcomes lock
resubmission. Ticketed file completion runs even after navigation; route
ownership is relinquished by navigation boundaries. No attachment producer
module is copied or edited by A1.

Still owed: combined compilation/regressions, stable node identity pin,
multiple seeded drafts using W12's DraftId, registry-backed empty workspace
and terminal flows, receipt reconciliation, terminal profile selection,
whole gates/captures/reviews. The node receipt/profile work is separate in
ADR0091. No full parity claim or user-flow capture exists yet.

Shared base6036eeb8 is merged. Creation captures `Machine.node` and refuses
before worktree/session effects if the connected welcome identity differs;
late completion also requires the same name/node mapping before handoff.
This guards A1's boundary; W6 is separately pinning capability/upload calls
and adding `prepare_draft_files_on`, so a later queued upload cannot retarget
a reused book name. The current Ticket-only adapter remains transitional.
`Editability` locks the actual draft editor while creating/unknown, retaining
copy/selection. Known initial-input failure offers an explicit same-session
retry. In-memory peer regressions assert identity mismatch sends no request
and a retry reads the registered session rather than issuing create again.

`view::creation::Launch` now distinguishes empty workspace, chat, blank shell
and configured profile plans, using the reviewed pure profile values5c39da5f.
Blank shell input is trimmed and followed by one CR (reference terminal.ts
100-103); profile argv is substituted literally and never typed as shell
source. A missing persisted profile visibly reconciles to Chat before submit;
a removal racing preparation is refused. Terminal plans refuse attached chat
payload instead of dropping it. These are pure plans, not working terminal RPCs.

The shared DraftSeed values carry explicit Name+NodeId, NewWorkspace project
context or authoritative ExistingWorkspaceId, canonical bounded ContextText
and W6 ChatHistory. Approved identitye45a5d89 was cherry-picked asd24486f1;
no registry/store/runtime producer was imported. The seed has no duplicate
UUID: W12's DraftId will be the seed_draft parameter when its module lands.
No seed opener or multiple-draft renderer is claimed consumer-ready yet.

### Top-level workspace intake skeleton

The new `Route::NewWorkspace` and `open_new_workspace(NewWorkspaceSeed, ...)`
accept a captured `Name`/`NodeId` before navigation. A busy draft (including
whitespace, retained/suppressed context and an uncertain submission) refuses
replacement. History moves through W6's canonical context intake; the form
retains placement/launch selections only. Selected project roots stay exact,
without client-side canonicalization. This route precedes workspace creation,
so it cannot require W12's existing-workspace LayoutKey.

This is a staged intake skeleton: its submit path reports unsupported while
W11's checked provision/start-lease driver and the creation RPC are pending.
It does not turn an empty workspace or a shell into a legacy chat. The
legacy chat consumer now handles the context preparation refusal exhaustively.
Existing-workspace tab seeding and working workspace/terminal submission are
still owed. No whole gate, capture or runtime completion is claimed.

The canonical ClientMessageId (W23's standalone checked values) is allocated
once before attachment/context preparation, retained by the creation ticket
and carried by the link Ask and W6 PreparedSubmission. A known first-input
retry keeps that identity and SessionId; a new ticket receives another id.
The legacy text transport still lacks submit_prompt activation, so retaining
this id does not claim server-side prompt deduplication. Pending local byte
acquisition/recovered uncertainty disables submission before preparation.

The workspace form now has separate Chat/Blank terminal controls and a
localized Create workspace action that also accepts an empty form without
requiring an agent. Changing the launch target retains all text/context.
Configured profile selection waits for the authoritative catalog; no local
empty catalog is advertised as the server's settings. The shared composer
renderer moved to draft/draw.rs to preserve the file budget; its existing
four literal allowances moved with the unchanged drawing, without increases.
Submission still reports unsupported while the registry driver is pending.

Legacy creation Internal errors are outcome-unknown rather than a proven
refusal. Existing node sessions::start launches/registers the process before
store.create_session and can return a generic store error afterwards; cleanup
cannot prove that the process had no external effect. The link retains the
original draft and blocks replacement/replay in that case. Duplex-peer tests
cover Internal at registration and at first-prompt delivery, with no success
or automatic repeated create. Typed pre-effect refusals retain their existing
retry behavior. Durable reconciliation still waits for the new receipt driver.

Known first-input retry also verifies the originally admitted NodeId retained
with the registered card. Reading a fresh Fleet identity cannot rebind an old
SessionId after an alias is reused for another node. A regression preserves
both original identities and the canonical message id after that refusal;
retry on the original destination reads the same session rather than creating.

### Scoped editor producer (staged, not consumer-ready)

`draft/workspace.rs` owns bounded editor content under `(LayoutKey, DraftId)`;
W12 owns presentation under `(LayoutKey, TabId)`. Its two exact read accessors
return that draft's TextField and optional ContextScope, never the global New
field. Scope stays absent until authoritative registry backing is available.
Target::Draft registers only through W12's existing exhaustive renderer seam;
a background panel makes its editor read-only rather than claiming input.
Seed acceptance does not navigate/open layout, so a later open refusal retains
the accepted editor. Existing identities refuse reseeding; capacity refuses
without evicting another editor. Isolated shell tests cover these boundaries.

W6's staged keyed payload bridge now accepts history into a newly admitted
empty workspace editor, then installs its initial text. Refused history intake
rolls back only that new editor and preserves other drafts. History requires
no authorized file root; file scope remains explicitly absent. Workspace
seeding does not request focus before W12 opens the tab. Scoped submission
still reports unsupported without clearing text until the registry/creation
driver is available. The seed method's temporary dead-code expectation must
be removed with W2's actual action consumer; this is no runtime readiness claim.

The link's Ask and every Answer now carry the canonical DraftKey in addition
to the local Ticket. Per-draft counters may legitimately allocate the same
ticket, so node pinning alone cannot distinguish their payload owners. The
legacy New consumer ignores other keys; its worktree adapter preserves the
New key. Duplex tests check the captured source on every reply and a shell
regression prevents another draft's equal ticket from completing New. Actual
workspace dispatch/settlement awaits W6's keyed preparation bridge and the
reviewed workspace runtime; this correlation change performs no new RPC.

The legacy placement walk's generic failure can follow a successful worktree
creation or setup command. It no longer converts such node/transport failures
into a known creation refusal permitting another worktree. The flow retains
its payload and reports outcome unknown until durable reconciliation exists.
Only an unsent ask or protocol preflight remains a no-dispatch refusal; an
explicit user cancellation keeps its existing stop behavior. This conservative
difference avoids blind duplication without claiming receipt recovery for the
legacy path. Pure transition tests cover both classes.

Blank-shell planning reserves the trailing CR within the node's existing
SendTerminal byte cap. An otherwise valid maximal Prompt must be refused
before provisioning rather than creating a shell whose initial input would
then exceed the transport limit. Boundary tests compare the complete encoded
input length against the same protocol constant.

The workspace renderer now delegates Chat to W6's scoped card with the exact
DraftKey and owner editability, including attachment/context pills and captured
paste/drop handlers. Background or attachment-blocked owners make the same
retained field read-only. Blank/profile terminal targets retain the plain field
and suppress agent attachment affordances. B2's font/send/fullscreen policy is
a separate pending producer; no global New field is substituted. The scoped
card's two temporary unused-item expectations are removed with this real caller.

### Original connection admission for creation (staged)

[Decision Log]
- Intent: keep a captured create/first-input/worktree intent on its original
  authenticated admission, including same-node reconnect and credential change.
- Existing assets: G4 already owns Machine.connection_context(), the actor's
  selected_connection(), and the before-poll/after-await typed RPC guard.
- Alternatives: NodeId alone misses reconnect retirement; another epoch/map
  would compete with the shared connection owner; no external API is needed.
- Choice: reuse that exact context/SelectedConnection through the whole legacy
  creation walk, and retain it beside the registered address for W12's handoff.
- Effects: actor preflight refuses before dispatch; retirement during an awaited
  call becomes outcome unknown with no replay. Protocol-9 prompt fallback is
  allowed only after typed local NotInProtocol, never a remote refusal.

Workspace actor entry moved out of the link root into workspace/entry.rs.
Existing non-create routing is unchanged; Start asks require the captured
admission, and the old raw-client Hub path refuses them. Every placement call
uses the selected guard. Retry GetSession results must name the exact old
session before first-input delivery. Isolated actor tests cover a replacement
admission on the same node and retirement after selection before first poll.

This does not establish final queued-delivery validity or a safe resident follow.
W21's standalone started_at proposal remains held pending W12/G4's live slot
and delivery capability; the app has not introduced a second resident registry.

The exact scoped mode accessor reuses B2's canonical surface::Mode leaf:
Chat maps to Chat; blank shell and configured profiles map to Terminal.
Resolved read-only profiles remain terminal mode. The renderer consumes this
same getter; missing node/workspace/draft identity yields None without a
global selected-agent fallback. This is presentation policy, not a root/config
or connection authority snapshot. B2's remaining controls use this seam.

Legacy Unavailable is also uncertain after dispatch: run_structured checks
shutdown after AgentProcess::spawn_with, drops the child and returns
ShuttingDown/Unavailable. That cannot prove the child executed no code. The
creation boundary conservatively retains the same payload for this code,
with duplex tests at registration and first-input stages. Generic outcome
wording covers both a live transport refusal and a disconnect; it does not
assert that the connection necessarily ended. Durable before-effect refusal
receipts will provide a narrower retry distinction in the new API.
