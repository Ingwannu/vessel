# Native projects and workspace parity

## Purpose and references

The native client must preserve host/project/workspace identity across navigation,
async mutations, setup and recovery. Reference: Paseo 0.15.4, commit
`1c00b365c9e0b74c1580eba33d905d693125ae21`; implementation is independently written Rust.

Read source evidence (paths relative to Paseo `packages/app/src`):

- `add-project-flow/model.ts:29-85,135-146`: host-bound page stack, preferred
  or sole host, back navigation and per-location drafts.
- `hooks/open-project.ts:80-114`: registration calls addProject only; no
  workspace or agent is created by registration.
- `screens/projects-screen.tsx:24-80,109-116`: host-specific rows, including
  projects without workspaces, exact host/project navigation.
- `workspace/workspace-archive.ts:82-135`: optimistic hide with rollback and
  per-target bulk failures. It does not equate workspace with a Git worktree.
- `panels/setup-panel.tsx:168-204`: running/last log auto-expansion with manual
  collapse precedence, carriage-return normalization and selectable output.
- `screens/new-workspace-screen.tsx:153-165`: first-agent naming context is
  distinct from starting that agent.
- Survey `p1-projects-workspaces.md`, sections 1-17 and 32-33 supplies the
  remaining source paths. Survey is source evidence, not runtime verification.

## Existing assets and decision

[Decision Log]
- Purpose and intent: extend native projects without a second registry or a
  second filesystem authority.
- Existing implementation and constraints: `view/workspace/projects.rs` owns
  cached project lists; `app/areas/workspace/projects.rs` has registration and
  host settings rows; `place.rs` and `link/workspace/start.rs` start sessions
  in projects/worktrees. `setup_log.rs` already decodes paged setup output.
- Alternatives considered: replacing these areas, generic file writes for
  settings, deriving workspaces from sessions or paths, adding a new library.
- Choice: modify existing screens, add pure typed flow state beside projects,
  and add typed link adapters only for existing RPCs. No dependency additions.
- Why: the node remains the only project/placement authority. File writes
  cannot reproduce the missing config authorization/revision contract, and
  sessions/worktrees cannot substitute for independent workspace identity.
- Advantages, costs and effects: small registration seams, explicit stale
  response rejection and reusable views; full parity waits for node contracts.

## Ownership and data flow

`view/workspace/projects/` owns pure navigation and operation state;
`link/workspace/project_requests.rs` adapts C1 project calls with the shared physical admission, bounded deadlines and correlated observations; `project_flow.rs` retains the older adapter for isolated legacy tests;
`app/areas/workspace/project_form.rs` owns registration form interactions.
The host project cache retains both its read ticket and the shared admission that produced it. The app submits an operation with its original ConnectionContext; link selects that exact physical client and echoes the context. The app accepts hydration only for the current admission and a matching ticket. Drawing uses kit controls and prepared EN/KO Fluent words.
No new crate dependency edge, local filesystem write, shell command, or node
configuration parser is introduced.

Scripts, service ports and command approval rendering belong to their existing
owners. `vessel.toml` parsing, approved digest and automation security remain
node-owned. Shared shell and link files receive registration only.

## Feature contract map and explicit differences

| Reference feature | Existing node contract | Implementation boundary |
| --- | --- | --- |
| Register existing directory; clone | project.add/clone/get/list | Extend host-bound form and preserve progress |
| New directory without Git | project.create_directory is staged on the separate C1 node tree; unreviewed/pending | Native flow still hidden; existing Git-init action retains explicit name |
| Suggested projects | None | Hidden; do not scan user provider homes |
| Rename and icon | None | Hidden; no client-only override pretending to persist |
| Remove project | project.remove | Confirm concrete host and project; node refusal remains visible |
| Project settings | Existing project.commands/approve; project read/write config is staged on the separate C1 node tree | Existing command view; revision editor awaits the reviewed config producer and physical-root exclusion |
| New workspace, local/branch-off/checkout-branch | session.create place + worktree.create | Existing new-chat flow extended; do not claim independent empty workspace support |
| Checkout PR | No worktree PR mode | Hidden pending node contract |
| Setup logs | worktree.output/setup | Reuse paged log and explicit approval; no trust-release equivalence |
| Workspace trust release | workspace.release_automation is pending in registry candidate1057336b; not yet reviewed/integrated | Hidden until the producer is available; releasing the gate does not approve or run commands |
| Archive/bulk archive/unarchive/recovery | Canonical WorkspaceId is integrated; registry/store candidate1057336b awaits V3, while W21 owns resource lifecycle | Actual adapter remains blocked on reviewed registry and generation/root safeguards; worktree.remove is never renamed Archive |
| Auto-archive after merge | No setting/call yet | This package owns the missing node design; hidden until authoritative capability exists |
| Automatic title and branch names | Pending node feature | Display authoritative names only; no app-generated branch rename |
| Host projects home | project.list and existing settings host selector | Retain host-specific empty/loading/error/populated states |

Contract availability is scoped to the app line versus separate staged node work. Missing runtime contracts were sent to the coordinator and their node owners;
no fabricated call or protocol number is introduced.

## Validation and security

Pure state tests cover duplicate submissions, host switches, stale replies,
removal cancellation and transport failure. Link tests use in-memory fakes;
never real agents, provider credentials or live nodes. Narrow checks per commit
and one complete native gate/capture matrix at package end. No push.
Runtime service ownership/permissions are unaffected: this package edits local
source only and does not deploy to a running service.

## Resume point

- Completed: reference and existing asset map; node contract gaps confirmed;
  ownership coordinated with scripts and node workspace tracks.
