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
