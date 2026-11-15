# Creation operations and terminal profiles

ADR 0091 records reference behavior, authority, alternatives and compatibility.
ADR 0082 owns workspace identity, backing reservations and start admission;
A1 owns the filesystem/receipt/launch adapter under the recorded transfer.
Consume the reviewed registry contract instead of deriving identity from cwd
or persisting another registry.

## Structure

- `vessel-proto::creation`: validated keys, revisions, phases and typed receipt
  snapshots; create/observe contracts composed with the shared registry types.
- `vessel-proto::terminal_profiles`: bounded profile values and pure literal
  prompt substitution; no process or configuration I/O.
- `vessel-store::creation`: admission/resource claims and receipt transitions
  through the existing SQLite writer/read pool, migration appended over head.
  Reserve count and worst-case retained logical bytes atomically before
  effects; retain every key/claim indefinitely, with typed capacity refusal
  instead of eviction. Existing-key reconciliation and bounded phase growth
  remain possible at capacity. ADR 0091 names the limits and required tests.
- `vessel-node::service::creation`: principal-bound revision observation and a
  test-only phase driver. The retained intake actor and real workspace/session/
  terminal driver are still planned, not shipping modules.
- `service/terminal_profiles` and the existing Store writer: person-only
  metadata, administrator configuration CAS and bounded singleton persistence.
  Actual custom-program launch still awaits the process/registry adapter.
- Existing method table: pending rows only after Quality B; no alternate
  dispatch or provisional version number.

## Implementation and evidence

The existing writer/savepoint was extended: creating another database, async
mutex or workspace registry would split admission authority. No external
storage dependency is needed. A new receipt reserves its full 8 KiB logical
budget and all resource claims before the transaction returns. Each bounded
4 KiB row is encoded before replacement; count/byte refusal precedes claims.
Existing-key reconciliation precedes capacity checks, including after restart.
Keys and claims are never removed when runtime resources disappear.

Profile values at 5c39da5f passed six narrow tests and strict proto Clippy;
row125 found no profile-source issue but returned the capacity contract.
Contract revision295281a2 passed row129. The earlier count-only store slice
b4353ae6 passed five real-writer tests; those results do not validate the new
byte accounting. Exact5bc's four-library suite executed 750 passing tests,
including the current capacity/restart/namespace coverage, and one failed
completed-resource deletion fixture. That fixture is corrected in5244;
run wp-create-node-narrow-20261115T120447Z-5244c045 executed 31 focused tests
with no failures, including exact acknowledgement, safe errors, namespaces,
restart and retained keys. Strict Store/Node all-target Clippy then passed
run wp-create-node-safety-20261115T125224Z-4ee19cff after the test-lint fixes.
No runtime driver,
public protocol assignment or receipt/launch source acceptance is claimed.

The first post-gate50c strict run stopped in the shared writer dispatcher at
105 lines after the combined additions. The owned terminal-profile and receipt
arms now delegate to their own existing savepoint/notification/typed-answer
logic. No budget is raised, and other writer branches and commit ordering are
unchanged. The failed run is retained; the corrected source still owes its check.

The post-merge static size check also found migrations.rs at501 lines. The two
owned version appends now live beside their SQL in schema/creation.rs, deriving
RECEIPTS_SCHEMA from the existing REGISTRY_SCHEMA and OWNER_SCHEMA from that.
The central list still fixes ordering and its array length checks the result;
registry visibility changes only within the schema module. No version, SQL or
size allowance changes. Integration repoints only the owned receipt assignment.

Approved shared identitye45a5d89 is merged. Announced gate50c head9c150c6e
is now merged with the reviewed registry and project admission/fixed-id
primitive. Its old unreviewed prototype remains excluded. New adapter/intake
source and the combined generated declarations still owe their narrow checks.
Source-created change requests
must carry ADR0082's explicit blocked-automation gate before provisioning.

Next: recover the registered checks, consume the reviewed registry Store and
fixed-id admission primitive, implement the owned filesystem/real driver,
then native integration, package gates and reviews. Observe/profile rows are
already pending; actual create RPCs remain unavailable.