- In progress: typed flow skeleton and supported project interactions.
- Next: narrow checks, connect supported interactions, merge announced app base,
  captures and native gate when the package is reviewable.
- Blocked: full workspace lifecycle, project revision editor and metadata
  mutations await node contracts. No gate, capture or parity claim yet.

## Form module boundary

The existing registration form methods/rendering move to project_form.rs; the
host list and navigation remain in projects.rs. No request, validation, callback
or test expectation changes in this extraction. Moved pixel values reference
existing exact-valued spacing tokens (the existing ten-point gap is six+four),
so the new file introduces no style-ratchet entry. Command approval and script
files remain outside this change.

## Correlated existing project actions

The supported legacy project mutations now carry a validated host alias and a
checked monotonically unique form ticket. Cancelling/reopening does not claim a
remote operation was rolled back; stale completions may reconcile the host cache
but cannot close or fail the newer draft. The link bounds the mutation wait to
the reference SDK's default and exposes timeout as an unknown outcome with a
refresh instruction, never as proof of rollback. Queue refusal settles through
the same ticket. Existing project registration still creates no session.

Removal requires an explicit confirmation showing exact host/name/path. The
current legacy node's refusal while occupied is explained honestly. Deliberate
initial presentation difference: confirmation is inline in the settings content,
rather than a modal; the node's removal semantics are unchanged. This is not the
new workspace archive operation. Four pure transition regressions cover duplicate
submission, cancellation/reopen, host change and counter exhaustion. Real link and
rendered interaction checks remain pending.

Three link regressions now exercise real framed NodeClient RPC over duplex memory:
exact remove target/ticket, queue refusal correlation, and read-only node refusal.
No real host, agent or filesystem project mutation is involved. Checks are still
pending; source is not called passed. The package base merge follows the central
announcement rather than independently merging root node code into the app.

The first native narrow Clippy (d84449ca) stopped on one doc-markdown lint in the
new deadline comment; that comment is corrected. The package then merged the
announced app package base 8d5513e4 without conflicts. The projects size exception
remains removed; shared registration files follow the new base split. No capture
or runtime completion is inferred from this source merge.

Host-alias mutation correlation now rejects stale payload hydration. After a
cancelled form or successful book rename/forget, delayed project results cannot
write the cache or settle a newer form. If that host is still known, the app asks
for its current authoritative list instead. Renaming/forgetting closes the old
host's project mutation drafts without undoing an already submitted node effect.
A headless GPUI regression combines alias reuse, stale success and a newer pending
form with actual text input/caret retention. Current native checks remain owed.


## Remaining project feature work after registry dependency

The canonical ownership split places create-flow UI/receipts with create,
archive/recovery orchestration with lifecycle, setup progress with scripts, and
sidebar rendering with its navigation package. This package continues project
registration/plain directory creation, metadata/icons, suggestions, settings,
editor targets and automatic archive. These are incomplete; existing screens and
pending value declarations are not parity proof.

Fresh source decisions for the next stage:

- Project equivalence is presentation, never workspace identity or authority.
  Paseo server/project-key.ts:6-38 normalizes forge remotes, GitHub path casing,
  explicit ports and subdirectories; no-remote roots stay host-specific. The app
  projects/workspace-structure.ts:161-179 falls back to distinct placements when
  the same equivalent key appears twice on one host. Reuse existing forge remote
  parsing and explicit NodeId/ProjectId placements rather than a cwd join key.
- Editor targets use a local desktop bridge, not the legacy daemon editor RPC.
  app/workspace/open-in-editor/planner.ts:59-112 requires local execution and a
  bridge; desktop/editor-targets/registry.ts:31-56 defines target order and
  registry.ts:66-130 rechecks target availability and absolute existing paths.
  Require authenticated managed-local target provenance from the desktop node
  controller before resolving an OS path or launching. Remote projects keep
  eligible forge/clipboard actions. No editor was launched in this research.
- Settings preserve raw vessel.toml and unknown fields. The existing project
  reader has a fixed byte cap and typed schema validation; generic file.write
  cannot stand in for project registration/revision authority. Saving never
  releases an external-source gate or approves a command digest.

Package base 2 supplies Machine.node from the authenticated welcome. Project
mutation keys now capture that NodeId as well as the name and local serial;
link refuses a mismatching NodeClient before sending any mutation. Forms and
removal confirmations retain their original node. Rebinding an alias does not
retarget an existing path draft, and a response from the old node cannot hydrate
a different current node. A framed in-memory test verifies no request is sent
to the replacement peer. The earlier import cleanup accidentally removed part
of a trait name; it is corrected explicitly rather than by substring editing.

Native registration checkpoint: strict all-target Clippy for view/link/app and
font validation passed on the current source. Narrow project tests passed 27/0:
2 headless app input/correlation, 4 framed link tests and 21 view cases selected
by the filter (including existing projection/Git cases). These are scoped checks,
not screen captures, real fleet behavior, full C1 or V4 acceptance.

## Manual readiness producer (staged, unverified)

`Shell::setup_readiness(&Name, NodeId) -> ProjectSetupReadiness` reports
Ready/Pending/Failed for one authenticated online host. The onboarding owner
retains and validates its sole Target and Visit; this query does not invent or
acknowledge a suggested-project checked set. A missing/loading project list or
running clone is Pending; read/clone failure, missing directories and a rebound
or disconnected host are Failed. Form validation errors remain scoped to the
captured Name+NodeId.

