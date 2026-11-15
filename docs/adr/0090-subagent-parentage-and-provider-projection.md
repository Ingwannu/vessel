# ADR 0090: Subagent parentage, durable detach and provider-child projection

Status: accepted (architecture contract only; implementation and exact provider
projection budgets still require review). Protocol number assigned at integration; the feature uses
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
driver state is implemented. These detailed values are awaiting V3 review;
the earlier architecture PASS did not approve numerical budgets.

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