Receipt observation uses the existing writer's post-commit effects, mirroring
its coalesced inbox wake-up pattern. Subscribe before the initial SQL read;
each wake-up re-reads the authoritative receipt. Notifications contain no
prompt/argv and retain only one unit value, so intermediate revisions may
coalesce. The writer sends them before acknowledgements; a cancelled caller
cannot suppress notification. A rolled-back savepoint produces no wake-up.
No new RPC or runtime provisioning is registered by this storage addition.

Coordinator assigned the MCP agent receipt extension to A1 on 2026-11-15.
ADR0091's amendment (V3 contract source acceptance, row154) binds namespaces to the admitted origin principal,
keeps user workspace/terminal creation person-only, and preserves existing
parentage/count leases before effects. V3 contract acceptance is required
before agent namespace/handler code; that prerequisite is now satisfied.

The staged implementation derives person/agent identities from Principal and
validates stored owner/index/claim metadata before returning a receipt. A
separate additive owner-column migration keeps old person digests unchanged.
Own-agent observation allows Agent kind only; workspace/terminal admission
remains person-only. Tests exercise independent origins, delegated identity,
forged fields, migration and corrupted metadata. New agent creation/count
lease orchestration remains unimplemented and is not approved by the contract
review or these storage changes. Source review and execution evidence are owed.

FirstInput::Identified retains W23's canonical ClientMessageId in the durable
reservation before spawn, including serialization, known prompt refusal and
unknown restart. Only an agent resource accepts this variant. Historical
Present rows remain observable without assigning a synthetic id; new agent
admission must use Identified and future drivers must refuse unkeyed agent
prompt replay. Present still describes blank-terminal typed input, whose
unknown side effect cannot be replayed. This value addition activates no
prompt method and is not server-side deduplication proof.

Pending first-turn archive behavior (not implemented): Paseo's
`server/agent/create-agent-lifecycle-dispatch.ts:217-250` subscribes once,
without replaying prior state, before initial prompt delivery. The first
completed/failed/canceled turn releases the listener and invokes archive.
A newly created managed worktree chooses workspace archive; otherwise only
that agent is archived. The future driver must use authoritative provision
results and the lifecycle owner's archive path, preserve its authority and
settle the listener before prompt dispatch. A cwd or legacy session listing
cannot select workspace archive. No automatic archive or flag is exposed by
this staged receipt/profile slice.

V3 clarified Accepted-before-Begin restart on 2026-11-15: unstarted Accepted
may continue under its original reserved ids after authoritative recovery,
current authority/Active checks, a single shared driver, and proof that no
resource or restored count entry already exists. In-flight remains Unknown.
The earlier blanket reviewer-summary prohibition on every restarted key was
too broad; ADR0091 now records the precise conditions before driver wiring.
Agents::reserve now refuses a duplicate id atomically before minting instead
of replacing its entry. Regressions preserve credential/grant/connections and
exercise concurrent exact-id admission at the last child slot. This hardening
reuses the existing count owner and does not implement the missing driver.

The browser bridge's exhaustive call classification now admits the five
pending unary methods. creation.subscribe is a bounded long poll rather than
a connection-scoped stream. The bridge preserves the existing node scope
checks: metadata/observation Read, full profile configuration Admin and
person-only profile operations. Fleet forwarding remains independently
restricted (full profile get/set are node-local). No public protocol number
or browser-specific authority is added by this classification.

A test-only typed Runtime interface now exercises the phase driver against
the real Store writer and an in-memory effect fake. It awaits each Begin
commit before the fake can run, retains reserved identities across prompt
refusal, and stops dispatch on Unknown/completed receipts. Crash fixtures
separate idle Accepted from committed-Begin/no-effect recovery. This is an
orchestration frame only: it is cfg(test), not a registered/shipping create
RPC, and provides no principal/count/start-lease/generation-fence proof. The
real controller and reviewed W11/runtime adapters remain missing.

The staged driver also has an explicit Deferred readiness result before Begin.
For attachment/context preparation it can park at ResourceReady with no prompt
effect and return its snapshot, rather than holding an active permit while
waiting indefinitely or declaring input accepted. W23 confirmed the eventual
canonical input owner requires an existing authorized session, commits Pending
and dispatches only for its private New token; that runtime seam is not checked
or consumable yet. A source fixture parks across restart, then simulates input
preparation finishing and dispatches only Prompt under the original child and
message identity. No second prompt ledger, automatic replay or acceptance from
creation metadata is implemented or claimed by this readiness frame.

