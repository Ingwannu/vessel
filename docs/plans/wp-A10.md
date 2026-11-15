# Named agent launch profiles in the native app

## Status and accepted decision

ADR-first app contract/dependency audit only. No profile DTO, method, service,
editor, picker adapter, preference marker or migration job is implemented here.
The announced package base9 db42d9c5 is merged; its green base gate is not a
check of this branch's combined A9 implementation.

N3 identifies the exact reviewed ADR0076 as d616e8f9b629984cc5b070ace1542e18580803ea,
file docs/adr/0076-named-launch-profiles.md, blob
4e4c7d5e30c95959fc8591d43203e25d26ce100d. V3's contract/document disposition
covers its expanded CRUD, draft/live application and legacy migration design.
Implementation/callability is expressly excluded. This plan references that
immutable owner decision; it does not copy/edit the moving node ADR or invent
another ADR number. The canonical app base does not yet carry that file/types.

The staged814bdd8f/current implementation DTOs are not an approved schema
handoff. In particular, replacement field spelling, canonical account facts,
options fencing and publication ordering await N3's implementation review.
NAMED_LAUNCH_PROFILES remains None. No protocol number or old-call fallback is
assumed and no request may be emitted against those proposals.

## Survey and source contract

Package rows703–705 and734–736 in gaps.md cover host profile CRUD/order,
application to drafts/live agents, and legacy migration. Reference source is
Paseo0.15.4 at1c00b365c9e0b74c1580eba33d905d693125ae21; paths below are under
packages/app/src unless another package is named.

| Part | Reference | Required behavior |
| --- | --- | --- |
| Host list | agent-profiles/settings/agent-profiles-section.tsx:30–231; internal/use-agent-profiles.ts:19–43 | One ordered host list. New appends; edit replaces the record at its stable id, so cleared fields disappear; adjacent reorder; destructive remove confirmation. Unread/disconnected/unsupported are distinct from read empty. |
| Editor | agent-profiles/settings/agent-profile-edit-modal.tsx; internal/profile-form-model.ts; internal/use-profile-form-inputs.ts | Appearance/name/agent/model/mode/thinking/features/notes. Host-wide discovery, not a workspace catalogue. Concrete reported defaults fill unset selections; nonempty retired selections retain their identity/label. Agent change clears dependent selections; model change clears incompatible thinking. Late feature replies must match the complete selection key and editor visit. |
| Picker/application | agent-profiles/internal/use-agent-profile-picker.ts:72–187; internal/materialize-profile.ts and tests | The model picker receives rows and one apply action from the same owner. Draft application copies concrete selections in one transition. Live application retains its actual agent/account, defers unknown modes, omits only a retired saved mode, and leaves absent fields unchanged. |
| Live completion | packages/server/src/server/session/agent-config/agent-config-session.ts:173–198 | Model, mode, thinking, then features; actual acknowledged prefix and first notice/refusal. Earlier applied values remain. No rollback or provider/process replacement is promised. |
| Migration | agent-profiles/migration/index.ts and index.test.ts | One job per host, retry-stable deduplication, existing profiles first, completion only after success. No-favorites needs no catalogue or host mutation. Failure/unknown ACK/catalogue unresolved remain retryable. |
| Public fields | packages/protocol/src/agent-profile.ts:12 | Notes, not a system prompt. Profiles are launch selections, not authentication/account management. |

The complete form-model source confirms that reported catalogues upgrade labels
without discarding stored ids, dependent changes invalidate feature keys, and
closed editors ignore publication. Its tests, materializer tests and migration
tests must become fixtures before their corresponding Rust behavior is built;
this document does not claim those fixtures have executed in Vessel.

## App contract and data flow

Use only the canonical implementation eventually handed off for ADR0076.
The semantic boundary is a stable UUID/name, actual HarnessRef and optional
node-owned ProfileName, ordered typed choices, optional appearance keys and
public notes. Model/mode/thinking/features stay canonical OptionChoice values;
there is no second provider-id mapping, arbitrary feature JSON or credential
field in a native view model. Bounds and constructors come from proto once.

The host list read returns the complete revisioned snapshot. Create/edit/remove/
reorder submit one whole next list against the revision actually read. A stale
revision keeps the person's unsaved edit and requires reconciliation; it never
silently overwrites a newer host list. Pending, denied, offline, unsupported,
corrupt/unreadable and successful empty states stay distinct. An uncertain
write is reread before any retry, not blindly resent.

The selection-dependent options call is a real host-wide N3 probe of that
harness/account/ordered choice selection. It must not borrow Settings' cached
startup defaults or a workspace's catalogue. Its original connection, editor
identity, operation and selection key accompany the answer. A stale response
cannot populate another editor, alter another account or trigger work on the
new selection. Unknown controls are unavailable, not synthesized capabilities.

Draft materialization uses N3's shared typed materializer and B2's original
admitted target/current local claim, then one normal draft transition. Ordinary
structured creation keeps fail-on-unoffered-choice behavior. A profile id is
not a durable live binding; later edits/deletion do not mutate existing sessions.