[Decision Log]
- Purpose: onboarding cannot advance merely because a form was closed.
- Constraint: cancelling the visible form does not cancel an already sent RPC.
- Alternatives: a page busy boolean or treating detached responses as successes.
- Choice: separate bounded manual activity tracking from current form correlation.
- Reason: a late success/failure must settle activity while remaining unable to
  hydrate another node or close another form. Up to 16 retained reconciliations
  are allowed; at capacity new dispatch refuses, existing results still settle.
- Tradeoff: failed activity remains until explicit retry for that host; readiness
  is conservative and never proves checked suggestions were all added.

Pure tests cover cancellation, stale replies, host identity and capacity; a
headless shell test covers clone observation and host rebinding. New source has
not compiled or run yet; earlier 27/0 and strict Clippy evidence applies only to
the preceding checkpoint.

## Add Project navigation source checkpoint (unverified)

`view::workspace::projects::navigation` models all seven source page kinds from
Paseo `app/src/add-project-flow/model.ts:29-146,153-269`: host, method, directory
search, GitHub search/location and new-directory parent/name. Preferred/sole
host skips the initial chooser; late discovery can only resolve that initial
page. Later refresh never retargets a captured Name+NodeId. Registration and each
method have separate support, and Browse returns an explicit local-bridge intent
without opening anything. Back restores earlier query state and eventually
closes; name and clone-parent drafts are kept by target+parent/remote. Paths are
node-local opaque draft values, never client-side filesystem identity.

[Decision Log]
- Purpose: preserve in-flow state across Back without dispatching mutations.
- Existing assets: ProjectActions owns in-flight correlation and readiness;
  PaletteCursor already owns keyboard result selection. Neither models this
  page stack or per-target draft cache, so navigation is a small separate model.
- Alternatives: further expand the current inline AddForm or use a generic wizard.
- Choice: one closed seven-page enum, three previous pages at most, bounded
  non-evicting draft cache (64 entries, 64 KiB per input/key).
- Reason: source transitions have at most four pages; bounded drafts prevent
  repeated Back/target changes retaining unlimited input. At capacity new keys
  refuse while existing edits still work.
- Effects: navigation and draft tests are added, but query/loading/error rendering,
  keyboard row wiring and all node-backed submission remain to be connected.
  This checkpoint is not the complete Add Project UI or a green consumer API.

## Fixed-target manual project form seam (unverified)

The synchronous readiness query must observe the same captured target as manual
submission. The old Settings renderer/opener consults Settings' host selection;
it cannot stand in for a fixed onboarding Target by itself.

`Shell::open_add_for_target(AddForm, &ProjectTarget, window, cx)` checks the exact
online Name+NodeId before opening and captures that target without consulting
Settings. `areas::workspace::project_form::for_target` renders only the matching
current form using the existing renderer, so onboarding does not render a different
host's project list. Settings delegates through the same renderer when bound.
The caller still owns its Target/Visit and checked-set acknowledgement. This
generic display target grants no managed-local OS-path provenance. Legacy form
methods remain existing supported registration/clone/explicit Git-init calls;
this seam does not claim the complete new-directory or suggestions flow.

A headless input test puts a remote host first, opens another exact target, and
checks that rebinding that alias cannot replace the draft. New code awaits strict
Clippy and project tests; no consumer-ready hash is declared here.


## Shared native projection dependency: next implementation

The registry/store dependency must pass before this runtime consumer is wired.
The next native delivery is the single shared workspace projection used by the
sidebar, panes and command center, with a bounded source review before adoption.
It takes priority over the remaining project presentation and editor features.

[Decision Log]
- Purpose: empty workspaces and multiple independent workspaces sharing one root
  must have stable rows, pane keys and navigation without a selected session.
- Existing assets: the authenticated fleet supplies NodeId; projects.rs supplies
  host project data; W12 owns WorkspaceLookup and LayoutKey; W2 owns parentage;
  W7 owns attention. Standard collections and Arc suffice; no new dependency.
- Alternatives: cwd grouping, choosing a member session as workspace identity,
  treating missing attention as zero, or adding another pane routing registry.
- Choice: modify these existing boundaries and add one registry cache/projector.
  NodeId+ProjectId is project placement identity; NodeId+WorkspaceId is workspace
  identity. A captured host alias is a route revalidated against its current
  authenticated node, never another identity source.
- Reason: aliases can be renamed/rebound, a workspace can be empty, membership
  can include retained historical resources, and loading is not an empty result.
- Effects: one complete registry snapshot is replaced atomically. The app must
  reject old tickets, wrong-node responses and decreasing global revisions,
  expose loading/offline/unsupported/failure separately, and stop one finite
  long-poll when its connection/host binding changes. Full snapshots are shared
  rather than cloned for each row consumer.

The view adapter implements the existing
`layout::ownership::WorkspaceLookup::{session_workspace,workspace_location}`.
`WorkspaceLocation::Root(WorkspaceRoot)` comes from canonical backing, never
from cwd. No authoritative snapshot returns LookupProblem::NotHydrated; a
disconnected/unsupported/failed owner returns Unavailable. A known unowned
session alone returns None and permits W12's explicit legacy session bridge.
The workspace entry includes the exact owning project and validated host route;
W12 owns the empty-workspace activation sink and pane-frame integration. Project
settings navigation rechecks the same captured placement before opening it.

W2's SessionInfo::parentage is interpreted only when the authenticated peer
advertises its assigned SUBAGENT_RELATIONS feature. Root and Detached have no
current parent; Attached carries its real SessionRef across nodes/workspaces.
Missing receipts on pending/old peers and invalid parentage stay Unknown.
Creator remains audit metadata. Registry membership cannot replace this check.
W7's attention supplies unread/waiting/failed; missing/deleted sessions are
omitted, not known-zero. Actual working phase is a separate session fact. Its
new native bucket and current merged producer are still unreviewed, so no new
status implementation or runtime readiness is claimed by this plan. Title/pin
come directly from the registry; lifecycle never substitutes for attention.