TypeScript generation on exact2eb391a1 compiled and reached the declaration
contract, then refused a global `Snapshot` name clash. Creation declarations
now use Creation-prefixed names and profile declarations use TerminalProfile
names. The profile collection derives its actual Vec dependency rather than
writing an untracked `Profile[]` type string. The omitted optional profile
icon is represented explicitly. The shared alias table classifies caller
creation keys/configured names, numeric revisions/waits, canonical handles
and argv content by its existing policy. ClientMessageId remains W23's value;
its first wire use is branded alongside WorkspaceId/WorkspaceOperationId.
Rust/JSON identity and pending version assignment remain with their owners.

Duplicate reservation compatibility audit: the other production caller is
service/again.rs::start_again. It obtains Sessions::ended first, which refuses
live/visible rows. Sessions::hand_over retires the old Agents entry before
forgetting the visible runtime row. Thus a valid legacy resume reaches a
vacant count entry; the new same-lock duplicate refusal protects held/restored
ids without replacing that resume admission path or its grants.

Stored profile/receipt decoding errors now cross the owned node boundary with
their typed code and safe outer diagnostic only. The shared service formatter
otherwise appends serde source chains, which may quote administrator argv or
receipt ownership values even on a read-scope metadata/observation call.
Changing the common formatter would affect unrelated error contracts, so this
policy stays in the creation/profile handlers. Synthetic malformed-value tests
require omission of that value and preservation of known Conflict refusals.

Creation mutations now return their own writer-held Receipt after the batch
commit acknowledgement. The previous unit acknowledgement followed by a
separate reader lookup could return a later mutation, losing the exact Begin
marker the driver must acknowledge before effects. The existing generic Done
and transaction/savepoint authority carry the typed result; no second writer
or read lock is introduced. A regression consumes an older Begin answer only
after a subsequent Finished commit and checks that it still names its own
revision/marker. Driver exclusion and current authority checks remain separate
requirements; an exact acknowledgement alone does not authorize an effect.

### Ownership transfer recorded for PLAN section 7

Coordinator ruling on 2026-11-15 transfers actual empty-workspace filesystem
provisioning and the receipt/launch adapter to A1/W3. W11 retains canonical
identity, registry persistence, start leases, project/config admission, native
projection, nested-project reroot, PR mode, unregister and the legacy destructive
route fence. W3 waits for the reviewed Store dependency and the explicit file
handoff; f255's cfg(test) provisioner remains reference only. There is one
workspace registry, no local cwd-based ownership cache and no copied persistence.
The new driver must reserve fixed backing identities through that authority,
keep approval/Uncertain outcomes, and hold current admission through registration.

### Private adapter development exception

The coordinator approved merging only reviewed registry258c828d and project
admission/fixed-id cd240807 into this private node tree ahead of the announced
head. No result from that temporary union may be handed off or gated until
the announced node head is merged. That restriction is now settled by merging
announced gate50c9c150c6e; no check was started on the temporary union.
Registry migration precedes the
creation/owner appends; their versions derive from the preceding constant.
The generated wire file is kept from the registry producer during conflict
resolution and requires combined regeneration after the announced merge;
the earlier freshness evidence does not cover this development union.

The owned workspace/provisioning module is literal cfg(test) staging. It
requires an actual stored receipt Begin marker, derives the workspace id
from that receipt, and uses the canonical backing reservation before effects.
It acquires the shared bounded project admission before detaching its tracked
task, keeps its own Arc through descriptor commit, and passes a clone to the
fixed-id backing primitive. Creating/Uncertain reservations never re-create
backing. Local provisioning changes no directory; managed creation always
uses Skip. Descriptor readiness is backing readiness only: requested setup,
source authorization, generation fences, resource/count/start leases and the
actual async create RPC/controller remain unavailable and are not claimed.

This modifies existing primitives rather than adding a registry, file store,
counter or external library. Test source covers committed-marker admission,
same-root distinct identities, same-operation descriptor retention, intent
conflict, fixed backing identity with bootstrap skipped, and uncertain recovery
without replay. No compile or execution evidence exists for this new adapter
yet. The provider's physical-root/config/approval limits remain intact.