Live eligibility comes from N3's actual session/account selection, rather than
comparing provider names or choosing a default account in the app. B2 owns the
current target/options/run authority; profile rows and activation share that
owner with the existing model controls. Before application, recheck original
admission and latest offered choice/pending/editability. The new node-owned
bundle call, not a loop over legacy single setters, supplies serialization,
partial-prefix and first-notice semantics. Timeout/disconnect remains uncertain.
No Route::Session-to-Draft fallback may redirect a captured action.

Every queued read, catalogue, mutation acknowledgement and marker installation
retains its exact original canonical ConnectionContext and weak DeliveryGuard.
The actor alone mints that guard; the consumer validates immediately before
installation. NodeId, a book name, Fleet equality, a UI visit or an operation
ticket alone is not a lease. No new connection epoch/controller or retained
raw client substitutes for the pending shared product producer.

## Ownership and reuse decision

[Decision Log]
- Purpose and intent: one host-owned reusable launch bundle, shown and applied
  through the existing Settings and composer owners without changing accounts.
- Existing assets/constraints: the Agents settings section already owns the
  idle-release setting; Providers/Models are A9. Kit owns sections, rows, fields,
  dialog/sheet, picker, buttons and appearance glyphs. View already classifies
  actual SessionOptions as Controls. Node/client own typed profile storage,
  CAS, dependent probing, materialization and serialized live application.
- Alternatives considered: reuse profiles.toml or keep_choices; client-local
  host presets; infer modern ProviderId from HarnessRef/name; loop legacy
  session.set_option; add another selector or connection generation.
- Selected approach: modify/reuse those existing owners. N3 owns the protocol
  and host operation; B2 owns the admitted composer target/getter/setter; the
  existing Settings frame gains its profile section without another navigation
  group; native view/link/area modules consume the canonical data only.
- Why this approach: credentials/account resolution, actual process authority,
  ordered partial application and store acknowledgement already have owners.
  Repeating them would lose the original request/account/lease boundary.
- Advantages, costs and effects: no new crate or external API/dependency; the
  app must wait for real producers rather than publish fabricated options.
  Appearance controls must reuse Kit, with any missing seam coordinated through
  its ownership ledger before code. The shared preference writer remains F5's.

Node CAS/SQLite, strict bounded values, actual harness plus named account,
category-tagged choices and deterministic migration UUIDs are ADR0076's stated
differences from Paseo's daemon-config patch/provider string/arbitrary values.
They are preserved rather than silently weakened in the screen.

## Legacy data and persistence boundary

The checked Vessel preference schema contains startup choices, not favorites.
keep_choices/Choices are not a favorites list, completion marker or account
association. No other application's preference/credential directory is read.
Migration accepts only explicit validated compatibility inputs; unresolved
mapping never guesses an account, drops input or claims completion.

N3 owns the shared bounded planning/CAS helper and actual-node completion
receipt. The app owns one in-flight migration per actual node and its local
completion marker, through F5's existing private writer. Marker installation
requires that receipt and the still-valid original guard. Node aliases must not
become separate accounts or independent completion identities. Marker-write
failure stays retryable; no second preferences file/writer is introduced here.
The exact local schema/recovery seam must be agreed with F5 before its edit.

## Dependency audit and stop boundary

| Work | W22 modern provider snapshot/binding needed? | Required producer before product code |
| --- | --- | --- |
| Read/edit/reorder host profiles | No: ADR0076 selects actual HarnessRef/account directly | Reviewed, assigned canonical N3 proto/client list/CAS/options implementation, plus the checked shared delivery product union for queued reads/ACKs. |
| Editor dependent choices | No provider-id join; N3 owns its selected-options probe | Canonical selection/catalogue schema and actual account/default facts. Current staged DTO additions are not a reviewed handoff. |
| Draft application | No W22 identity inference | N3 typed materializer plus B2 admitted original draft/current local claim and normal creation contract. |
| Live application | No provider/account substitution | N3 eligible-session read/serialized bundle producer, B2 real target/current options/run authority, checked queued-delivery guard. |
| Migration | No static provider table | N3 validated mapping/planner and actual-node receipt, app-owned source records, F5 marker/write seam, checked delivery guard for installation. |

Thus the domain does not require W22's modern snapshot, but every real queued
consumer still depends on the shared delivery producer, and the N3
implementation itself has no assigned/callable handoff. Pure implementation
against canonical DTOs is also unavailable on base9; no shadow types or moving
node carrier are added. Stop at this documentation boundary based on the reviewed node ADR; the app
plan itself is not an implementation approval.
No profile service call, controller, editor, marker or draft/live mutation has
been enabled. A9 and A10 remain unfinished; no whole package gate, captures or
feature approval follows from this document.


## Resume order

Wait for N3's reviewed, assigned canonical implementation/schema handoff and
the checked real queued-delivery producer; do not consume staged814/current DTOs.
Reconcile the combined base9/A9 tree with a narrow check before resuming code.
Coordinate B2 for the actual draft/live target and option owner, and F5 before
any local migration marker schema/write. Then build the pure rules and their
source fixtures, followed by link/area consumers through existing shared controls.
All private fake nodes/display and their captured children are stopped; no
background build/capture job or legacy-data/credential read is left running.
Only own six tracker rows were annotated with this documentation status; all
remain skeleton with consumers/markers unimplemented.
