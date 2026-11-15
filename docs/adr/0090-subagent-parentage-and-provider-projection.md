# ADR 0090: Subagent parentage, durable detach and provider-child projection

Status: accepted (architecture and finite provider values/budgets; implementation
remains under review). Protocol number assigned at integration; the feature uses
one pending `Option<ProtocolVersion>` constant. No method is registered before
the central Quality B method table reaches this branch.

## Decision Log

- Purpose and intent: reproduce the managed-child track, return-to-parent,
  explicit detach and provider-native child timelines without confusing an
  independently managed session with a provider-owned task.
- Existing implementation and constraints: `Creator::Agent { node, session }`
  records immutable creation provenance and is currently traversed for operate
  ownership. Sessions can run on different nodes. `session.list`, `session.get`
  and `session.mark` exist. Native child events are currently discarded by the
  Claude translator; native Codex notification decoding is its own edge. The
  store already serializes metadata writes and preserves them across resume.
- Alternatives considered: removing Creator, mutating reserved labels, hiding
  a child only in the app, treating a provider tool task as a normal session,
  exposing arbitrary provider JSON, adding another transport or database.
- Selected approach: retain Creator for audit and store a first-class detach
  receipt separately. A single current-parent resolver applies the receipt to
  ownership and UI projections. Provider children have distinct typed ids,
  descriptors, events and timeline pages, projected from actual provider
  observations persisted in the parent log.
- Reason: an audit field must not be rewritten to change present-day delegation;
  an app-local hide cannot revoke delegated operate rights. Provider child
  tasks cannot honestly offer stop, detach, independent archive or a managed
  session id when the provider did not create one.
- Advantages, costs and effects: one metadata migration; no new dependency or
  crate edge. A detached session continues running with the same workspace,
  transcript, native handle and credential grant. New operations by the former
  parent must fail ownership checks. Existing read-scope policy is unchanged.

## Reference read

Paseo 0.15.4:
- `packages/app/src/subagents/select.ts:74-99,136-168`: cross-host managed
  addresses, oldest-first children, separately keyed provider tasks and nesting.
- `subagents/track.tsx:71-125`: status-count pill and ordered child rows.
- `subagents/archive-finished.ts:38-52,122-151,167-199`: recheck eligibility,
  sequential managed archive with progress/retry; provider tasks are locally
  hidden, never deleted from their provider.
- `subagents/use-detach-subagent.ts:18-45`: confirm, detach, navigate on success.
- `packages/server/src/server/session.ts:3436-3455`: detach dispatch;
  `:8104-8218`: provider list and timeline handlers.
- `packages/protocol/src/messages.ts:1971-1995,4882-4934`: provider child RPC
  requests and projected timeline replies.

## Managed child contract

Reuse `SessionRef { node, session }` for a complete parent identity. Add a typed
receipt carrying the former parent and the committed detach time. An attached
child's current parent derives from Creator; a receipt makes it independent.
The caller does not supply the parent, workspace or a replacement Creator.

Planned `session.detach` needs operate, rejects direct and delegated agent
principals, and is forwardable for a person. It returns the current session,
former typed parent and `Detached | AlreadyDetached`. A never-child is an
explicit refusal. A repeat after a lost response returns the original durable
receipt without changing its timestamp. Session/workspace/native-handle/log
identity and live process are preserved; there is no stop, move or replay.

The writer atomically determines never-child / first detach / repeat and commits
before replying. The node finishes publication even when the caller disconnects.
Ownership admission consults current parentage rather than immutable Creator;
ancestor traversal must stop at a detached boundary, including stored sessions
and delegated cross-node callers. Registry grants never grow. Depth limits are
retained conservatively rather than resetting them as a way around the limit.
Metadata publication follows the existing serialized marks/resume mechanism.

The migration appends a nullable receipt column after the current head migration,
with ONE schema constant derived from that head's newest constant. Store append,
resume and metadata writes preserve it. Rollback tests drop this column first.
The receipt is not event-log history and does not replace user messages.

## Provider child contract

A managed parent owns provider descriptors keyed by a validated opaque child id;
optional parent-child ids express nesting within that provider. Descriptors retain
provider title, task description, subtitle, reported lifecycle and timestamps.
The track prioritizes a task description over a repeated provider type title.

Planned list and timeline calls name the managed parent plus typed child/cursor
identities. They reuse existing permission admission and bounded event transport;
no provider JSON crosses the decoding edge. Unsupported providers return typed
unsupported, not an invented empty successful list. Child frames remain out of
the main conversation and project to their own timeline. Parent event sequence
provides stable paging/correlation. No provider task gets a fabricated managed
session or a detach/archive RPC.