### Bounded creation intake

Modify/reuse: a single creation intake composes the existing Store receipt
lookup/admission and the shared count-owner callback. Neither an external
queue nor a per-key lock table is needed. The finite admission mutex serializes
lookup, count acquisition and durable admission; a separate bounded active
book records only drivers still holding their permit. Existing keys are read
before that gate and rechecked under it, so a duplicate can attach at capacity
without acquiring another count slot. A permit stays with the driver through
settlement, including after its original request disappears.

This first intake is literal cfg(test) staging alongside the phase driver.
Its resource callback may reserve/recheck shared authority but may not launch
effects. A failed/cancelled durable admission drops only that attempt's opaque
resource lease. Completed, in-flight and Unknown receipts are observations;
only Accepted or a known refusal may request continuation. The callback must
still prove authoritative startup recovery, current principal/workspace state,
and absence of an already-restored child/count entry. The intake alone cannot
supply those proofs, spawn a background task, or make a create RPC available.
Real-writer regressions are owed for simultaneous identical last-slot admission,
capacity/conflict lookup, finite waiting, lease rollback and retained identity.

The count lease reuses Agents::reserve's existing locked grant/depth/count
decision instead of adding a counter. A local opaque entry marker fences its
rollback: dropping an old attempt cannot remove a later entry with the same
SessionId. A pending lease rolls back only its own unregistered entry; after
a matching committed Resource Begin it retains the count entry on cancellation
because spawn may have happened. The legacy caller explicitly detaches this
guard and keeps its existing forget/retire cleanup. The new Begin consumer
remains cfg(test) until the real tracked receipt driver is wired. No lease
checks current Store/resource absence, root authority or workspace start
admission on behalf of that missing adapter.

The staged failure settlement accepts only the same operation's committed
Resource/Known refusal and an unchanged unregistered/unused entry. Unknown,
another operation, or an entry already used by an agent cannot release count.
The actual spawner must still prove the no-effect classification before
writing that failure; receipt metadata alone is not a process-state oracle.
Its revision must be exactly the committed Begin's successor, so an earlier
retry's late failure for the same operation cannot release the new attempt.

A focused old-writer fixture retains the closed Store clone across recovery:
it may read Accepted, but its attempted Begin fails Closed and the fake effect
trace stays empty; the new writer can continue the same reserved ids. This
tests pre-Begin writer fencing only. It does not prove that an already-begun
old process/task cannot produce a late effect; tracked shutdown, current
generation checks and the actual runtime adapter still owe that separate proof.

The internal refused outcome carries the canonical creation Certainty value
explicitly. Generic backing refusals currently retain Uncertain reservations
and return Unknown, so a future caller cannot mistake the typed refusal reason
alone for a proven no-effect retry. Narrower Known cases require source proof
and tests before relaxing this conservative development behavior.

The exact5bc four-library run executed 750 passing tests and one failed fixture:
the completed-resource deletion test set SessionInfo.status before Store create,
but Store intentionally starts a live log. The fixture now appends the actual
Exited status record and verifies the stored projection before deleting. The
permanent-key/claim assertions remain unchanged; no lifecycle guard is weakened.

Exact5bc TypeScript generation now executes successfully (three proto tests
and one bridge declaration test). The remote step then refused freshness
because its generated proto.ts differed from the checkout. The authenticated
post-step artifact manifest was verified and that exact generated patch is
now applied; no hand-written wire types or invented feature version is used.
The earlier Snapshot name-clash failure remains preserved separately. The
freshness rerun wp-create-node-wire-20261115T123918Z-d97f3429 passed; generated
declarations now match the checked source. That does not assign a protocol
version or activate the missing creation/launch methods.


### Empty-workspace receipt/backing integration (staged)

Modify/reuse: the existing phase driver now has a concrete empty-workspace
adapter over Workspaces::provision_workspace and the same Store. A prepared
server-resolved intent hashes the typed local/worktree/visibility/source gate
with a domain separator; request data cannot supply an unrelated fingerprint.
Only an empty Workspace/Create reservation with no resource or first input is
accepted. The actual committed Begin precedes provisioning, descriptor readiness
precedes Finished/Completed, and completed retry never enters backing again.
Explicit setup Run is refused by this backing-only adapter rather than being
reported complete after Skip. The separate setup owner still has to implement it.