Reference: projects/workspace-structure.ts:53-116 joins each host's project and
workspace records, including projects without workspaces. Its 161-179 placement
fallback avoids treating an ambiguous project equivalence key as identity.
Cross-host project equivalence is later presentation work; it cannot alter
authorization, workspace membership or this stable placement key.

The announced shared pane line was merged, retaining both layout and project
action registration arms. Fast root size/boundary and whitespace checks passed;
new merged-head native compilation and this runtime projection remain owed.


The first combined strict native check after the pane-base merge stopped in the
new navigation test module: thirteen setup transitions discarded their typed
navigation result under unused-results. Those setup results are now explicitly
bound; the existing state, draft, capacity and refusal assertions are unchanged.
The checked run did not reach a successful app/link result or runtime tests.
A corrected immutable-head strict check remains required before consumer handoff.


## Project list provenance before registry projection (staged)

The legacy project list was indexed by alias while its replies carried neither
node identity nor request order. A delayed read or clone follow from a replaced
peer could therefore hydrate that alias with another node's ProjectId facts.
This is unsafe input for shared project placement, even though mutation tickets
already check their authenticated peer.

[Decision Log]
- Purpose: make the existing project display cache safe to join with the registry.
- Existing assets: ProjectTarget already captures Name+NodeId; ProjectsView holds
  display data; the link already authenticates NodeClient and follows clones.
- Alternatives: infer the response node from the alias at completion, or duplicate
  the project list in each row owner. Neither retains the actual response source.
- Choice: modify the existing list request with a typed read key, echo it in the
  result, and check the authenticated client before dispatch and current peer
  before hydration. Clone updates carry their actual NodeId. Each alias retains
  only its latest read ticket; disconnect/forget/rebinding revokes old reads.
- Reason: same-peer refresh reordering and different-peer alias reuse are distinct
  races; both must be rejected before data enters the shared display cache.
- Effects: MAX_PROJECT_READ_HOSTS bounds retained host read slots, with no
  eviction and checked monotonic ticket
  allocation with no wrap. Exhaustion refuses dispatch. Readiness waits for a
  pending authoritative refresh. The link cancels old-peer clone follows when a
  route is rebound; project revisions keep reordered progress/list replies from
  regressing clone completion. No new RPC, dependency or workspace identity.

New pure tests cover refresh/rebind/forget/reconnect/capacity/exhaustion and clone
revision ordering. A headless app test exercises delayed old-node list/progress
and readiness; a framed in-memory link test checks refusal before sending to a
rebound peer. These changes have not compiled or run. They do not provide the
registry snapshot, workspace activation, status join or full C1 acceptance.

Readiness additionally requires a verified project-cache peer binding; a current
fleet welcome cannot authenticate a previously unbound cached list by itself.
The existing readiness fixture now enters its list through the same correlated
hydration path, rather than assigning unscoped display state directly.


The project-settings entry now reuses the existing Settings/detail renderer
through Shell::open_project_settings(&ProjectTarget, ProjectId, window, cx).
It verifies authenticated peer, project-cache provenance, settled read and exact
project membership before changing the screen. Unavailable/NotHydrated/Missing
are typed refusals, preserving the previous route and selection. Existing
project rows capture the verified target rather than consulting Settings' host
at click time. This is source-only; workspace activation remains W12's separate
sink and the complete shared registry projection is still pending.

Opening a removal confirmation also requires a settled project list from the
same authenticated peer. A new welcome cannot grant removal authority to an old
peer's cached project id. The headless race case checks refusal before a
confirmation exists, followed by normal confirmation on the verified peer.

Read-ticket lookup and detachment take the existing validated book Name type,
using direct map lookup instead of scanning string labels. Book hooks retain
that type until they reach older display-only helpers. The RPC remains unchanged.

The first strict read-provenance run (8f7bf2ee) passed view compilation, then
stopped on the extended clone-follow map's type complexity in link. Its key
now has one private FollowKey alias without changing its representation or
ownership. App compilation/runtime checks remain owed on the latest source.

The older c1baa native check reached app tests and found the book Name has no
Display implementation. Tests and the new settings route now explicitly obtain
its validated text with as_str, leaving the shared Name API unchanged. The app
line's announced base6 (18eedad4) merged cleanly; its stale Cargo.lock is a known
base issue and no --locked flag is used. Current combined checks remain required.

Base6 exposes the shared connection::ConnectionContext/ConnectionEpoch and
link State::selected_connection with SelectedConnection::call preflight, physical
retirement and completion fencing. The real shared registry projection must use
that one actor-owned admission mechanism; no area-owned epoch allocator or
first-alias lookup will be added. Current project display tickets cover alias/
node/read ordering, not this full physical-admission cutover, so they remain a
prerequisite rather than a ready runtime producer. The readiness fixture now
uses the shared Machine constructor, retaining new base admission fields.

### Captured project admission (source staged; execution owed)

[Decision Log]
- Purpose: a same-node reconnect must not inherit a project draft, cache or reply.
- Existing constraint: NodeId and read order fence alias rebinding but do not identify a physical connection.
- Alternatives: a second project epoch allocator or raw NodeClient cloning.
- Choice: C1 list/mutation requests use the shared ConnectionContext and SelectedConnection; the original context is echoed through queue rejection and result application.
- Reason: the connection actor remains the only allocator and retirement authority.
- Impact: lists are hydrated only by the current admission, forms and removal confirmations retain their opening admission, and a retired in-flight write is OutcomeUnknown rather than a claim of rollback. No automatic replay occurs.