Projection bounds and defaults will be pinned against the opened reference and
existing event-page contracts before code. Truncation is explicit; unread gaps,
unknown timestamps and unavailable history remain distinguishable from empty.
The native app receives status and timeline from this contract, and local hide
state remains separate from server/archive state.

## Provider projection bounds and default decision

Paseo's `provider-subagents/store.ts` keeps descriptors in an unbounded map;
Claude `providers/claude/sidechain-tracker.ts:90-166` uses the actual
parent_tool_use_id as the provider task id, separate from a managed session.
The new typed values pin the following proposed bounds before any projector or
driver state is implemented. V3 scoped PASS on55181bb9 approves these numerical values and input DTOs;
the earlier architecture PASS alone did not approve numerical budgets.

| Boundary | Named bound | Reason / behavior at the bound |
| --- | --- | --- |
| Child ids | ProviderChildId::MAX_BYTES=512 | UTF-8 byte/control validation; no invented SessionId |
| Descriptor text field | ProviderText::MAX_BYTES=64KiB | Bounded actual text, no silent truncation |
| Descriptors per managed parent | MAX_CHILDREN=128 | Refuse further retention and publish IngestionBudget |
| Serialized descriptor aggregate | MAX_DESCRIPTOR_BYTES=2MiB | Count actual JSON bytes including escaping before retention |
| Translator message/tool ids across all children of a parent | MAX_TRACKED_IDS=2048 and MAX_TRACKED_ID_BYTES=1MiB | No eviction that could turn an old frame into a new one; explicit ingestion gap |
| Nested provider depth | MAX_CHILD_DEPTH=8 | Cycles/deeper nesting report Nesting, never fabricate a parent |
| Parent records per projection | MAX_SCAN_RECORDS=4096 | Bound CPU even when unrelated main events dominate |
| Parent bytes per projection | MAX_SCAN_BYTES=8MiB | Stop before adding an over-budget record; explicit scan gap |
| Read deadline | READ_DEADLINE=5seconds | Includes durable reads/scan; no unbounded wait |
| Returned child timeline items | ChildLimit=1..=MAX_TIMELINE_ITEMS=200, default200 | Zero is refused; no unlimited read sentinel |

Absent cursor means the retained tail. Before/After cursors name the existing
parent EventSeq; sparse child records intentionally retain the original parent
seq/time. No independent child cursor/epoch is introduced. List/timeline results
must distinguish Complete, Partial(reason, first scanned sequence), Unsupported
and Missing. An empty successful list requires actual provider support evidence;
absence of observations is never proof of support or of no children. Retained
history gaps, scan saturation and decoder budget saturation stay visible.

Descriptor observations are complete snapshots after the driver has applied
provider omit/clear/sticky-status rules. Missing timestamp/status remains unknown;
no wall-clock timestamp or running status is invented. Provider tasks have only
local hide/navigation. Canonical child timeline items reuse AgentEvent and the
central required_protocol/for_protocol seams; provider wrappers themselves are
never recursively accepted as child content. Actual observation registration
and bounded collection wrappers remain implementation work.

This deliberately bounds Paseo's after-cursor default0(all), unbounded retained
maps and scan work. Limits are security/resource boundaries, not silent success
with dropped children. No additional dependency, mutable runtime config or wire
number is introduced.

## Compatibility and verification

The single feature constant starts `None`. New reply fields and events are
withheld for every older peer, direct/list/page/subscription/nested/forwarded
path, until integration assigns a number. Pending calls must be refused by both
client and server before dispatch. Generated TypeScript is regenerated when the
wire is registered. Linux, macOS and Windows share the same metadata and event
logic; this feature has no signals or platform-specific process control.

Tests must cover first/repeat/never-child detach, concurrent repeats, lost reply,
store reopen and resume, no workspace/native id/log changes, former-parent and
ancestor refusal after detach (direct and forwarded), bounded provider decoding,
nested child identity, separate main/child timelines, cursor paging and older-peer
withholding. Fakes are test-only; no installed agent or real sign-in is exercised.

## Resume point

Validated DTOs, the nullable receipt migration, atomic writer, current-parent
admission/publication and a pending detach method are staged in the W2 node tree.
They are not reviewed implementation or consumer-ready; exact narrow and package
checks remain owed. Provider observations/projection, native consumer wiring,
generated wire declarations and final package gates remain outstanding. See
`docs/plans/wp-a4-node.md` for module/data flow and evidence boundaries.

## Registered projection and queue owner