The entry-check callback is a staged controller port after Begin, not retained
connection/project/root authority across provisioning awaits. The production
controller must still join current authorization/generation, project admission
and canonical lifecycle/start fences before activation. This module, its driver
and provisioning remain literal cfg(test); no new RPC or protocol availability
is claimed. Real Store/project regressions join descriptor publication to receipt
completion and prove a denied entry has no backing reservation, then continues
with the same stable ids. Execution of these newly authored cases is owed.

Announced node gate52 head1c762ccf is merged (metadata registry protocol25, schema
still20). Creation and terminal-profile activation constants remain None. The
lifecycle listing/schema append is pending its own announced integration; no
independent listing type or workspace visibility inference was introduced here.


The exact4bac strict run reached Node and returned thirteen authored lint
findings (explicit Arc cloning and unused atomic return values). They are now
corrected without changing the reservation/count behavior. The newly joined
empty fixture follows the same atomic-result rule. Exact3c TypeScript generation
compiled and ran two passing cases, then refused an unclassified
WorkspaceOperationId alias before generating a patch. That shared UUID is now
classified as a Handle, matching WorkspaceId and the alias policy; the empty
artifact was not applied. Combined generation and corrected strict execution
remain owed on this source.


### Actor/count/start-lease join (staged)

Modify/reuse: Sessions::create_receipted joins the existing tracked Sessions
creation/persistence path, the same Agents reservation, canonical registry start
lease and committed Resource Begin. It binds count retention before any further
await, verifies the stored exact receipt and still-pending lease, refuses an
existing runtime/stored resource, and forces the one reserved SessionId. Only
actual actor registration plus session persistence permits completing the same
registry lease; only then is Resource Finished written. No first input is sent
or accepted: the result is ResourceReady with the original canonical message id.
Errors/cancellation retain admission history and do not prove a no-spawn refusal.

This is a lower registration primitive, literal cfg(test), below the missing
current authorization, profile/placement, workspace visibility/listing and tracked
public controller. Its fake-ACP test uses the actual session runtime, Store and
registry rather than claiming a fake effect list proves actor registration. The
fixture's manually registered workspace does not prove project/root authority.
New test execution and strict checks remain owed; no MCP/native API is activated.

The exact7bd post-join strict run returned two private-module access errors and
six resulting inference diagnostics. Test-only re-exports now expose only the
Provisioned result and empty-adapter entry/types; no product module is widened.
Announced lifecycle93970acb is merged: the shared immutable SessionListing mirror
and tuple ListSessions are canonical, and creation appends now follow the shared
BACKGROUND_SCHEMA. The combined old migration list measured501 product lines;
its coordinator-approved move-only central producer is separate from the two
owned creation registrations. No size allowance is raised or SQL rewritten.


The central sequence move is isolated on announced93970acb asd94165df2,
without creation code or SQL changes. This private development stack merges
that exact producer and then registers the two owned entries in a separate
commit. Their versions derive from BACKGROUND_SCHEMA and the owned predecessor;
integration may repoint that single assignment if another append lands first.
The parent SQL file shrinks to476 lines, and the unchanged historical sequence
prefix is retained before the two additions. No legacy size record existed for
that file, so none is added or raised. The intermediate development merge is
not an adoption candidate without its immediately following registration commit.

The staged bound-session bridge now uses the reviewed canonical SessionListing
and WorkspaceVisibility types from announced93970acb. Before admission, the pure
with_workspace_listing validator derives Shown/Background from the workspace and
refuses a conflicting explicit spec value; it does not inspect cwd or inherit a
legacy source's listing. The actor join repeats the check against the stored
workspace before spawn. Its actual fake-actor fixture now requires the same
Background mirror in both persisted SessionInfo and spec. This proves only the
staged join when executed; the public controller must still call validation
before count/receipt work, and legacy/unbound inheritance remains its own owner.
New source execution is owed; no listing RPC/protocol availability is changed.