The line lock fix dd13ce37 is merged. The queued stale-lock check023 was cancelled via its verified waiting ticket and replaced once on ad4de37f; this preserves that historical check as cancelled, not a compiler failure. New admission code is later and still requires exact changed-head checks. Clone observation uses the same selected connection and a shared bounded call budget, without introducing another node registry or inferring ownership from paths.

The fixed onboarding renderer is `project_form::for_connection`; it requires
both the current shared admission and the draft's captured admission. Even a
same-node replacement cannot display its predecessor's form. The legacy local
renderer also disables submission when the physical admission no longer matches.
`Shell::project_inventory_on` returns Pending/Unavailable/Empty/Existing from the
same admitted cache plus manual-work readiness. An empty pending, failed or
truncated list never chooses Empty. These are source-only additions until their
exact remote checks pass; checked-suggestion acknowledgement is still absent.

The legacy alias-only Workspace dispatch explicitly refuses C1 project list and
mutation variants, including ProjectChange. Only the new captured-admission path
can send them; Commands/Approve and other owners' workspace resources retain
their existing route. The small actor dispatch moved from link/lib.rs into the
owned project request module, shrinking the crate root. A framed regression
checks that the old project-write route sends no RPC.

The corresponding project-settings sink is now
`open_project_settings_for_connection(&ConnectionContext, ProjectId, ...)`.
It validates the row's original admission before the existing checked cache and
project membership route. It is the intended cross-area sink for S2/palette;
WorkspaceId-based activation still belongs to the pending registry projection
and W12's one activation adapter. Current-source checks remain owed.

### Canonical registry projection skeleton (not registered or compiled yet)

`view/workspace/registry_projection.rs` and its rows/validation/tests are a source
skeleton waiting for the reviewed registry proto to reach the app line through
B0. The node integrator has merged the dependency in batch11, but has not yet
announced its gate47 head; no full node head is merged into this app tree.
The module is intentionally not registered in workspace/mod.rs yet. Existing
native checks do not compile or execute this skeleton. Do not consume it as a
checked producer or a runtime implementation.

[Decision Log]
- Purpose: one C1 projection for S2 sidebar, W22 palette and W12 ownership/roots.
- Existing assets: canonical RegistrySnapshot/WorkspaceDescriptor/memberships and W12 WorkspaceLookup/LayoutKey; existing project/worktree RPC values supply explicit metadata, never cwd ownership.
- Alternatives: caller-local registries, duplicate workspace ids, root inference or treating unsupported as empty.
- Choice: retain canonical values with the shared physical admission and a unique read ticket; borrow rows with NodeId+ProjectId placement and the existing LayoutKey::Workspace key. Encoded metadata has a16MiB aggregate cache budget,64 retained hosts and no implicit eviction; a streaming counter measures without a second serialized buffer.
- Reason: atomic registry membership remains the sole owner fact. Actor routing always retains an explicit original ConnectionContext; cross-node metadata lookup never chooses a client or alias.
- Impact: loading/offline/unsupported/refused remain distinct. Same-node reconnects retire old replies. Invalid/duplicate ids or memberships fail closed. Root mapping exposes only an explicitly identified exact project/worktree root; nested execution folders are External until the canonical reroot contract lands. Historical memberships remain ownership facts, not liveness or eligibility.

The skeleton deliberately does not invent status/root-agent facts: W2 and W7
have not published checked native producer subsets. No generation, process exit,
start authority, archive/restore effects or resource readiness is inferred. The
next wiring adds the selected node read and W12's one activation sink once the
shared proto reaches the app, then exact checks and V1 source review. The draft
pure tests cover independent same-cwd ids, stale epochs/tickets, unsupported vs
unowned, contradictory snapshots, root mismatch/archive history and budgets;
none has run yet.

The shared admission is independent of Reach, which reports the session-list
read outcome. Project requests/cache reconciliation now use Machine's actual
connection context even when sessions were refused; a session-list failure cannot
retire a still-authenticated project connection. The headless project test keeps
that semantic refusal while checking the same admission. The old ad4 strict check
failed first on an unquoted NodeId doc identifier; owned new doc identifiers are
quoted, with the original failed log retained. No successful compile is inferred
for newer code from that old run.


### Cache reads must also observe original-lease retirement

The captured context and a settled read ticket do not by themselves prove a
live admission. A transport may retire before the queued fleet-retired event is
installed. C1 must retain the canonical link `DeliveryGuard` with each complete
project/registry cache and validate it on every readiness, inventory and lookup
query, as well as immediately before reply installation. A retired guard makes
Existing/Empty unavailable even if the fleet still paints the old context.
Forms and writes keep their original context/ticket checks in addition to this
read guard. The guard is weak and cannot keep a retired client alive.

This work waits for the shared production export/minter and its reviewed real
consumer to land; C1 does not change the shared staging attributes or mint a
replacement guard. Pure view models remain free of link ownership, while the
app's private cache adapter must validate the canonical guard before exposing
those models to onboarding, sidebar, palette or pane lookup. No context-only
runtime readiness or early helper adoption is claimed.

The reviewed values-only carrier has source/blob identity evidence for all
20 files but awaits the app lead's adoption. Its values provide no Store,
actor or native runtime. The next combined node base may legitimately contain
newer content in shared files, in which case the landed node files replace the
carrier. The unregistered projection remains excluded from current checks.