The pending unary Read methods are session.provider_children and session.provider_timeline. Their sole gate is SUBAGENT_RELATIONS from the canonical method table. Provider projections share NodeService.child_reads, a non-queueing semaphore with AT_ONCE=4; excess calls refuse Unavailable before disk work. READ_DEADLINE includes metadata lookup, durable read and reduction. Expiry before any actual record is an Unavailable refusal, never a fabricated Partial.from; expiry after actual progress reports ScanBudget from the first real scanned parent record. Each durable batch retains the store's existing page/byte cap as well as the projection's total budget. Reads snapshot the starting next_seq so concurrent appends cannot extend a scan indefinitely. No independent provider storage, credential access or runtime configuration is added.

Absent observation evidence reports Missing (history unavailable), not a successful empty list or an inferred Unsupported provider. Implementing drivers explicitly report Supported; the canonical wrapper carries descriptors, read-only activity or an honest gap. New reply withholding runs through the existing central Reply::withheld path before added-agent compatibility, including nested fleet replies; method requirements still come only from the canonical table.

### Native Claude observation boundary

The native router consumes typed `task_started`, lifecycle patch/notification and
usage frames, plus actual `parent_tool_use_id` sidechains. A modern announcement's
real `task_id` is the durable provider identity; tool-use ids are bounded aliases.
This deliberately differs from a transient first-tool-id key: the same task can
be reannounced with a different tool id after resume without duplicating a child.
Legacy sidechain-only transports retain their actual tool-use identity and unknown
status. No main tool return fabricates child completion. Explicit foreground
children inherit an observed parent cancellation; unknown/background metadata does
not. Shell/housekeeping announcements never become child sessions.

Each child reuses the existing canonical translator with an isolated streaming and
tool deduplication cache. All routing/cache id copies share the approved finite
count/UTF8 budget. Admission checks happen before retention; saturation emits a
sticky partial observation without eviction/replay. Timestamps stay unknown unless
reported. Resume emits a retained-prefix gap because native replay before this
runtime is unproven. Native Claude remains unselectable until its provider owner
lands the real permissions/transport floor; pure wire fixtures do not prove that
runtime or enable it.

### Bounded rescan and resume metadata

A forward query may scan an older retained prefix to recover provider descriptors.
If its scan budget ends before the caller's cursor, the reply preserves that input
cursor and reports partial coverage, never a backward continuation. A decoded first
record refused for size contributes its real sequence to partial coverage; a
pre-read deadline instead refuses without inventing a sequence.

Durable descriptor projection keeps known optional facts when a resumed driver
omits them. `Some` replaces the earlier fact; an explicit empty text clears text.
This follows provider sticky metadata semantics without a second provider store.
Unknown fields do not erase prior observed lifecycle or timestamps on restart;
provider parentage remains read-only metadata and never managed authorization.

Routing-map budgets count both retained key and value copies, including identical
canonical strings, in addition to translator streaming/tool keys. Pair admission
is transactional before insertion. Descriptor text/identity copies are separately
bounded by the actual serialized descriptor-set budget.

### Native Codex observation boundary

The child router reads actual collaboration receiver thread ids and sub-agent
activity markers in a dedicated typed native envelope; the canonical parent
ThreadItem decoder remains the timeline owner's boundary. Only the managed root
or an already registered child can announce another route. Native sender ids must
match that source. Child turn/item/delta/plan conversion reuses existing canonical
protocol structs and mapper functions with separate deduplication per child.
Parent tool completion is never a child lifecycle proof: reported agentsStates,
activity interruption and child turn lifecycle supply those facts. Missing state
and timestamps remain unknown. Prompt marker keys and routing keys share the same
finite identity budget as Claude; saturation reports partial without eviction.

The current first native version does not issue provider thread/read or retain
unattributed pre-announcement notifications. Resume reports a missing retained
prefix; unknown source threads are ignored rather than adopted into this parent's
history. Historical child replay and final streamed-text suffix repair remain
explicit gaps until the canonical provider/timeline owner supplies those paths.
Native Task/Agent display details remain that owner's canonical SubAgent mapper;
this router does not decode semantic tool input a second time.

A declared native Claude task does not require a tool-use alias to exist. Its real
task id and lifecycle remain visible when that optional alias is absent; the
router reports partial retained history because unlabelled sidechain attribution
cannot be proven. It never invents an alias or replaces the actual task identity.

The client and RPC whole-method-table test fakes explicitly refuse all pending relationship methods, including both provider reads. This completes the generated trait without adding fake provider success or changing existing test expectations; integration checks must compile those consumer test targets as well as node/agent/proto.
