# Creation operations and terminal profiles

ADR 0091 records reference behavior, authority, alternatives and compatibility.
ADR 0082 owns workspace identity/provisioning; consume its reviewed contract
instead of deriving identity from cwd or persisting another registry.

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
- `vessel-node::creation` and `service/creation`: admission, operation actor,
  revision observation and delegation to workspace/session/terminal services.
- Existing terminal/config services: persist profiles with existing protected
  settings writes; launch using existing process/group/environment handling.
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
byte accounting. Current byte enforcement, corruption checks, saturation,
concurrent last-slot, bounded growth and full-restart tests are staged for
narrow checks and source review. No receipt service, RPC or launch is approved.

Approved shared identitye45a5d89 is merged. Unreviewed registry producer code
was removed; registry ownership/provisioning remains behind the planned typed
boundary until its producer passes review. Source-created change requests
must carry ADR0082's explicit blocked-automation gate before provisioning.

Next: narrow proto/store checks; receipt source review; service orchestration
with typed registry/runtime fake; pending-method registration; native
integration, package gates and reviews.

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