The unregistered root projection now delegates identity mapping to the canonical
`WorkspaceDescriptor::root()` after checking explicit catalog ids/path consistency.
Its registered Git fact is available only for a supported exact placement; a
managed backing also requires a Ready Git project in the catalog. Missing or
contradictory catalogs and unsupported nested placement remain unavailable. This
is metadata from the node's record, not a filesystem inspection or physical-root
continuity claim. The new pure regression remains uncompiled with the skeleton.


The earlier native strict run on `236d6258` stopped in link: a redundant NodeId
qualification, the clone observer's wildcard over Answer, four helper unwraps
whose combined cfg expression hid the test-module context from lint recognition,
and the shared dispatch function reaching101 lines after project registration.
The source correction uses the imported type, explicitly classifies the observer's
unrelated answers and splits the logically identical test/target cfg attributes.
The dispatch budget still depends on the separately owned shared preferences
extraction. C1 does not duplicate that change; its local extraction was reverted
before adoption. Once the shared line lands it, C1 adds only its project arm. No size/lint
allowance is raised. These changes still require current-head checks. The queued
older native tests remain their original source evidence, not projection proof.


The staged pure projection exposes `read_context(NodeId)` only to identify the
metadata source selected for its node lookup. The private app cache adapter must
validate that exact source's canonical weak guard before lookup; a guard from a
different alias cannot validate another entry's data. This helper never chooses
a client or action route. The pane's captured action context remains separate.
An additional pure regression covers higher-revision source selection and
conflicting equal revisions; it remains uncompiled until module registration.


### Editor opening is a local desktop operation

Direct reference reads clarify the execution boundary:
`packages/app/src/workspace/open-in-editor/button.tsx:100–159` and
`directory.ts:20–72` gate desktop targets with `useIsLocalDaemon`; remote workspaces
do not send their paths to the local editor bridge. Desktop
`features/editor-targets/ipc.ts:13–58` validates the launch payload and dispatches
locally, while `registry.ts:31–56,66–132` fixes the 25-target order, applies
workspace-specific availability, expands home paths and requires existing absolute
local paths. Installation is rechecked at launch. No node editor RPC follows from
these desktop sources.

[Decision Log]
- Purpose: an identical-looking remote path must never open an unrelated local directory.
- Existing assets: authenticated managed-local target provenance, canonical connection admission, existing workspace placement and the desktop intent queue.
- Alternatives: classify localhost URLs, use the first book alias, compare cwd strings, or launch an editor on the remote node. Those lose the execution boundary or change the desktop behavior.
- Choice: future native editor discovery/launch stays in a local desktop runtime behind the canonical live managed-local target and the original connection guard; registry ids provide placement only.
- Reason: local execution is an authenticated controller fact, independent of alias/path spelling; editor support is not a new node capability.
- Impact: no local-target fallback for remote/unavailable peers. Target selection, positive line/column validation, installation checks, safe argument construction and process launch require a fake-runtime-tested implementation before activation.

This is source analysis only. No real HOME/PATH editor inventory, editor/browser
launch, node call or desktop runtime implementation was performed. The 25 target
implementations, platform launch behavior and batch-file quoting still require
the complete target-specific implementation and review; reading the registry and
boundary does not establish their parity.


## Base 9 integrated registry dependency history

The following producer history is preserved from the node line, not a current
claim that its old checks remain pending. Registry/admission are now in announced
node 9c150c6e (all 18 combined stages green) and app base 9 db42d9c5. Metadata
protocol 25 is separately announced on node 1c762ccf; the app waits for its lead
base before registering the live projection. Native guard/query proof and the
remaining C1 effects are still incomplete.

## Final ownership correction

The canonical plan assigns this package project UI plus registry/status/rename/pin
and ADR 0082 node identity. Create-flow UI/receipts and terminal profiles belong
to the create package; archive/restore UI belongs to lifecycle. They consume the
shared registry rather than re-declaring ids. Setup progress UI belongs to scripts.
Project settings and editor targets remain this package. The registry's first
staged contract is `vessel_proto::WorkspaceId` and `vessel_proto::registry`,
including an atomic `RegistrySnapshot { node, revision, workspaces, memberships }`.
Session/terminal ownership is explicit via WorkspaceResource; no old SessionInfo
field or cwd-based ownership is introduced. Protocol remains unassigned.

Narrow validation: local proto test cancelled while still queued (no compiler
ran) to move the dependency-critical contract check to the build machine.
The contract is staged until its narrow test and Clippy complete; no native or
root whole-gate/capture/consumer-ready claim yet.

## Registry persistence implementation

The SQLite append follows the base's newest migration through one local constant.
The public Store handle uses its existing writer savepoints/read pool. Descriptor
CAS, resource admission and global revision are committed together. Admission also
advances the descriptor revision, so an archive claim made from an older member
snapshot fails. Archiving blocks further admission. Creation uses fixed backing
reservations and a descriptor+reservation completion transaction; these are not
the create package's user operation/subscribe receipts.

Regression cases: shared cwd stays distinct, stale CAS rollback, unique ownership,
archive/member race, changed provision intent, corrupt rows and counter exhaustion.
Existing inbox rollback fixture gains only this migration's DROP statements before
its original rollback; no production migration runner or head assertion is changed.

Resume: staged contract 702277bd passed 5 proto registry tests; its narrow Clippy
is queued. Store persistence and Archiving/root adapter are implemented locally,
awaiting narrow store checks. No node RPC is registered, no app wire integration,
no full gate or captures yet. Shared registry types are not called reviewed.

The continuation's staged provisioning service reuses worktree creation with a fixed,
durable backing id. Local provisioning creates only a registry record. Replays
return the same descriptor; an id with a different intent is refused. Approval
is returned before a worktree is made. Startup marks interrupted Creating
reservations Uncertain; no automatic filesystem replay occurs. Descriptor
publication and reservation Ready are one transaction. The node module remains on that continuation until its real caller wires it.
This dependency retains the unchanged legacy worktree.create implementation,
which chooses its own random id.

Contract narrow Clippy on 702277bd returned three documentation-markdown lints
(no type/runtime error). They are corrected in the next commit and rechecked;
the failed log remains evidence. Tests on that commit were 5 passed/0 failed.

The first store narrow run (53e835a1) stopped while prebuilding the node because
SQLite does not decode u64 directly. The registry reader now decodes i64 and
checks conversion to the typed nonnegative revision; no default or truncation.
The original failed log is retained.

Store narrow run c620d7c2: 87 passed/0 failed. Local Clippy found unused SQL row
counts, doc markup and discarded signed-conversion causes. The next change binds
row counts and retains TryFromIntError as a typed source. Registry invalidation
uses a coalesced watch emitted only after a successful writer commit; consumers
re-read authoritative lifecycle/ownership, with no polling or cwd guess. New
watcher/owner regressions are added and await checks. This is not a runtime port
allocator or lifecycle drain implementation.

## Dependency split and pending registry calls

The identity-only dependency has its own value-types plan. Registry/store remains
separate and unreviewed until its whole root gate and source review pass. Quality B
pending rows now cover workspace.registry/list/get/rename/pin under the single
unassigned feature constant; no protocol number is selected here. A finite
long-poll uses the existing waiter shares and the store's committed watch. Empty
and archived records remain authoritative, with stable NodeId in the snapshot.
The root Request/UnexpectedReply definitions moved to rpc/request.rs without
changing their public re-export, to keep the single method table below its budget.

Actor admission uses an opaque pre-spawn lease, exact ownership and an archive
fence. Restart marks unfinished admissions uncertain; never replay automatically.
Named lifetime count and worst-case byte reservations are in registry/capacity.rs.
Tests cover lease/archive interleaving, stale completion after retry, capacity
reconciliation without eviction, and oversized record rollback. These tests and
new pending handlers still require current narrow and full-package validation.
Project metadata/config/settings/editor and trust-release runtime work remains
incomplete. No native integration/capture/parity completion is implied.

External-source automation now has a canonical typed source, persisted in the
provision intent and descriptor. The pending workspace.release_automation method
compares exact source+revision and requires Active/person authorization. It does
no execution. Blocked provisioning forces setup Skip; tests assert skipped state
and absence of the isolated marker. Wrong source, stale revision and an archive
claim all refuse release without changing state. These changes await current
checks. The first registered-call narrow build found three exhaustive mapping
omissions (agent withholding, old replies, forwarding); all new registry methods
now explicitly choose those mappings, with no wildcard or head-case rewrite.

The registry dependency migration contains only its live descriptor/reservation/
resource/revision tables. Project presentation storage is deferred to the later
project package's append; no unused project-metadata table is reserved here.


Earlier registry dependency checkpoint: local Store suite 96 passed/0 failed,
node registry provisioning 5/0, TypeScript generation 4/0. Strict all-target
Clippy then found explicit pin defaults, a borrowed error adapter, one oversized
existing worktree function after fixed-id admission, and an unused test result.
These are corrected without changing head test expectations. Stable terminal
restart now has a distinct exact-predecessor lease API and two new adversarial
cases plus at-capacity restart coverage; current checks and A12 remain owed.
Existing Ready admission is not process-exit proof. Its consumer must join the
old run before restarting; no signal/lifetime heuristic is substituted here.

## Dependency whole-gate findings and correction

The frozen registry dependency at 1958fd7b passed strict root/TS/forge Clippy,
wire generation and build checks. Its whole Rust run had 2304 passed, 8 failed,
12 ignored: the explicit forwardable-method expectation lacked the six new
registry methods; seven legacy migration fixtures left the registry append in
place before rewinding user_version. The registry admission/restart/watch tests
passed in that run. Web unit tests passed 449 and forge tests 4; browser e2e
passed 65 with one unresolved command-list Escape/filter failure at
conversation.spec.ts:258. This is a failed gate, not acceptance or parity proof.

The correction adds the six methods to the explicit expected set and removes
only the new registry tables in rollback fixtures. The title-origin fixture now
targets its preexisting schema checkpoint instead of newest-minus-one, so a
later append cannot change which migration it tests. Production migration SQL,
authorization, method scopes and assertions remain intact. Targeted reruns and a
new whole gate remain owed; the unrelated browser failure must also be checked.

## Dependency boundary: provisioning stays in the continuation

The registry/store dependency excludes the cfg(test) filesystem provisioner and
its node-only tests. Under A10, an unwired node service stays on its continuation
branch until its real caller removes staging in a wiring commit. Its source is
retained in the separate project-management continuation; it has no consumer-
ready or shipped provisioning API claim. This dependency contains the live
registry read/mutation handlers, durable metadata/reservations/membership and
start-admission seams. Creation receipts, actual backing provisioning and actor
launch remain separate work. No default workspace is synthesized.

The first fixture-correction narrow failed at compile time because devices.rs
uses its own inline fixture builders rather than support/fixtures.rs. The
rollback SQL now has one small test-only module used by both fixture families.
The failed run is retained; a corrected narrow and whole gate remain owed.


## Corrected dependency narrow evidence

At exact source 53c13163, proto/store all-target tests passed 465/0/1, including
the corrected legacy migration fixtures, admission/restart and registry watch
cases. Store benchmarks also completed. The subsequent Clippy command did not
execute because local Rust compilation was disabled centrally; this is not a
compiler failure. Strict proto/store/node Clippy is now submitted to the remote
build gate. No new Rust work will run on the development host.

The unrelated command-list failure was classified by its web owner as a base
caret timing defect: a deterministic regression failed 3/3 before its fix and
passed with the fix. Its combined node gate is still running. This dependency
will merge the announced complete node head when available, as instructed,
before retrying the whole gate. No web source was edited here. Source review
and dependency adoption remain pending.


## Announced caret-fix node merge

The complete announced gate44 node head ede20014 is merged under A8, rather than
taking its web fix alone. Its root gate passed Rust2377/0/12, web464, e2e68 and
forge4; that is base evidence, not this registry union's gate. The node head owns
rpc/contract.rs and rpc/table.rs; the registry's redundant request.rs extraction
is removed, and its six pending rows are appended at the method table's end.
The head's existing rows, tests, protocol and schema entries remain intact. The
registry append still follows the unchanged newest head migration. Architecture
and protocol documents preserve the head's new process/service-port facts and
append the independent registry section. Changed-head TS/narrow/full checks are
required next, all on the remote build machine.

The explicit forwardable expectation appends only this package's six names in
their new method-table order. The head's existing entries and order are unchanged;
this prevents the table move itself from causing an expected-order regression.


## Merged-head strict and generated-wire evidence

Exact c524681c6 on the announced node head passed strict whole-workspace Clippy
and proto/web TS-feature all-target Clippy remotely. The TS generator's four
selected tests passed; the named TS step correctly remained red because its
generated proto.ts differed in three method-union lines after table relocation.
The build VM's exact generated artifact and wire patch were recovered through
the supported gate artifact path, source-commit provenance and all SHA256SUMS
verified, then applied verbatim and compared byte-for-byte. No TypeScript was
hand-edited. The new final candidate also retains the expected-method-order
correction; its complete A12 and V3 registry review remain owed.

The first merged-head job was explicitly promoted under a coordinator exception
because the dependency blocks six core adapters: the safe queued-only endpoint
verified it held no build slot, terminal cancellation was confirmed, and the
same immutable c524 commit was submitted once with priority. Its original logs
remain retained. No running build or unrelated job was canceled.

## Pure registry projection registration on base 9

The base now carries the reviewed canonical registry types and real handlers;
only the pure view module is registered here. It shares RegistrySnapshot,
ProjectList, WorktreeList and the existing LayoutKey/WorkspaceLookup types, keeps
bounded host/byte accounting and exact Context/read correlation, and exposes
borrowed project/title/pin/lifecycle/membership/root facts. No registry actor,
network request, guarded result envelope, Shell lookup or navigation sink is
registered. Base 9's pending feature remains Unsupported at a live boundary.

Ten authored pure regressions cover independent same-path identities and exact
memberships, stale/capacity/conflicting snapshots, explicit startup uncertainty,
root/catalog consistency, read-source selection and cross-platform paths. Their
VM execution is owed. This registration does not prove live currentness or grant
S2/pane/onboarding readiness: the real cache must retain the original canonical
DeliveryGuard, validate immediately on installation and EVERY query, and be
exercised through the real shared actor fixture. Both products arrive only via
the lead's checked G4/common owner line; no local guard constructor or shared
attribute promotion is added. Dynamic status/role joins stay unavailable until
their canonical producers and complete coverage are present.

## Guarded registry metadata producer on package base 13

[Decision Log]
- Purpose: one authoritative metadata reader feeds pane/session ownership and
  project/workspace rows without guessing from cwd or a selected agent.
- Existing constraint: History already publishes the canonical weak
  DeliveryGuard. The cross-App real actor fixture remains with its shared owner;
  this package must not expose its private minting or staging attributes.
- Alternatives: cache by node alone, manufacture a guard for testing, or make
  missing startup data authoritative empty. Each would lose physical admission.
- Choice: RegistryRead selects the captured ConnectionContext once, then reads
  the registry and catalogs through that same SelectedConnection. A private
  response envelope validates at receive, and the App retains its weak guard and
  checks it again on EVERY rows/ownership/location query. Same-node reconnects
  therefore cannot inherit cached ownership before a fresh complete observation.
- Impact: four original-admission calls or queued responses share a finite
  permit budget; each response holds its permit through its receive callback.
  Combined queued metadata is capped by the canonical 16 MiB cache ceiling.
  Before that check, at most four reads can retain three separately framed
  replies (each subject to the transport's bound). No extra serialized copy is
  allocated for accounting. The UI rotates admitted hosts and does not immediately
  retry errors. Rename/forget releases that host's tickets and cache slot.

This is metadata only. Separate project/worktree catalog reads are not an atomic
snapshot; inconsistent or unsupported root placement stays unavailable. The
cache does not attest physical continuity, root exclusion, backing generation,
resource exit, role/status ordering or mutation authority. Actions retain their
own original SelectedConnection and node admission. No empty-workspace creation,
archive/restore, setup execution or auto-archive is activated here.

The actual Link actor/framed tests cover a real observed reply, queued retirement
and queued permit retention. App tests cover startup/refusal/forget only. Both
new suites and changed-source strict checks are owed at this checkpoint; a live
cross-App cache freshness claim additionally requires the shared owner's real
fixture. No fake guard constructor, public fixture, shared attribute change or
S2/pane/onboarding readiness is asserted by source registration.
